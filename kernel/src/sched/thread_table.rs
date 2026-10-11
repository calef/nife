//! **The thread table**: the fixed array of thread control block pointers, generational over
//! [`MAX_THREADS`] slots, that every thread id names.
//!
//! Cut out of `sched.rs` unchanged by milestone 840 (the scheduler file is split along its seams).
//!
//! Name: provisional (milestone 840's lane, 2026-10-11 UTC; §271 (the scheduler file splits into
//! submodules of `sched`)).

use super::*;

/// **The most threads that were ever alive at once on this boot.**
///
/// The instrument that makes [`MAX_THREADS`] a measured number rather than a felt one, and the
/// reason a future lane does not have to repeat the two instrumented runs that found the
/// ceiling in the first place. It answers the only question a capacity limit poses, "how close
/// is the suite to it", which the refusal itself cannot: by the time a spawn is refused the
/// table is full and every earlier number is gone.
///
/// A high-water mark rather than a ledger, deliberately. Attributing threads to tests the way
/// `kernel::testing`'s frame ledger attributes frames would need a per-test reading, and the
/// answer would be misleading anyway: what fills this table is not one test's spend but the
/// services every earlier test left running on purpose (`notes/frames.md`'s "held" list). One
/// number for the whole boot is the honest shape.
///
/// Updated on the two inserts that can grow the table, under `IPC_TABLES`, so it never races;
/// `fetch_max` rather than a compare-store because the write is cheap and this is not a hot path
/// (a spawn already costs a page).
pub(super) static PEAK_THREADS: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);

/// **Scheduled on-CPU time, one timer tick at a time, keyed by thread slot** (milestone 282 (a thread's CPU time, and the `top` it makes possible),
/// DECISIONS §150 (how does a thread's CPU time reach userspace?)). `abi::survey::record::CPU_TIME` is what reads it.
///
/// **Why an array beside the table rather than a `u64` on `Thread`**, which is what §150's build
/// note described and is the one place this differs from it. The increment happens in
/// [`on_tick`], in interrupt context, and the only name a core has for its running thread there is
/// a tid in its own per-CPU block. Turning that tid into a `&mut Thread` means `IPC_TABLES`, and a
/// timer interrupt that waits on a lock another core holds is a scheduler-latency hole opened at
/// every tick on every core; `try_lock` is no better, because a dropped sample is not a coarse
/// number, it is a wrong one, and it would be dropped exactly when the machine is busiest. The
/// slot index is already in the tid's low 32 bits (`generational_table`'s packing), so an array
/// keyed by slot needs no lock, no lookup, and no ordering beyond relaxed.
///
/// **Not on `cpu::PerCpu` either**, and that is a measured constraint rather than a preference:
/// milestone 527 (the survey selector, and a thread's placement) grew `PerCpu` by one `u64`, took
/// `size_of::<PerCpu>()` from 128 to 136, and cost riscv64's IPC fastpath 5.4% because `PERCPU[id]`
/// stopped indexing with a shift. `kernel/src/cpu.rs` now const-asserts that size. This counter is
/// per **thread**, not per core, so it was never a candidate for that struct; the note is here
/// because the next person to add a counter will consider it.
///
/// **Two kilobytes of `.bss`**, which is `MAX_THREADS` slots at eight bytes and does not move with
/// the workload.
///
/// # The race, stated rather than assumed (rule 4)
///
/// A core's tick increments only the slot of the thread that core is running, so the **write** is
/// uncontended by construction and needs no cross-core synchronisation. A survey reading a slot
/// another core is incrementing is a relaxed load of a value in flight: it reads a number that was
/// true a moment ago, never a torn one, because the unit is a naturally aligned `u64`. That is the
/// same bargain the per-CPU `TICKS` array already takes, and it is the right one for a statistic
/// nobody branches on.
///
/// # Slot reuse
///
/// The counter is zeroed when a slot is **filled**, not when it is emptied, which is what lets a
/// corpse keep the time it earned until its supervisor reaps it. A survey can report a `DEAD`
/// thread, and reporting it with a fresh occupant's zero (or a previous occupant's total) would be
/// the plausible wrong number this tree already ruled against.
pub(super) static CPU_TICKS: [core::sync::atomic::AtomicU64; MAX_THREADS] =
    [const { core::sync::atomic::AtomicU64::new(0) }; MAX_THREADS];

