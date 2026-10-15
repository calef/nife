//! **Spawning a client of the service**, and the two waits that order a client after the
//! servers and caretakers it depends on.
//!
//! Cut out of `fs_service.rs` unchanged by lane `maintainer/last-three-over-2000`, which left
//! the parent an index (§282 (when a file is split, the parent becomes an index)) and under §266
//! (a Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

use super::*;

/// The most extra stack pages [`spawn_fs_client`] will map below the one `run` gives every
/// program. Small on purpose: a client that needs more than this is a client whose frames want
/// looking at, not a number that wants raising.
pub(super) const CLIENT_EXTRA_STACK: usize = 8;

/// Spawn one client holding exactly a file-service endpoint, a report endpoint and its view of
/// the shared page. Returns the report endpoint.
///
/// `extra_stack` is pages **below** the single one `run` maps. The hand-written `fs_test_client` roles
/// need none: they hold a handle and a small buffer. The navigating shell (milestone 47's
/// commands) needs some, and the number is a measurement rather than a guess: with none it
/// overflowed by 192 bytes, which presented as a data abort on its own `sp` and then as the
/// 60-second lost-wakeup watchdog, because the kernel test was still waiting for a report from a
/// process that had died. A shell carries a path stack (eight levels of name), a parsed path,
/// and a listing buffer, all by value, so 4 KiB is genuinely tight for it. Same discipline as
/// the FS server's stack (DECISIONS §27): sized by what it did, not by what looks generous.
///
/// **Milestone 50 moved it again, the same way.** `Endowment` grew a sink and a source, each of
/// which can carry a whole `FileGrant`, and the shell now holds one endowment per pipeline
/// stage; four extra pages overflowed by 48 bytes at the same `sp` and with the same symptom.
/// Recorded because it will happen a third time: the shell's stack is sized by the largest value
/// `grant_plan` hands it, so a field added there is a page needed here.
/// `pub(super)` (rather than private, its shape before milestone 152) so a sibling test module can
/// spawn a client with a nonzero `extra_stack` directly, bypassing [`start`]'s convenience wrapper
/// (which hardcodes `0`): milestone 152's schedule-store roles needed more than the one-page
/// default, found short under `script/test`'s own aarch64 run (a data abort at the stack's guard
/// page). Those roles went with `session_reviver` on 2026-09-27; the granted-directory spawners
/// below still pass their callers' stack pages through it.
pub fn spawn_fs_client(
    client_image: &'static [u8],
    file_ep: RendezvousId,
    file_shared: u64,
    role: u64,
    arg: u64,
    arg2: u64,
    extra_stack: usize,
) -> RendezvousId {
    assert!(
        extra_stack <= CLIENT_EXTRA_STACK,
        "an FS client asked for more stack than this wiring maps",
    );
    let report = crate::sched::create_rendezvous();
    crate::sched::spawn(move || {
        let mut maps = [Mapping {
            va: 0,
            phys: 0,
            flags: Flags::user_data(),
        }; FILE_PAGES + CLIENT_EXTRA_STACK];
        // The whole channel, because this is the spawn the throughput benchmark comes through and
        // a client may not ask for more than it mapped (`filesystem_protocol::fs::TRANSFER_PAGES`). It costs
        // fifteen extra page-table entries against the same frames, not fifteen extra frames.
        let n = map_channel(&mut maps, FILE_VA_CLIENT, file_shared, FILE_PAGES);
        for (k, m) in maps[n..n + extra_stack].iter_mut().enumerate() {
            m.va = USER_STACK_VA - (k as u64 + 1) * FRAME_SIZE;
            m.phys = page_frame();
        }
        run(
            client_image,
            Spawn {
                arg0: role,
                arg1: arg,
                // A third word, because the `rm` program is started the way a **grant** is
                // (`filesystem_protocol::grant`): a spec word and two words of name. Every other client
                // here takes a role and one number and leaves this zero.
                arg2,
                grants: &[
                    rendezvous_cap(file_ep, Rights::WRITE), // slot 0: CALL the FS server
                    rendezvous_cap(report, Rights::WRITE),  // slot 1: report to the kernel
                ],
                maps: &maps[..n + extra_stack],
            },
        )
    })
    .expect("could not spawn the FS client");
    report
}

