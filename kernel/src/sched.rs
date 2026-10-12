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
// On 2026-10-12 (UTC) the tables (`ipc_tables`) and kernel-thread spawning (`spawning`) followed,
// for §282 (when a file is split, the parent becomes an index). The scheduler core stays:
// `script/fastpath-footprint` finds `schedule` and `finish_switch` by their `sched::` path, the
// fastpath helpers beside them inline into it, and `syscall::dispatch` reaches `exit` and
// `yield_now` here. `init` stays too: moving it moved a drop glue off aarch64's pinned hot section.
mod capabilities;
mod configure;
mod futex;
mod inspection;
mod interrupt_route;
mod ipc_tables;
mod notification;
mod placement;
mod process;
mod region_reaping;
mod rendezvous;
mod spawning;
mod switch_decision;
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
pub use ipc_tables::*;
pub use notification::*;
pub use placement::*;
pub use process::{ProcessId, create_process_from, destroy as destroy_process, with_holders};
#[cfg(feature = "system_tests")]
pub use process::{process_state, process_threads_started, reap_corpses_now, strand_member};
pub use region_reaping::*;
pub use rendezvous::*;
pub use spawning::*;
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
/// 2026-08-15; module name provisional). Armed around the initial ramdisk demo on the board tour,
/// it re-reads the watched ranges on the timer tick and prints every byte that changed since the
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
// hang watcher inside `kernel::user::riscv_initial_ramdisk_demo`, which armed it around that demo's
// blocking receive; calef retired the program that demo loaded and the function went with it. This
// is the `AGENTS.md` "an exception is allowed and must say so" case, written where a reader meets
// it: the instrument stays because it is how a board hang gets diagnosed, it was the evidence that
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
    // The corruption tripwire, when armed (the board tour's initial ramdisk demo window). One
    // relaxed load when it is not, which is every other tick everywhere. IRQ context is safe for
    // its println: the console's IrqSafeMutex masks interrupts while held, so the interrupted
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
/// **`x86_64` only, and both the read (in `switch_operands`) and this install are `#[cfg]`-gated
/// at the switch site**, not carried through the shared switch tuple. An earlier version threaded
/// the value through the tuple the way the cycle-counter grant then did and trusted the optimizer
/// to fold a constant `None` away on the other two architectures; it did not (icount measured
/// `yield_switch` and `ctx_switch` up 13% on aarch64). The cycle-counter grant turned out to leak
/// the same way for the same reason (the fold does not happen in the debug build the icount gate
/// measures), and milestone 300 (decompose the icount baseline drift) gave it this exact treatment;
/// see `install_cycle_counter_grant`. A port grant is present in every x86 build, so keeping it off
/// the other two ISAs' switch path takes a `#[cfg]`, not a constant. See the x86-only rationale in
/// DECISIONS §152 (the port-range capability).
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

        // The pick: this core's queue, the idle thread when it is empty, and the outgoing thread
        // requeued if it can still run. `None` keeps the current thread with no switch.
        let Some(next) = switch_decision::choose_next(sched, current) else {
            break 'decide None;
        };

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

        // The incoming thread's cycle-counter grant (milestone 229, DECISIONS §139 option 4), read
        // here for the same reason `switch_operands` reads the root: this is the last point the
        // lock is held. A kernel thread's is `false`, like every user thread nobody granted it to.
        // Written into the `#[cfg]`-gated variable declared above the block, not the switch tuple,
        // so it costs the shipping build nothing at all (milestone 300; see
        // `install_cycle_counter_grant` for why the old tuple-and-fold did not fold in the debug
        // build the icount gate measures).
        #[cfg(any(test, feature = "system_tests", feature = "cycle_counter_grant"))]
        {
            next_cycle_counter = sched.threads.get(next).unwrap().cycle_counter_grant;
        }

        // The incoming root and the four raw pointers the switch needs, read under the lock.
        // SAFETY: the lock is held and interrupts are masked, both tids are live, and `next_tcb`
        // is `next`'s page pointer, switched in and made current just above.
        Some(unsafe { switch_decision::switch_operands(sched, current, next, next_tcb) })
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
