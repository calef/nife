//! **Timers**: one-shot deadlines that signal a notification, armed and cancelled by their holder
//! and expired from the tick.
//!
//! Cut out of `sched.rs` unchanged by milestone 840 (the scheduler file is split along its seams).
//!
//! Name: provisional (milestone 840's lane, 2026-10-11 UTC; §271 (the scheduler file splits into
//! submodules of `sched`)).

use super::*;

/// The most timers that can exist at once, whole machine: the notification registry's bound, for
/// its reason. A timer is per-waiter (a sleeping thread, a retransmit window), so §147's consumers
/// want about one each. The expiry walk is O(this) at worst, and runs only on a due tick.
pub(super) const MAX_TIMERS: usize = 256;

/// A timer's name: a generational name over the timer registry, what an `Object::Timer`
/// capability carries. *(Provisional, as the object type is.)*
pub type TimerId = u64;

/// **What lives at the start of a timer's page**: the proved decision core, armed with the
/// notification to signal and the bits to signal it with.
pub(super) struct TimerPage {
    state: inter_process_communication::timer::Timer<(NotificationId, u64)>,
}

/// **The cached earliest deadline, in counter ticks**: never later than any armed timer's deadline
/// (the invariant `inter_process_communication::timer` proves), `NEVER` when nothing is armed.
///
/// An atomic beside `IPC_TABLES` rather than a field in it, because the tick reads it **without**
/// the lock, which is the whole point: the idle tick is one load and one compare. Every write happens
/// under `IPC_TABLES`, so writers never race each other; a tick on another core may read a value one
/// write stale, and both directions are safe. Stale-high (an arm not yet visible) delays that expiry
/// to a later tick, bounded by the lock release that publishes it; stale-low costs one walk that
/// finds nothing. `Relaxed` is enough for that reason, and the walk itself re-reads everything
/// under the lock.
pub(super) static EARLIEST_DEADLINE: AtomicU64 =
    AtomicU64::new(inter_process_communication::timer::NEVER);

/// The timer behind a name, or `None` if it no longer resolves. Caller holds `IPC_TABLES`; the
/// `'static` is [`notification_of`]'s.
pub(super) fn timer_of(sched: &IpcTables, id: TimerId) -> Option<&'static mut TimerPage> {
    let phys = *sched.timer_table.get(id)?;
    // SAFETY: retyped exclusively for this timer, its region pinned while the name resolves,
    // direct-mapped, and serialized by IPC_TABLES, which every caller holds.
    Some(unsafe { &mut *(crate::arch::mmu::phys_to_virt(phys) as *mut TimerPage) })
}

/// Create a timer **in `region`'s memory**, disarmed. [`create_notification_from`]'s shape,
/// registry checked before a page is spent.
pub fn create_timer_from(region: u64) -> Option<TimerId> {
    let mut guard = IPC_TABLES.lock();
    let sched = guard.as_mut()?;
    if sched.timer_table.len() >= MAX_TIMERS {
        return None;
    }
    let phys =
        crate::memory_region::retype_object_page(region, crate::memory_region::ObjectKind::Timer)?;
    // SAFETY: fresh page, exclusively ours, direct-mapped.
    unsafe {
        (crate::arch::mmu::phys_to_virt(phys) as *mut TimerPage).write(TimerPage {
            state: inter_process_communication::timer::Timer::new(),
        });
    }
    sched.timer_table.insert_with(|_| phys)
}

/// **`Timer::ARM`**: arm `id` to signal `bits` into `notification` at `deadline` (counter ticks),
/// replacing any pending deadline. A deadline already reached signals now, from here, with a
/// thread's placement (the arming thread's core is warm). `Gone` if either name is stale.
pub fn timer_arm(
    id: TimerId,
    deadline: u64,
    notification: NotificationId,
    bits: u64,
) -> Result<(), abi::Error> {
    use inter_process_communication::timer::{Arm, lowered};
    let mut guard = IPC_TABLES.lock();
    let sched = guard.as_mut().expect("no scheduler");
    // Refuse a dead notification at arm time rather than letting the expiry discover it: the
    // caller can still act on `Gone` now, and nobody can act on it at the deadline.
    notification_of(sched, notification).ok_or(abi::Error::Gone)?;
    let page = timer_of(sched, id).ok_or(abi::Error::Gone)?;
    let now = crate::arch::timer::now();
    match page.state.arm(deadline, (notification, bits), now) {
        Arm::Pending => {
            let cached = EARLIEST_DEADLINE.load(Ordering::Relaxed);
            EARLIEST_DEADLINE.store(lowered(cached, deadline), Ordering::Relaxed);
            Ok(())
        }
        // A local wake never returns a core to poke; the notification was checked above, so a
        // `Gone` here is impossible under this same hold.
        Arm::DueNow((n, b)) => signal_locked(sched, n, b, WakePlacement::Local).map(|_| ()),
    }
}

