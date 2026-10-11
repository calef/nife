//! **Notifications**: creating one, signal, wait and poll, and binding one to a thread so a signal
//! reaches a receiver parked on its rendezvous.
//!
//! Cut out of `sched.rs` unchanged by milestone 840 (the scheduler file is split along its seams).
//!
//! Name: provisional (milestone 840's lane, 2026-10-11 UTC; §271 (the scheduler file splits into
//! submodules of `sched`)).

use super::*;

/// The most notifications that can exist at once, whole machine. A registry bound like
/// [`MAX_RENDEZVOUS`], and half of it, because a notification is a per-thread or per-service
/// doorbell where a rendezvous is a per-conversation object: §101's consumers want about one each.
/// The pages come from the creators' own regions; this bounds only the name table, which is 16 bytes
/// a slot. Raise it when a real workload refuses a `RETYPE_OBJ` here, not before.
pub(super) const MAX_NOTIFICATIONS: usize = 256;

/// A notification's name: a generational name over the notification registry, what an
/// `Object::Notification` capability carries. Stale-safe like every other name.
pub type NotificationId = u64;

/// **What lives at the start of a notification's page**: the proved state machine, and the one
/// thread it is bound to, if any. The binding sits here and on the thread (`Thread::bound_notification`)
/// because each side needs it without a search: a signal asks "who is bound to me", and a receive
/// asks "what is bound to me".
pub(super) struct NotificationPage {
    pub(super) state: inter_process_communication::notification::Notification<Thread>,
    /// Set once by `BIND` and never cleared. A generational name, so a dead thread leaves it stale
    /// and every reader treats a miss as unbound.
    pub(super) bound: Option<ThreadId>,
}

/// The notification behind a name, or `None` if it no longer resolves. Caller holds `IPC_TABLES`.
/// The `'static` is [`rendezvous_of`]'s, for the same reason: the page is pinned while the name
/// resolves, and `IPC_TABLES` serializes every access.
pub(super) fn notification_of(
    sched: &IpcTables,
    id: NotificationId,
) -> Option<&'static mut NotificationPage> {
    let phys = *sched.notification_table.get(id)?;
    // SAFETY: retyped exclusively for this notification, its region pinned while the name
    // resolves, direct-mapped, and serialized by IPC_TABLES, which every caller holds.
    Some(unsafe { &mut *(crate::arch::mmu::phys_to_virt(phys) as *mut NotificationPage) })
}

/// Create a notification **in `region`'s memory**: one page retyped and pinned, the object at its
/// start, a fresh name. `RETYPE_OBJ`'s engine, and [`try_create_rendezvous_from`]'s shape, including
/// its fix: the registry is checked *before* a page is spent. `None` when the region or the registry
/// is full; userspace gets one flat `OutOfMemory` for both, as it does for a rendezvous.
pub fn create_notification_from(region: u64) -> Option<NotificationId> {
    let mut guard = IPC_TABLES.lock();
    let sched = guard.as_mut()?;
    if sched.notification_table.len() >= MAX_NOTIFICATIONS {
        return None;
    }
    let phys = crate::memory_region::retype_object_page(
        region,
        crate::memory_region::ObjectKind::Notification,
    )?;
    // SAFETY: fresh page, exclusively ours, direct-mapped.
    unsafe {
        (crate::arch::mmu::phys_to_virt(phys) as *mut NotificationPage).write(NotificationPage {
            state: inter_process_communication::notification::Notification::new(),
            bound: None,
        });
    }
    // Cannot fail: capacity was checked above under this same lock hold.
    sched.notification_table.insert_with(|_| phys)
}

/// **The mailbox of a receive the bound notification ended.** `(BOUND, word, 0, 0, BOUND)`:
/// §101's `w0 = 2` and the word in `w1`, and the tag again in `w4`, the one register of a receive
/// that no sender can write. See `abi::notification::BOUND`, and notes/notification-objects.md for
/// why `w0` alone is forgeable.
pub(super) const fn bound_delivery(word: u64) -> [u64; 5] {
    [
        abi::notification::BOUND,
        word,
        0,
        0,
        abi::notification::BOUND,
    ]
}

