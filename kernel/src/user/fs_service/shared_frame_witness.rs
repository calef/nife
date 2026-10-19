//! **The shared-frame witness** (milestone 599 (a frame per filesystem client channel)): two live
//! clients on one file service, each on its own window, and the negative control that puts them on
//! one.
//!
//! Cut out of `fs_service.rs` unchanged by lane `maintainer/last-three-over-2000`, which left
//! the parent an index (§282 (when a file is split, the parent becomes an index)) and under §266
//! (a Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

use super::*;

/// **Wire two live clients on one file service, for the shared-frame witness** (milestone 599 (a
/// frame per filesystem client channel), provisional).
///
/// This is the wiring finding 1 of `notes/shared-page-audit.md` describes: two live clients on one
/// file service, one rewriting the frame the other staged its name in. **After milestone 599 each
/// client claims its own pool window** ([`claim_window`]) and a badged endpoint naming it, so the
/// two map different frames and the file server reads each request from its caller's own window. The
/// attacker's write to its own window can no longer reach the victim's, which is the isolation the
/// witness now asserts. A **sync endpoint** sequences them deterministically so the test does not
/// depend on a race landing.
///
/// Both `fs_test_client` roles (`ROLE_SHARE_VICTIM`, `ROLE_SHARE_ATTACKER`) read one slot layout:
/// FILE at 0 (a badged view of the FS-service endpoint), REPORT at 1, SYNC at 2, and each maps its
/// own window's frame at `FILE_VA_CLIENT`. The victim calls the server; the attacker only writes its
/// frame and hands off, because holding a frame the service maps is the whole of what made a second
/// writer dangerous before the fix.
///
/// `shared` puts both clients on one window, the pre-599 wiring, as the witness's negative control.
///
/// Returns `(readiness, victim_report, held)`: the service's readiness sentinels if this call wired
/// it, the endpoint the victim reports its verdict on, and the holding that hands back both clients
/// and the three endpoints (milestone 608). The attacker never reports; it signals the victim and
/// exits. `None` when no RedoxFS disk is attached, or when the pool has no free window
/// (which cannot happen with two clients against [`CLIENT_WINDOWS`] of at least three).
///
/// Provisional name (this lane's coinage); an architect names functions.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub fn start_shared_frame_witness(
    blk_image: &'static [u8],
    fs_server_image: &'static [u8],
    client_image: &'static [u8],
    victim_role: u64,
    attacker_role: u64,
    shared: bool,
) -> Option<(Readiness, RendezvousId, super::holding::Holding)> {
    let (file_ep, _file_shared, readiness) = ensure(blk_image, fs_server_image)?;
    let ep_region = crate::memory_region::create(3).expect("no endpoint region for the witness");
    let endpoint =
        || crate::sched::create_rendezvous_from(ep_region).expect("no endpoint for the witness");
    let sync = endpoint();
    let victim_report = endpoint();
    let attacker_report = endpoint();
    let mut held = super::holding::Holding::new();
    held.add_region(ep_region);

    // The service must be serving before either client calls: the FS server is parked in its
    // readiness SEND until this drains it, so a victim that called first would block forever. The
    // test binds `_readiness` and never drains it again (milestone 599).
    wait_for_service(readiness);
    // A window per client: its own frame, and a badge equal to its window index, so the server
    // reads each request from the caller's own frame.
    //
    // **`shared` is the negative control**: the attacker is put on the victim's own window, which is
    // the wiring every client had before milestone 599. The witness must then report the
    // substitution, and that is what makes its isolated run evidence rather than a test that would
    // pass whatever the wiring did.
    let (victim_win, victim_phys) = claim_window()?;
    let (attacker_win, attacker_phys) = if shared {
        (victim_win, victim_phys)
    } else {
        claim_window()?
    };

    // Spawn a witness client. Slot layout FILE=0 (the FS endpoint), REPORT=1, SYNC=2 (`READ|WRITE`
    // so a role can both `SEND` and `RECEIVE`). It maps its own window's frame at `FILE_VA_CLIENT`.
    //
    // Two shapes of the FILE endpoint, so the test covers both (milestone 599): a client that
    // **mints its own badge** gets an unbadged endpoint with `GRANT` and its window index in `arg1`,
    // and calls `abi::rendezvous::BADGE` itself; one that is **handed a pre-badged endpoint** gets a
    // badged view directly, the shape the progenitor builds for a confined child.
    let spawn_witness = move |role: u64, report: RendezvousId, win: u64, phys: u64, mint: bool| {
        crate::sched::spawn(move || {
            let mut maps = [Mapping {
                va: 0,
                phys: 0,
                flags: Flags::user_data(),
            }; FILE_PAGES];
            let n = map_channel(&mut maps, FILE_VA_CLIENT, phys, FILE_PAGES);
            let file = if mint {
                rendezvous_cap(file_ep, Rights::WRITE.union(Rights::GRANT))
            } else {
                rendezvous_cap_badged(file_ep, Rights::WRITE, win)
            };
            run(
                client_image,
                Spawn {
                    arg0: role,
                    arg1: if mint { win } else { 0 }, // the window a minting client badges for
                    arg2: 0,
                    grants: &[
                        file,                                                    // slot 0: the FS endpoint
                        rendezvous_cap(report, Rights::WRITE), // slot 1: report to the kernel
                        rendezvous_cap(sync, Rights::READ.union(Rights::WRITE)), // slot 2: handshake
                    ],
                    maps: &maps[..n],
                },
            )
        })
        .expect("could not spawn a shared-frame witness client")
    };

    // The victim first, so it is parked in its first `SEND` on the sync endpoint before the attacker
    // runs; the attacker's first act is a `RECEIVE` on the same endpoint, so the order they start in
    // does not change the handshake, but starting the victim first keeps the ordering obvious. The
    // victim mints its own badge (exercising the mint syscall); the attacker is handed a pre-badged
    // endpoint (the progenitor's shape). The attacker never calls the server, so its badge is
    // immaterial; what matters is that it maps a different window and cannot reach the victim's.
    held.add_thread(spawn_witness(
        victim_role,
        victim_report,
        victim_win,
        victim_phys,
        true,
    ));
    held.add_thread(spawn_witness(
        attacker_role,
        attacker_report,
        attacker_win,
        attacker_phys,
        false,
    ));

    Some((readiness, victim_report, held))
}
