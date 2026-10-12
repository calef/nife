//! **Somebody else's threaded C, unmodified, on nife's C library** (milestone 836 (a C library,
//! stage 2: threads); `design/fatal-risks/1-only-software-written-for-nife.md`).
//!
//! hackbench (rt-tests 2.11, `helpers/build-hackbench.sh`) in its thread mode: groups of sender
//! and receiver `pthread`s pass 100-byte messages over pipes or `socketpair`s, every receiver
//! blocking in `read` until a sender's `write` wakes it, after every thread has said it is ready
//! through one pipe and been started through another with `poll`. The source is not touched; what
//! differs from a Linux build is on the command line (the helper says what and why).
//!
//! The program runs as a process (`std_threads_tests::run_as_process_with`): its threads are
//! those of milestone 812 (`std::thread::spawn` runs real threads in one address space), joined to
//! one process, so the process's end is checked to reap every one. hackbench times itself with
//! `gettimeofday`, so it is given a clock as `c_program_tests` gives one.
//!
//! **Each test skips when hackbench is not in the archive**, on `c_program_tests`' terms: fetching
//! its source in a gate is §46 (thin primitives or whole subsystems)'s decision. All three ISAs,
//! nothing architecture-specific asserted (§19 (architectural parity is a tenet)).
//!
//! The group sizes keep every run inside a process's 16 threads (`std`'s limit, milestone 812's
//! `BUGS`): hackbench's own default, 10 groups of 40, is 400.

use super::*;

/// hackbench's heap, in pages: 4 MiB. Each thread's stack comes out of it (`std` gives one 64 KiB,
/// what hackbench asks for), so the 256 pages an ordinary `std` program is given hold fewer
/// threads than a process may have.
const HEAP_PAGES: u64 = 1024;

/// **Run hackbench told `line`, with a clock, and return what it printed.** Its process must end
/// with every thread reaped, and the clock service comes back.
fn run_hackbench(line: &str, out: &mut [u8]) -> usize {
    let image = program("hackbench").expect("checked by the caller");
    let region =
        crate::memory_region::create(clock_service::REGION_PAGES).expect("no region for a clock");
    let (clock, clock_thread) =
        clock_service::start_in(program("clock").expect("no clock in the archive"), region);
    let mut clock_held = crate::user::holding::Holding::new();
    clock_held.add_thread(clock_thread);
    clock_held.add_region(region);
    // Its startup verdict first, so the offset is published before the program reads the page.
    let _ = crate::sched::ipc_receive(clock.report);
    // The argument page: `line` as the shell would send it, read-only at `ARGS_PAGE`.
    let args_region = crate::memory_region::create(1).expect("no argument page region");
    let args = crate::memory_region::retype_page(args_region).expect("no argument page");
    {
        // SAFETY: a frame just retyped for this page alone, direct-mapped, which nothing else
        // holds until the program is given it read-only.
        let page = unsafe {
            &mut *(crate::arch::mmu::phys_to_virt(args) as *mut [u8; FRAME_SIZE as usize])
        };
        page.fill(0);
        grant_plan::argv(line.as_bytes(), page).expect("the test's line is not an argv");
    }
    let clock_page = clock.page_phys;
    let len = super::std_threads_tests::run_as_process_with(
        image,
        "hackbench",
        out,
        &super::std_threads_tests::Setup {
            maps: &[
                (std_runtime_protocol::ARGS_PAGE, args, false),
                (std_runtime_protocol::CLOCK_PAGE, clock_page, false),
            ],
            read_pages: &[
                (std_runtime_protocol::ARGS_SLOT, args),
                (std_runtime_protocol::CLOCK_SLOT, clock_page),
            ],
            heap_pages: HEAP_PAGES,
            ..Default::default()
        },
    )
    .len;
    assert!(
        super::wait_for(|| crate::sched::reclaim_region(args_region).is_ok()),
        "hackbench's argument page did not come back"
    );
    assert!(
        clock_held.release(),
        "hackbench's clock service outlived its holding: a service this test cannot give back",
    );
    len
}

