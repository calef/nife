//! `std_threads_tests`: **milestone 812 (`std::thread::spawn` runs real threads in one address
//! space)'s exit test**, on all three architectures (§19 (architectural parity is a tenet)).
//!
//! The `std_threads` program is ordinary `std` Rust: four threads, an `AtomicU64` and a
//! `Mutex<u64>` a million increments each, a `thread_local!` per thread, and
//! `available_parallelism`. It runs here the way a loader that gives a program threads builds it:
//! a process created around its space, its first thread joined to that process, and the runtime's
//! fixed slots filled in the process's shared table, the three new ones included (its own process
//! with `BIND`, its own first thread, its own space). Then its transcript is the verdict.
//!
//! The block's second check, `rayon`'s parallel sum, is `std_rayon`, run the same way.

use super::*;
use crate::cap::Rights;
use crate::sched;

/// The program's ELF bytes, if this archive carries it (it is built by `cargo xtask std-exerciser`,
/// which `script/test` runs first).
fn std_threads_image() -> Option<&'static [u8]> {
    program("std_threads")
}

/// **Run a `std` program as a process, the way a loader that gives it threads builds it**, and hand
/// back its whole stdout. Its space is loaded from `image`, a process is created around it, its
/// first thread joins that process, and the runtime's fixed slots go in the process's shared table,
/// the three milestone 812 added included: its own process with `BIND`, its own first thread and its
/// own space. Then the program runs to its end, and the test checks that its process ended with
/// every thread reaped and its space gone, and gives every region back.
fn run_as_process(image: &'static [u8], what: &str, out: &mut [u8]) -> usize {
    run_as_process_with(
        image,
        what,
        None,
        None,
        super::std_service::BUDGET_PAGES,
        out,
    )
}

/// [`run_as_process`], **told `line` and given a wall clock** when the caller has them: the words
/// on an argument page at `ARGS_SLOT` and `ARGS_PAGE`, made by `grant_plan::argv` as the shell
/// makes them, and a clock service's page read-only at `CLOCK_SLOT` and `CLOCK_PAGE`, as
/// `fs_service::start_std_narrowed_clocked` grants them. Added by milestone 836 (a C library, stage
/// 2: threads) for a threaded C program, which needs all three: threads, its words, and the clock it
/// times itself with. `budget_pages` is its heap, which every thread's stack comes out of.
pub(super) fn run_as_process_with(
    image: &'static [u8],
    what: &str,
    line: Option<&str>,
    clock_page: Option<u64>,
    budget_pages: u64,
    out: &mut [u8],
) -> usize {
    let (mut space, entry) = load(image, 0).expect("could not load the program");
    // The argument page, from a region this test gives back.
    let args_region = crate::memory_region::create(1).expect("no argument region");
    let args = line.map(|line| {
        let phys = crate::memory_region::retype_page(args_region).expect("no argument page");
        // SAFETY: a frame just retyped and zeroed, reached through the direct map, which nothing
        // else holds until the program is given it read-only below.
        let page = unsafe {
            &mut *(crate::arch::mmu::phys_to_virt(phys) as *mut [u8; FRAME_SIZE as usize])
        };
        grant_plan::argv(line.as_bytes(), page).expect("the test's own line is not an argv");
        space
            .map_physical(
                std_runtime_protocol::ARGS_PAGE,
                phys,
                Flags::user_rodata(),
                crate::revoke::PageMapSource::NoCapability,
            )
            .expect("could not map the argument page");
        phys
    });
    if let Some(phys) = clock_page {
        space
            .map_physical(
                std_runtime_protocol::CLOCK_PAGE,
                phys,
                Flags::user_rodata(),
                crate::revoke::PageMapSource::NoCapability,
            )
            .expect("could not map the clock page");
    }

    // The deep stack a std program's main thread needs, from a region this test gives back.
    let stack_region =
        crate::memory_region::create(std_runtime_protocol::STACK_PAGES).expect("no stack region");
    for k in 1..std_runtime_protocol::STACK_PAGES {
        let frame = crate::memory_region::retype_page(stack_region).expect("no stack frame");
        space
            .map_physical(
                USER_STACK_VA - k * FRAME_SIZE,
                frame,
                Flags::user_data(),
                crate::revoke::PageMapSource::NoCapability,
            )
            .expect("could not map the program's stack");
    }
    let name = readopt_user_address_space(space).expect("register the program's space");

    let process_region = crate::memory_region::create(4).expect("no process region");
    let pid = sched::create_process_from(process_region, name).expect("no process");
    let tid = sched::create_thread_control_block(process_region).expect("no tcb");
    let page = sched::configure_thread_control_block_joining(tid, entry, USER_STACK_TOP, pid, 0)
        .expect("the program's first thread could not join its process");
    assert_eq!(
        page,
        current_cpu_protocol::PAGE_VA,
        "a process's first member reads the page every thread read before"
    );

    let report_region = crate::memory_region::create(1).expect("no report region");
    let report = sched::create_rendezvous_from(report_region).expect("no report rendezvous");
    let budget =
        crate::memory_region::create(budget_pages).expect("no heap budget for the program");
    for (slot, cap) in [
        (
            std_runtime_protocol::MEMORY_REGION_SLOT,
            crate::cap::memory_region_cap(budget),
        ),
        (
            std_runtime_protocol::STDOUT_SLOT,
            crate::cap::rendezvous_cap(report, Rights::WRITE),
        ),
        (
            std_runtime_protocol::PROCESS_SLOT,
            crate::cap::process_capability(pid, Rights::BIND),
        ),
        (
            std_runtime_protocol::THREAD_SLOT,
            crate::cap::thread_control_block_cap(tid, Rights::WRITE),
        ),
        (
            std_runtime_protocol::SPACE_SLOT,
            crate::cap::address_space_cap(name, Rights::READ),
        ),
    ] {
        sched::thread_control_block_insert_cap(tid, cap, Some(slot))
            .expect("could not fill a runtime slot in the process's table");
    }
    for (slot, phys) in [
        (std_runtime_protocol::ARGS_SLOT, args),
        (std_runtime_protocol::CLOCK_SLOT, clock_page),
    ] {
        if let Some(phys) = phys {
            sched::thread_control_block_insert_cap(
                tid,
                crate::cap::page_frame_cap(phys, Rights::READ),
                Some(slot),
            )
            .expect("could not fill a page slot in the process's table");
        }
    }
    sched::start_thread_control_block(tid, [0; 3]).expect("start the program");

    let len = drain_until_end(report, pid, out, what);
    assert!(
        super::wait_for(|| !sched::is_thread_present(tid)),
        "{what}'s main thread never left"
    );
    assert!(
        super::wait_for(|| sched::process_state(pid).is_some_and(|(m, ended)| m == 0 && ended)),
        "{what}'s process did not end with all its threads reaped",
    );
    // The last member's reaper takes the space out of the registry after it has left the
    // process, once it has let go of the scheduler's lock, so this waits as well.
    assert!(
        super::wait_for(|| user_address_space_root(name).is_none()),
        "{what}'s space outlived its process"
    );
    // The same reaper removes the last thread from the table after that, and a region still
    // holding a thread refuses `reclaim` until it is gone, so each region gets the same grace.
    for region in [
        budget,
        report_region,
        process_region,
        stack_region,
        args_region,
    ] {
        assert!(
            super::wait_for(|| sched::reclaim_region(region).is_ok()),
            "a region {what} used did not come back"
        );
    }
    len
}

