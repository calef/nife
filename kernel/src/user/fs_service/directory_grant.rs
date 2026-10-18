//! **A per-directory grant** (milestone 47 (navigation and naming)): the root directory capability,
//! a subtree caretaker that narrows it, and the program confined to that subtree.
//!
//! Cut out of `fs_service.rs` unchanged by lane `maintainer/last-three-over-2000`, which left
//! the parent an index (§282 (when a file is split, the parent becomes an index)) and under §266
//! (a Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

use super::*;

/// **Wire a per-directory grant and the program confined to it** (milestone 47,
/// notes/dir-capability.md). The same three-process shape [`start_granted`] wires, one rung up:
///
/// ```text
///   FS server ──file IPC──► fs_subtree_caretaker ──narrowed file IPC──► the confined program
///            (the image root)         (one subtree, one rights set)
/// ```
///
/// The caretaker holds the root directory capability and the confined program holds an endpoint
/// to the caretaker and **nothing that names the FS server**, which is what makes "it cannot
/// reach the parent directory or a sibling" a statement about its capability table rather than about a
/// branch. That argument is `fs_file_caretaker`'s, and it is load-bearing here for an extra
/// reason: the FS server's handle table is per *server*, so a rights-carrying handle on its own
/// would not confine a program that could still name [`filesystem_contract::fs::ROOT`].
///
/// `rights` is a [`filesystem_contract::dir`] mask. It is what the caretaker *asks* for; the FS server
/// intersects it with the root's and refuses if the answer is smaller, so a wiring that asked
/// for more than exists fails at the caretaker's first request rather than silently serving
/// less.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub struct DirGrant {
    /// The directory the caretaker descends into, one component under the image root. Must fit
    /// [`filesystem_contract::grant::MAX_NAME`].
    pub name: &'static str,
    /// The [`filesystem_contract::dir`] rights the subtree capability is to carry.
    pub rights: u64,
    /// The confined program's `arg0` (its role) and `arg1`.
    pub role: u64,
    pub arg: u64,
    /// Its `arg2`. Zero for every client that takes a role and a number; the `rm` program is
    /// started with a **grant's** three words instead (a spec and two of name), so it uses all
    /// three and its "role" word is the spec.
    pub arg2: u64,
    /// Stack pages beyond the one `run` maps, for a confined program that needs them. The
    /// hand-written attacker needs none; a shell does (see [`spawn_fs_client`]).
    pub stack_pages: usize,
}

/// **Wire the filesystem and hand back the root directory capability**, for a boot rather than
/// for a test (milestone 50).
///
/// This is the one entry point the interactive boot uses. It brings up the block server and the
/// FS server, drains both readiness sentinels (so the service is *running* and not merely
/// spawned by the time the progenitor exists), and returns `(the file-service endpoint, the physical frame
/// its clients map)`. `None` means no RedoxFS disk is attached to this run, which is the normal
/// case for a plain `cargo xtask run`, and every caller treats it as "this boot has no
/// filesystem" rather than as an error.
///
/// The endpoint **is** the directory capability (DECISIONS §27), rooted at the image root. A
/// boot hands it to the shell unnarrowed on purpose: it is the machine's own prompt, and the
/// interesting confinement claims are about what the shell then hands to the programs it spawns.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub fn root_directory(
    blk_image: &'static [u8],
    fs_server_image: &'static [u8],
) -> Option<(RendezvousId, u64)> {
    let (file_ep, file_shared, readiness) = ensure(blk_image, fs_server_image)?;
    wait_for_service(readiness);
    Some((file_ep, file_shared))
}

/// **Wire a subtree caretaker and hand back the narrowed capability**, without spawning
/// whatever is going to hold it.
///
/// [`start_granted_dir`]'s first half, split out because milestone 50's redirection witness is
/// a shell that holds a directory **and** a terminal, a spawn channel and a budget, so its
/// client half is nothing like [`spawn_fs_client`]'s. What it shares with every other confined
/// program is exactly what is here: a caretaker between it and the FS server, and the page they
/// both map.
///
/// Returns `(narrow_ep, file_shared)`, with both handshakes already drained for
/// [`wait_for_caretaker`]'s reason, so a caller may spawn its client the moment this returns.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub fn narrow_dir(
    blk_image: &'static [u8],
    fs_server_image: &'static [u8],
    caretaker_image: &'static [u8],
    name: &'static str,
    rights: u64,
) -> Option<(RendezvousId, u64)> {
    let narrow_ep = crate::sched::create_rendezvous();
    let caretaker_ready = crate::sched::create_rendezvous();
    let (file_shared, _caretaker) = spawn_caretaker(
        blk_image,
        fs_server_image,
        caretaker_image,
        name,
        rights,
        narrow_ep,
        caretaker_ready,
    )?;
    Some((narrow_ep, file_shared))
}