/// The slot a generational thread name occupies, for indexing [`CPU_TICKS`].
///
/// `generational_table` packs `(generation << 32) | slot`, so the slot is the low word. No bound
/// check here on purpose: every caller indexes [`CPU_TICKS`] with `get`, so an out-of-range answer
/// is `None` rather than a panic, and `cpu::NO_TID` (`u64::MAX`) lands out of range for free, which
/// is what makes "this core is running nothing" cost no branch of its own.
pub(super) const fn slot_of(tid: ThreadId) -> usize {
    (tid & 0xffff_ffff) as usize
}

/// **Charge one timer tick to the thread this core is running** (milestone 282 (a thread's CPU time, and the `top` it makes possible)).
///
/// The whole of the accounting, called from [`on_tick`] in interrupt context: one relaxed load of
/// this core's `current`, one bounds-checked index, one relaxed increment. Nothing here can block,
/// allocate, or take a lock, which is the property that let this go on the tick path at all.
///
/// A core running nothing yet (`cpu::NO_TID`) charges nobody, and costs the same branch the bounds
/// check already spends. The idle thread is a thread and is charged like any other; it is not in
/// any supervision domain, so no survey reports it.
pub(super) fn charge_tick() {
    if let Some(counter) = CPU_TICKS.get(slot_of(current_thread_id())) {
        counter.fetch_add(1, Ordering::Relaxed);
    }
}

/// Start a freshly filled slot's CPU time at zero. Called by both inserts, under `IPC_TABLES`.
pub(super) fn clear_cpu_ticks(tid: ThreadId) {
    if let Some(counter) = CPU_TICKS.get(slot_of(tid)) {
        counter.store(0, Ordering::Relaxed);
    }
}

/// The high-water mark [`PEAK_THREADS`] holds. Printed by the test suite's closing summary.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))] // the closing summary is the only reader
pub fn peak_thread_count() -> usize {
    PEAK_THREADS.load(Ordering::Relaxed)
}

/// The thread table: generational names (`crates/slots`, notes/generational-names.md) over
/// **page-resident** TCBs (milestone 19c.2). Each `Thread` lives at the start of one page from
/// the kernel's own budget (`kmem`), so the static `MAX_THREADS`-sized BSS pool that B.2 built
/// as a scaffold is gone: the kernel reserves no per-thread memory it hasn't been handed, the
/// last uncovered corner of milestone 14's no-open-ended-spending thesis. B.2 named this moment
/// ("the pool upgrades to retype-backed storage behind the table when init lands"); this is it.
///
/// A page's address never changes (direct-mapped, and its `kmem` region is pinned), which
/// supplies the pinning the per-thread `Box` and then the pool both provided: the context-switch
/// assembly and the intrusive queues hold pointers straight into these pages. The table stores
/// the pointer; the generational name is what everything else carries (stale-safe as ever).
///
/// 19c.3 will let a user process retype a TCB from *its own* untyped by the same mechanism, the
/// page merely coming from a different budget; kernel threads keep drawing from `kmem`.
/// A TCB pointer that may cross cores. The pointer itself moving between cores is harmless: the
/// `Thread` it names is touched only under `IPC_TABLES` (which serializes all table access) and, for
/// its queue link, under the intrusive discipline at [`thread_control_block_ptr`]. This is the same soundness the
/// old static `TcbPool`'s `unsafe impl Sync` rested on, now attached to the pointer the table
/// stores rather than a separate array.
#[derive(Clone, Copy)]
pub(super) struct ThreadControlBlockPointer(*mut Thread);

// SAFETY: see the type's doc; sending the pointer is sound because dereferencing it is gated.
unsafe impl Send for ThreadControlBlockPointer {}

pub(super) struct Threads {
    pub(super) table: generational_table::Table<ThreadControlBlockPointer, MAX_THREADS>,
}

impl Threads {
    pub(super) const fn new() -> Self {
        Self {
            table: generational_table::Table::new(),
        }
    }

    pub(super) fn get(&self, tid: ThreadId) -> Option<&Thread> {
        let p = self.table.get(tid)?.0;
        // SAFETY: a pointer we stored at insert, into a live kmem page not yet recycled (remove
        // kills the name before recycling); IPC_TABLES serializes access.
        Some(unsafe { &*p })
    }

