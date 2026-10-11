//! A round-robin scheduler, and the preemption that makes it mean something.
//!
//! # The whole point of the project, arriving
//!
//! DECISIONS §5, written before a line of kernel existed:
//!
//! > A userspace process is an arbitrary ELF binary. It has its own stack, it never yields, and
//! > it will loop forever because we will write a bug. Under cooperative scheduling, one bad
//! > user program hangs the machine permanently.
//!
//! This file is where that stops being true. The timer fires, the handler calls [`schedule`],
//! and the CPU is **taken away** from a thread that never asked to give it up.
//!
//! There is a test named `a_thread_that_never_yields_is_preempted_anyway`. It spawns a thread
//! whose entire body is `loop { count += 1 }`: no yields, no syscalls, not even a function
//! call. Under any cooperative scheduler that is a hung machine. Here it is a Tuesday.
//!
//! # Three rules, and each of them is a bug if you get it wrong
//!
//! **1. Release the run-queue lock BEFORE switching.** Switch away while holding it and the
//!    lock is now held by a thread that is not running. The next thread to want it spins
//!    forever waiting for a thread that will never be scheduled, because scheduling requires
//!    the lock. A deadlock of a shape that would take a day to find.
//!
//! **2. Interrupts stay masked across the switch.** Between "I decided to switch" and "I
//!    switched" there must be no window for a timer interrupt to decide *again*. And the mask
//!    is per-thread, because each thread's `schedule()` frame lives on its own stack, which is
//!    exactly what makes this work at all.
//!
//! **3. A brand-new thread must unmask interrupts itself.** Every *resumed* thread gets its
//!    interrupt state back from `eret` restoring `SPSR_EL1`. A thread that has never run has no
//!    `SPSR` to restore. `thread_trampoline` does `msr daifclr, #2` for exactly this reason,
//!    and without it the first thread you spawn can never be preempted, which would be a
//!    cooperative scheduler with extra steps.

use core::sync::atomic::{AtomicPtr, AtomicU32, AtomicU64, Ordering};

use intrusive_fifo::Unqueued;
use thread_wake_handshake::{SwitchOutVerdict, WakeVerdict};

use crate::cpu;
use crate::sync::{IrqSafeMutex, rank};
use crate::thread::{
    CapabilityTableLock, Context, QuotaToken, State, Thread, ThreadId, Wait, WaitRole, switch_to,
};

/// How many times we have actually taken the CPU away from a thread. The number that says
/// preemption is real.
static PREEMPTIONS: AtomicU64 = AtomicU64::new(0);

/// The same count, **per core**, which is the question the global one cannot answer.
///
/// **Its own array rather than a `cpu::PerCpu` field, and that is a measurement rather than a
/// preference.** `PerCpu` is exactly 128 bytes, so `PERCPU[id]` indexes with a shift; one more
/// `AtomicU64` field took it to 136 and cost 150 bytes on the riscv64 `ipc_send_receive` closure,
/// over `script/fastpath-footprint`'s bound. See the assertion beside `cpu::PERCPU`. Nothing on
/// the IPC fastpath reads this, so it has no business sharing a cache line budget with what does.
///
/// Written only by the owning core, in [`count_preemption`], which runs on the timer preemption
/// path and not on any IPC path. Read by anyone.
static PREEMPTIONS_PER_CPU: [AtomicU64; cpu::MAX_CPUS] =
    [const { AtomicU64::new(0) }; cpu::MAX_CPUS];

/// The thread running on **this core** right now.
///
/// Per-CPU as of §11 step 3b (`cpu::PerCpu::current`); it used to be one field on the global
/// `IpcTables`. Reading it is a plain atomic load and needs no lock: it is this core's own slot.
fn current_thread_id() -> ThreadId {
    cpu::current().current.load(Ordering::Relaxed)
}

// **The seams cut out of this file**, each a child of `sched` so it reaches `IPC_TABLES` and the
// thread table exactly as code here does. Milestone 812 (`std::thread::spawn` runs real threads in
// one address space) cut the first three because the file was at its §266 (a Rust source file stays
// under 2,000 lines) ceiling: configuring an embryo, the futex and the process object. Milestone 840
// (the scheduler file is split along its seams) cut the rest along the lines §271 (the scheduler
// file splits into submodules of `sched`) rules. Each of those takes `use super::*` and is
// re-exported whole with `pub use <child>::*`, because it is part of this module split across
// files, not a client of it: every name resolves where it did before the cut, and every path a
// caller outside `sched` names is unchanged.
mod capabilities;
mod configure;
mod futex;
mod inspection;
mod interrupt_route;
mod notification;
mod placement;
mod process;
mod region_reaping;
mod rendezvous;
mod thread_control_block;
mod thread_table;
mod timer;

pub use capabilities::*;
#[cfg(target_arch = "riscv64")]
pub use configure::current_thread_pointer;
#[cfg(any(test, feature = "system_tests", feature = "cycle_counter_grant"))]
#[cfg_attr(not(any(test, feature = "system_tests")), allow(unused_imports))] // as the function
pub use configure::grant_cycle_counter;
#[cfg(feature = "system_tests")]
pub use configure::grant_cycle_counter_to_current;
pub use configure::{
    configure_thread_control_block, configure_thread_control_block_joining,
    configure_thread_control_block_with_thread_pointer, set_thread_pointer,
};
#[cfg_attr(not(feature = "system_tests"), allow(unused_imports))]
pub use futex::{futex_queued, futex_waiters};
pub use futex::{futex_wait, futex_wake, is_current_space};
pub use inspection::*;
pub use interrupt_route::*;
pub use notification::*;
pub use placement::*;
pub use process::{ProcessId, create_process_from, destroy as destroy_process, with_holders};
#[cfg(feature = "system_tests")]
pub use process::{process_state, reap_corpses_now, strand_member};
pub use region_reaping::*;
pub use rendezvous::*;
pub use thread_control_block::*;
pub use thread_table::*;
pub use timer::*;

/// **The running thread's capability table, per core** (provisional name, 2026-10-04 UTC): what
/// [`current_cap`] reads instead of taking `IPC_TABLES` to find the thread.
///
/// Null before this core runs a thread. Written only by [`set_current`], on the owning core, in the
/// same breath as `PerCpu::current`, so the two cannot disagree; read only by the owning core, with
/// interrupts masked.
///
/// **Its own array rather than a `cpu::PerCpu` field**, for [`PREEMPTIONS_PER_CPU`]'s measured
/// reason: `PerCpu` is exactly 128 bytes and one more word makes it 136. **Each entry on a line of
/// its own**, because every core writes its entry on every switch and reads it on every syscall, and
/// a shared line would trade the lock this removes for line traffic between the same cores (the
/// `PerCpu` straddle in notes/job-mix/null-syscall-under-load.md's BUGS is that shape).
///
/// **Why the pointer stays valid without a lock.** It names the page of the thread running on this
/// core, and a running thread's page is not recycled: `Threads::remove` is reached only from
/// [`reap_switched_out`], for a thread its successor has switched off, and from
/// [`reap_region_objects`], whose `region_reap_verdict` refuses a `Ready` or `Running` thread and
/// one any core is still standing on. `schedule` replaces the pointer before the switch that makes
/// the old thread reapable. A thread can no more outlive this pointer than it can outlive its own
/// kernel stack, and for the same reason.
#[repr(align(64))]
struct CurrentCapabilities(AtomicPtr<CapabilityTableLock>);

static CURRENT_CAPABILITIES: [CurrentCapabilities; cpu::MAX_CPUS] =
    [const { CurrentCapabilities(AtomicPtr::new(core::ptr::null_mut())) }; cpu::MAX_CPUS];

/// Make `tid` this core's running thread: its name in `PerCpu::current` and its capability table in
/// [`CURRENT_CAPABILITIES`]. The only writer of either, so a lookup never finds one thread's name
/// beside another's table. Caller holds `IPC_TABLES`.
///
/// # Safety
///
/// `tcb` is `tid`'s page pointer as the thread table stores it (`Threads::pointer`), live.
unsafe fn set_current(tid: ThreadId, tcb: *mut Thread) {
    cpu::current().current.store(tid, Ordering::Relaxed);
    // SAFETY: the caller's contract; an address computation, nothing is read.
    let table = unsafe { crate::thread::capability_table_of(tcb) };
    if let Some(entry) = CURRENT_CAPABILITIES.get(cpu::id()) {
        entry.0.store(table.cast_mut(), Ordering::Relaxed);
    }
}

/// **The running thread's capability table, locked**, with `IPC_TABLES` not held. `None` before
/// this core runs a thread. See [`CURRENT_CAPABILITIES`] for why the pointer is valid, and
/// [`crate::sync::lock_found`] for why the entry is read with interrupts masked.
#[inline]
fn current_capabilities() -> Option<crate::sync::IrqSafeGuard<'static, crate::cap::CapabilityTable>>
{
    crate::sync::lock_found(|| {
        let table = CURRENT_CAPABILITIES
            .get(cpu::id())?
            .0
            .load(Ordering::Relaxed);
        // SAFETY: non-null means the table of the thread running on this core, which is the
        // caller, and a running thread's page is not recycled (see `CURRENT_CAPABILITIES`). The
        // `'static` is a lie told only as long as the guard, which the caller drops before it can
        // stop being the running thread (no caller blocks or switches holding it).
        unsafe { table.as_ref() }
    })
}

/// A synchronous IPC rendezvous point: the two wait queues and the pending-signal count.
///
/// **The state machine is the `inter_process_communication` crate**, which owns the queues and the
/// decision logic (send, receive, signal) and carries machine-checked proofs of its one invariant, "at
/// most one wait queue is ever non-empty" (DECISIONS §14, milestone 18; notes/verification.md). The
/// six IPC functions below decide *what* to do by calling the proved logic and spend their own code
/// only on the bookkeeping the queues cannot express (mailboxes, waking a thread onto a run queue,
/// the one-shot Reply that leaves a caller blocked).
///
/// Intrusive as of milestone 14 phase A.3: a wait-queue entry is the TCB itself, threaded through
/// the same link the run queues use, so blocking on an rendezvous cannot allocate and "a thread waits
/// on one rendezvous at a time" is physical (one link). The safety contract for the pointers is the
/// queue discipline at [`thread_control_block_ptr`].
type Rendezvous = inter_process_communication::Rendezvous<Thread>;