/// [`narrow_dir`], **with a caretaker the caller can end** (milestone 121 (`ripgrep` on nife:
/// enumeration as a capability)).
///
/// [`narrow_dir`]'s caretaker parks in `RECEIVE` on an endpoint no region owns, so it lives for the
/// boot. That is fine for a test that grants one directory and was a frame-ledger failure for one
/// that grants two more: the aarch64 suite ran out of page frames three tests later. Here both
/// endpoints come out of a region of their own, `display_service`'s shape and for its reason,
/// and the returned [`super::holding::Holding`] reclaims that region, which wakes the caretaker to
/// die.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub fn narrow_dir_held(
    blk_image: &'static [u8],
    fs_server_image: &'static [u8],
    caretaker_image: &'static [u8],
    name: &'static str,
    rights: u64,
) -> Option<(RendezvousId, u64, RendezvousId, super::holding::Holding)> {
    // Three endpoints, a page each, and one spare: the caretaker's two, and the program's stdout.
    // All three live in the region so that one reclaim returns every endpoint this grant made. The
    // kernel's rendezvous registry is a fixed 512 and the suite runs close to it: two endpoints a
    // walk test left live were enough to exhaust it in `uuid_tests` on 2026-09-27.
    let ep_region = crate::memory_region::create(4).expect("no endpoint region for a caretaker");
    let endpoint =
        || crate::sched::create_rendezvous_from(ep_region).expect("no endpoint for a caretaker");
    let narrow_ep = endpoint();
    let caretaker_ready = endpoint();
    let report = endpoint();
    let (file_shared, caretaker) = spawn_caretaker(
        blk_image,
        fs_server_image,
        caretaker_image,
        name,
        rights,
        narrow_ep,
        caretaker_ready,
    )?;
    let mut held = super::holding::Holding::new();
    held.add_thread(caretaker);
    held.add_region(ep_region);
    Some((narrow_ep, file_shared, report, held))
}

/// The caretaker both of the above start, serving `narrow_ep` and reporting once on
/// `caretaker_ready`. Returns the file channel and the caretaker's thread.
#[allow(clippy::too_many_arguments)]
fn spawn_caretaker(
    blk_image: &'static [u8],
    fs_server_image: &'static [u8],
    caretaker_image: &'static [u8],
    name: &'static str,
    rights: u64,
    narrow_ep: RendezvousId,
    caretaker_ready: RendezvousId,
) -> Option<(u64, crate::thread::ThreadId)> {
    assert!(
        filesystem_contract::grant::fits(name.as_bytes()),
        "a granted name rides in two argument words; this one does not fit",
    );
    let (file_ep, file_shared, readiness) = ensure(blk_image, fs_server_image)?;

    let (lo, hi) = filesystem_contract::grant::pack_name(name.as_bytes());
    let spec = filesystem_contract::grant::spec(name.len(), rights);

    let tid = crate::sched::spawn(move || {
        run(
            caretaker_image,
            Spawn {
                arg0: lo,
                arg1: hi,
                arg2: spec,
                grants: &[
                    rendezvous_cap(file_ep, Rights::WRITE), // slot 0: CALL the FS server
                    rendezvous_cap(narrow_ep, Rights::READ), // slot 1: serve the narrowed capability
                    rendezvous_cap(caretaker_ready, Rights::WRITE), // slot 2: readiness, once
                ],
                maps: &[Mapping {
                    va: FILE_VA_CLIENT,
                    phys: file_shared,
                    flags: Flags::user_data(),
                }],
            },
        )
    })
    .expect("could not spawn the subtree caretaker");

    // Both handshakes before the client exists. See [`wait_for_caretaker`]: this is the call
    // that found the bug, because a caretaker whose descent is answered only after the service
    // is drained gives its client an unbounded window to write over the staged name.
    wait_for_service(readiness);
    wait_for_caretaker(caretaker_ready);
    Some((file_shared, tid))
}

#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub fn start_granted_dir(
    blk_image: &'static [u8],
    fs_server_image: &'static [u8],
    caretaker_image: &'static [u8],
    client_image: &'static [u8],
    grant: DirGrant,
) -> Option<RendezvousId> {
    let DirGrant {
        name,
        rights,
        role: client_role,
        arg: client_arg,
        arg2: client_arg2,
        stack_pages,
    } = grant;
    let (narrow_ep, file_shared) =
        narrow_dir(blk_image, fs_server_image, caretaker_image, name, rights)?;

    // The confined program. Its slot 0 looks exactly like a directory capability from inside,
    // and is one: the same protocol, the same page, a namespace of one subtree.
    Some(spawn_fs_client(
        client_image,
        narrow_ep,
        file_shared,
        client_role,
        client_arg,
        client_arg2,
        stack_pages,
    ))
}
