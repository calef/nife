//! **Milestone 37 (prove RedoxFS's crash consistency): the crash test's own service, on its own
//! disk** (DECISIONS §34 (RedoxFS is the primary filesystem, on three conditions), condition 1). A
//! filesystem killed mid-transaction, and the fresh server that mounts what it left behind.
//!
//! Cut out of `fs_service.rs` unchanged by lane `maintainer/last-three-over-2000`, which left
//! the parent an index (§282 (when a file is split, the parent becomes an index)) and under §266
//! (a Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

use super::*;

/// What [`start_crash`] hands back: the two readiness endpoints, and the endpoint the driver
/// reports its acknowledged write on.
///
/// `fs_ready` carries **two** messages in sequence, and that is the design rather than an
/// accident: the FS server's ordinary readiness sentinel when the mount is up, and then
/// `fixture::crash::CUT` from inside the injector, immediately before it traps. The second one
/// is what tells the test the kill was the injector's doing and not something else going wrong,
/// and it is what gives the recovery mount a defined moment to start at instead of a guess.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub struct CrashRun {
    pub blk_ready: RendezvousId,
    pub fs_ready: RendezvousId,
    pub driver_report: RendezvousId,
}

/// The heap budget each of milestone 37's two FS servers draws from. Smaller than
/// [`FS_BUDGET_PAGES`] on purpose: an untyped is a reservation, and three 8 MiB ones do not fit
/// beside everything else this boot builds in 128 MiB. 2 MiB is five times the 352 KiB
/// high-water a real mount under this allocator actually reaches (DECISIONS §27's measurement),
/// and the crash workload is one open and two 66-byte writes.
pub(super) const CRASH_BUDGET_PAGES: u64 = 512;

/// The crash disk's block-service endpoint and shared block page, remembered between the two
/// halves of the test: the recovery server is a **different process** on the **same** block
/// server, which is endpoint-only naming doing its job. The block server never learns that its
/// client died and was replaced, because it never knew who its client was.
static CRASH_BLK_EP: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
static CRASH_BLK_SHARED: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

/// **Phase one: wire a filesystem on its own disk and kill it mid-transaction** (milestone 37).
///
/// Three processes on virtio block device 2, which nothing else in the boot touches. The disk is
/// dedicated on purpose: this test deliberately leaves a filesystem half-written, and pointing it
/// at the shared fixture would couple every later test to whether this one ran first. DECISIONS
/// §27 records what an order-coupled gate costs (three investigations, three incompatible root
/// causes, none of them real), so the fixture is regenerated per run and owned by one test.
///
/// The FS server is spawned **armed**: die one block write into the second `WRITE` request, with
/// that block torn in half. One is the count that cannot miss, because a write transaction always
/// issues at least one block write and a larger count is a server that never dies and a test that
/// hangs.
/// The mmio slot the crash test's dedicated disk arrives on, named once so [`start_crash`] and
/// [`is_crash_disk_present`] cannot drift apart about which device this test owns.
const CRASH_DISK_INDEX: usize = 2;

/// **Is the crash test's own disk attached to this boot?**
///
/// Milestone 214 (design/roadmap/0214-print-and-return-skips.md), on a test that prints
/// "skipping" and returns being counted as passed: the fixture check used to live inside
/// `std_tests::assert_a_kill_mid_transaction_recovers`, which printed "skipping" and returned.
/// A helper cannot skip on a test's behalf, because `skip!()` returns from the function it is
/// written in, so the two callers asked their question one frame too late and were counted as
/// passes. This is the same guard-then-run shape those tests already use for
/// [`NO_FS_SERVER`], and it belongs in this module because the device index does.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub fn is_crash_disk_present() -> bool {
    crate::virtio::find_block_device_n(CRASH_DISK_INDEX).is_some()
}

#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub fn start_crash(
    blk_image: &'static [u8],
    fs_server_image: &'static [u8],
    client_image: &'static [u8],
) -> Option<CrashRun> {
    use core::sync::atomic::Ordering;
    let (blk_ep, blk_ready, blk_shared) = spawn_block_server(
        blk_image,
        crate::virtio::find_block_device_n(CRASH_DISK_INDEX)?,
    );
    CRASH_BLK_EP.store(blk_ep, Ordering::Relaxed);
    CRASH_BLK_SHARED.store(blk_shared, Ordering::Relaxed);

    let file_shared = file_channel();
    let file_ep = crate::sched::create_rendezvous();
    let fs_ready = crate::sched::create_rendezvous();
    spawn_fs_server(
        fs_server_image,
        FsServer {
            slot: 1,
            blk_ep,
            blk_shared,
            file_ep,
            file_shared,
            ready: fs_ready,
            budget_pages: CRASH_BUDGET_PAGES,
            // Die one block write into the SECOND write request, tearing that block at 2048
            // bytes. The first write is acknowledged and must survive; the second is the one the
            // property is about.
            crash: (2, 1, 2048),
            extra_windows: 0, // a single-client crash service maps only window 0 (milestone 599)
        },
    );

    let driver_report = crate::sched::create_rendezvous();
    spawn_fs_client(
        client_image,
        file_ep,
        file_shared,
        3,
        0,
        0,
        0,
        driver_report,
    );
    Some(CrashRun {
        blk_ready,
        fs_ready,
        driver_report,
    })
}

/// **Phase two: mount the disk the dead server left behind** (milestone 37).
///
/// A fresh FS-server process, on the same block server and the same block page, with its own file
/// endpoint, its own file page and its own stack. It carries nothing over from the process that
/// died: what it recovers, it recovers from the platter, through the ordinary `Server::open` every
/// FS server in this system mounts with. Its readiness sentinel arriving at all is the first half
/// of the result, because that open fails outright on an image it cannot make sense of.
///
/// Returns `(ready, report)`.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub fn recover_crash(
    fs_server_image: &'static [u8],
    client_image: &'static [u8],
) -> (RendezvousId, RendezvousId) {
    use core::sync::atomic::Ordering;
    let file_shared = file_channel();
    let file_ep = crate::sched::create_rendezvous();
    let ready = crate::sched::create_rendezvous();
    spawn_fs_server(
        fs_server_image,
        FsServer {
            slot: 2,
            blk_ep: CRASH_BLK_EP.load(Ordering::Relaxed),
            blk_shared: CRASH_BLK_SHARED.load(Ordering::Relaxed),
            file_ep,
            file_shared,
            ready,
            budget_pages: CRASH_BUDGET_PAGES,
            crash: (0, 0, 0), // this one is not armed: it is the one that has to survive
            extra_windows: 0, // single-client crash service (milestone 599)
        },
    );
    let report = crate::sched::create_rendezvous();
    spawn_fs_client(client_image, file_ep, file_shared, 4, 0, 0, 0, report);
    (ready, report)
}