/// **The bound thread, if it is parked in a receive right now**, with the rendezvous it is parked
/// on. This is the one boolean the crate's `signal` takes on trust, so the argument that it is right
/// lives here.
///
/// "Parked in a receive" is `Blocked` on a rendezvous as a `Receiver` **with nothing delivered
/// yet**, and the last clause is not decoration. A receiver a sender has already served stays
/// `Blocked` with its `wait_on` intact until the wake completes, and that can be a whole context
/// switch later (a wake deferred behind `on_cpu` finishes in `finish_switch`). It is off the queue
/// and its mailbox is full; delivering a notification there would overwrite the message. Every way
/// off a receiver queue marks the thread delivered or aborted in the same critical section (a
/// sender's `serve`, an IRQ signal's `serve`, a drain's `abort`, a bound delivery's `serve`), except
/// `finish_blocked_resident`, which ends the thread (`Finished`, not `Blocked`). So this test is
/// exactly "still linked on that receiver queue", and [`deliver_bound`] asserts it.
///
/// `RECEIVE`, `RECEIVE_CAP` and `Irq::WAIT` all park this way, so the binding wakes all three: one rule
/// for "blocked receiving on an endpoint", which is §101's phrase.
pub(super) fn bound_receiver(
    sched: &IpcTables,
    page: &NotificationPage,
) -> Option<(ThreadId, RendezvousId)> {
    let tid = page.bound?;
    let t = sched.threads.get(tid)?;
    match t.handshake.wait_on {
        Some(Wait::Rendezvous(ep, WaitRole::Receiver))
            if t.handshake.state == State::Blocked && !t.handshake.is_delivered() =>
        {
            Some((tid, ep))
        }
        _ => None,
    }
}

/// **Deliver `word` to a bound thread parked receiving on `ep`**: unlink it from that rendezvous's
/// receiver queue, fill its mailbox with [`bound_delivery`], and record the delivery. The caller
/// wakes it. Caller holds `IPC_TABLES` and got `(tid, ep)` from [`bound_receiver`] in the same hold.
///
/// **The unlink is `remove_receiver`'s drain-and-repush**, O(receivers queued on `ep`), which is the
/// cost §101's binding pays for reaching into another object's queue with a singly linked FIFO.
/// A server's endpoint usually holds one receiver (the server itself), so this is one pop and one
/// push in the common case.
pub(super) fn deliver_bound(sched: &mut IpcTables, tid: ThreadId, ep: RendezvousId, word: u64) {
    let ptr = thread_control_block_ptr(sched, tid);
    // `ptr` is only compared by the remove; the token it hands back goes home to the thread.
    let unlinked = rendezvous_of(sched, ep).and_then(|r| r.remove_receiver(ptr));
    debug_assert!(
        unlinked.is_some(),
        "a bound receiver was not on the receiver queue it was parked on"
    );
    if let Some(token) = unlinked {
        hold_token(token);
    }
    let t = sched
        .threads
        .get_mut(tid)
        .expect("bound receiver vanished under IPC_TABLES");
    t.mailbox = bound_delivery(word);
    t.handshake.serve(); // delivered: this wake passes the boot-8 gate
    trace::record(trace::Event::Served, tid, 10);
}

/// Where a notification wake queues the thread it woke. A signal from a thread is a
/// [`wake`]: local, because the signaller's core is warm and the pair is often a conversation. A
/// signal from interrupt context is [`wake_load_aware`], the device-IRQ placement of §28 (SMP
/// placement) step 2, because a timer expiry (milestone 106 (a wait that ends on either the interrupt
/// or the deadline)) or an interrupt carries no such locality.
#[derive(Clone, Copy)]
pub(super) enum WakePlacement {
    Local,
    // The timer expiry walk (`expire_timers`, milestone 106) and `signal_notification_from_interrupt`.
    LoadAware,
}

/// Wake `tid` by `placement`. Returns the remote core to poke once `IPC_TABLES` is released, as
/// [`wake_load_aware`] does; a local wake never needs one.
pub(super) fn wake_placed(
    sched: &mut IpcTables,
    tid: ThreadId,
    placement: WakePlacement,
) -> Option<usize> {
    match placement {
        WakePlacement::Local => {
            wake(sched, tid);
            None
        }
        WakePlacement::LoadAware => wake_load_aware(sched, tid),
    }
}