/// The most threads that can be alive at once, whole machine (milestone 14 phase A). A documented
/// limit of the image rather than a heap that can be exhausted: spawn past it fails cleanly, the
/// same contract callers already have for out-of-memory.
///
/// # The ledger, and why 256
///
/// **128 until 2026-08-27, and the suite had been sitting on the ceiling for some time without
/// anybody being able to see it.** Milestone 169's leak-fix lane found it the hard way: with its
/// `raw_mode`/`rmle` leak fixed, the aarch64 run reached further than any run before it and
/// stopped dead at `time_tests::a_shell_with_no_usable_clock_times_the_command_anyway`, with
/// **128 of 128 live and 121-123 `Blocked`**, twice, at the same point both times. Not a leak:
/// `thread_leak_police` (no runnable spinner left over) passed both runs. What is alive is the
/// accumulated cost of this tree's many individually-reasonable services that are
/// **intentionally permanent for the boot** (`notes/frames.md`'s "held" list: the FS servers,
/// two credential store instances, `login`, both `net_stack` transports, mDNS, `gpu_driver`,
/// `compositor`, and more since), each accepted on its own merits over many milestones and never
/// once priced against this shared ceiling collectively.
///
/// **What the measurement then showed, and it is worse than the report that prompted it.**
/// [`PEAK_THREADS`] and `kernel::testing`'s closing `threads:` line were built for this raise, so
/// the number is read rather than guessed. On `main`, at 128, the aarch64 suite reports a peak of
/// **exactly 128 with zero spare and still passes**: it is not that the ceiling is about to bite,
/// it is that the ceiling is already refusing spawns and the refusals were being swallowed. Raise
/// the ceiling and nothing else, and the same suite says what it actually wanted:
///
/// | architecture | peak live threads | with the ceiling at 256 |
/// |---|---|---|
/// | aarch64 | **130** | 126 spare |
/// | riscv64 | **129** | 127 spare |
/// | `x86_64` | **57** | 199 spare (its userspace suite is smaller: 192 run, 56 skipped) |
///
/// Those are the figures on the tree this was measured on; they move by a thread or two as the
/// boot's service list changes (the merge that landed while this branch was open took aarch64 to
/// 129), which is exactly why the number is printed by every run rather than only written here.
///
/// So the tightest real demand is 130, and 256 is **1.97x it**. Headroom rather than a fitted
/// number, deliberately, because every milestone that adds a boot service spends some of this and
/// the failure mode is an unrelated test refusing a spawn far from the cause. What that headroom
/// costs is measured below rather than asserted, which is the only reason it can be called cheap.
///
/// # What it costs, per slot, measured
///
/// Nothing here scales with the ceiling *except* through these, and each was checked at 256:
///
/// - **The boot stack, 20 bytes a slot.** [`init`] installs the tables, and an unoptimised build
///   carries the thread table on its frame. That frame was 43,952 bytes at 128 (the deepest in the
///   kernel) and the suite's boot-stack high-water was 54,336 of 65,504, 82%. Installing
///   [`EMPTY_TABLES`] instead of building a `Threads` local and moving it in cut the frame to
///   15,696 at 128 and made the slope 20 bytes rather than ~80; at 256 the high-water is **48,760
///   (74%)**, lower than before the raise. `stack::report_high_water`'s gate is 61,440.
/// - **`kmem::KERNEL_OBJ_PAGES`, 7 pages a *live* thread** (`thread::STACK_PAGES` = 6, plus the
///   TCB page), which is why that carve went 1024 -> 2048. Spent per live thread, not per slot,
///   so unused headroom costs only that constant's own `[u64; KERNEL_OBJ_PAGES]` free stack.
/// - **`ps::MAX_ROWS`**, which `kernel::user::survey_tests` const-asserts is `>= MAX_THREADS`,
///   because a `ps` holding the widest grant must have room for every row. It sizes stack-resident
///   `[Row; MAX_ROWS]` arrays in `ps`, `watch` and `pgrep`; measured with `-Z emit-stack-sizes` at
///   256, `_start` is 4,240 bytes in `ps`, 4,320 in `pgrep` and 8,464 in `watch` (which holds
///   two), against the 12 pages (49,152 bytes) `system_initializer::CHILD_STACK_PAGES` gives every
///   child. 17% at the worst.
/// - **`revoke::MAX_SPACES`** and **`thread::FreeAddressSpace`**, both of which are now written as
///   arithmetic on this constant rather than as a literal that has to be remembered. The second
///   was a bare `[u64; 128]` with a `debug_assert` for a comment; it could have drifted silently.
/// - **Nothing on the `spawn_el0` benchmark, but only after a second fix.** The first attempt at
///   this raise cost that benchmark **+348,133 icount ticks (+16.8%)** and failed `script/bench
///   --check`, through two scans of equal size whose cost tracked this constant rather than what
///   the machine held: `delete_page_frame_caps_where`'s walk of every thread's capability table
///   (141,359) and `revoke`'s registry, whose `MAX_SPACES` is derived from this one (138,584).
///   Both are bounded by live occupancy now (`generational_table`'s `top`, `revoke::Registry`'s),
///   which took the benchmark to **1,212,888, 41% below the 128-slot baseline**, and, the part
///   that matters here, made it **flat against this constant**: doubling again to 512 moves it 587
///   ticks. Re-baselining instead was available and refused, because the cost belonged to slots
///   nothing occupied and would have been paid again by every future raise. notes/benchmarks.md
///   has the attribution table.
/// - **Two `[u64; MAX_THREADS]` scratch arrays in [`reap_region_objects`]**, which took that frame
///   to 4,624 bytes at 256, over `script/stack-frame-check`'s 4,096-byte guard-page ceiling. Both
///   are gone: the function's own comment already prescribed rescanning rather than collecting,
///   for exactly this reason, and these two were the sites it had not been applied to.
///
/// # BUGS
///
/// - **`design/decisions/0096-process-kernel-or-event-kernel.md` prices the confinement claim off
///   the old number** ("`MAX_THREADS` is 128, so kernel stacks total 3.00 MiB, static"). The shape
///   of that argument survives (the bound is still static and still the product of two constants)
///   but the figure does not, and a decision record is not a lane's to edit. Whoever ratifies this
///   raise owes §96 a corrected sentence.
/// - **Revocation still scans every live thread, and `MILESTONE 183` is the fix.** Bounding the
///   sweep by occupancy makes it independent of *this* constant; it does not make it independent
///   of how many threads are actually alive. A boot with real tenancy pays the walk in full. That
///   milestone ("a physical-range index for capability holders, so revocation stops scanning every
///   thread") is the structural answer, and the numbers above are evidence for it rather than
///   against it.
/// - **The peak is a whole-boot high-water, not a per-test charge.** It cannot say *which* test
///   pushed the table up, only that something did, and for this table that is the honest shape:
///   what fills it is what earlier tests deliberately left running. See
///   `kernel::testing`'s `report_thread_peak`, which explains why it reports and does not gate.
pub const MAX_THREADS: usize = 256;

struct IpcTables {
    /// The thread table: generational names over page-resident TCBs. See [`Threads`];
    /// design/kernel-objects-from-untyped.md D2 records the path, notes/tcb.md the storage.
    threads: Threads,
    /// Neither the run queue nor `current` live here any more: both moved to per-CPU storage
    /// (`cpu::PerCpu`, DECISIONS §11 steps 3a and 3b), because a single shared queue and a
    /// single "running thread" are exactly what every core would otherwise contend on and
    /// overwrite. What stays is genuinely whole-machine: the thread table and the endpoints.
    ///
    /// Every IPC rendezvous. Indexed by the `usize` inside an `Object::Rendezvous` capability, which
    /// only the kernel mints, so the index is always in range.
    /// **The rendezvous registry** (milestone 19a; design/init-and-granular-spawn.md). An rendezvous
    /// is page-resident now: it lives at the start of a page retyped from some untyped region
    /// (a process's own, via `RETYPE_OBJ`, or the kernel's, via [`create_rendezvous`]), and that
    /// region is pinned so the page can never be freed under a blocked thread. The registry
    /// entry is the page's physical address; the generational name (`crates/slots`, the same
    /// machinery as Tids) is what an `Object::Rendezvous` capability carries, so the day endpoints
    /// can die, stale names will already fail safely.
    rendezvous_table: generational_table::Table<u64, MAX_RENDEZVOUS>,
    /// The kernel's **current** object chunk: where the kernel's endpoints (boot services, tests)
    /// are retyped from, so every rendezvous lives uniformly in a pinned page regardless of who paid.
    /// Carved lazily on the first [`create_rendezvous`] and **replaced when it fills**, which is what
    /// makes the kernel's rendezvous supply grow instead of being a compile-time guess.
    ///
    /// A filled chunk's handle is deliberately forgotten. Its pages stay pinned and its endpoints
    /// stay live, and nothing ever hands a kernel chunk back: kernel endpoints are destroyed only by
    /// tearing down the region hosting them (see the `doomed_eps` walk), and no path tears down a
    /// kernel chunk. If one ever should, this becomes an array and that is the change to make.
    kernel_ep_region: Option<u64>,
    /// How many chunks have been carved, so growth is bounded by something rather than by nothing.
    kernel_ep_chunks: usize,
    /// **The notification registry** (milestone 151 (notification objects), DECISIONS §101 (notification objects)): the rendezvous registry's
    /// shape one object type over. Each entry is the physical address of the page a
    /// [`NotificationPage`] lives at the start of, retyped from its creator's region; the
    /// generational name is what an `Object::Notification` capability carries.
    notification_table: generational_table::Table<u64, MAX_NOTIFICATIONS>,
    /// **The timer registry** (milestone 106 (a wait that ends on either the interrupt or the
    /// deadline), DECISIONS §147 (a timer a userspace service cannot hold)): the notification
    /// registry's shape one object type over. Each entry is the physical address of the page a
    /// [`TimerPage`] lives at the start of; the expiry walk iterates it.
    timer_table: generational_table::Table<u64, MAX_TIMERS>,
    /// **The futex wait queues** (milestone 812 (`std::thread::spawn` runs real threads in one
    /// address space), §269 (how threads share a process) fork 2): threads parked in
    /// `AddressSpace::WAIT`, by `(space, address)`. Under this lock for the reason every wait queue
    /// here is: the check of the word, the park, and a waker's take must be one order. See
    /// [`futex_wait`].
    futexes: inter_process_communication::futex::Futexes<Thread, FUTEX_BUCKETS>,
    /// The process registry (milestone 812): each entry the physical address of a process's page.
    processes: generational_table::Table<u64, { process::MAX_PROCESSES }>,
}

/// How many buckets the futex table hashes into. A wake walks one bucket, so this bounds the walk
/// at `MAX_THREADS / FUTEX_BUCKETS` waiters on average; 64 is four threads a bucket with the machine
/// full. Twenty-four bytes each.
const FUTEX_BUCKETS: usize = 64;

/// The most endpoints that can exist **at once**: the registry's bound.
///
/// This used to say it capped creations over the kernel's lifetime, on the grounds that rendezvous
/// teardown did not exist. That went stale when object revocation made destruction real: tearing
/// down a region removes every rendezvous whose page lives in it and `generational_table::Table::remove` frees the
/// slot for reuse, so this is a concurrent bound now. Corrected rather than left, because a stale
/// bound is the kind of comment that gets believed during a capacity argument.
pub(crate) const MAX_RENDEZVOUS: usize = 512;

/// **The most rendezvous that were ever live at once on this boot**, and how many of those live at
/// that moment sat on the kernel's own chunks. [`MAX_RENDEZVOUS`]'s instrument, built for
/// `PEAK_THREADS`'s reason by milestone 601 (the region table prints its peak), after the lane of
/// milestone 152 (durable delegation) met this ceiling in `timetable_tests`: a durable test created
/// its report endpoint with [`create_rendezvous`] on every run, and kernel-chunk rendezvous are
/// never freed (see `kernel_ep_region`), so the registry filled across the suite.
///
/// The split is the ledger's first cut. A rendezvous on a kernel chunk lives for the whole boot;
/// one retyped from a region goes when its region is reclaimed. So the kernel-chunk count only
/// ever rises, and it is the part of the peak no teardown can recover.
///
/// Both updated under `IPC_TABLES` at the one insert, so they never race.
///
/// # The ledger: what holds rendezvous at the peak
///
/// Measured 2026-09-26 with a temporary per-test print, on `main` at `484f3ebe` plus #1347:
///
/// | architecture | peak | spare | on kernel chunks | retyped from a region |
/// |---|---|---|---|---|
/// | aarch64 | **505** | 7 | 466 | 39 |
/// | riscv64 | **491** | 21 | 452 | 39 |
/// | `x86_64` | **315** | 197 | 302 | 13 |
///
/// The registry starts each suite empty and the peak is its last reading: it only grows, because
/// almost everything in it sits on a kernel chunk. So the ledger is who called
/// [`create_rendezvous`], aarch64 per module (kernel-chunk count first, then all live):
///
/// | module | kernel chunks | all live | |
/// |---|---|---|---|
/// | `kernel::sched` tests | 46 | 47 | the scheduler's own IPC tests |
/// | `user::tests` | 45 | 46 | the oldest userspace tests, one or two each |
/// | `ntp_tests` | 37 | 37 | |
/// | `login_tests` | 36 | 66 | the other 30 are retyped from session regions |
/// | `compositor_tests`, `display_tests` | 34 | 34 | |
/// | `rm_program_tests`, `dir_capability_tests`, `disk_tests` | 51 | 51 | the file services |
/// | `sink_tests`, `date_tests`, `time_tests` | 40 | 40 | |
/// | 23 other modules | 97 | | |
/// | created between tests | 80 | | services still wiring after a test returned |
///
/// Nothing on a kernel chunk is returned, even when the test that asked for it hands everything
/// else back, so every test that wires a service with [`create_rendezvous`] spends the registry
/// for the rest of the boot. The fix the lane of milestone 152 used on its own test is the
/// pattern: retype the endpoint from the run's own region, so it goes when the region does.
/// `design/roadmap/0671-tests-retype-their-rendezvous-from-their-own-region.md` is that work
/// across the suite. Reports and does not gate, for [`MAX_THREADS`]'s reason.
static PEAK_RENDEZVOUS: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
static KERNEL_CHUNK_RENDEZVOUS: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);

/// `(peak live, created on kernel chunks so far)`: see [`PEAK_RENDEZVOUS`]. Printed by the test
/// suite's closing summary.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))] // the closing summary is the only reader
pub fn rendezvous_pressure() -> (usize, usize) {
    (
        PEAK_RENDEZVOUS.load(Ordering::Relaxed),
        KERNEL_CHUNK_RENDEZVOUS.load(Ordering::Relaxed),
    )
}

/// An rendezvous's name: a generational `slots` name over the rendezvous registry (19a). What an
/// `Object::Rendezvous` capability carries. `u64` like a `ThreadId`, and stale-safe the same way.
pub type RendezvousId = u64;