/// `std_tests::drain_sink`, except that a process that ends without its end-of-output fails the
/// test rather than leaving it blocked in `RECEIVE` forever. A thread's fault ends its whole
/// process, and a dead program sends no end-of-output.
fn drain_until_end(
    report: sched::RendezvousId,
    pid: sched::ProcessId,
    out: &mut [u8],
    what: &str,
) -> usize {
    // Generous: `std_threads` takes about five emulated minutes under TCG. A program that neither
    // writes nor ends for this long is deadlocked, and a red test says so where a hang would not.
    let deadline = crate::arch::timer::now() + 1800 * crate::arch::timer::frequency();
    let mut len = 0usize;
    loop {
        while sched::rendezvous_waiting_senders(report) == 0 {
            assert!(
                crate::arch::timer::now() < deadline,
                "{what} neither wrote nor ended for half an hour, after: {}",
                core::str::from_utf8(&out[..len]).unwrap_or("<not utf-8>")
            );
            if sched::process_state(pid).is_none_or(|(_, ended)| ended) {
                panic!(
                    "{what} ended before its output did, after: {}",
                    core::str::from_utf8(&out[..len]).unwrap_or("<not utf-8>")
                );
            }
            sched::yield_now();
        }
        let words = sched::ipc_receive(report);
        let mut chunk = [0u8; byte_sink_protocol::INLINE_MAX];
        match byte_sink_protocol::unpack(words[0], words[1], words[2], &mut chunk) {
            byte_sink_protocol::Msg::Bytes(n) => {
                assert!(
                    len + n <= out.len(),
                    "{what}: wrote more than the buffer holds"
                );
                out[len..len + n].copy_from_slice(&chunk[..n]);
                len += n;
            }
            byte_sink_protocol::Msg::Eof => return len,
            byte_sink_protocol::Msg::Malformed => panic!("{what}: a sink message out of contract"),
        }
    }
}

