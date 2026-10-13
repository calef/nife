//! **The thread running on this core**: its id, and its capability table held per core so a
//! syscall's lookup takes no global lock.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// The thread running on **this core** right now.
///
/// Per-CPU as of §11 step 3b (`cpu::PerCpu::current`); it used to be one field on the global
/// `IpcTables`. Reading it is a plain atomic load and needs no lock: it is this core's own slot.
pub(super) fn current_thread_id() -> ThreadId {
    cpu::current().current.load(Ordering::Relaxed)
}

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
pub(super) struct CurrentCapabilities(AtomicPtr<CapabilityTableLock>);

pub(super) static CURRENT_CAPABILITIES: [CurrentCapabilities; cpu::MAX_CPUS] =
    [const { CurrentCapabilities(AtomicPtr::new(core::ptr::null_mut())) }; cpu::MAX_CPUS];

/// Make `tid` this core's running thread: its name in `PerCpu::current` and its capability table in
/// [`CURRENT_CAPABILITIES`]. The only writer of either, so a lookup never finds one thread's name
/// beside another's table. Caller holds `IPC_TABLES`.
///
/// # Safety
///
/// `tcb` is `tid`'s page pointer as the thread table stores it (`Threads::pointer`), live.
pub(super) unsafe fn set_current(tid: ThreadId, tcb: *mut Thread) {
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
pub(super) fn current_capabilities()
-> Option<crate::sync::IrqSafeGuard<'static, crate::cap::CapabilityTable>> {
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