/// **The one signal path**, for a thread's `SIGNAL` and for a kernel-originated signal alike.
/// Caller holds `IPC_TABLES`. `Err(Gone)` if the name no longer resolves; otherwise the remote core
/// to poke, if the wake placed a thread there.
///
/// **Why one function with a placement rather than two**: milestone 106's timer
/// (DECISIONS §147 (a timer a userspace service cannot hold), `Timer::ARM(deadline, notification)`) will signal from the tick, in interrupt
/// context, on the interrupt stack. Everything here is already legal there, for the reason
/// `irq_notify` is: `IPC_TABLES` masks interrupts while held, a wake is an enqueue and not a switch,
/// and nothing allocates. So the kernel's signal and the user's differ only in where the woken
/// thread is placed, and a second copy of the delivery logic would be the drift the tree keeps
/// paying for.
pub(super) fn signal_locked(
    sched: &mut IpcTables,
    id: NotificationId,
    bits: u64,
    placement: WakePlacement,
) -> Result<Option<usize>, abi::Error> {
    use inter_process_communication::notification::Signal;
    let page = notification_of(sched, id).ok_or(abi::Error::Gone)?;
    let bound = bound_receiver(sched, page);
    match page.state.signal(bits, bound.is_some()) {
        Signal::Woke(waiter, word) => {
            let tid = hold_token(waiter);
            let t = sched
                .threads
                .get_mut(tid)
                .expect("a notification waiter vanished under IPC_TABLES");
            t.mailbox = [word, 0, 0, 0, 0];
            t.handshake.serve();
            trace::record(trace::Event::Served, tid, 9);
            Ok(wake_placed(sched, tid, placement))
        }
        Signal::ToBound(word) => {
            let (tid, ep) = bound.expect("the crate answered ToBound without a bound receiver");
            deliver_bound(sched, tid, ep, word);
            Ok(wake_placed(sched, tid, placement))
        }
        Signal::Counted | Signal::Empty => Ok(None),
    }
}

/// **`Notification::SIGNAL`**: OR `bits` in, waking a waiter or the bound receiver. Never blocks.
pub fn notification_signal(id: NotificationId, bits: u64) -> Result<(), abi::Error> {
    let mut guard = IPC_TABLES.lock();
    let sched = guard.as_mut().expect("no scheduler");
    // A local wake never returns a core to poke.
    signal_locked(sched, id, bits, WakePlacement::Local).map(|_| ())
}

/// **Signal a notification from interrupt context**: the kernel-originated signal milestone 106's
/// timer expiry will call from the tick, and §147's argument says IRQ delivery can use too. Safe
/// from an interrupt handler for [`irq_notify`]'s reason, and placed load-aware for its reason. A
/// stale name is dropped silently, as `irq_notify` drops a revoked route: an expiry with no
/// notification left to signal has nowhere to go, which is not an error.
///
/// **Still no caller outside the tests, and milestone 106 is why.** The timer's expiry walk
/// ([`expire_timers`]) fires several timers under one hold of `IPC_TABLES`, so it calls
/// [`signal_locked`] with this function's placement directly rather than taking and dropping the
/// lock once per timer through here. This entry stays for a kernel source that signals one
/// notification at a time (§147's argument that IRQ delivery could use it); a kernel test exercises
/// it so it is not dead code that merely compiles.
#[cfg_attr(not(feature = "system_tests"), allow(dead_code))]
pub fn signal_notification_from_interrupt(id: NotificationId, bits: u64) {
    let remote = {
        let mut guard = IPC_TABLES.lock();
        let sched = guard.as_mut().expect("no scheduler");
        signal_locked(sched, id, bits, WakePlacement::LoadAware)
            .ok()
            .flatten()
    };
    if let Some(target) = remote {
        crate::arch::irq::send_reschedule(target);
    }
}

/// **`Notification::WAIT`**: take the word, or block until a signal arrives. `Err(Gone)` if the
/// notification is stale, or is destroyed while this thread waits (the region sweep aborts its
/// waiters, exactly as it aborts a rendezvous's).
pub fn notification_wait(id: NotificationId) -> Result<u64, abi::Error> {
    use inter_process_communication::notification::Wait as Waited;
    let immediate = {
        let mut guard = IPC_TABLES.lock();
        let sched = guard.as_mut().expect("no scheduler");
        let current = current_thread_id();
        let page = notification_of(sched, id).ok_or(abi::Error::Gone)?;
        // The running thread's token: if it queues, it stays live, since a thread waiting here is
        // Blocked, which the reaper never frees.
        match page.state.wait(take_token(sched, current)) {
            Waited::Word(word, me) => {
                hold_token(me);
                Some(word)
            }
            Waited::Blocked => {
                let t = sched.threads.get_mut(current).expect("running thread");
                t.handshake.park(Wait::Notification(id)); // only a signal (or an abort) may wake us
                trace::record(trace::Event::BlockSelf, current, id as u8);
                None
            }
        }
    };
    if let Some(word) = immediate {
        return Ok(word);
    }
    schedule(); // blocks; a signal fills our mailbox and wakes us
    let mut guard = IPC_TABLES.lock();
    let sched = guard.as_mut().expect("no scheduler");
    let t = sched
        .threads
        .get_mut(current_thread_id())
        .expect("running thread");
    debug_assert!(
        t.handshake.is_delivered(),
        "notification wait resumed with nothing delivered"
    );
    if t.handshake.take_aborted() {
        return Err(abi::Error::Gone);
    }
    Ok(t.mailbox[0])
}