/// The pages in one of the kernel's rendezvous chunks. **Not a ceiling.** When a chunk fills,
/// [`create_rendezvous`] carves another, so this is a batch size and nothing else.
///
/// It used to be a ceiling, and it was the wrong shape of number, because it grew with the SUITE
/// rather than with the system: 64 lasted until the 27+28 merge, 96 until supervision and `std::net`
/// merged the same day, 128 until milestone 33's compositor tests, which wire 26 endpoints across
/// four scenes (a display, a doorbell, a report per client, an input rendezvous per focusable client)
/// and wanted 160. Every parallel branch fit on its own, and the union
/// of their test boots is what crossed the line, which is a cost no branch can see before it merges.
/// So the failure mode was a merge-time panic telling whoever merged to raise a constant, over and
/// over, for a reason none of them caused. Growing on demand retires that whole class of papercut:
/// there is no number to raise, and the only remaining limit is [`MAX_RENDEZVOUS`], which is a real
/// bound with a real meaning.
///
/// 32 pages (128 KiB) is deliberately modest. A normal boot carves exactly one chunk and the rest of
/// the supply is never touched, which is the point of carving lazily.
const KERNEL_EP_CHUNK_PAGES: u64 = 32;

/// How many chunks the kernel will carve before refusing. Derived so the **page supply can never be
/// the binding limit before the registry is**: enough chunks to host [`MAX_RENDEZVOUS`] endpoints, one
/// page each. That is what makes exhaustion always report the honest reason (the registry is full)
/// rather than an arbitrary carve size. Derived rather than written down so the two cannot drift.
const MAX_KERNEL_EP_CHUNKS: usize = MAX_RENDEZVOUS.div_ceil(KERNEL_EP_CHUNK_PAGES as usize);

/// The rendezvous behind a name, or `None` if the name no longer resolves. Caller holds `IPC_TABLES`.
///
/// This used to panic on a miss, because endpoints could not be destroyed (their regions stayed
/// pinned), so a miss was kernel corruption. Object revocation made destruction real: a stale
/// `Rendezvous` capability (its rendezvous reclaimed out from under a holder) is now ordinary user
/// input, so this returns `None` and the callers turn that into a clean error rather than a panic.
///
/// The `'static` is the page's pinned-ness made into a lifetime: while the name resolves the page is
/// pinned and direct-mapped, and `IPC_TABLES` serializes every access to what it holds.
fn rendezvous_of(sched: &IpcTables, ep: RendezvousId) -> Option<&'static mut Rendezvous> {
    let phys = *sched.rendezvous_table.get(ep)?;
    // SAFETY: retyped exclusively for this rendezvous, its region pinned while the name resolves,
    // direct-mapped, and serialized by IPC_TABLES, which every caller holds.
    Some(unsafe { &mut *(crate::arch::mmu::phys_to_virt(phys) as *mut Rendezvous) })
}

/// Mark the current thread's blocking IPC as aborted (a stale rendezvous, or one revoked while it
/// blocked): the syscall layer reads-and-clears this after the primitive returns and hands back an
/// error. A helper because several IPC paths set it. Caller holds `IPC_TABLES`.
///
/// **`#[cold]`, and it is a claim about the callers rather than about this body.** Every call site
/// is the `else` of a `rendezvous_of` that returned `None`, which means the name a program invoked
/// no longer resolves. A healthy IPC never reaches it, and it is reached from four functions that
/// are all on `script/fastpath-footprint`'s closures, so inlining it put the abort path's bytes on
/// the fast path four times over (milestone 188 phase 3).
#[cold]
#[inline(never)]
fn set_ipc_aborted(sched: &mut IpcTables, tid: ThreadId) {
    if let Some(t) = sched.threads.get_mut(tid) {
        t.handshake.abort();
        // **The capability staged for the aborted send goes with it** (the 2026-10-03 security
        // audit's follow-up). A `SEND_CAP` or `CALL` that parked put its delegation, or the Reply
        // the kernel minted, in `outgoing_cap` for the receiver to take. An abort means no receiver
        // ever will: the rendezvous is gone. Left in place, the next plain `SEND` this thread
        // parked on a *different* rendezvous would hand that capability to whoever `RECEIVE_CAP`s
        // there, a delegation the sender made to one endpoint delivered to another. The sender
        // still holds its own copy (`SEND_CAP` narrows a copy, it never moves the source), so
        // nothing is lost by dropping this one.
        t.outgoing_cap = None;
    }
}

/// **Refuse the current thread's send, because the rendezvous carries an interrupt** (milestone 603
/// (provisional), DECISIONS §101 ruling B). An abort, so the syscall layer's existing
/// `take_ipc_aborted` branch is the only one the common path pays for, plus the reason, which only
/// that branch reads ([`take_ipc_refused`]). The sender never parked, so the abort does not weaken
/// the boot-8 gate for anything: it is taken immediately, as a stale endpoint's is.
///
/// `#[cold]` for [`set_ipc_aborted`]'s reason: no healthy IPC reaches it, and it is reached from
/// three functions on `script/fastpath-footprint`'s closures.
///
/// Name: provisional (milestone 603 (provisional)): calef names public items.
#[cold]
#[inline(never)]
fn set_ipc_refused(sched: &mut IpcTables, tid: ThreadId) {
    if let Some(t) = sched.threads.get_mut(tid) {
        t.handshake.abort();
        t.ipc_refused = true;
    }
}

/// **Read and clear why the current thread's aborted send was aborted**: `true` when the rendezvous
/// carries an interrupt and refused it ([`set_ipc_refused`]), `false` when it was stale or revoked.
/// Called by the syscall layer only after [`take_ipc_aborted`] returned `true`, so an IPC that was
/// not aborted never pays for it.
///
/// Name: provisional (milestone 603 (provisional)): calef names public items.
#[cold]
#[inline(never)]
pub fn take_ipc_refused() -> bool {
    let mut guard = IPC_TABLES.lock();
    let Some(sched) = guard.as_mut() else {
        return false;
    };
    let tid = current_thread_id();
    sched
        .threads
        .get_mut(tid)
        .map(|t| core::mem::take(&mut t.ipc_refused))
        .unwrap_or(false)
}

/// **Read and clear the current thread's IPC-aborted flag** (object revocation). The syscall layer
/// calls this right after an rendezvous IPC primitive returns: `true` means the rendezvous was stale, or
/// revoked while the thread blocked on it, so the caller gets an error instead of the primitive's
/// placeholder result. Kernel-side IPC callers never set it (their endpoints are never revoked), so
/// they need not check it.
pub fn take_ipc_aborted() -> bool {
    let mut guard = IPC_TABLES.lock();
    let Some(sched) = guard.as_mut() else {
        return false;
    };
    let tid = current_thread_id();
    sched
        .threads
        .get_mut(tid)
        .map(|t| t.handshake.take_aborted())
        .unwrap_or(false)
}

/// Rank **above the allocators**, because the reaper (`finish_switch`) drops a dead `Thread` in
/// its pool slot while holding this, and that drop *frees*: the kernel stack's pages go back to
/// the frame allocator through the kernel MMU lock, and the stack's VA range to its free list.
/// Freeing takes the same locks allocating does, so the rank must sit above them.
///
/// Nothing under this lock **allocates** any more (milestone 14 phase B.2): spawn writes the new
/// `Thread` into a static pool slot, and the queues have been intrusive since A.2, so a queue
/// operation is a couple of pointer writes, from the timer IRQ or anywhere else. §9's
/// no-allocation-in-IRQ rule holds by construction.
static IPC_TABLES: IrqSafeMutex<Option<IpcTables>> = IrqSafeMutex::new(rank::IPC_TABLES, None);

/// The scheduler before anything is in it, as a `const` rather than an expression [`init`] builds.
///
/// Both tables are `const fn` constructors already, so this is a `.rodata` aggregate the installer
/// copies from instead of a value assembled on the boot stack. `init`'s own comment carries the
/// measurement and why it decides how large [`MAX_THREADS`] may be.
const EMPTY_TABLES: IpcTables = IpcTables {
    threads: Threads::new(),
    rendezvous_table: generational_table::Table::new(),
    kernel_ep_region: None,
    kernel_ep_chunks: 0,
    notification_table: generational_table::Table::new(),
    timer_table: generational_table::Table::new(),
    futexes: inter_process_communication::futex::Futexes::new(),
    processes: generational_table::Table::new(),
};

/// **Per-cpu ring of the last few scheduler events** (first-silicon diagnostics, 2026-08-14; the
/// module name is provisional). A boot-7 bench dump on the VisionFive 2 showed an end state no
/// legal transition sequence produces (a thread `Blocked` at a non-syscall pc; a receiver
/// `Running` as another core's current for ten seconds), and an end state alone cannot say which
/// transition wrote it. This keeps the last [`trace::DEPTH`] events each core performed, and
/// [`dump_threads`] prints them, so the *path* into the wedge is on the serial log.
///
/// Cost and honesty:
///
/// - One relaxed `fetch_add` and one relaxed store per event, on paths that already hold `IPC_TABLES`
///   or run in IRQ context with interrupts masked, so each ring has exactly one writer (its own
///   core) and no entry can tear (one `u64`).
/// - **Compiled out of `--features bench` builds**, so the benchmark numbers the tripwire watches
///   are not measuring the instrument. The board tour build carries no features and keeps it.
/// - A dump reads other cores' rings racily (drain/steal events are recorded outside `IPC_TABLES`);
///   an entry is atomic, so the worst case is an event missing from the tail, never a torn one.
///
/// The bench build gets a no-op twin of the same two-function surface (below), so the call sites
/// are identical in every configuration and nothing here needs a dead-code allow.
#[cfg(not(feature = "bench"))]
mod trace;

/// The bench build's no-op twin of [`trace`]: same names, same signatures, nothing recorded, so
/// the benchmark numbers the tripwire watches never measure the instrument and the call sites
/// need no `cfg` of their own.
#[cfg(feature = "bench")]
mod trace {
    /// Mirrors the real module's [`Event`](super::trace::Event) variants; carried only so the
    /// call sites name the same paths in both configurations.
    #[derive(Clone, Copy)]
    pub enum Event {
        SwitchTo,
        BlockSelf,
        Wake,
        WakeDeferred,
        WakeCompleted,
        PlaceRemote,
        StealServe,
        InboxDrain,
        WakeRefused,
        Served,
        #[cfg_attr(not(feature = "soak_test"), allow(dead_code))]
        Migrated,
    }

    #[inline]
    pub fn record(_kind: Event, _tid: u64, _aux: u8) {}

    /// Always zero here, because nothing is recorded. The bench boot runs no soak (both diverge
    /// before the other could start), so no caller can be misled by it.
    #[cfg(feature = "soak_test")]
    pub fn counted(_kind: Event) -> u64 {
        0
    }

    pub fn dump(_cpu: usize) {}
}

/// **A corruption tripwire over `IpcTables`'s registries** (first-silicon diagnostics,
/// 2026-08-15; module name provisional). Armed around the initrd demo on the board tour, it
/// re-reads the watched ranges on the timer tick and prints every byte that changed since the
/// last look: address, tick, before and after. A legal change (a spawn writing a fresh `ThreadControlBlockPointer`, a
/// reap bumping a generation) prints as a recognizable delta at a table offset; a stray write
/// prints as bytes nothing in the choreography explains. The instrument does not judge, it shows,
/// because boots 7 through 9 proved the judging is the part that goes wrong.
///
/// What it watches: the thread table and the rendezvous registry (the `slots` arrays and their
/// generations), which are quiescent between spawns and creates. The per-cpu blocks are
/// deliberately NOT watched: `ticks`, `runnable`, `current` and the queues churn on every
/// scheduler entry by design, so a checksum there measures the scheduler working, not corruption.
///
/// Cost and honesty:
///
/// - Unarmed (every build's steady state), the tick-path cost is one relaxed load.
/// - Armed, the owner core re-reads ~13 KiB per tick. Diagnostic-build money, spent only inside
///   the demo window on the board tour.
/// - Compiled out of `--features bench` builds exactly as the event rings are, so the tripwire
///   benchmarks never measure the instrument.
/// - The watched memory is concurrently mutated under `IPC_TABLES` while the check reads it lock-free;
///   a torn read of an in-flight legal write can print as a divergence. That is a false alarm
///   only in the sense that the mutation was legal; the printed delta says so itself.
/// - The instrument's own state (the watch table, the shadow) is serialized by
///   `memory_corruption_canary_gate::Gate`, a one-word state machine with loom-searched guards. Its first protocol
///   was two hand-written flags here, and that pair raced (2026-08-15): a `check` that lost the
///   single-flight slot returned silently having checked nothing, which the kernel test read as
///   a missed corruption (the thead-c906 flake, notes/cpu-models.md BUGS), and a re-arm could
///   rewrite the plan under a checker that had seen `ARMED` but not yet won the slot. See
///   `crates/memory_corruption_canary_gate` for both holes and the harnesses that falsify the old spelling.
#[cfg(not(feature = "bench"))]
// **No caller since milestone 295, and kept deliberately** (2026-09-14). Its one consumer was the
// hang watcher inside `kernel::user::riscv_initrd_demo`, which armed it around that demo's blocking
// receive; calef retired the program that demo loaded and the function went with it. This is the
// `AGENTS.md` "an exception is allowed and must say so" case, written where a reader meets it: the
// instrument stays because it is how a board hang gets diagnosed, it was the evidence that
// overturned the VisionFive 2 "hang" (notes/visionfive2.md, fifth stop), its own serialization is
// loom-checked in `crates/memory_corruption_canary_gate`, and re-deriving it at a bench at 2am is
// the cost this avoids. Unarmed it is one relaxed load on the tick path, which is what makes
// keeping it cheap. Re-point it at a window that can hang and delete this note.
#[allow(dead_code)]
mod canary;

