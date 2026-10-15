//! **The tables behind `IPC_TABLES`**: [`IpcTables`] itself, the one lock it sits behind, the
//! limits that size it ([`MAX_THREADS`], [`MAX_RENDEZVOUS`], the futex buckets and the kernel's
//! endpoint chunks), and the rendezvous-pressure instrument.
//!
//! Cut out of `sched.rs` unchanged on 2026-10-12 (UTC) by lane maintainer/near-ceiling-splits, so
//! the parent stays under §266 (a Rust source file stays under 2,000 lines) and moves toward §282
//! (when a file is split, the parent becomes an index). The fields are `pub(super)`, which in a child
//! of `sched` is the visibility a private field of `sched` had: `sched` and its children, the same
//! set §271 (the scheduler file splits into submodules of `sched`) kept the table to. The functions
//! that read the tables on the IPC fastpath stay in `sched.rs`, beside `schedule`.
//!
//! Name: provisional (lane maintainer/near-ceiling-splits, 2026-10-12 UTC), for the module and the
//! file.

use super::*;

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
pub(super) type Rendezvous = inter_process_communication::Rendezvous<Thread>;

/// The most threads that can be alive at once, whole machine: a documented limit of the image
/// rather than a heap that can be exhausted, so spawn past it fails cleanly, as out-of-memory does.
///
/// **Why 256.** The boot keeps many services alive for its whole life (`notes/frames.md`'s "held"
/// list), and every milestone that adds one spends some of this. [`PEAK_THREADS`] and
/// `kernel::testing`'s closing `threads:` line print the real demand on every run, which was a
/// peak of 130 (aarch64), 129 (riscv64) and 57 (`x86_64`) when this was set: 256 is about twice
/// the tightest, because the failure mode is an unrelated test refusing a spawn far from the cause.
///
/// What scales with it, each checked at 256:
///
/// - The boot stack, about 20 bytes a slot: [`init`] installs [`EMPTY_TABLES`] rather than building
///   a `Threads` local, keeping the boot frame under `stack::report_high_water`'s gate.
/// - `kmem::KERNEL_OBJ_PAGES`, 7 pages per *live* thread (`thread::STACK_PAGES` plus the TCB page).
/// - `ps::MAX_ROWS`, const-asserted `>= MAX_THREADS` in `kernel::user::survey_tests`, which sizes
///   stack-resident row arrays in `ps` and `pgrep` well inside
///   `system_initializer::CHILD_STACK_PAGES`.
/// - `revoke::MAX_SPACES` and `thread::FreeAddressSpace`, derived from this constant.
/// - Nothing on the `spawn_el0` benchmark: the scans that used to track this constant
///   (`delete_page_frame_caps_where`, `revoke`'s registry) are bounded by live occupancy, so the
///   benchmark is flat against it (notes/benchmarks.md). Re-baselining would have charged every
///   future raise for empty slots.
/// - No `[u64; MAX_THREADS]` scratch array on a stack: [`reap_region_objects`] rescans, keeping its
///   frame under `script/stack-frame-check`'s guard-page ceiling.
///
/// # BUGS
///
/// - Revocation still scans every *live* thread; milestone 183 (a physical-range index for
///   capability holders) is the structural fix.
/// - The peak is a whole-boot high-water, not a per-test charge: it cannot name the test that
///   pushed it (`kernel::testing`'s `report_thread_peak`).
pub const MAX_THREADS: usize = 256;

pub(super) struct IpcTables {
    /// The thread table: generational names over page-resident TCBs. See [`Threads`];
    /// design/kernel-objects-from-untyped.md D2 records the path, notes/thread-control-block.md the storage.
    pub(super) threads: Threads,
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
    pub(super) rendezvous_table: generational_table::Table<u64, MAX_RENDEZVOUS>,
    /// The kernel's **current** object chunk: where the kernel's endpoints (boot services, tests)
    /// are retyped from, so every rendezvous lives uniformly in a pinned page regardless of who paid.
    /// Carved lazily on the first [`create_rendezvous`] and **replaced when it fills**, which is what
    /// makes the kernel's rendezvous supply grow instead of being a compile-time guess.
    ///
    /// A filled chunk's handle is deliberately forgotten. Its pages stay pinned and its endpoints
    /// stay live, and nothing ever hands a kernel chunk back: kernel endpoints are destroyed only by
    /// tearing down the region hosting them (see the `doomed_eps` walk), and no path tears down a
    /// kernel chunk. If one ever should, this becomes an array and that is the change to make.
    pub(super) kernel_ep_region: Option<u64>,
    /// How many chunks have been carved, so growth is bounded by something rather than by nothing.
    pub(super) kernel_ep_chunks: usize,
    /// **The notification registry** (milestone 151 (notification objects), DECISIONS §101 (notification objects)): the rendezvous registry's
    /// shape one object type over. Each entry is the physical address of the page a
    /// [`NotificationPage`] lives at the start of, retyped from its creator's region; the
    /// generational name is what an `Object::Notification` capability carries.
    pub(super) notification_table: generational_table::Table<u64, MAX_NOTIFICATIONS>,
    /// **The timer registry** (milestone 106 (a wait that ends on either the interrupt or the
    /// deadline), DECISIONS §147 (a timer a userspace service cannot hold)): the notification
    /// registry's shape one object type over. Each entry is the physical address of the page a
    /// [`TimerPage`] lives at the start of; the expiry walk iterates it.
    pub(super) timer_table: generational_table::Table<u64, MAX_TIMERS>,
    /// **The futex wait queues** (milestone 812 (`std::thread::spawn` runs real threads in one
    /// address space), §269 (how threads share a process) fork 2): threads parked in
    /// `AddressSpace::WAIT`, by `(space, address)`. Under this lock for the reason every wait queue
    /// here is: the check of the word, the park, and a waker's take must be one order. See
    /// [`futex_wait`].
    pub(super) futexes: inter_process_communication::futex::Futexes<Thread, FUTEX_BUCKETS>,
    /// The process registry (milestone 812): each entry the physical address of a process's page.
    pub(super) processes: generational_table::Table<u64, { process::MAX_PROCESSES }>,
}

