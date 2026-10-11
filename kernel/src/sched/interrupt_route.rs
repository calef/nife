//! **Interrupt routes**: which rendezvous an interrupt line is bound to, and the notification that
//! turns a raised line into a message.
//!
//! Cut out of `sched.rs` unchanged by milestone 840 (the scheduler file is split along its seams).
//!
//! Name: provisional (milestone 840's lane, 2026-10-11 UTC; §271 (the scheduler file splits into
//! submodules of `sched`)).

use super::*;

/// intid -> rendezvous id + 1 (0 means "not routed"). A hardware interrupt, delivered as a
/// message to whoever holds the matching rendezvous.
///
/// **A plain atomic array, read lock-free from the interrupt handler.** The handler runs in a
/// context where taking a lock to *find out where to send the message* would be one more thing
/// that can go wrong; a bounded array of atomics cannot. 256 covers every INTID we will see
/// (SGIs 0-15, the timer PPI at 30, virtio SPIs in the 40s).
pub(super) const MAX_INTID: usize = 256;
pub(super) static IRQ_ROUTES: [AtomicU64; MAX_INTID] = [const { AtomicU64::new(0) }; MAX_INTID];

/// Route a hardware interrupt to an rendezvous. From now on, when `intid` fires, whoever is
/// blocked on `ep` wakes; if nobody is, the signal is remembered so it is not lost.
///
/// **And from now on `ep` refuses every send** (DECISIONS §101 (notification objects), calef's
/// ruling B, 2026-09-26). An interrupt reaches its driver as `w0 = 1`, which is also a word any
/// sender can put in `w0`, so an endpoint that carried both could not tell a driver which one woke
/// it. The refusal is the rendezvous's own (`Rendezvous::bind_to_interrupt`), so it holds for every
/// endpoint bound here whoever created it: the fifteen the kernel creates for its drivers, which
/// no program is ever granted, and the caller-supplied ones `soak::bind_tick_routes` binds, which
/// is why this is done here rather than at each call site. `SEND`, `SEND_CAP` and `CALL` answer
/// [`abi::Error::NotPermitted`]; see `set_ipc_refused`.
///
/// Marked before the route is published, so there is no instant at which the interrupt is live on
/// an endpoint that still accepts a send. A stale `ep` is routed and not marked, exactly as before:
/// `irq_notify` drops a signal to a name that does not resolve, and a name that does not resolve
/// cannot be sent to either.
pub fn bind_irq(intid: u32, ep: RendezvousId) {
    assert!((intid as usize) < MAX_INTID, "intid {intid} out of range");
    {
        let guard = IPC_TABLES.lock();
        if let Some(rendezvous) = guard.as_ref().and_then(|sched| rendezvous_of(sched, ep)) {
            rendezvous.bind_to_interrupt();
        }
    }
    // +1 so 0 keeps meaning "not routed". A name can never be u64::MAX (the registry mints
    // (generation << 32) | slot with slot < 256), so the increment cannot wrap.
    IRQ_ROUTES[intid as usize].store(ep + 1, Ordering::Release);
}

/// The rendezvous an interrupt is routed to, if any. Read from the IRQ handler; lock-free.
pub fn irq_route(intid: u32) -> Option<RendezvousId> {
    if (intid as usize) >= MAX_INTID {
        return None;
    }
    match IRQ_ROUTES[intid as usize].load(Ordering::Acquire) {
        0 => None,
        n => Some(n - 1),
    }
}

/// **Deliver an interrupt as a message.** Called from the IRQ handler.
///
/// If a thread is blocked waiting on the rendezvous, wake it. If not, count the signal so the
/// next `RECEIVE` returns immediately rather than blocking on an interrupt that already happened.
/// **An interrupt is not a rendezvous**: it must not wait for a receiver, and it must not be
/// lost if the receiver is briefly busy.
///
/// Safe to call from IRQ context: it takes `IPC_TABLES`, which the interrupted code
/// cannot have been holding, because `IrqSafeMutex` masks interrupts for exactly as long as it
/// is held. See DECISIONS §9.
pub fn irq_notify(ep: RendezvousId) {
    // A device interrupt routed to a driver, counted for `vmstat`'s `in` column (milestone 126).
    crate::machine_statistics::interrupt();
    // A device-IRQ wake is LOAD-AWARE (DECISIONS §28.2), unlike a rendezvous wake, which stays
    // local. If the woken driver lands on a *remote* core, `wake_load_aware` returns that core so we
    // can poke it after IPC_TABLES is released (the `place_on` discipline: push under the lock, SGI
    // after). The SGI send from IRQ context is a plain controller write, safe here.
    let remote = {
        let mut guard = IPC_TABLES.lock();
        let sched = guard.as_mut().expect("no scheduler");

        // `signal` wakes a waiting receiver or counts the signal; it never blocks or joins a queue. A
        // stale name (the rendezvous an interrupt was bound to has been revoked) is simply dropped: an
        // interrupt with no live rendezvous has nowhere to go, which is not an error.
        let Some(rendezvous) = rendezvous_of(sched, ep) else {
            return;
        };
        if let Some(waiter) = rendezvous.signal() {
            let waiter = hold_token(waiter);
            let t = sched.threads.get_mut(waiter).unwrap();
            t.mailbox = [1, 0, 0, 0, 0];
            t.handshake.serve(); // the signal is the delivery (the boot-8 gate)
            trace::record(trace::Event::Served, waiter, 7);
            wake_load_aware(sched, waiter)
        } else {
            None
        }
    };
    if let Some(target) = remote {
        crate::arch::irq::send_reschedule(target);
    }
}