/// The bench build's no-op twin of [`canary`], so the call sites carry no `cfg` of their own.
#[cfg(feature = "bench")]
mod canary {
    pub fn arm(_ranges: &[(usize, usize)]) {}
    pub fn disarm() {}
    pub fn check() -> bool {
        false
    }
}

/// Adopt the context we are already running in as thread 0.
///
/// It has no stack of its own and no saved context. **The first switch *away* from it fills
/// that in**, which is why the boot thread needs no special case: a thread's context is written
/// by the act of leaving it.
pub fn init() {
    // **The floating-point unit is shut on this core before any thread exists**
    // (milestone 447 (a thread's vector registers are its own)).
    //
    // Here rather than in `arch::init`, and the reason is a real trap rather than taste. What the
    // invariant is *about* is threads: `crate::fp` marks a thread `live` when it takes the first-use
    // trap, and a core that started with the unit already open never produces one, so every thread
    // on it stays `live == false` and two of them quietly share a register file. This is the
    // function where threads begin to exist, so the property and the mechanism are in the same
    // place. `arch::init` looked like the obvious home and is not one: **RISC-V's boot hart never
    // calls it.** `main`'s RISC-V tour installs `stvec` with `arch::exceptions::init()` directly,
    // reaches `sched::init` and never passes through `arch::init` at all, and OpenSBI hands the
    // kernel a hart with `sstatus.FS` already set. That cost this milestone a red suite and is
    // exactly the shape of failure AGENTS.md's ladder is about: it worked on two architectures and
    // was invisible on the third.
    crate::arch::fp::init();

    // **The machine statistics page, before the first thread** (milestone 126, DECISIONS §225 (`free` sees the machine and your share)),
    // for `fp::init`'s reason: this is where threads begin, so it is where the counters that watch
    // them begin, on all three architectures through the one function each boot path calls.
    crate::machine_statistics::publish();

    let mut sched = IPC_TABLES.lock();

    // **Install the empty tables FIRST, then name the boot thread through them**, rather than
    // building a `Threads` as a local and moving it in afterwards. That ordering is a stack
    // measurement rather than a preference, and it is what lets [`MAX_THREADS`] be raised at all.
    //
    // `IpcTables` is two generational tables and nothing else that matters, and the thread table
    // is `MAX_THREADS` slots wide. Built as a local and then moved into the `Option`, an
    // unoptimised build carries the table twice on the boot stack, so **every slot added to
    // `MAX_THREADS` cost about 80 bytes of boot stack** rather than the ~40 the table itself is.
    // Measured 2026-08-27: at 128 slots this function's frame was 43,952 bytes, the deepest in
    // the kernel and the reason the suite's boot-stack high-water sat at 54,336 of 65,504 (82%);
    // a probe raise to 256 took the high-water to 64,640 (98%) and failed `stack::report_high_water`'s
    // own gate, in a run where nothing else about the boot had changed. Assigning
    // [`EMPTY_TABLES`], a `const`, gives the compiler a `.rodata` aggregate to copy from and
    // leaves the boot thread's insert to go straight into the installed table.
    *sched = Some(EMPTY_TABLES);
    let tables = sched
        .as_mut()
        .expect("the tables were just installed on this line");
    // The table names the boot thread at insert. The first name a fresh table mints is 0 by
    // construction (slot 0, generation 0), so "the boot thread is tid 0" survives, now as a
    // property of the table rather than a hardcoded key.
    let boot_tid = tables
        .threads
        .insert_with(|tid| {
            let mut boot = Thread::boot();
            boot.id = tid;
            boot
        })
        .expect("a fresh table refused its first insert");

    // This core (core 0) is running the boot thread.
    let boot_tcb = tables
        .threads
        .pointer(boot_tid)
        .expect("the boot thread was just inserted");
    // SAFETY: `boot_tid`'s page pointer from the table, live.
    unsafe { set_current(boot_tid, boot_tcb) };

    drop(sched); // release before spawning, which takes the lock itself

    // (The run queue and inbox used to have capacity reserved here, so a push from the timer IRQ
    // could never allocate. The queues are intrusive now: a push is two pointer writes and
    // *cannot* allocate, so there is nothing to reserve. §9's rule became structural.)

    // The idle thread. Its entire body is "wait for an interrupt, then let the scheduler look for
    // work." It is deliberately kept OUT of the ready queue (see cpu::PerCpu::idle): the scheduler picks it
    // only when nothing else is runnable, so it never steals a turn from real work.
    //
    // Built on its own TCB page, as `spawn_on` builds every other kernel thread (milestone 124 (a
    // thread is born where it lives: the spawn path's copies)), rather than as a value carried
    // there: the by-value `Thread::spawn` this used held three `Thread`s in one frame in an
    // unoptimised build, 4368 bytes once the capability table grew to 32 slots, over the 4096-byte
    // guard page (milestone 126 (the `procps` package), 2026-09-27, UTC).
    let mut sched = IPC_TABLES.lock();
    let s = sched.as_mut().unwrap();
    let idle_id = s
        .threads
        .insert_in_place(|tid, dst| {
            // SAFETY: `dst` is the fresh, exclusively-ours TCB page `insert_in_place` claimed.
            unsafe { Thread::spawn_into(|| run_idle(), tid, dst) }
        })
        .expect("could not create the idle thread (no kernel stack, no TCB page, or a full table)");
    drop(sched);
    // NOT pushed onto `ready`: the idle thread is a fallback, not a peer.
    cpu::current().idle.store(idle_id, Ordering::Relaxed);
}

/// The raw TCB pointer of a live thread, for queueing (milestone 14 phase A.2). Caller holds
/// `IPC_TABLES`.
///
/// The pointer's validity while queued is the queue discipline, stated once here: a thread on a
/// run queue or inbox is `Ready`, a thread on an rendezvous wait queue is `Blocked` (A.3), the
/// reaper frees only `Finished` threads, and a thread is never two of those at once. The `Box` in
/// the table pins the address (see `IpcTables::threads`), so a pointer taken here is good until
/// the thread is popped, however many queue hops (inbox to run queue) it makes in between.
/// The queue-able pointer to a live thread.
///
/// Returns [`core::ptr::NonNull`] rather than `*mut`, and that is not decoration: the pointer is derived from a
/// `&mut Thread` handed out by the thread table, so **non-nullness is a fact of construction rather
/// than a promise the caller keeps**. Saying so in the type removes null from the intrusive queue's
/// safety contract entirely, which is one of the two things CodeQL's `rust/access-invalid-pointer`
/// alerts were pointing at (milestone 45). What the type still cannot express is that the pointee
/// outlives its time on the queue; that is the caller's rule 2, and no type available here can carry
/// it for an intrusive structure.
fn thread_control_block_ptr(sched: &mut IpcTables, tid: ThreadId) -> core::ptr::NonNull<Thread> {
    core::ptr::NonNull::from(
        sched
            .threads
            .get_mut(tid)
            .expect("thread_control_block_ptr of a dead thread"),
    )
}

/// **Take `tid`'s queue token off its own TCB, to put it on a queue** (milestone 139 (drive the
/// unsafe count down), round 10). Caller holds `IPC_TABLES`.
///
/// The token is there exactly when the thread is on no queue (see `thread::Thread::own_token`), so
/// a `None` here is a thread somebody is about to put on a second queue ([`missing_token`]).
#[inline(always)]
fn take_token(sched: &mut IpcTables, tid: ThreadId) -> Unqueued<Thread> {
    // Two `let`-`else`s rather than a combinator chain: the icount gate measures a debug build,
    // where each closure is a call.
    let Some(t) = sched.threads.get_mut(tid) else {
        missing_token()
    };
    let Some(token) = t.own_token.take() else {
        missing_token()
    };
    token
}

/// **A thread that should hold its token does not**: it is on a queue already (and is about to be
/// put on a second), or it is a running thread whose token went somewhere it should not. Loud in
/// every build, because the alternative is the corrupted list the token exists to rule out.
///
/// One cold function for every site rather than an `expect` at each, measured rather than assumed:
/// an `expect` sets up a message and a location inline, and seven of them were the larger part of
/// what the token added to `script/fastpath-footprint`'s closures.
#[cold]
#[inline(never)]
fn missing_token() -> ! {
    panic!("a thread without its queue token: on a queue already, or its token was lost")
}

/// **Give a token a queue just handed back to the thread it names, and say which thread that is.**
/// Every pop and removal in this file ends here or on another queue, so the thread holds its token
/// again before anything decides what happens to it. Caller holds `IPC_TABLES`.
///
/// This is where the old sites' `(*waiter.as_ptr()).id` went: they read the id through the pointer
/// a queue returned, and so does this, once.
#[inline(always)]
fn hold_token(token: Unqueued<Thread>) -> ThreadId {
    // SAFETY: a token's thread is live for as long as the token exists (`Threads::mint_token`'s
    // first obligation), `IPC_TABLES` (held by every caller) serializes access to every `Thread`,
    // and no other reference to this one is live: the caller has just taken it off a queue.
    let t = unsafe { &mut *token.as_ptr() };
    t.own_token = Some(token);
    t.id
}

/// Put an already-created thread onto core `target`'s run queue. Caller holds `IPC_TABLES`.
///
/// Local: straight onto our own queue (`IPC_TABLES` masks interrupts, which `with_runq` needs). Remote:
/// into the target's inbox, and the SGI (sent after `IPC_TABLES` is released, by the caller) makes it
/// drain. The inbox push under `IPC_TABLES` is rank-safe (INBOX < `IPC_TABLES`), and the inbox's own lock supplies
/// the release/acquire that orders our thread-table insert before the target's drain (§11).
///
/// **Returns the core that owes an SGI**, `Some(target)` when the thread went into a remote inbox
/// and `None` when it went onto this core's own run queue, and that return value is the whole
/// point rather than a convenience. A caller must not decide "was this remote?" a second time by
/// comparing `target` against [`cpu::id()`] again: this function runs under `IPC_TABLES`, which
/// masks interrupts, while the caller's second comparison does not, so the calling thread can be
/// preempted and **stolen onto a different core** in between and the two answers disagree. When
/// they disagree in the direction that skips the SGI, the placed thread sits `Ready` in a remote
/// core's inbox that nothing will ever drain (only the reschedule-SGI handler calls
/// [`drain_inbox`]), the idle target refuses to steal because [`cpu::PerCpu::runnable`] counts that
/// inbox as its own work, and the machine wedges with every core idle. That is the 2026-08-28
/// riscv64 CPU-matrix hang, whose trace ring caught the migration in the act:
///
/// ```text
/// core 2: ... switch:0x0 ... switch:0x1000000076 steal:0x0/3 ...   gave tid 0 away to core 3
/// core 3: ... drain:0x1 switch:0x0 place:0x500000077/2 block:0x0/177 switch:0x4
/// ```
///
/// Tid 0 read `target == cpu::id()` on core 2, was preempted and stolen to core 3, and finished the
/// same `spawn_on` there: the push went to core 2's inbox and the stale "local" answer skipped the
/// poke.
///
/// **A skipped SGI is usually invisible, which is what makes it dangerous.** The next SGI aimed at
/// that core for any reason drains the whole inbox, so a strand is normally repaired within
/// milliseconds and nothing is ever seen. It wedges only when the stranded thread is the work
/// everything else was about to wait on, so no further SGI is generated. Do not take "it has not
/// hung" as evidence that a placement path pokes correctly. See notes/scheduler.md.
#[must_use = "a remote placement owes a reschedule SGI once IPC_TABLES is released, or the thread \
              sits in an inbox nothing drains"]
