//! **A per-file grant** (milestone 31 (a capability shell) phase 2): a caretaker narrows the
//! directory to one file, and the confined program holds only the caretaker.
//!
//! Cut out of `fs_service.rs` unchanged by lane `maintainer/last-three-over-2000`, which left
//! the parent an index (§282 (when a file is split, the parent becomes an index)) and under §266
//! (a Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

use super::*;

/// **Wire a per-file grant and the program that holds it** (milestone 31 phase 2,
/// notes/grant-expression.md). This is what `run wc report.txt` resolves to, wired by the
/// kernel's test suite instead of by the shell, so the mechanism is gated on both ISAs.
///
/// Three processes, and the shape is the point:
///
/// ```text
///   FS server ──file IPC──► fs_file_caretaker ──narrowed file IPC──► the confined program
///                (a directory)          (one file, one direction)
/// ```
///
/// The caretaker holds the directory capability. The program holds an endpoint to the caretaker
/// and **nothing that names the FS server**, so "it cannot reach a second file" is a statement
/// about its capability table, not about a check it is trusted to pass. The narrowing is an address
/// space, which is why the attacker test below is a witness rather than an assertion.
///
/// One frame is shared by all three. Every request on both hops is a blocking `CALL`, so the
/// client is parked inside its own call for the whole time the caretaker is using the page; a
/// second frame would buy a copy and no isolation, since the client is entitled to the bytes
/// either way.
///
/// The grant, as one value, because its four fields are one decision: which file, in which
/// direction, handed to which program started how. Splitting them across a long argument list
/// invites a caller to get `rights` and `role` the wrong way round, and both are bare integers.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub struct Grant {
    /// The one name the caretaker will answer for. Must fit [`filesystem_contract::grant::MAX_NAME`].
    pub name: &'static str,
    /// `grant::READ`, or `READ | WRITE`.
    pub rights: u64,
    /// The confined program's `arg0` (its role) and `arg1`.
    pub role: u64,
    pub arg: u64,
}

#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub fn start_granted(
    blk_image: &'static [u8],
    fs_server_image: &'static [u8],
    caretaker_image: &'static [u8],
    client_image: &'static [u8],
    grant: Grant,
) -> Option<(RendezvousId, super::holding::Holding)> {
    let Grant {
        name,
        rights,
        role: client_role,
        arg: client_arg,
    } = grant;
    assert!(
        filesystem_contract::grant::fits(name.as_bytes()),
        "a granted name rides in two argument words; this one does not fit",
    );
    let (file_ep, file_shared, readiness) = ensure(blk_image, fs_server_image)?;
    // The caretaker's two endpoints and the program's report, from one region the returned
    // holding reclaims, as [`narrow_dir_held`] does for a directory.
    let ep_region =
        crate::memory_region::create(GRANT_ENDPOINT_PAGES).expect("no endpoint region for a grant");
    let endpoint =
        || crate::sched::create_rendezvous_from(ep_region).expect("no endpoint for a grant");
    let narrow_ep = endpoint();
    let caretaker_ready = endpoint();
    let report = endpoint();

    let (lo, hi) = filesystem_contract::grant::pack_name(name.as_bytes());
    let spec = filesystem_contract::grant::spec(name.len(), rights);

    // The caretaker: the directory capability, the narrowed endpoint it serves, and the shared
    // page. The grant itself (the name and the direction) rides in its START arguments, so a
    // per-file grant costs no frame at all.
    let caretaker = crate::sched::spawn(move || {
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
    .expect("could not spawn the file caretaker");

    // Both handshakes before the client exists, in this order and for [`wait_for_caretaker`]'s
    // reason: the servers are parked inside their announcements until they are drained, and the
    // caretaker's own request is staged in the page all three share.
    wait_for_service(readiness);
    wait_for_caretaker(caretaker_ready);

    // The confined program. Its slot 0 looks exactly like a directory capability from inside, and
    // is not one: same protocol, same page, a namespace of one name.
    let mut held = super::holding::Holding::new();
    held.add_thread(caretaker);
    held.add_region(ep_region);
    held.add_thread(spawn_fs_client(
        client_image,
        narrow_ep,
        file_shared,
        client_role,
        client_arg,
        0,
        0,
        report,
    ));
    Some((report, held))
}