/// Wire the service (or reuse this boot's) and spawn the hand-written client
/// (`fixtures/src/fs_test_client/main.rs`): the file-service endpoint, which IS its directory capability, the
/// report endpoint, and its view of the shared file page. It names nothing else in the system.
///
/// Returns `(readiness, report)`: the two readiness endpoints if this call wired the service,
/// and the endpoint the client reports on.
pub fn start(
    blk_image: &'static [u8],
    fs_server_image: &'static [u8],
    client_image: &'static [u8],
    client_role: u64,
) -> Option<(Readiness, RendezvousId)> {
    let (file_ep, file_shared, readiness) = ensure(blk_image, fs_server_image)?;
    // 0 = the end-to-end proof; 1 = the fs_read benchmark loop.
    let report = spawn_fs_client(client_image, file_ep, file_shared, client_role, 0, 0, 0);
    Some((readiness, report))
}

/// **Drain the FS service's two readiness sentinels**, which is a sequencing act and not only an
/// assertion.
///
/// Both servers announce with a blocking `SEND`, so each is parked inside its own announcement
/// until somebody receives it. Nothing they serve can be answered before that. Every caller that
/// needs the service to be *running* rather than merely spawned has to come through here first,
/// and [`wait_for_caretaker`] documents what that ordering is load-bearing for.
///
/// `None` means an earlier caller in this boot already wired the service and drained these.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub fn wait_for_service(readiness: Readiness) {
    let Some((blk_ready, fs_ready)) = readiness else {
        return;
    };
    // `None` means the block service was already running when this one was wired, so its sentinel
    // has been drained by whoever started it and there is nothing here to wait for.
    if let Some(blk_ready) = blk_ready {
        assert_eq!(
            crate::sched::ipc_receive(blk_ready)[0],
            filesystem_protocol::READY,
            "the block server did not bring the RedoxFS device up",
        );
    }
    assert_eq!(
        crate::sched::ipc_receive(fs_ready)[0],
        filesystem_protocol::READY,
        "the FS server did not open the RedoxFS image",
    );
}

/// **Wait for a caretaker's startup request to have been answered, before the program it
/// confines exists at all.**
///
/// This is a correction, and the bug it fixes is worth stating where it was made rather than
/// only in the note. All three processes share **one frame** (the module comment above argues
/// that is sound because every request on both hops is a blocking `CALL`, so the client is
/// parked inside its own call for the whole time the caretaker is using the page). That
/// argument holds once the caretaker is serving. It does not hold at **startup**, where the
/// caretaker stages the granted name in the page and then blocks in a `CALL` to the FS server:
/// a confined program that already exists writes its own first name over that page, and the FS
/// server resolves whatever it finds there.
///
/// It is not even a race in the common case. When this call is the one that wires the service,
/// the FS server is parked inside its readiness `SEND`, so the caretaker's descent cannot be
/// answered until [`wait_for_service`] drains it, and the client has that whole window to
/// clobber the page. The caretaker then dies (it will not serve a hole) and its client blocks
/// forever on a call nobody will answer, which is how this presented: a userspace `ebreak`
/// followed by the 60 s lost-wakeup watchdog. It passed on aarch64 and failed on riscv, which
/// is the shape of a timing bug and was one.
///
/// The fix is ordering, not a second page: drain the service, wait for the caretaker's own
/// sentinel, and only then spawn the confined program.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub(super) fn wait_for_caretaker(caretaker_ready: RendezvousId) {
    assert_eq!(
        crate::sched::ipc_receive(caretaker_ready)[0],
        filesystem_protocol::READY,
        "the caretaker could not open what it was granted, so there is nothing to attenuate",
    );
}