/// Spawn a thread and place it on a **specific** core (SMP step 3c).
///
/// The cross-core placement primitive. `spawn` puts work on the calling core; this puts it on
/// `target`, which is what lets the machine actually spread load. A remote target is handed the
/// thread through its inbox and then poked with the reschedule SGI. (Wiring `spawn` itself to
/// round-robin over `target` is the trivial next step, once the mechanism is proven.)
pub fn spawn_on<F: FnOnce() + Send + 'static>(target: usize, f: F) -> Option<ThreadId> {
    let (id, remote) = {
        let mut guard = IPC_TABLES.lock();
        let sched = guard.as_mut()?;
        // **The Thread is built on its own TCB page, not carried there** (milestone 124). The old
        // shape called `Thread::spawn(f)` for a value and moved it through this closure, and every
        // instantiation of this generic function carried 3888 to 4592 bytes of frame as a result:
        // over the 4096-byte guard page, which is the size at which one frame can step past the
        // guard in a single move and corrupt the neighbouring stack with no fault at all.
        let id = sched.threads.insert_in_place(|tid, dst| {
            // SAFETY: `dst` is the fresh, exclusively-ours TCB page `insert_in_place` claimed; it
            // is aligned for `Thread` and holds no live one, so `write` drops nothing.
            unsafe { Thread::spawn_into(f, tid, dst) }
        })?;
        // Record where it was placed, so a survey can report it (`abi::survey::record::PLACEMENT`).
        // Here rather than inside `place_on`, and that siting is the whole cost argument: `place_on`
        // is also the wake path, so a store there would be a store per wake on the IPC fastpath,
        // which is what put `Thread::last_cpu` behind a soak-build feature. One store per thread
        // creation is free by comparison. `insert_in_place` just returned this id, so the `if let`
        // is belt and braces rather than a case that happens.
        if let Some(t) = sched.threads.get_mut(id) {
            t.placement = target as u8;
        }
        // The placement decision is made ONCE, here, with interrupts masked, and carried out of
        // the critical section as a value. Re-deriving it below from `target != cpu::id()` is the
        // lost-wakeup bug `place_on` documents: this thread can be stolen onto another core
        // between the two reads.
        let remote = place_on(target, take_token(sched, id));
        (id, remote)
    }; // IPC_TABLES released here, before the SGI, so the target's schedule() can take it

    if let Some(target) = remote {
        // Poke the target: its handler drains the inbox we just pushed to and reschedules.
        crate::arch::irq::send_reschedule(target);
    }
    Some(id)
}

pub fn spawn<F: FnOnce() + Send + 'static>(f: F) -> Option<ThreadId> {
    // Placement is the power of two choices (DECISIONS §28): the new thread lands on the lighter of
    // two randomly sampled cores, not always on the spawner's, so work spreads instead of piling on
    // one core beside idle ones (the FS-server starvation lesson). `spawn_on` carries the thread to
    // the chosen core over the §11 inbox/SGI path.
    spawn_on(pick_spawn_target(), f)
}

/// Spawn a thread against a **quota**: at most `budget` of these may be alive at once.
///
/// Reserving a slot is an atomic decrement; the slot lives inside the spawned `Thread` as a
/// [`QuotaToken`] and comes back when the thread is reaped. Returns `None` if the budget is
/// exhausted (too many children already alive) OR the kernel is out of memory: the caller cannot
/// tell the two apart, and does not need to: either way it could not spawn, and it must degrade
/// rather than panic. This is the bound that stops a spawn flood or a leaked-thread pile-up from
/// exhausting kernel memory. See notes/quotas.md and notes/security.md.
///
/// # It has no caller today, and that is worth saying plainly
///
/// Its one caller was the kernel-wired `shell_service`'s spawn service, which DECISIONS §28 retired
/// and milestone 41 deleted. Nothing in the kernel spawns against a quota now, because **the bound
/// moved**: a userspace process spawns out of its own untyped budget (§10, §16), so the budget *is*
/// the quota and it is enforced by retyping rather than by a counter. This function is the bound for
/// *kernel* threads, and no kernel thread is currently spawned in a loop by anything untrusted.
///
/// Kept rather than deleted because removing a documented safety mechanism is a design decision, not
/// dead-code triage, and notes/quotas.md and notes/security.md both describe it. Allowed
/// unconditionally and on purpose (DECISIONS §38, disposition 3): there is no configuration in which
/// something calls it, and pretending otherwise with a `cfg` predicate would be the dishonest option.
#[allow(dead_code)]
pub fn spawn_with_quota<F: FnOnce() + Send + 'static>(
    budget: &'static AtomicU32,
    f: F,
) -> Option<ThreadId> {
    // Reserve a slot: decrement only if there is one. A compare-exchange loop, so it is exactly
    // one atomic decrement and it never dips below zero (returning `None` = "quota exhausted").
    let mut remaining = budget.load(Ordering::Relaxed);
    loop {
        if remaining == 0 {
            return None;
        }
        match budget.compare_exchange_weak(
            remaining,
            remaining - 1,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(actual) => remaining = actual,
        }
    }

    // The reserved slot is held by this token from here on, so every early return below hands it
    // back by dropping it, and a thread that is built carries it until it is reaped.
    let token = QuotaToken::new(budget);

    let mut guard = IPC_TABLES.lock();
    let sched = guard.as_mut()?;
    // Built in place on its TCB page rather than as a value (see `spawn_on`, milestone 124): the
    // by-value path held three `Thread`s in one frame in an unoptimised build.
    let id = sched.threads.insert_in_place(|tid, dst| {
        // SAFETY: `dst` is the fresh, exclusively-ours TCB page `insert_in_place` claimed.
        unsafe { Thread::spawn_into(f, tid, dst) }
    })?;
    // `insert_in_place` just returned this id, so the thread is there.
    if let Some(t) = sched.threads.get_mut(id) {
        t.quota = Some(token); // returned to `budget` when the thread is reaped
    }
    let token = take_token(sched, id);
    // This core's queue: IPC_TABLES held, IRQs masked.
    cpu::current().with_runq(|q| q.push_back(token));
    Some(id)
}

/// Give up the CPU voluntarily.
pub fn yield_now() {
    schedule();
}

/// The current thread exited cleanly (`SYS_EXIT`). Never returns.
pub fn exit() -> ! {
    depart(abi::fault::EVENT_EXIT, 0, 0, true)
}

/// **`SYS_EXIT_THREAD`** (milestone 812): end the calling thread, and its process only if no other
/// member runs. `word`, if not zero, is cleared and its futex waiters woken first: see
/// `futex::clear_and_wake`. Never returns.
pub fn exit_thread(word: u64) -> ! {
    futex::clear_and_wake(word);
    depart(abi::fault::EVENT_EXIT, 0, 0, false)
}

/// The current thread faulted (a bad access, an illegal instruction) and is being killed. Never
/// returns. `pc` is the faulting instruction and `addr` the faulting address (0 if the fault class
/// carries none). The arch fault handlers call this from the faulting thread's kernel stack, the
/// same context `exit` runs in, so the departure below is identical bar the event code and words.
pub fn fault(pc: u64, addr: u64) -> ! {
    depart(abi::fault::EVENT_FAULT, pc, addr, true)
}

/// **A thread's last act: report its death, then leave the CPU forever** (milestone 22, §26).
///
/// Two outcomes, decided by whether the thread was spawned with a supervision rendezvous (its
/// `fault_ep`, set at `START` from the reserved fault slot):
///
///   - **Unsupervised** (`fault_ep == None`): today's behaviour exactly. Mark `Finished` and
///     `schedule()` away; the next thread's `finish_switch` reaps it once it is off this stack.
///   - **Supervised**: build the five-word §26 message, retain it on the corpse for postmortem,
///     deliver it to the supervision rendezvous (waking a waiting supervisor, or parking the corpse
///     on the rendezvous so the message is not lost if none is waiting), and mark the thread `Dead`.
///     A `Dead` corpse is never reaped by `finish_switch`; it persists, registers and address
///     space intact, until the supervisor reaps it with §16 revocation.
///
/// Either way we are still running on the thread's own kernel stack, so we cannot free it here; we
/// only mark state and `schedule()`, exactly as `exit` always has.
fn depart(event: u64, pc: u64, addr: u64, whole: bool) -> ! {
    {
        let mut guard = IPC_TABLES.lock();
        let sched = guard.as_mut().expect("depart before sched::init");
        let current = current_thread_id();

        // A member of a process ends it on `exit` or a fault, and on `exit_thread` when it was the
        // last to run (milestone 812, `process::depart`); its own supervision is what it reports.
        let (fault_ep, label) = process::depart(sched, current, whole, [event, pc, addr])
            .map_or((None, 0), |(ep, label)| (Some(ep), label));

        // **A thread that departs answers nobody again** (milestone 254). Whatever `CALL` it
        // collected and never replied to is a caller parked on nothing, discoverable only through
        // the reply capability that is about to be dropped with this table. Free those callers
        // here, with the stale-reply sweep `strand_reply_caller` insists on, so they return
        // `Error::Gone` rather than blocking for the life of the machine. This is QNX's headline
        // behaviour and the case notes/blocked-thread-teardown.md's survey found this kernel alone
        // in lacking.
        strand_callers_of(sched, current);

        match fault_ep {
            None => {
                if let Some(t) = sched.threads.get_mut(current) {
                    t.handshake.state = State::Finished;
                }
            }
            Some(ep) => {
                let msg = [event, current, pc, addr, 0];
                if let Some(t) = sched.threads.get_mut(current) {
                    // Retain the message on the corpse (postmortem) and stage it as the mailbox a
                    // parked-corpse delivery will hand the supervisor. Dead: never runs again.
                    t.fault_msg = Some(msg);
                    t.mailbox = msg;
                    t.handshake.state = State::Dead;
                }
                deliver_death(sched, current, ep, msg, label);
            }
        }
        // Not requeued and not removed: we are still on this stack. The switch below leaves it,
        // and for a Finished thread the next thread reaps it; a Dead one waits for the supervisor.
    }

    schedule();
    unreachable!("a departed thread was scheduled again");
}

/// Deliver a corpse's five-word death message to its supervision rendezvous. Caller holds `IPC_TABLES`
/// and has already marked the corpse `Dead` with `msg` in its mailbox.
///
/// This is the ordinary synchronous-send rendezvous (`Rendezvous::send`), reused: if a supervisor is
/// blocked in `RECEIVE`, hand it the message and wake it; if none is, the corpse joins the rendezvous's
/// sender queue with the message in its mailbox, so the notification waits there rather than being
/// lost (the same guarantee an ordinary blocked sender gets, and the reason a data-carrying death
/// uses the sender queue rather than the data-less IRQ signal count). The corpse is never woken:
/// `ipc_receive` recognises a `Dead` sender and leaves it dead after taking its message, the same way
/// it leaves a `CALL` caller blocked. If the rendezvous itself is gone (the supervisor was torn down
/// first), the message is simply dropped, like an interrupt with no live rendezvous.
///
/// `label` is the badge the builder put on the child's supervision capability (milestone 105 (the
/// two forks), §148 (resolves by asking the kernel) as amended). It goes to a supervisor already waiting here through [`hand_over_label`]; a
/// supervisor that arrives later collects it from the corpse in [`collected_without_serving`].
fn deliver_death(
    sched: &mut IpcTables,
    corpse: ThreadId,
    ep: RendezvousId,
    msg: [u64; 5],
    label: u64,
) {
    let Some(rendezvous) = rendezvous_of(sched, ep) else {
        return;
    };
    // The corpse's token: it is still the running thread, on no queue. If it joins the sender queue
    // below it stays put, since nothing wakes or reaps a Dead thread until the supervisor drains it
    // and revokes; otherwise its token comes back and goes home to its TCB, where the reap drops it.
    match rendezvous.send(take_token(sched, corpse)) {
        inter_process_communication::Send::Rendezvous(receiver, me) => {
            hold_token(me);
            let receiver = hold_token(receiver);
            let r = sched.threads.get_mut(receiver).unwrap();
            r.mailbox = msg;
            hand_over_label(r, label);
            r.handshake.serve(); // delivered: this wake passes the boot-8 gate
            trace::record(trace::Event::Served, receiver, 8);
            wake(sched, receiver);
        }
        inter_process_communication::Send::Blocked => {
            // The corpse is parked on the sender queue now, its mailbox already holding `msg`.
            // Record the parking so a dump shows where the death message waits.
            if let Some(t) = sched.threads.get_mut(corpse) {
                t.handshake.wait_on = Some(Wait::Rendezvous(ep, WaitRole::Sender));
            }
        }
        // The supervision rendezvous carries an interrupt (§101 ruling B). Dropped, like a death
        // to a rendezvous that is gone: its `w0` is `EVENT_FAULT` or `EVENT_EXIT`, and `EVENT_FAULT`
        // is 1, which a driver would read as its interrupt. Reachable only by configuring an
        // interrupt's endpoint as a fault endpoint, which needs a `Rendezvous` capability to it that
        // no program is granted today.
        inter_process_communication::Send::Refused(me) => {
            hold_token(me);
        }
    }
}