    pub(super) fn get_mut(&mut self, tid: ThreadId) -> Option<&mut Thread> {
        let p = self.table.get(tid)?.0;
        // SAFETY: as `get`, and `&mut self` carries IPC_TABLES's exclusivity.
        Some(unsafe { &mut *p })
    }

    /// **A thread's capability table**, for a caller holding `IPC_TABLES` that needs no other part
    /// of the thread. The table is past the `Thread` in its page (`thread::capability_table_of`), so
    /// the reference is derived from the page pointer, never from a `&Thread`.
    pub(super) fn capabilities(&self, tid: ThreadId) -> Option<&CapabilityTableLock> {
        let p = self.table.get(tid)?.0;
        // SAFETY: the stored page pointer of a live thread; the page outlives `&self`, because
        // only `remove` (which takes `&mut self`) recycles it.
        Some(unsafe { &*crate::thread::capability_table_of(p) })
    }

    /// `get_mut` and [`capabilities`](Self::capabilities) at once, for the sites that write the
    /// `Thread` and its table together. Disjoint by construction: the table is past the struct's
    /// end, so the `&mut Thread` does not cover it.
    pub(super) fn get_mut_with_capabilities(
        &mut self,
        tid: ThreadId,
    ) -> Option<(&mut Thread, &CapabilityTableLock)> {
        let p = self.table.get(tid)?.0;
        // SAFETY: as `get_mut` for the `Thread` and `capabilities` for the table; the two do not
        // overlap.
        Some(unsafe { (&mut *p, &*crate::thread::capability_table_of(p)) })
    }

    /// **The raw pointer the table stores, which is the start of the thread's TCB page.**
    ///
    /// `get` and `get_mut` narrow it to a reference over `size_of::<Thread>()` bytes, and that is
    /// exactly what the caller must not have when it wants the FP register file: that lives in the
    /// same page, past the end of the struct, so a pointer derived from a reference would be
    /// reaching outside its own provenance (`thread::fp_state_of`'s safety note). This hands back
    /// the pointer the page cast produced, unnarrowed.
    pub(super) fn pointer(&self, tid: ThreadId) -> Option<*mut Thread> {
        Some(self.table.get(tid)?.0)
    }

    /// Insert: claim a page from the kernel budget, build the `Thread` (carrying its own minted
    /// name) into it, and store the pointer under that name. `None` (page recycled, `f` never
    /// run) if the budget or the table is exhausted.
    pub(super) fn insert_with(&mut self, f: impl FnOnce(ThreadId) -> Thread) -> Option<ThreadId> {
        let page = crate::kmem::page()?;
        // A kernel thread's TCB page is `kmem`'s and comes home to it at death; if the table is
        // full it never held a Thread, so recycle now.
        let name = self.insert_at(page, f);
        if name.is_none() {
            crate::kmem::recycle(page);
        }
        name
    }

    /// `insert_with`'s place-writing twin: claim a page from the kernel budget and let `build`
    /// construct the Thread into it. Recycles the page on any failure, exactly as `insert_with`
    /// does, so a refused spawn costs nothing.
    pub(super) fn insert_in_place(
        &mut self,
        build: impl FnOnce(ThreadId, *mut Thread) -> bool,
    ) -> Option<ThreadId> {
        let page = crate::kmem::page()?;
        let name = self.insert_at_in_place(page, build);
        if name.is_none() {
            crate::kmem::recycle(page);
        }
        name
    }

    /// Insert a Thread that already has a page (milestone 19c.3): a user-retyped TCB, whose page
    /// is its creator's region's, not `kmem`'s. On a full table the page is the region's to
    /// account (spend-only), so nothing is recycled here.
    pub(super) fn insert_from_page(
        &mut self,
        page: u64,
        f: impl FnOnce(ThreadId) -> Thread,
    ) -> Option<ThreadId> {
        self.insert_at(page, f)
    }

