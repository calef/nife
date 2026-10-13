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
#[path = "sched/trace_disabled.rs"]
mod trace;

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
#[path = "sched/canary_disabled.rs"]
mod canary;

#[cfg(test)]
mod tests;

mod current;
mod departure;
mod ipc_outcome;
mod preemption;
mod switch;
mod wake;

use current::*;
use departure::*;
pub use departure::{exit, exit_thread, fault, yield_now};
use ipc_outcome::*;
pub use ipc_outcome::{take_ipc_aborted, take_ipc_refused};
use preemption::*;
pub use preemption::{on_tick, preempt_if_needed};
pub(crate) use switch::finish_switch;
use switch::*;
pub use switch::{init, is_running, mark_current_fp_live, schedule};
use wake::*;