/// Called from the timer IRQ. **Records** that a switch is wanted; does not switch.
///
/// **`#[inline(never)]`, and the reason is a measurement rather than a preference.** On riscv64 the
/// timer interrupt and a syscall arrive through the same `riscv_trap_body`, so anything inlined
/// here lands in a symbol `script/fastpath-footprint` counts **flat**: its bytes are charged to
/// every syscall although no syscall fetches them, which is the over-count milestone 368 (the entry set is flat, so an inlining flip can move 12% into it) records.
/// Keeping this a call rather than an inline puts the tick path's bytes in the tick path's own
/// symbol, where they belong and where the gate can see them for what they are. One `jal` per tick
/// per core, at 100 Hz, against bytes on the line every syscall shares.
#[inline(never)]
pub fn on_tick() {
    cpu::current().need_resched.store(true, Ordering::Relaxed);
    // **Charge this tick to whatever is on this CPU** (milestone 282 (a thread's CPU time, and the `top` it makes possible), DECISIONS §150 (how does a thread's CPU time reach userspace?)). One
    // bounds-checked index and one relaxed increment, which is the whole of the accounting; see
    // [`CPU_TICKS`] for why the counter is an array beside the table rather than a field in it.
    charge_tick();
    // **And the machine's own view of the same tick** (milestone 126, DECISIONS §225): busy or
    // idle, and how many threads were waiting, for `vmstat` and `top`'s summary.
    count_tick();
    // The corruption tripwire, when armed (the board tour's initrd-demo window). One relaxed
    // load when it is not, which is every other tick everywhere. IRQ context is safe for its
    // println: the console's IrqSafeMutex masks interrupts while held, so the interrupted
    // context on this core cannot be mid-print (the irq_notify argument, one lock over).
    // The answer is ignored on purpose: a sampling instrument may skip a beat when another
    // core's pass (or an arm) holds the gate, and the tick must never spin in IRQ context.
    let _ = canary::check();

    // **Timer expiry** (milestone 106, DECISIONS §147): one relaxed load, one counter read and one
    // compare when nothing is due, which is the cost `notes/timed-wait.md` priced before this was
    // built. Only a due tick takes `IPC_TABLES`, so four cores ticking at 100 Hz do not contend for
    // the whole-machine lock to find out that nothing happened.
    if inter_process_communication::timer::is_due(
        EARLIEST_DEADLINE.load(Ordering::Relaxed),
        crate::arch::timer::now(),
    ) {
        expire_timers();
    }

    // **The soak's cross-core hook** (milestone 221, DECISIONS 138's option D). A saturated
    // workload never migrates, because a rendezvous wake is local (§28.2), `wake_load_aware` is
    // reachable only from a device interrupt, and a work steal needs an idle core the machine does
    // not have. The timer is the one event that workload cannot starve, and this is the one
    // architecture-neutral place all three dispatchers already reach in real interrupt context, so
    // a soak build signals a rendezvous from here and its waiters take the whole real wake path
    // down through `irq_notify`. It compiles to nothing anywhere else; see kernel/src/soak.rs.
    #[cfg(feature = "soak_test")]
    crate::soak::signal_waiters();

    // **A kernel line held for the log service, signalled from a context that holds no lock**
    // (milestone 342 (the kernel and the `console` server drive one UART from two address
    // spaces)). One relaxed load when nothing is held, which is almost every tick. The print that
    // held it could not signal: it may have been printing under `IPC_TABLES`. See `kernel_log`.
    crate::kernel_log::signal_if_safe();
    #[cfg(feature = "console_flood")]
    crate::kernel_log::flood_tick();
}

/// The machine statistics page's half of a tick, out of line because every architecture's
/// exception dispatcher is in `script/fastpath-footprint`'s flat `syscall_entry` set and a tick is
/// not a syscall. Lock-free, like `charge_tick`: the idle tid and the run-queue length are this
/// core's own relaxed mirrors.
#[inline(never)]
fn count_tick() {
    let here = cpu::current();
    let idle = current_thread_id() == here.idle.load(Ordering::Relaxed);
    crate::machine_statistics::tick(
        here.switches.load(Ordering::Relaxed),
        idle,
        here.runnable() as u64 + u64::from(!idle),
    );
}

pub fn take_need_resched() -> bool {
    cpu::current().need_resched.swap(false, Ordering::Relaxed)
}

/// Install the incoming thread's cycle-counter grant on the core about to run it, immediately after
/// its address-space root (milestone 229, DECISIONS §139 option 4).
///
/// **Built only under `test` or `--features cycle_counter_grant`, and both the read (in `schedule`)
/// and this install are `#[cfg]`-gated at the switch site**, not carried through the shared switch
/// tuple. Milestone 139 threaded the grant through the tuple and called this unconditionally on every
/// arch, trusting the optimizer to fold the constant `false` away when no grant is built; milestone
/// 237 made the mechanism a feature and left the fold in place. Under the debug build the icount gate
/// measures, that fold does not happen: the const-`false` tuple element and the empty install stayed
/// in the shipping switch and cost `yield_switch` +35.6 and `ctx_switch` +36.0 ticks per switch on
/// aarch64 (milestone 300). This is the same leak milestone 299 found for the port grant, and the fix
/// is the same shape: `#[cfg]` the read and the install, keep the tuple at its pre-139 width. When the
/// feature is off `PMUSERENR_EL0` (and riscv64's `scounteren.CY`) keeps the closed value milestone 228
/// wrote at boot for the whole life of the kernel, and none of this code exists to touch it. Closing
/// what we claim is closed is right whether or not anyone can be granted an exception, which is why
/// 228's default write is NOT behind the feature and this is. `kernel/Cargo.toml`'s
/// `cycle_counter_grant` block carries the rest of the measurement.
#[cfg(any(test, feature = "system_tests", feature = "cycle_counter_grant"))]
fn install_cycle_counter_grant(granted: bool) {
    crate::arch::timer::set_cycle_counter_grant(granted);
}

/// Install the incoming thread's x86 port grant into the core about to run it, at the last point
/// `IPC_TABLES` is held; milestone 315 (a port revoke that reaches every core) moved it there and
/// the call site says why. The lazy write
/// lives in `arch::segments`.
///
/// **`x86_64` only, and both the read (in `schedule`) and this install are `#[cfg]`-gated at the
/// switch site**, not carried through the shared switch tuple. An earlier version threaded the value
/// through the tuple the way the cycle-counter grant then did and trusted the optimizer to fold a
/// constant `None` away on the other two architectures; it did not (icount measured `yield_switch`
/// and `ctx_switch` up 13% on aarch64). The cycle-counter grant turned out to leak the same way for
/// the same reason (the fold does not happen in the debug build the icount gate measures), and
/// milestone 300 gave it this exact treatment; see `install_cycle_counter_grant`. A port grant is
/// present in every x86 build, so keeping it off the other two ISAs' switch path takes a `#[cfg]`,
/// not a constant. See DECISIONS §152's x86-only rationale.
#[cfg(target_arch = "x86_64")]
fn install_port_grant(grant: Option<(u16, u16)>) {
    crate::arch::segments::set_port_range_grant(grant);
}