/// How many buckets the futex table hashes into. A wake walks one bucket, so this bounds the walk
/// at `MAX_THREADS / FUTEX_BUCKETS` waiters on average; 64 is four threads a bucket with the machine
/// full. Twenty-four bytes each.
pub(super) const FUTEX_BUCKETS: usize = 64;

/// The most endpoints that can exist **at once**: the registry's bound.
///
/// This used to say it capped creations over the kernel's lifetime, on the grounds that rendezvous
/// teardown did not exist. That went stale when object revocation made destruction real: tearing
/// down a region removes every rendezvous whose page lives in it and `generational_table::Table::remove` frees the
/// slot for reuse, so this is a concurrent bound now. Corrected rather than left, because a stale
/// bound is the kind of comment that gets believed during a capacity argument.
pub(crate) const MAX_RENDEZVOUS: usize = 512;

/// **The most rendezvous that were ever live at once on this boot**, and how many of those live at
/// that moment sat on the kernel's own chunks. [`MAX_RENDEZVOUS`]'s instrument, built by milestone
/// 601 (the region table prints its peak). A rendezvous on a kernel chunk lives for the whole boot
/// (`kernel_ep_region` never frees one); one retyped from a region goes when its region is
/// reclaimed, so the kernel-chunk count only rises. Both are updated under `IPC_TABLES` at the one
/// insert, so they never race.
///
/// # The ledger at the peak
///
/// Measured 2026-09-26:
///
/// | architecture | peak | spare | on kernel chunks | retyped from a region |
/// |---|---|---|---|---|
/// | aarch64 | **505** | 7 | 466 | 39 |
/// | riscv64 | **491** | 21 | 452 | 39 |
/// | `x86_64` | **315** | 197 | 302 | 13 |
///
/// Every test that wires a service with [`create_rendezvous`] spends the registry for the rest of
/// the boot (on aarch64 the largest spenders are the scheduler's own tests, the oldest userspace
/// tests, `ntp_tests`, `login_tests` and the file and display services, and 80 are created between
/// tests). The fix is to retype the endpoint from the run's own region, so it goes when the region
/// does: `design/roadmap/0671-tests-retype-their-rendezvous-from-their-own-region.md`. Reports and
/// does not gate, for [`MAX_THREADS`]'s reason.
pub(super) static PEAK_RENDEZVOUS: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);
pub(super) static KERNEL_CHUNK_RENDEZVOUS: core::sync::atomic::AtomicUsize =
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
pub(super) const KERNEL_EP_CHUNK_PAGES: u64 = 32;

/// How many chunks the kernel will carve before refusing. Derived so the **page supply can never be
/// the binding limit before the registry is**: enough chunks to host [`MAX_RENDEZVOUS`] endpoints, one
/// page each. That is what makes exhaustion always report the honest reason (the registry is full)
/// rather than an arbitrary carve size. Derived rather than written down so the two cannot drift.
pub(super) const MAX_KERNEL_EP_CHUNKS: usize =
    MAX_RENDEZVOUS.div_ceil(KERNEL_EP_CHUNK_PAGES as usize);

/// Rank **above the allocators**, because the reaper (`finish_switch`) drops a dead `Thread` in
/// its pool slot while holding this, and that drop *frees*: the kernel stack's pages go back to
/// the frame allocator through the kernel MMU lock, and the stack's VA range to its free list.
/// Freeing takes the same locks allocating does, so the rank must sit above them.
///
/// Nothing under this lock **allocates** any more (milestone 14 phase B.2): spawn writes the new
/// `Thread` into a static pool slot, and the queues have been intrusive since A.2, so a queue
/// operation is a couple of pointer writes, from the timer IRQ or anywhere else. §9's
/// no-allocation-in-IRQ rule holds by construction.
pub(super) static IPC_TABLES: IrqSafeMutex<Option<IpcTables>> =
    IrqSafeMutex::new(rank::IPC_TABLES, None);

/// The scheduler before anything is in it, as a `const` rather than an expression [`init`] builds.
///
/// Both tables are `const fn` constructors already, so this is a `.rodata` aggregate the installer
/// copies from instead of a value assembled on the boot stack. `init`'s own comment carries the
/// measurement and why it decides how large [`MAX_THREADS`] may be.
pub(super) const EMPTY_TABLES: IpcTables = IpcTables {
    threads: Threads::new(),
    rendezvous_table: generational_table::Table::new(),
    kernel_ep_region: None,
    kernel_ep_chunks: 0,
    notification_table: generational_table::Table::new(),
    timer_table: generational_table::Table::new(),
    futexes: inter_process_communication::futex::Futexes::new(),
    processes: generational_table::Table::new(),
};