    /// **The place-writing insert** (milestone 124): `build` receives the minted name and the TCB
    /// page, and writes the Thread there itself. `false` from `build` means it wrote nothing, and
    /// the slot is left exactly as it was found.
    ///
    /// The difference from `insert_at` is where the Thread is constructed. That one takes a
    /// `FnOnce(ThreadId) -> Thread`, so the value travels through the closure's return and a temporary
    /// before `ptr.write` puts it on the page; a `Thread` is large and a debug build copies at
    /// every hop. This hands the destination down instead. See `Thread::spawn_into`.
    fn insert_at_in_place(
        &mut self,
        page: u64,
        build: impl FnOnce(ThreadId, *mut Thread) -> bool,
    ) -> Option<ThreadId> {
        let ptr = crate::arch::mmu::phys_to_virt(page) as *mut Thread;
        let mut built = false;
        let name = self.table.insert_with(|tid| {
            built = build(tid, ptr);
            if built {
                // The register file beside the struct, in the same page (milestone 447). Here
                // rather than in a `Thread` constructor because a constructor builds a value and
                // this is a fact about a *place*: `kmem` pages are not zeroed, so the `live` flag
                // has to be written before anything reads it. See `thread::init_fp_state`.
                //
                // SAFETY: `ptr` is the start of a page this insert exclusively owns, and `build`
                // has just written a live `Thread` at it.
                unsafe { crate::thread::init_fp_state(ptr) };
                // SAFETY: as above; the name is not yet handed to anyone, so nothing else can
                // reach the table.
                unsafe { crate::thread::init_capability_table(ptr) };
            }
            ThreadControlBlockPointer(ptr)
        })?;
        if !built {
            // `build` declined (no kernel stack). Take the name back out: the slot never held a
            // Thread, so there is nothing to drop, and `remove` here would drop uninitialised
            // bytes. `forget_slot` bumps the generation and frees the slot without touching the
            // TCB page, which is the caller's to recycle. `Table::remove` is exactly right and
            // not a leak: the slot holds a `ThreadControlBlockPointer`, and dropping that drops a pointer. The
            // `Thread` drop lives in `Threads::remove`, which is not on this path because no
            // Thread was ever constructed.
            self.table.remove(name);
            return None;
        }
        self.note_peak();
        self.mint_token(name);
        // A reused slot starts its CPU accounting at zero; see `CPU_TICKS`.
        clear_cpu_ticks(name);
        Some(name)
    }

    /// The shared engine: write the built Thread into `page` and name it. The Thread carries its
    /// own `thread_control_block_kmem`, which `remove` reads to decide whether the page returns to `kmem`.
    fn insert_at(&mut self, page: u64, f: impl FnOnce(ThreadId) -> Thread) -> Option<ThreadId> {
        let ptr = crate::arch::mmu::phys_to_virt(page) as *mut Thread;
        let name = self.table.insert_with(|tid| {
            // SAFETY: a fresh, exclusively-ours page; `write` moves the Thread in, no drop of
            // uninitialized bytes.
            unsafe { ptr.write(f(tid)) };
            // And its register file beside it in the same page; see `insert_at_in_place` for why
            // this is here rather than in a `Thread` constructor.
            //
            // SAFETY: as above, with the `Thread` now live at `ptr`.
            unsafe { crate::thread::init_fp_state(ptr) };
            // And its capability table, past the register file, for the same reason.
            //
            // SAFETY: as above; the name is not yet handed to anyone.
            unsafe { crate::thread::init_capability_table(ptr) };
            ThreadControlBlockPointer(ptr)
        });
        if let Some(name) = name {
            self.note_peak();
            self.mint_token(name);
            // A reused slot starts its CPU accounting at zero; see `CPU_TICKS`.
            clear_cpu_ticks(name);
        }
        name
    }

    /// **Mint a new thread's queue token: the only place in the kernel a token is made**
    /// (milestone 139 (drive the unsafe count down), round 10). Every thread the table holds was
    /// inserted through `insert_at` or `insert_at_in_place`, and both call this once, on the name
    /// they just minted, so every thread has exactly one token for its whole life. From here the
    /// token moves: onto a queue at a push, back onto [`Thread::own_token`] at a pop or a removal
    /// (`hold_token`), and nowhere else. See `thread::Thread::own_token` for the three places it
    /// can be.
    ///
    /// The `unsafe` is `Unqueued::new`'s three obligations, discharged once here instead of at
    /// each of the twenty-one pushes, sends and receives that used to assert them:
    ///
    /// - **Valid while reachable.** The token points at the `Thread` on its TCB page, which lives
    ///   until `Threads::remove`. That is reached only for a `Finished` thread off its CPU
    ///   (`reap_switched_out`), which is on no queue because a running thread holds its own token,
    ///   and for a thread region teardown ends (`finish_blocked_resident`), which unlinks it from
    ///   every wait queue first. Either way the token dies with the page, in `own_token`, or was
    ///   already dropped by the removal that unlinked it.
    /// - **On no queue, and the only token.** The name is a moment old and no other token for this
    ///   page can exist: the previous occupant's died with it.
    /// - **Links touched only by a queue, with no reference live across a queue operation.** Every
    ///   queue holding a `Thread` is behind `IPC_TABLES` or an inbox lock, and the `&mut` below
    ///   ends before this returns.
    fn mint_token(&mut self, name: ThreadId) {
        if let Some(t) = self.get_mut(name) {
            let node = core::ptr::NonNull::from(&mut *t);
            // SAFETY: the three obligations above.
            t.own_token = Some(unsafe { Unqueued::new(node) });
        }
    }