/// Pick another thread and go there.
///
/// May be called from normal context (a voluntary `yield_now`) or from the tail of the timer
/// IRQ handler (a preemption). The two paths are identical from here down, which is a large
/// part of why this is only forty lines.
// In the pinned hot section: milestone 796 (pin the hot trap path's placement).
#[cfg_attr(target_os = "none", unsafe(link_section = ".text.hot.sched.schedule"))]
pub fn schedule() {
    // **Not from an interrupt stack** (milestone 124). A switch parks the running `sp` in the
    // outgoing thread's `Context` and resumes it there, arbitrarily later; a per-core interrupt
    // stack cannot promise those bytes will still be the thread's, so a thread parked there would
    // resume on whatever the next interrupt on that core had written. The interrupt path defers its
    // switch to `preempt_if_needed`, which its dispatcher calls one frame outside the trampoline,
    // back on the interrupted thread's own stack.
    //
    // Debug-only because it costs a `sp` read and a scan of `MAX_CPUS` spans on the hottest path in
    // the kernel, and because `script/stack-depth-check` proves the same property statically in CI,
    // on both architectures, by showing no context switch is reachable from the interrupt-stack
    // entry point. This is the runtime half of that pair, for the edges a call graph cannot see.
    debug_assert!(
        !crate::interrupt_stack::contains(crate::arch::current_sp()),
        "schedule() called on an interrupt stack: the outgoing thread would be parked on memory \
         that belongs to a core (see kernel/src/interrupt_stack.rs)",
    );

    // Rule 2: no interrupts across the decision *or* the switch. Between "I chose a thread" and
    // "I am running it" there must be no window for the timer to choose again.
    //
    // The saved state is a local, on **this thread's stack**, which is exactly what makes it
    // correct: when someone eventually switches back to us, `switch_to` returns here, and this
    // frame (with the right `was_enabled` in it) is still sitting where we left it.
    let was_enabled = crate::arch::interrupts::disable();

    // The incoming thread's cycle-counter grant, carried out of the decision block the same way and
    // for the same reason (milestone 300, the fix milestone 299 gave the port grant just above).
    // Written inside the block under the lock, read at the install site after the lock drops; `false`
    // unless the block decides to switch. Built only when a grant can exist (`test` or
    // `--features cycle_counter_grant`), so every shipping build's `schedule()` gains nothing at all
    // and the switch tuple stays at its pre-139 width. See `install_cycle_counter_grant`.
    #[cfg(any(test, feature = "system_tests", feature = "cycle_counter_grant"))]
    let mut next_cycle_counter = false;

    // A labeled block, so every exit path leaves through the SAME point: the guard drops at the
    // block's end and interrupts are restored ONCE, AFTER it. The earlier version called
    // `interrupts::restore(was_enabled)` and `return` from *inside* this block, which re-enabled
    // interrupts while still holding `IPC_TABLES`: a one-instruction window in which a
    // timer could fire, re-enter `schedule()`, and try to take a lock we already held. It was
    // intermittent and it was real; see the lock-rank violation it produced.
    let switch = 'decide: {
        let mut guard = IPC_TABLES.lock();
        let Some(sched) = guard.as_mut() else {
            break 'decide None;
        };

        let current = current_thread_id();

        // **Forcible teardown, before anything else** (DECISIONS §16 amendment; §28 made it
        // load-bearing). A thread `DESTROY` marked killed must never run again. Convert it to a
        // `Finished` corpse here, at the top of the decision, so *every* path below reaps it: not
        // only the switch path, but the "nothing else to run, keep current" path a runaway alone on
        // its core takes. Before §28's scattering, a killed runaway shared a core with other work, so
        // a switch always happened and the old requeue-time check sufficed; once a runaway can be the
        // only thread on its core, that check was unreachable and the runaway spun forever.
        // The *test* stays here, on the hot path, because it is two loads and a compare. The
        // *body* is out of line (milestone 188 phase 3): converting a killed thread and freeing its
        // callers is a teardown, so its bytes have no business in the kernel's hottest function.
        if sched.threads.get(current).is_some_and(|t| {
            t.killed
                && (t.handshake.state == State::Running
                    || (t.handshake.state == State::Blocked && !t.handshake.wake_pending))
        }) {
            finish_killed_current(sched, current);
        }
        let state = sched.threads.get(current).map(|t| t.handshake.state);

        // **Only a still-Running thread goes back on the ready queue.** A thread that reached
        // here after marking itself `Blocked` (it is waiting for IPC), `Finished`, or `Dead` (a
        // supervised corpse) must not be rescheduled, and this one line is what makes blocking work:
        // `schedule()` can be called from the timer IRQ *while* a thread is mid-way through blocking
        // itself, and it must not undo that by helpfully requeueing it.
        let runnable = state == Some(State::Running);

        let idle_tid = cpu::current().idle.load(Ordering::Relaxed);

        let next = match cpu::current().with_runq(|q| q.pop_front()) {
            // The popped token goes home to its thread, which holds it while it runs, and the id is
            // what everything below uses.
            Some(t) => hold_token(t),
            None => {
                if runnable {
                    // Keep it. A thread yielding into an empty run queue simply carries on. (The
                    // idle thread lands here too: nothing to do, so it wfi's again.) No switch.
                    break 'decide None;
                }
                // Current is Blocked or Finished and the ready queue is empty. This is NOT a
                // deadlock: a thread blocked on a device interrupt is waiting for an event that
                // will arrive. Fall back to the idle thread, which wfi's until it does.
                if idle_tid == u64::MAX || current == idle_tid {
                    // No idle thread yet (before init finished), or the idle thread itself is
                    // somehow not runnable, which cannot happen. Either way there is genuinely
                    // nothing to run.
                    match state {
                        Some(State::Finished) => {
                            panic!("the last thread exited; nothing left to run")
                        }
                        _ => panic!("nothing runnable and no idle thread"),
                    }
                }
                idle_tid
            }
        };

        // **The running thread must never come off its own run queue** (boot 8's downstream
        // catastrophe, guarded here on its own merits). The pop above precedes the requeue below,
        // so a legal schedule can never hand back `current`; if it ever does, something queued a
        // thread that was still running, and switching into it would restore `t.context`, a
        // pointer to a frame this very thread has already resumed and consumed: execution
        // time-travels to its previous switch-out point on a reused stack and spins there
        // forever, off every instrument. Debug builds fail loudly; the board build heals by
        // keeping the thread running, which is the only state that is still coherent.
        if next == current {
            heal_self_pop(sched, current);
            break 'decide None;
        }

        // Requeue the outgoing thread if it can still run, but never the idle thread, which lives
        // outside the ready queue. A killed thread is already `Finished` (handled at the top), so it
        // is not runnable here and is reaped by `finish_switch` after the switch, no queue surgery.
        if runnable && current != idle_tid {
            // `preempt` marks Ready and deliberately leaves `on_cpu` set: the thread is in a
            // queue AND still standing on this core until finish_switch runs, which is the one
            // legal overlap and the reason only this core may pop its own queue (the extracted
            // protocol's steal-vs-switch-out rule; see crates/wake_handshake).
            let t = sched.threads.get_mut(current).unwrap();
            t.handshake.preempt();
            // The token a running thread holds on its own TCB. Round robin: the back.
            let Some(token) = t.own_token.take() else {
                missing_token()
            };
            cpu::current().with_runq(|q| q.push_back(token));
        }

        // Running, with on_cpu set until ITS successor's finish_switch, one switch from now.
        // **Did this thread just cross cores?** Recorded here rather than at any wake site, because
        // this is the one place every path to a CPU passes through, whatever moved the thread: a
        // rendezvous wake onto the waker's core, a work steal, a load-aware placement, or spawn.
        // See `thread::Thread::last_cpu` for why the placement counter could not answer this, and
        // for why this is behind `feature = "soak_test"`: it is the hottest line of the hottest
        // function, and shipping it everywhere put `ipc_fastpath` over milestone 132's bound.
        #[cfg(feature = "soak_test")]
        {
            let here = cpu::id();
            let t = sched.threads.get_mut(next).unwrap();
            let from = t.last_cpu;
            t.last_cpu = here as u8;
            if from != u8::MAX && from as usize != here {
                trace::record(trace::Event::Migrated, next, from);
            }
        }
        let next_tcb = sched.threads.pointer(next).unwrap();
        // SAFETY: the page pointer of a live thread, and `IPC_TABLES` (held) serializes every
        // access to its `Thread`. One lookup serves the handshake and `set_current` both.
        unsafe { (*next_tcb).handshake.switch_in() };
        // SAFETY: `next_tcb` is `next`'s page pointer from the table, live.
        unsafe { set_current(next, next_tcb) };
        trace::record(trace::Event::SwitchTo, next, 0);

        // Hand the outgoing thread to the incoming one to finish up AFTER the switch, when it is
        // provably off its stack: reap it if it Finished, clear its on_cpu (and complete a
        // deferred wake) otherwise. Not here, and not by another core: we are still running on
        // its stack this instant. `current` is the local (the outgoing tid); `set_current`
        // above already moved the per-CPU current to `next`. See finish_switch.
        //
        // **And count the switch, through the same block** (milestone 629 (the context-switch statistic stops costing the switch path)): `vmstat`'s `cs` is this
        // word, copied to the machine statistics page by the tick, so the switch pays one add on a
        // block it is already writing rather than a walk to the page. See `PerCpu::switches`.
        let here = cpu::current();
        here.switched_from.store(current, Ordering::Relaxed);
        here.switches.fetch_add(1, Ordering::Relaxed);

        // The incoming thread's low half. A kernel thread gets the empty reserved table, which
        // makes every low address fault, which is exactly right: it has no business down there.
        //
        // **And its current-CPU page, off the same borrow**, which is the whole of this kernel's
        // half of calef's 2026-09-21 ruling that a thread observing itself is a page rather than a
        // crossing. Here rather than at any wake or placement site for `last_cpu`'s reason one
        // field over: this is the one point every path to a CPU passes through, whatever moved the
        // thread. And on this core rather than the one that queued it, which is what makes writer
        // and reader the same hardware thread and the ordering argument a short one
        // (`crates/current_cpu_protocol`). A kernel thread has no space and needs no page: it has
        // no userspace to read one from.
        let next_root = match sched.threads.get(next).unwrap().space.as_ref() {
            Some(space) => {
                space.publish_current_cpu(cpu::id() as u64);
                space.ttbr0()
            }
            None => crate::arch::mmu::reserved_root(),
        };

        // The incoming thread's cycle-counter grant (milestone 229, DECISIONS §139 option 4), read
        // here for the same reason the root is: this is the last point the lock is held. A kernel
        // thread's is `false`, like every user thread nobody granted it to. Written into the
        // `#[cfg]`-gated variable declared above the block, not the switch tuple, so it costs the
        // shipping build nothing at all (milestone 300; see `install_cycle_counter_grant` for why the
        // old tuple-and-fold did not fold in the debug build the icount gate measures).
        #[cfg(any(test, feature = "system_tests", feature = "cycle_counter_grant"))]
        {
            next_cycle_counter = sched.threads.get(next).unwrap().cycle_counter_grant;
        }

        // Copy the raw pointers out before the lock drops. The assembly writes through the
        // context slot and reads the incoming context, and both threads' TCB pages keep their
        // contents pinned.
        //
        // **The TCB page pointer rather than `get_mut`** (milestone 447), and it is one probe per
        // side rather than the two the obvious shape would cost. Each thread's FP register file
        // lives in the free space of the same page, past the end of the struct, so a `&mut Thread`
        // is exactly the wrong thing to hold: its provenance stops at the struct.
        // `Threads::pointer` hands back the page cast unnarrowed and both addresses fall out of it.
        let prev_ptr = sched.threads.pointer(current).unwrap();

        // The thread pointer, from the outgoing thread to the incoming one (milestone 812), here
        // because nothing returns to user mode before the switch; `arch::thread_pointer` has each
        // architecture's half.
        //
        // SAFETY: live threads' TCB pages from the table, under `IPC_TABLES`; one field each.
        unsafe {
            crate::arch::thread_pointer::hand_over(
                &mut (*prev_ptr).thread_pointer,
                (*next_tcb).thread_pointer,
            );
        }

        // SAFETY: a live thread's TCB page, held under IPC_TABLES. Field projection through a raw
        // pointer, and `fp_state_of`'s contract is exactly what `pointer` returns.
        let (prev_slot, prev_fp): (*mut *mut Context, *mut crate::arch::fp::FpState) = unsafe {
            (
                &raw mut (*prev_ptr).context,
                crate::thread::fp_state_of(prev_ptr),
            )
        };

        // `next_ctx`, and on x86 the incoming thread's port grant (milestone 299) beside it, out of
        // ONE lookup. The two arms are byte-identical bar the port read, and the split is
        // deliberate: the port read goes into the `x86_64`-only variable declared above the block,
        // not the switch tuple, so it costs the other two architectures nothing at all. The
        // cycle-counter grant is carried the same way, by milestone 300 (decompose the icount
        // baseline drift, and re-baseline only what is proven), so neither grant widens this
        // tuple: it is back to its pre-139 width `(prev_slot, next_ctx, next_root)` on every
        // shipping build, plus milestone 447's two register-file pointers.
        #[cfg(not(target_arch = "x86_64"))]
        // SAFETY: as `prev_ptr`; `next` was popped off this core's run queue and marked `Running`
        // inside this same locked block.
        let (next_ctx, next_fp): (*mut Context, *const crate::arch::fp::FpState) = unsafe {
            let next_ptr = sched.threads.pointer(next).unwrap();
            (
                (*next_ptr).context,
                crate::thread::fp_state_of(next_ptr).cast_const(),
            )
        };
        #[cfg(target_arch = "x86_64")]
        // SAFETY: as the arm above, plus the port grant, which is a plain field read.
        let (next_ctx, next_fp, next_port_grant): (
            *mut Context,
            *const crate::arch::fp::FpState,
            Option<(u16, u16)>,
        ) = unsafe {
            let next_ptr = sched.threads.pointer(next).unwrap();
            (
                (*next_ptr).context,
                crate::thread::fp_state_of(next_ptr).cast_const(),
                (*next_ptr).port_range_grant,
            )
        };

        // **The incoming thread's authority to reach x86 I/O ports from ring 3, installed here and
        // not after the lock drops** (milestone 315; it was beside the address-space install until
        // then). The TSS I/O-bitmap grant is the one per-core fact a *revocation* has to be able to
        // take back from a core it is not running on, and `delete_port_range_caps_impl` does that by
        // NMI while holding this lock. Reading the grant under the lock and installing it outside
        // left two windows that broadcast could not close: a core could install a grant the sweep had
        // already cleared, and the NMI could land in the middle of the install. Both shut if every
        // writer of a bitmap holds `IPC_TABLES`, which costs, on a machine where nothing holds a
        // port, one compare inside the critical section instead of one outside it.
        //
        // Nothing can execute a ring-3 `in`/`out` between here and the switch: this core is in the
        // kernel with interrupts masked the whole way.
        #[cfg(target_arch = "x86_64")]
        install_port_grant(next_port_grant);

        Some((prev_slot, next_ctx, next_root, prev_fp, next_fp))
    };
    // Rule 1: THE LOCK IS RELEASED HERE, before the switch. Holding it across `switch_to` would
    // leave it held by a thread that is not running, and the next thread to want it would spin
    // forever waiting for a thread that can only be scheduled by taking the lock.

    if let Some((prev_slot, next_ctx, next_root, prev_fp, next_fp)) = switch {
        // Install the incoming thread's address space FIRST. `TTBR0_EL1` is one register, shared
        // by everybody, and a thread that resumes at EL0 in the previous thread's low half is
        // running a stranger's code. (No-ops, including no TLB flush, when the root is already
        // right, which is every switch between two kernel threads.)
        //
        // SAFETY: `next_root` is `reserved_root()`, or the composed value of the `AddressSpace`
        // owned by thread `next`, which the block above popped off this core's run queue and marked
        // `Running` with `on_cpu` set before releasing `IPC_TABLES`. No other core can pick it up in that
        // state and nothing reaps a thread that is on a CPU, so the root is still live here even
        // though the lock is not held. The lock is released on purpose (rule 1, above), which is
        // exactly why this obligation cannot be a borrow and has to be a sentence.
        unsafe { crate::arch::mmu::switch_user_root(next_root) };

        // And the incoming thread's authority to read the cycle counter, which is the same kind of
        // fact about the same instant: one register, shared by everybody, that has to say what the
        // thread about to run was granted rather than what the last one was. Needs no `unsafe`,
        // because unlike `TTBR0_EL1` this register names no memory and points at nothing that can
        // be freed: getting it wrong opens or closes a counter, it does not hand a thread a
        // stranger's pages. Costs a compare when the value already matches, which is every switch
        // on a machine where nothing is granted. `#[cfg]`-gated, not folded (milestone 300): it
        // exists only in a build that can grant the counter, the same shape as the port grant just
        // below. See `arch::timer::set_cycle_counter_grant` and `install_cycle_counter_grant`.
        #[cfg(any(test, feature = "system_tests", feature = "cycle_counter_grant"))]
        install_cycle_counter_grant(next_cycle_counter);

        // And the register file the two threads are about to share a core over (milestone 447).
        // This is beside `switch_to` rather than inside it because the two save different
        // quantities for different reasons: `switch_to` saves what a *function call* may destroy,
        // and this moves what a *thread* owns. It runs here, as the outgoing thread, with the lock
        // released and interrupts masked, so the register file is already right when the switch
        // lands. `crate::fp::hand_over` has the four cases and the CVE that decided them.
        //
        // SAFETY: `prev_fp` and `next_fp` name the `FpState`s of the outgoing and incoming threads,
        // pinned by the same argument the two lines below make for their contexts.
        unsafe { crate::fp::hand_over(prev_fp, next_fp) };

        // SAFETY: both pointers name live `Context`s owned by boxed `Thread`s in the map, and
        // interrupts are masked so nothing can reorder underneath us.
        //
        // This call does not return here. It returns *in another thread*, at the point where
        // that thread last called `switch_to`. We come back only when somebody switches to us.
        unsafe { switch_to(prev_slot, next_ctx) };

        // We are now the incoming thread, resuming. Reap whoever we switched away from, if it had
        // finished: it is off its stack now, and we are on the same core that set `to_reap`.
        finish_switch();
    }

    crate::arch::interrupts::restore(was_enabled);
}

