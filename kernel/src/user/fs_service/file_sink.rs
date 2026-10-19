//! **A file behind a byte sink, and a file behind a byte source** (milestone 50 (pipes and
//! redirection)).
//!
//! Cut out of `fs_service.rs` unchanged by lane `maintainer/last-three-over-2000`, which left
//! the parent an index (§282 (when a file is split, the parent becomes an index)) and under §266
//! (a Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

use super::*;

/// **Put a file behind a byte sink** (milestone 50, notes/sink-protocol.md).
///
/// Wires the FS service (or reuses this boot's) and spawns `fixtures/src/file_sink.rs`: it holds
/// the FS-service endpoint, a report endpoint, and the page it shares with the FS server, and it
/// serves one endpoint whose only expressible request is "append these bytes".
///
/// `sink` is the capability a program's output slot gets, and **that endpoint is the whole of
/// what the writer holds**, which is the property the milestone rests on: it is created here
/// and handed out with `WRITE` to whoever is redirected and `READ` to the sink.
///
/// Both endpoints are retyped from `endpoints`, a region the caller owns and reclaims once the
/// sink has reported `DONE` and exited ([`FILE_SINK_PAGES`] of it).
/// The pages [`start_file_sink`] or [`start_file_source`] retypes from its caller's region: two
/// endpoints each.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const FILE_SINK_PAGES: u64 = 2;

#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub struct FileSink {
    /// The FS service's two readiness endpoints, if this call is the one that wired it.
    pub readiness: Readiness,
    /// The byte sink. Goes into a program's output slot with `WRITE`.
    pub sink: RendezvousId,
    /// Readiness first, then `DONE` and the byte total at end of stream.
    pub report: RendezvousId,
}

#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub fn start_file_sink(
    blk_image: &'static [u8],
    fs_server_image: &'static [u8],
    file_sink_image: &'static [u8],
    endpoints: u64,
) -> Option<FileSink> {
    let (file_ep, file_shared, readiness) = ensure(blk_image, fs_server_image)?;
    let sink = crate::sched::create_rendezvous_from(endpoints).expect("no file sink endpoint");
    let report = crate::sched::create_rendezvous_from(endpoints).expect("no file sink report");
    crate::sched::spawn(move || {
        run(
            file_sink_image,
            Spawn {
                arg0: 0,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(sink, Rights::READ), // slot 0: the byte sink it serves
                    rendezvous_cap(file_ep, Rights::WRITE), // slot 1: CALL the FS server
                    rendezvous_cap(report, Rights::WRITE), // slot 2: readiness and the total
                ],
                maps: &[Mapping {
                    va: FILE_VA_CLIENT,
                    phys: file_shared,
                    flags: Flags::user_data(),
                }],
            },
        )
    })
    .expect("could not spawn the file sink");
    Some(FileSink {
        readiness,
        sink,
        report,
    })
}

/// **Read back what the file sink wrote**, in a different process with a different FS session.
///
/// Spawns `fixtures/src/file_source.rs`, and only after the sink has reported that it closed the
/// file, because the two share the FS server's one file page (the [`wait_for_caretaker`] lesson:
/// one page is sound between parties that are never using it at once, and sequencing is what makes
/// that true).
///
/// It streams the file's contents out **over the sink contract**, so the bytes that reach the
/// test arrive in the same sixteen-byte framing a `println!` does. Returns `(out, report)`, both
/// retyped from `endpoints` as [`start_file_sink`]'s are.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub fn start_file_source(
    blk_image: &'static [u8],
    fs_server_image: &'static [u8],
    file_source_image: &'static [u8],
    endpoints: u64,
) -> Option<(RendezvousId, RendezvousId)> {
    let (file_ep, file_shared, _) = ensure(blk_image, fs_server_image)?;
    let out = crate::sched::create_rendezvous_from(endpoints).expect("no file source endpoint");
    let report = crate::sched::create_rendezvous_from(endpoints).expect("no file source report");
    crate::sched::spawn(move || {
        run(
            file_source_image,
            Spawn {
                arg0: 0,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(out, Rights::WRITE), // slot 0: where the file's bytes go
                    rendezvous_cap(file_ep, Rights::WRITE), // slot 1: CALL the FS server
                    rendezvous_cap(report, Rights::WRITE), // slot 2: the size it found
                ],
                maps: &[Mapping {
                    va: FILE_VA_CLIENT,
                    phys: file_shared,
                    flags: Flags::user_data(),
                }],
            },
        )
    })
    .expect("could not spawn the file source");
    Some((out, report))
}