/// **`Notification::POLL`**: take the word without blocking; `0` if nothing was pending.
pub fn notification_poll(id: NotificationId) -> Result<u64, abi::Error> {
    let mut guard = IPC_TABLES.lock();
    let sched = guard.as_mut().expect("no scheduler");
    let page = notification_of(sched, id).ok_or(abi::Error::Gone)?;
    Ok(page.state.poll())
}

/// **`Notification::BIND`**: bind notification `id` to thread `tid`, once each (§101: "at most one
/// notification may be bound to a TCB", and one bound TCB per notification).
///
/// - `Gone`: the notification is stale, or the thread is gone or already dead.
/// - `NotPermitted`: either side is already bound to something that still exists. A binding whose
///   other half has been destroyed does not count, so a thread whose notification was reclaimed
///   can be bound again.
///
/// **A word already waiting is delivered at once if the thread is already receiving.** Without
/// this, a signal counted before the bind would sit in the word until the thread's *next* receive,
/// which for a server blocked forever in `RECEIVE` is never: not lost, but not delivered either.
pub fn notification_bind(id: NotificationId, tid: ThreadId) -> Result<(), abi::Error> {
    let mut guard = IPC_TABLES.lock();
    let sched = guard.as_mut().expect("no scheduler");
    let page = notification_of(sched, id).ok_or(abi::Error::Gone)?;
    if page.bound.is_some_and(|b| sched.threads.get(b).is_some()) {
        return Err(abi::Error::NotPermitted);
    }
    let t = sched.threads.get(tid).ok_or(abi::Error::Gone)?;
    if matches!(t.handshake.state, State::Finished | State::Dead) {
        return Err(abi::Error::Gone);
    }
    if t.bound_notification
        .is_some_and(|n| notification_of(sched, n).is_some())
    {
        return Err(abi::Error::NotPermitted);
    }
    page.bound = Some(tid);
    sched
        .threads
        .get_mut(tid)
        .expect("checked above under this hold")
        .bound_notification = Some(id);

    if page.state.word() != 0
        && let Some((tid, ep)) = bound_receiver(sched, page)
    {
        let word = page.state.poll();
        deliver_bound(sched, tid, ep, word);
        wake(sched, tid);
    }
    Ok(())
}

/// **The receive-side half of the binding**: on entry to a receive, a bound thread takes a word
/// that was counted while it was elsewhere, and returns without blocking. `None` when the thread
/// has no live binding or the word is zero. Caller holds `IPC_TABLES`.
///
/// It returns the bare word rather than the five-word delivery because a `u64` comes back in a
/// register and an array through memory; the caller builds [`bound_delivery`] from constants.
///
/// **`#[cold]` and `#[inline(never)]`, and the claim is about who pays.** The receive fastpath
/// tests `bound_notification.is_some()` on the thread it already holds, which is §101's "one load
/// and one compare"; only a bound thread reaches this body. Keeping it out of line keeps its bytes
/// out of `script/fastpath-footprint`'s closure, the same reason `set_ipc_aborted` is out of line.
#[cold]
#[inline(never)]
pub(super) fn take_bound_signal(sched: &mut IpcTables, tid: ThreadId) -> Option<u64> {
    let id = sched.threads.get(tid)?.bound_notification?;
    match notification_of(sched, id)?.state.poll() {
        0 => None,
        word => Some(word),
    }
}

/// **Tear down every notification whose page lies in `[base, end)`** (a region being destroyed):
/// abort and wake each waiter, then drop the name. The rendezvous sweep's shape, including its rule
/// to rescan rather than list, for its stack-depth reason. A thread bound to a notification destroyed
/// here keeps a stale name, which every reader treats as unbound. Caller holds `IPC_TABLES`.
pub(super) fn reap_region_notifications(sched: &mut IpcTables, base: u64, end: u64) {
    loop {
        let doomed = sched
            .notification_table
            .iter()
            .find(|&(_, &phys)| base <= phys && phys < end)
            .map(|(name, _)| name);
        let Some(name) = doomed else { break };
        if let Some(page) = notification_of(sched, name) {
            page.state.drain_waiters(|w| {
                let tid = hold_token(w);
                set_ipc_aborted(sched, tid);
                wake(sched, tid);
            });
        }
        sched.notification_table.remove(name);
    }
}