/// **Convert a killed current thread into a corpse, and free the callers it can no longer answer.**
/// The body of [`schedule`]'s forcible-teardown check, out of line (milestone 188 phase 3): the
/// predicate is two loads on the kernel's hottest path, the body is a teardown that no IPC reaches.
///
/// A thread `DESTROY` marked killed must never run again, and this runs at the top of the decision
/// so *every* path below reaps it: not only the switch path, but the "nothing else to run, keep
/// current" path a runaway alone on its core takes. Before §28's scattering a killed runaway shared
/// a core with other work, so a switch always happened and the old requeue-time check sufficed;
/// once a runaway can be the only thread on its core, that check was unreachable and the runaway
/// spun forever.
///
/// A killed thread never reaches `depart`, so this is the one place its callers can be freed before
/// its capability table goes (milestone 254).
///
/// Caller holds `IPC_TABLES` and has already checked that `current` is killed and `Running`.
#[cold]
#[inline(never)]
fn finish_killed_current(sched: &mut IpcTables, current: ThreadId) {
    // A killed thread that has just parked itself (milestone 812: a process member its process's
    // end marked killed while it ran) is unlinked from what it waits on, since nothing will wake it.
    if sched
        .threads
        .get(current)
        .is_some_and(|t| t.handshake.state == State::Blocked)
    {
        finish_blocked_resident(sched, current);
    }
    if let Some(t) = sched.threads.get_mut(current) {
        t.handshake.state = State::Finished;
    }
    strand_callers_of(sched, current);
    process::on_killed(sched, current); // a supervised member reports its process's end
}

/// **The running thread came off its own run queue, which cannot legally happen.** [`schedule`]'s
/// guard against boot 8's downstream catastrophe, out of line (milestone 188 phase 3) because it is
/// unreachable on every path this kernel is meant to take.
///
/// The pop precedes the requeue, so a legal schedule can never hand back `current`; if it ever
/// does, something queued a thread that was still running, and switching into it would restore
/// `t.context`, a pointer to a frame this very thread has already resumed and consumed: execution
/// time-travels to its previous switch-out point on a reused stack and spins there forever, off
/// every instrument. Debug builds fail loudly; the board build heals by keeping the thread running,
/// which is the only state that is still coherent.
#[cold]
#[inline(never)]
fn heal_self_pop(sched: &mut IpcTables, current: ThreadId) {
    if cfg!(debug_assertions) {
        panic!("schedule() popped its own current thread from the run queue");
    }
    if let Some(t) = sched.threads.get_mut(current) {
        t.handshake.state = State::Running;
    }
}

/// Reap the thread this core just switched away from, if it had finished.
///
/// The safe half of the two-part reaper. `schedule()` records a finished outgoing thread in this
/// core's `to_reap` *before* the switch; this runs on the incoming thread *after* the switch, when
/// the outgoing thread is provably off its stack (its registers are saved and we are on a
/// different stack). Dropping the `Thread` unmaps its stack and frees its address space, which is
/// exactly why it must not happen while any core still stands on it.
///
/// Called from two places, because a thread can resume two ways: from `schedule()` (an existing
/// thread returning from `switch_to`) and from `thread_entry` (a brand-new thread, which never
/// passes through `schedule()`'s post-switch point). Both run on this core, so both see this core's
/// `to_reap`. See DECISIONS §11 and thread.rs.
// In the pinned hot section: milestone 796 (pin the hot trap path's placement).
#[cfg_attr(
    target_os = "none",
    unsafe(link_section = ".text.hot.sched.finish_switch")
)]
pub(crate) fn finish_switch() {
    let prev = cpu::current()
        .switched_from
        .swap(cpu::NO_TID, Ordering::Relaxed);
    if prev == cpu::NO_TID {
        return;
    }
    let mut guard = IPC_TABLES.lock();
    let Some(sched) = guard.as_mut() else {
        return;
    };
    let Some(t) = sched.threads.get_mut(prev) else {
        return;
    };
    // The predecessor's context is saved now (we are running, so switch_to completed). What that
    // makes legal is `thread_wake_handshake::Handshake::finish_switch`'s verdict: reap a Finished
    // predecessor, complete a wake that was deferred mid-switch-out, or simply clear `on_cpu` so
    // other cores may run it. The transition is the crate's (loom searches it; see
    // notes/interleaving.md); the reap and the queue push are ours.
    let verdict = t.handshake.finish_switch();
    match verdict {
        // Out of line, and `#[cold]` says why: a switch-out reaps only when the outgoing thread
        // FINISHED, which no IPC does. Left inline its bytes sat in `finish_switch`, which is on
        // both IPC closures, and `script/fastpath-footprint` could only exclude the *callees* it
        // reaches (`KernelStack`, `AddressSpace`, `drop_in_place`) and not the arm's own setup.
        // Milestone 188 phase 3: the classification now lives in the code rather than in a regex.
        SwitchOutVerdict::Reap => reap_switched_out(guard, prev),
        SwitchOutVerdict::WakeCompleted => {
            trace::record(trace::Event::WakeCompleted, prev, 0);
            // A deferred wake was deferred precisely because the waker did NOT queue it, so the
            // token is still on the thread. IRQs are still masked on both callers' paths.
            let Some(token) = t.own_token.take() else {
                missing_token()
            };
            cpu::current().with_runq(|q| q.push_back(token));
        }
        SwitchOutVerdict::Cleared => {}
    }
}

/// **Reap the thread this core just switched away from.** The `Reap` arm of [`finish_switch`],
/// out of line because it runs when a thread has ended and never on an IPC.
///
/// **Two short critical sections with the expensive part between them**, since 2026-10-04.
///
/// 1. Under the caller's `IPC_TABLES` guard: take the thread's kernel stack and its address space
///    out, and mark it [`being_reaped`](crate::thread::Thread::being_reaped). It stays in the
///    table, `Finished`, so its name still resolves and region teardown still refuses it.
/// 2. With no lock held: free the stack, then the address space. The stack is six page unmaps,
///    each discharging its TLB obligation; on riscv64 every one is an SBI remote fence that
///    interrupts every other hart and waits for them, and on x86_64 an NMI shootdown round. Until
///    2026-10-04 the stack was freed under `IPC_TABLES`, so every IPC and every capability lookup
///    on every other core waited behind a thread exiting anywhere. The job mix measured it as the
///    cheapest syscall nearly doubling in cost once four cores were busy
///    (notes/job-mix/null-syscall-under-load.md). The address space could never be dropped under
///    the lock: its teardown is `memory_region::destroy` (milestone 14 (kernel objects from
///    untyped) phase B.4), whose §13 revocation sweep takes `IPC_TABLES` itself.
/// 3. Under `IPC_TABLES` again: remove the thread. Only now does it stop occupying its region.
///
/// **Everything the thread owned is gone before step 3, and that order is the point.** Region
/// teardown (`reclaim_region`) relies on a resident thread's space being gone before the thread is
/// (its `being_reaped` refusal holds the region until step 3), and an owner whose `DESTROY` succeeds
/// may reuse the memory at once. Before
/// 2026-10-04 the space was dropped just after the thread was removed, so a `DESTROY` on another
/// core could reclaim the region in between, while the space's page tables (in that region) and
/// its revocation-database entries were still live. The rest of the `Thread` (its quota token, its
/// capability table) still drops under the lock in step 3.
///
/// Nothing else removes a `Finished` thread between steps 1 and 3: the only other remover is region
/// teardown, and `region_reap_verdict` refuses a thread that is being reaped.
///
/// **Since §249 the space is taken out of the address-space registry in step 2**, by the name in the
/// thread's copy, and the region sweep may have taken it already (this thread is a corpse off its
/// stack); `Table::remove` hands it to one of the two.
///
/// # BUGS
///
/// - **A corpse whose TCB is outside region R and whose root is inside it still has a narrow
///   window.** If this reaper takes the space in step 2 and a `DESTROY(R)` on another core runs
///   wholly before the drop finishes, the sweep finds no entry, R comes back, and the drop's
///   `revoke::forget_root` can land on a root that page has been given to since. The sweep taking
///   the space first is the common order and is safe; this is the other order. Before §249 it was open for the whole life of an unreaped corpse rather than for one drop. It is
///   handed to
///   milestone 765 (a destroyed region cannot free the root a running thread walks), whose refusal
///   is the natural place to count a space that is still being dropped.
#[cold]
#[inline(never)]
fn reap_switched_out(guard: crate::sync::IrqSafeGuard<'_, Option<IpcTables>>, prev: ThreadId) {
    let process = guard
        .as_ref()
        .and_then(|s| s.threads.get(prev))
        .and_then(|t| t.process);
    process::reap_corpse(guard, prev);
    if let Some(pid) = process {
        process::reap_corpses(pid); // members its end finished where they were blocked
    }
}

/// Move a blocked thread back to the ready queue. Caller holds the lock.
///
/// The decision lives in `thread_wake_handshake::Handshake::try_wake`, extracted so loom can search its
/// interleavings on the host (notes/interleaving.md); this function is the kernel-side half, the
/// queue push and the trace ring. The two rules the verdicts carry, kept here in one breath
/// because this is where a reader meets them: **the undelivered-wake gate** (boot 8: a wake whose
/// critical section delivered nothing has not dequeued the thread from its rendezvous and has
/// nothing for its receive to return, so it is refused and recorded as `refuse:tid` on the ring), and
/// **the wake-before-switch-out deferral** (a thread still on its CPU has a stale saved context,
/// so the wake parks in `wake_pending` and its own core's `finish_switch` completes it once the
/// context is provably saved; found by a 2-in-10 flake, notes/intrusive-queues.md).
// In the pinned hot section: milestone 796 (pin the hot trap path's placement).
#[cfg_attr(target_os = "none", unsafe(link_section = ".text.hot.sched.wake"))]
fn wake(sched: &mut IpcTables, tid: ThreadId) {
    if let Some(t) = sched.threads.get_mut(tid) {
        match t.handshake.try_wake() {
            WakeVerdict::NotBlocked => {}
            WakeVerdict::Refused => {
                trace::record(trace::Event::WakeRefused, tid, 0);
            }
            WakeVerdict::Deferred => {
                // A completed rendezvous is forward progress even when its queueing is deferred:
                // keep the hang watchdog's heartbeat alive so a slow-but-live IPC pipeline
                // (std_net) is not read as a deadlock (test builds only).
                #[cfg(any(test, feature = "system_tests"))]
                crate::testing::note_progress();
                trace::record(trace::Event::WakeDeferred, tid, 0);
            }
            WakeVerdict::Queue => {
                #[cfg(any(test, feature = "system_tests"))]
                crate::testing::note_progress();
                // Blocked -> Ready: whatever unlinked it handed its token back (`hold_token`), so
                // it is on the thread; a `None` is a thread still linked into a wait queue.
                let Some(token) = t.own_token.take() else {
                    missing_token()
                };
                trace::record(trace::Event::Wake, tid, 0);
                // Onto this core's queue: a rendezvous wake stays local on purpose (§28.2), the
                // message is in registers and the cache is warm. Every caller (ipc_*, irq_notify)
                // holds IPC_TABLES, so interrupts are masked.
                cpu::current().with_runq(|q| q.push_back(token));
            }
        }
    }
}

/// **Record that the running thread has started using the floating-point unit** (milestone 447).
///
/// The scheduler half of [`crate::fp::enable_for_current`], and the only thing that ever sets the
/// flag. Returns false when there is no scheduler or no current thread, which is a trap the caller
/// must turn into a fault rather than return from: the flag is what stops the same instruction
/// taking the same trap forever.
///
/// **Taking `IPC_TABLES` inside a trap handler is ordinary here**, not a liberty. Every syscall
/// does it from the same place, through `syscall::dispatch`, and the argument is the same: this
/// trap came from a thread that was *running*, so this core cannot already hold the lock. What
/// would break that is kernel code executing an FP instruction while holding it, and the kernel is
/// built `softfloat` and executes none.
pub fn mark_current_fp_live() -> bool {
    let mut guard = IPC_TABLES.lock();
    let Some(sched) = guard.as_mut() else {
        return false;
    };
    let current = current_thread_id();
    let Some(pointer) = sched.threads.pointer(current) else {
        return false;
    };
    // The register file is in the TCB page beside the struct, so this goes through the page pointer
    // rather than through a `&mut Thread`; see `Threads::pointer`.
    //
    // SAFETY: a live thread's TCB page, held under IPC_TABLES.
    unsafe { (*crate::thread::fp_state_of(pointer)).set_live() };
    true
}

pub fn count_preemption() {
    PREEMPTIONS.fetch_add(1, Ordering::Relaxed);
    PREEMPTIONS_PER_CPU[cpu::id()].fetch_add(1, Ordering::Relaxed);
}

/// **The deferred half of preemption**: switch away if this core's tick asked for it.
///
/// Called by both ISAs' trap dispatchers *after* the handler has returned to the interrupted
/// thread's own stack, which is the whole reason it is a function rather than four lines at the
/// bottom of `handle_irq` where it lived until milestone 124. The handler may have run on this
/// core's interrupt stack, and `schedule()` may only be called on a stack the interrupted thread
/// owns; see `kernel/src/interrupt_stack.rs`.
///
/// Portable, and identical on both architectures, which is the other half of why it moved: the four
/// lines were written twice and had drifted in their comments already.
pub fn preempt_if_needed() {
    if take_need_resched() && is_running() {
        count_preemption();
        schedule();
    }
}

pub fn is_running() -> bool {
    IPC_TABLES.lock().is_some()
}

#[cfg(test)]
mod tests;