/// **Four `std` threads share an atomic, a mutex and an address space, and each keeps its own
/// thread-local**, which is the exit test of milestone 812's block.
///
/// The transcript says: both counters read four million, every thread read back its own
/// thread-local and the main thread's is untouched, the shared state was there after the first
/// thread exited, and `available_parallelism` is the online count (§269 (how threads share a
/// process) fork 7). After the program ends, its process has ended with every member reaped.
///
/// Falsification: replayable `system_tests/falsifications/user.std_threads_tests.four_std_threads_share_a_mutex_an_atomic_and_a_space.patch`
#[test_case]
fn four_std_threads_share_a_mutex_an_atomic_and_a_space() {
    let Some(image) = std_threads_image() else {
        crate::testing::skip!(super::std_service::NO_STD_EXERCISER);
    };
    let mut got = [0u8; 512];
    let len = run_as_process(image, "std_threads", &mut got);
    let online = crate::smp::online_harts_mask().count_ones();
    let mut want = [0u8; 512];
    let want_len = {
        use core::fmt::Write;
        struct Buf<'a>(&'a mut [u8], usize);
        impl Write for Buf<'_> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let end = self.1 + s.len();
                self.0
                    .get_mut(self.1..end)
                    .ok_or(core::fmt::Error)?
                    .copy_from_slice(s.as_bytes());
                self.1 = end;
                Ok(())
            }
        }
        let mut b = Buf(&mut want, 0);
        write!(
            b,
            "threads 4\natomic 4000000\nmutex 4000000\nthread-locals each its own\n\
             main thread-local 0\nspace outlived the first exit true\nparallelism {online}\n"
        )
        .expect("the expected transcript fits");
        b.1
    };
    assert_eq!(
        core::str::from_utf8(&got[..len]).unwrap_or("<not utf-8>"),
        core::str::from_utf8(&want[..want_len]).unwrap_or("<not utf-8>"),
        "std_threads's transcript",
    );
}

/// **Unmodified `rayon` runs its parallel sum on every core**: the exit test's second check
/// (milestone 812's block: "A second check runs `rayon`'s parallel sum to the same answer").
///
/// `std_rayon`'s global pool must be one worker per online core, its parallel count over four
/// million items must be four million, and `rayon::broadcast` must reach every worker, which it
/// can only do if each of them is a thread that really runs.
///
/// Falsification: replayable `system_tests/falsifications/user.std_threads_tests.rayons_parallel_sum_runs_on_every_online_core.patch`
#[test_case]
fn rayons_parallel_sum_runs_on_every_online_core() {
    let Some(image) = program("std_rayon") else {
        crate::testing::skip!(super::std_service::NO_STD_EXERCISER);
    };
    let mut got = [0u8; 256];
    let len = run_as_process(image, "std_rayon", &mut got);
    let text = core::str::from_utf8(&got[..len]).unwrap_or("<not utf-8>");
    let online = u64::from(crate::smp::online_harts_mask().count_ones());
    let line = |key: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(key))
            .and_then(|n| n.parse::<u64>().ok())
    };
    assert_eq!(line("rayon threads "), Some(online), "rayon's pool: {text}");
    assert_eq!(
        line("rayon sum "),
        Some(4_000_000),
        "rayon's parallel sum: {text}"
    );
    assert_eq!(
        line("rayon broadcast reached "),
        Some(online),
        "rayon's broadcast did not reach a worker on every core: {text}"
    );
}

/// **The `std` heap's one lock, measured under four threads** (the block's item 12: "measure it under
/// four threads before replacing it"). `std_heap_contention` does the same allocations with one
/// thread and then four and prints both times; this prints them to the log and checks only that both
/// arrived. Under QEMU the numbers are a ratio, not a speed. `notes/std/threads.md` and
/// `notes/std/allocation.md` record them.
///
/// It also checks that a thread's end gives its allocation cache back to the heap (milestone 561 (a
/// per-CPU allocator is what the current-CPU page was for)): the program then runs 64 threads in
/// turn, each ending with a full cache, twice the program's heap budget between them.
///
/// Falsification: replayable `system_tests/falsifications/user.std_threads_tests.the_std_heap_is_measured_under_four_threads.patch`
#[test_case]
fn the_std_heap_is_measured_under_four_threads() {
    let Some(image) = program("std_heap_contention") else {
        crate::testing::skip!(super::std_service::NO_STD_EXERCISER);
    };
    let mut got = [0u8; 1024];
    let len = run_as_process(image, "std_heap_contention", &mut got);
    let text = core::str::from_utf8(&got[..len]).unwrap_or("<not utf-8>");
    for line in text.lines() {
        crate::println!("    (std heap under threads: {line})");
    }
    assert!(
        text.lines()
            .any(|l| l == "threads ended with full caches 64"),
        "threads that ended kept their allocation caches from the heap: {text}",
    );
    for line in ["one thread us ", "four threads us "] {
        assert!(
            text.lines().any(|l| l
                .strip_prefix(line)
                .is_some_and(|n| n.parse::<u64>().is_ok())),
            "std_heap_contention printed no `{line}` figure: {text}",
        );
    }
}
