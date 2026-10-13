//! **How a thread leaves the CPU**: yielding, exiting, faulting, and the death message a
//! supervisor receives.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

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
pub(super) fn depart(event: u64, pc: u64, addr: u64, whole: bool) -> ! {
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
pub(super) fn deliver_death(
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