/// **`Timer::CANCEL`**: disarm `id`. `true` if a deadline was pending and now never fires. The cache
/// is deliberately left where it is: lowering is all an arm does, a walk is what raises it, and a
/// stale-low cache costs one walk that fires nothing. `Gone` if the name is stale.
pub fn timer_cancel(id: TimerId) -> Result<bool, abi::Error> {
    let mut guard = IPC_TABLES.lock();
    let sched = guard.as_mut().expect("no scheduler");
    Ok(timer_of(sched, id).ok_or(abi::Error::Gone)?.state.cancel())
}

/// **The expiry walk**, from the tick, in interrupt context, on the interrupt stack: fire every due
/// timer, then recompute the cache exactly. Legal here for [`signal_notification_from_interrupt`]'s
/// reason (a wake is an enqueue, `IPC_TABLES` masks interrupts, nothing allocates), and it goes
/// through the same [`signal_locked`] with the same load-aware placement.
///
/// **Rescan rather than list**, the region sweeps' shape: each step finds the first due timer,
/// disarms it and signals, so no buffer bounds how many may fire in one tick. The cost is O(timers)
/// per firing, paid only on a due tick.
///
/// `#[cold]` and out of line so its bytes stay in its own symbol rather than in `on_tick`'s, for
/// `on_tick`'s own reason (riscv64 counts the trap path flat).
#[cold]
#[inline(never)]
pub(super) fn expire_timers() {
    use inter_process_communication::timer::{earliest, next_due};
    let mut poke: u64 = 0; // one bit per core a wake placed a thread on
    {
        let mut guard = IPC_TABLES.lock();
        let Some(sched) = guard.as_mut() else { return };
        let now = crate::arch::timer::now();
        loop {
            let fired = next_due(
                sched
                    .timer_table
                    .values()
                    // SAFETY: each value is a live timer page, as `timer_of` argues.
                    .map(|&phys| unsafe {
                        &mut (*(crate::arch::mmu::phys_to_virt(phys) as *mut TimerPage)).state
                    }),
                now,
            );
            let Some((notification, bits)) = fired else {
                break;
            };
            // A notification destroyed while the timer was armed: nobody left to tell, dropped
            // as `signal_notification_from_interrupt` drops a stale name.
            if let Ok(Some(cpu)) =
                signal_locked(sched, notification, bits, WakePlacement::LoadAware)
            {
                poke |= 1 << cpu;
            }
        }
        let cached = earliest(sched.timer_table.values().map(|&phys| {
            // SAFETY: as above.
            unsafe { &(*(crate::arch::mmu::phys_to_virt(phys) as *const TimerPage)).state }
        }));
        EARLIEST_DEADLINE.store(cached, Ordering::Relaxed);
    }
    for cpu in 0..cpu::MAX_CPUS {
        if poke & (1 << cpu) != 0 {
            crate::arch::irq::send_reschedule(cpu);
        }
    }
}

/// **Tear down every timer whose page lies in `[base, end)`**: drop the name. A timer has no
/// waiters of its own, so there is nothing to abort; an armed one simply never fires. The cache may
/// be left stale-low, which is safe. Caller holds `IPC_TABLES`.
pub(super) fn reap_region_timers(sched: &mut IpcTables, base: u64, end: u64) {
    loop {
        let doomed = sched
            .timer_table
            .iter()
            .find(|&(_, &phys)| base <= phys && phys < end)
            .map(|(name, _)| name);
        let Some(name) = doomed else { break };
        sched.timer_table.remove(name);
    }
}