/// `args` formatted into `buf`, as a `&str`; the suite has no allocator to `format!` with.
fn line<'a>(buf: &'a mut [u8; 160], args: core::fmt::Arguments) -> &'a str {
    use core::fmt::Write;
    struct Buf<'b>(&'b mut [u8], usize);
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
    let mut b = Buf(buf, 0);
    b.write_fmt(args).expect("the expected line fits");
    let len = b.1;
    core::str::from_utf8(&buf[..len]).expect("formatted from str")
}

/// What every complete thread-mode run prints: its plan, the same words Linux's prints, and a time,
/// which is QEMU's and is not asserted.
fn assert_hackbench_complete(text: &str, groups: u32, fds: u32, loops: u32) {
    let mut buf = [0u8; 160];
    let plan = line(
        &mut buf,
        format_args!(
            "Running in threaded mode with {groups} groups using {} file descriptors each \
             (== {} tasks)",
            2 * fds,
            groups * fds * 2,
        ),
    );
    assert!(
        text.lines().any(|l| l == plan),
        "hackbench did not plan the run it was told (want `{plan}`): {text}",
    );
    let mut buf = [0u8; 160];
    let each = line(
        &mut buf,
        format_args!("Each sender will pass {loops} messages of 100 bytes"),
    );
    assert!(
        text.lines().any(|l| l == each),
        "hackbench did not plan {loops} messages a sender: {text}",
    );
    // Printed only after every thread was joined, so every message was received.
    assert!(
        text.lines().any(|l| l.starts_with("Time: ")),
        "hackbench never printed its time, so it did not join every thread: {text}",
    );
}

macro_rules! skip_without_hackbench {
    () => {
        if program("hackbench").is_none() {
            crate::testing::skip!(
                "no hackbench in this archive: build it with helpers/build-hackbench.sh, which \
                 fetches its source (milestone 836)"
            );
        }
        if clock_service::machine_has_no_rtc() {
            crate::testing::skip!(clock_service::NO_RTC);
        }
    };
}

/// **hackbench's thread mode over pipes**: two groups of three senders and three receivers, so
/// twelve threads and the main one, 100 messages from each sender to each receiver of its group.
/// `pthread_create` with a stack size, blocking `read` and `write` on `pipe`s from many threads at
/// once, `poll` on the start pipe, and `pthread_join` of every thread.
///
/// Falsification: attested 2026-10-12. On patagonia, aarch64 under HVF, with the platform layer's
/// pipe `read` waiting inside the descriptor table's lock (as stage 1's reads hold it): the first
/// receiver to wait held the table, no sender could reach its pipe, and this test ran into the
/// suite's 90-second watchdog. A replay needs hackbench in the archive, which no gate builds.
#[test_case]
fn unmodified_hackbench_threads_over_pipes() {
    skip_without_hackbench!();
    let mut got = [0u8; 1024];
    let len = run_hackbench("hackbench -T -p -g 2 -f 3 -l 100", &mut got);
    let text = core::str::from_utf8(&got[..len]).unwrap_or("<not utf-8>");
    crate::println!("    hackbench -T -p printed:\n{text}");
    assert_hackbench_complete(text, 2, 3, 100);
}

/// **hackbench's default transport, `socketpair(AF_UNIX, SOCK_STREAM)`, in thread mode**: one group
/// of seven senders and seven receivers, fifteen threads with the main one, one short of a
/// process's limit.
///
/// Falsification: attested 2026-10-12. On patagonia, aarch64 under HVF, with `socketpair`
/// answering `ENOSYS`: hackbench printed its plan, then `Creating fdpair (error: Function not
/// implemented)`, and this test failed for want of its time line. A replay needs hackbench in the
/// archive, which no gate builds.
#[test_case]
fn unmodified_hackbench_threads_over_socket_pairs() {
    skip_without_hackbench!();
    let mut got = [0u8; 1024];
    let len = run_hackbench("hackbench -T -g 1 -f 7 -l 100", &mut got);
    let text = core::str::from_utf8(&got[..len]).unwrap_or("<not utf-8>");
    crate::println!("    hackbench -T printed:\n{text}");
    assert_hackbench_complete(text, 1, 7, 100);
}