    /// Remove and destroy: drop the TCB in place (its stack, address space, and quota token go
    /// with it), kill the name so no copy of the `ThreadId` ever resolves again, then recycle the page.
    pub(super) fn remove(&mut self, tid: ThreadId) {
        let Some(&ThreadControlBlockPointer(ptr)) = self.table.get(tid) else {
            return;
        };
        // Read the page's origin BEFORE the drop consumes the Thread. A kernel TCB's page goes
        // home to `kmem`; a user TCB's page belongs to its region (spend-only, reclaimed only at
        // region destroy), so the reaper leaves it.
        // SAFETY: live per the table, exclusive per `&mut self`.
        let from_kmem = unsafe { (*ptr).thread_control_block_kmem };
        // SAFETY: as above. Drop first (KernelStack's unmap-and-recycle, AddressSpace teardown,
        // the QuotaToken), then kill the name, then the page goes home: nothing can reach the
        // dropped Thread afterward.
        unsafe { core::ptr::drop_in_place(ptr) };
        self.table.remove(tid);
        if from_kmem {
            crate::kmem::recycle(crate::arch::mmu::virt_to_phys(ptr as u64));
        }
    }

    pub(super) fn len(&self) -> usize {
        self.table.len()
    }

    /// Record the table's occupancy against [`PEAK_THREADS`]. Called on the two paths that can
    /// make the table grow, which is every insert: `insert_at` and `insert_at_in_place` are what
    /// the three public inserts delegate to.
    fn note_peak(&self) {
        PEAK_THREADS.fetch_max(self.table.len(), Ordering::Relaxed);
    }

    /// Every live TCB, for whole-table sweeps (revocation). Each live name resolves to a
    /// distinct page pointer, so the `&mut`s are disjoint.
    pub(super) fn iter_mut(&mut self) -> impl Iterator<Item = &mut Thread> + '_ {
        // SAFETY: each stored pointer is a distinct live page (one page per thread), and
        // `&mut self` carries IPC_TABLES's exclusivity across the whole sweep.
        self.table
            .values()
            .map(|&ThreadControlBlockPointer(p)| unsafe { &mut *p })
    }

    /// [`iter_mut`](Self::iter_mut) with each thread's capability table beside it, for the
    /// revocation sweeps. Disjoint as [`get_mut_with_capabilities`](Self::get_mut_with_capabilities).
    pub(super) fn iter_mut_with_capabilities(
        &mut self,
    ) -> impl Iterator<Item = (&mut Thread, &CapabilityTableLock)> + '_ {
        // SAFETY: as `iter_mut`, and the table is past each `Thread`'s end in its own page.
        self.table
            .values()
            .map(|&ThreadControlBlockPointer(p)| unsafe {
                (&mut *p, &*crate::thread::capability_table_of(p))
            })
    }

    /// Every live TCB from slot `from` onward, with its slot index, for a **resumable** sweep
    /// (`rendezvous::SURVEY`, milestone 126 (the `procps` package)). The slot is the caller's cursor; see
    /// `generational_table::Table::iter_from` for why a position would not do.
    pub(super) fn iter_from(&self, from: usize) -> impl Iterator<Item = (usize, &Thread)> + '_ {
        // SAFETY: as `iter_mut`, and shared rather than exclusive: each stored pointer is a
        // distinct live page, and `&self` carries IPC_TABLES for the walk.
        self.table
            .iter_from(from)
            .map(|(slot, _, &ThreadControlBlockPointer(p))| (slot, unsafe { &*p }))
    }
}
