//! **Where a thread runs**: spawn placement, the secondary cores' idle loops and inboxes, work
//! stealing, and the choice of a core for a woken thread.
//!
//! Cut out of `sched.rs` unchanged by milestone 840 (the scheduler file is split along its seams).
//!
//! Name: provisional (milestone 840's lane, 2026-10-11 UTC; §271 (the scheduler file splits into
//! submodules of `sched`)).

use super::*;

/// Make **this (secondary) core** a scheduler participant.
///
/// The boot core is set up by [`init`]; a secondary calls this once, as it comes online. It adopts
/// the context it is already running on as this core's idle thread (`cpu::current`/`cpu::idle`), and
/// reserves this core's run queue so `schedule()`'s push never allocates from the timer IRQ (§9),
/// exactly as `init` does for the boot core. After this, the core's run queue is empty, so it runs
/// its idle thread until work lands on the queue.
///
/// Interrupts must be masked (the caller has not enabled them yet), which is what `with_runq` needs.
pub fn adopt_secondary_idle() {
    // This core's turn at the line in `init` above: it is about to own threads.
    crate::arch::fp::init();

    let id = {
        let mut guard = IPC_TABLES.lock();
        let sched = guard
            .as_mut()
            .expect("adopt_secondary_idle before sched::init");
        let id = sched
            .threads
            .insert_in_place(|tid, dst| {
                // SAFETY: `dst` is a fresh, exclusively-owned TCB page, per `insert_in_place`.
                unsafe { Thread::write_adopted_current(dst, tid) };
                true
            })
            .expect("thread table full while bringing a core online");
        // This core is currently running that thread.
        let tcb = sched
            .threads
            .pointer(id)
            .expect("the idle thread was just inserted");
        // SAFETY: `id`'s page pointer from the table, live.
        unsafe { set_current(id, tcb) };
        id
    };

    // And it is also this core's idle fallback.
    cpu::current().idle.store(id, Ordering::Relaxed);
    // (No queue capacity to reserve: the queues are intrusive and a push cannot allocate.)
}

/// The reschedule / migration SGI. When one core hands another a thread (via its inbox), it fires
/// this at the target; the target's handler drains its inbox and reschedules. INTID 0, distinct
/// from the rendezvous-bound test SGIs (1 and 2). SMP step 3c.
///
/// aarch64 only in practice: RISC-V's twin path (`arch/riscv64/exceptions.rs`) recognises the IPI
/// from the SBI software-interrupt cause rather than from an interrupt id, so it needs no constant,
/// and `x86_64` sends its own reschedule IPI (`arch/x86_64/irq.rs`) rather than reading one.
#[cfg_attr(not(target_arch = "aarch64"), allow(dead_code))]
pub const RESCHED_SGI: u32 = 0;

/// Drain this core's migration inbox into its run queue, and request a reschedule.
///
/// Called from the reschedule-SGI handler: another core pushed one or more threads into our inbox
/// and poked us. We move them onto our own (single-owner) run queue and set `need_resched`, so the
/// handler's tail runs `schedule()` and picks them up. IRQ context, so interrupts are masked, which
/// is what `with_runq` needs; we hold nothing else, so taking the inbox is rank-safe (§11).
pub fn drain_inbox() {
    // Every caller is a cross-core interrupt arm (one per architecture), so this is where the
    // machine statistics page counts them (milestone 126).
    crate::machine_statistics::interrupt();
    let mut moved = 0u64;
    let mut inbox = cpu::current().inbox.lock();
    while let Some(thread) = inbox.pop_front() {
        // The token the inbox hands back goes straight onto the run queue. Nothing is dereferenced:
        // the handoff is pure token movement, which is why this needs no `IPC_TABLES`.
        cpu::current().with_runq(|q| q.push_back(thread));
        moved += 1;
    }
    // The inbox is empty now; mirror that under the lock (DECISIONS §28). The threads moved into the
    // run queue, whose own mirror `with_runq` just updated, so the total load is unchanged.
    cpu::current().note_inbox_len(inbox.len());
    // And count them: this is the only place a thread crosses from a remote core's hands into this
    // core's queue, which makes it the one honest observation point for "the placement arrived".
    cpu::current().note_adopted(moved);
    drop(inbox);
    if moved > 0 {
        trace::record(trace::Event::InboxDrain, moved, 0);
        cpu::current().need_resched.store(true, Ordering::Relaxed);
    }
}

pub(super) fn place_on(target: usize, thread: Unqueued<Thread>) -> Option<usize> {
    // A REMOTE parked cpu's inbox is drained by nothing, so placing there is a thread nothing
    // will ever run: the VisionFive 2 first-silicon hang (notes/visionfive2.md, third stop). The
    // online-set sweep removed every count-as-index chooser, and this is the audit lane's
    // tripwire for the next one: loud in debug/test builds, where every merge boots it. The
    // release board build compiles it out and keeps [`dump_threads`]'s parked-inbox line as the
    // field diagnostic.
    //
    // Placing onto ONESELF is exempt, and the exemption is load-bearing, found by this assertion
    // firing on the suite's own boot: a secondary's bring-up probe (`smp::secondary_main` step 6)
    // spawns onto its own core one step before that core sets its online bit, and a local
    // placement goes straight onto the running core's own run queue, which that core drains by
    // definition. The hazard this guards is exactly the remote case.
    debug_assert!(
        target == cpu::id() || {
            let mask = crate::smp::online_harts_mask();
            mask & (1 << target) != 0
        },
        "placement onto parked cpu {target} (online mask {:#b}): nothing drains a parked core's \
         inbox, so this thread would never run",
        crate::smp::online_harts_mask(),
    );
    if target == cpu::id() {
        cpu::current().with_runq(|q| q.push_back(thread));
        None
    } else {
        // Read before the push gives the token away. SAFETY: a token's thread is live while the
        // token exists (`Threads::mint_token`), and IPC_TABLES (held) serializes every `Thread`.
        let tid = unsafe { (*thread.as_ptr()).id };
        // The inbox mutex serializes access to the link.
        let mut inbox = cpu::inbox_of(target).lock();
        inbox.push_back(thread);
        // Mirror the target's inbox depth so it counts as load (DECISIONS §28); under the lock, so
        // the store is serialised with any concurrent drain.
        cpu::of(target).note_inbox_len(inbox.len());
        trace::record(trace::Event::PlaceRemote, tid, target as u8);
        Some(target)
    }
}

/// **Power of two choices: which core should a new thread run on** (DECISIONS §28.1). Sample two
/// random cores' runnable counters (relaxed, possibly stale, which §28 accepts) and return the
/// lighter. Near-optimal balancing that reads at most two remote counters no matter how many cores
/// there are, where a full least-loaded scan would contend on every counter and age badly. On a
/// single online core it is a no-op; the two samples may coincide, degrading to one choice harmlessly.
pub(super) fn pick_spawn_target() -> usize {
    let n = crate::smp::online_count();
    if n <= 1 {
        return cpu::id();
    }
    // The k-th ONLINE cpu, not index k: the online set is not contiguous from zero on real boards
    // (first-silicon bench, 2026-08-14: {1,2,3} online, and modulo-count placed init into parked
    // slot 0's inbox forever). See smp::online_cpus.
    let a = crate::smp::nth_online(cpu::current().rng_next() as usize);
    let b = crate::smp::nth_online(cpu::current().rng_next() as usize);
    if cpu::of(a).runnable() <= cpu::of(b).runnable() {
        a
    } else {
        b
    }
}

/// **Serve a pending work-steal request** (DECISIONS §28.3), at a scheduler entry where interrupts
/// are masked (the reschedule-SGI handler). If an idle core asked this core for work, hand it one
/// thread from our run queue and poke it. We give from the *queue*, never the thread on the CPU, and
/// only if we have one to spare; an empty give leaves the requester to ask again next tick, the
/// bounded cost §28 accepts. Pull-based and lock-free between run queues: the only shared structure
/// touched is the requester's inbox.
pub fn serve_steal_request() {
    // Take the request, which also clears the slot so the next idle core can queue a fresh one. The
    // acquire/release pairing and the read-and-clear live in `steal_request`, where loom checks
    // them (notes/interleaving.md); this function spends its own code on the hand-off.
    let Some(requester) = cpu::current().steal_request.take() else {
        return;
    };
    let requester = requester as usize;
    // One thread off our own queue, if any. `with_runq` keeps the runnable mirror exact.
    let thread = cpu::current().with_runq(|q| q.pop_front());
    if let Some(t) = thread {
        // SAFETY: reading the id of the thread we just popped and hold exclusively.
        let tid = unsafe { (*t.as_ptr()).id };
        trace::record(trace::Event::StealServe, tid, requester as u8);
        // The token we just popped goes to the requester's inbox; the inbox mutex serialises the
        // handoff and orders our pop before the requester's drain (the `place_on` discipline).
        let mut inbox = cpu::inbox_of(requester).lock();
        inbox.push_back(t);
        cpu::of(requester).note_inbox_len(inbox.len());
        crate::arch::irq::send_reschedule(requester);
    }
}

/// **An idle core asks a loaded core for work** (DECISIONS §28.3), from the idle loop. Pick the
/// most-loaded other core and, if it has a queued thread to spare, request one over its steal slot
/// and poke it with the reschedule SGI; the victim serves it at its next scheduler entry, the stolen
/// thread lands in our inbox, and that SGI's drain runs it. One outstanding request per victim
/// (`work_steal_slot::Slot::claim` is a compare-exchange from empty), so a crowd of idle cores
/// collapses to one steal per victim per round.
pub(super) fn try_initiate_steal() {
    // Do not steal if we have work of our own arriving: our run queue is empty (that is why the idle
    // thread is running), but the inbox may hold threads a remote just handed us that our own next
    // scheduler entry will drain. `runnable` counts those, so this guard defers to our own work.
    if cpu::current().runnable() > 0 {
        return;
    }
    let me = cpu::id();
    let mut victim = None;
    let mut best = 0usize;
    for c in crate::smp::online_cpus() {
        if c != me {
            // Steal only a run-queue backlog, never a victim's inbox in transit (see `runq_len`).
            let r = cpu::of(c).runq_len();
            if r > best {
                best = r;
                victim = Some(c);
            }
        }
    }
    if let Some(v) = victim
        && cpu::of(v).steal_request.claim(me as u32)
    {
        crate::arch::irq::send_reschedule(v);
    }
}

/// **The idle thread's body**, shared by the boot core's spawned idle thread and every secondary's
/// adopted idle context. Each pass: try to steal work from a loaded core (§28), park in `wfi` until
/// an interrupt (the stolen thread's SGI, a spawn's SGI, or the tick), then yield so the scheduler
/// runs whatever arrived. Never returns; it is the fallback the scheduler picks only when this
/// core's run queue is empty.
///
/// Before an idle thread existed, a moment where every thread was blocked waiting for I/O was a
/// kernel panic. It is never in the ready queue, so it never competes with real work, and it is
/// per-CPU as of §11 step 3b, so an idle core parks in its own `wfi`.
///
/// **It deliberately does not drain its own inbox before parking** (DECISIONS §133, calef,
/// 2026-08-28). Doing so would make a missed reschedule-SGI self-healing, and that was refused on
/// purpose: `drain_inbox` has one caller per architecture, the SGI handler, so a missed poke is
/// permanent rather than late, and a permanent wedge announces itself as a watchdog dump naming
/// the stranded thread and the undrained inbox. A self-healing drain would turn the same defect
/// into threads occasionally starting late, which nothing files, bisects or gates. That mattered
/// concretely: the `place_on` stale-locality lost wakeup was found from one such dump and never
/// reproduced in the wild, 0 crossings in over 1,600 `spawn_on` calls across five instrumented
/// runs. The section records what would reopen it.
pub fn run_idle() -> ! {
    loop {
        // **The capability-slot gauge** (milestone 231). Here because this is the one place the
        // kernel reaches after every phase of a boot with nothing else to do; see
        // `cap::report_peak` for why waiting for the mark to settle is what makes it one line.
        crate::cap::report_peak();
        // And the progenitor's stack gauge, from the same place for the same reason.
        crate::progenitor_stack::report_peak();
        try_initiate_steal();
        crate::arch::wait_for_interrupt();
        yield_now();
    }
}

/// **Which core should a device-IRQ wake place its driver on** (DECISIONS §28.2). The least-loaded
/// online core, with the current (IRQ-handling) core winning ties: only a *strictly* less-loaded
/// core displaces it. That is what makes this load-aware without thrashing. A driver that takes a
/// completion interrupt every request (the block server through a RedoxFS mount) wakes on the same
/// affinity core each time and, since that core is no more loaded than any other while it is the
/// only work, stays there. When a core does pile up (the `std_net` RX path landing beside real work),
/// a strictly-lighter core pulls the driver off it, so the pipeline stops re-concentrating. A full
/// scan is fine: device IRQs are not the spawn hot path and `MAX_CPUS` is small.
pub(super) fn pick_wake_target() -> usize {
    let here = cpu::id();
    let mut best = here;
    let mut best_load = cpu::current().runnable();
    // The online SET, never `0..count` (first-silicon sweep, 2026-08-14): on the VisionFive 2 the
    // set is {1,2,3}, and the count-as-index loop would read parked slot 0's zeroed `runnable()`,
    // which wins every comparison, so every device-IRQ wake would land in a dead core's inbox. It
    // also never considered online cpu 3. See smp::online_cpus.
    for c in crate::smp::online_cpus() {
        if c == here {
            continue;
        }
        let load = cpu::of(c).runnable();
        if load < best_load {
            best_load = load;
            best = c;
        }
    }
    best
}

/// A **device-interrupt** wake (DECISIONS §28.2): load-aware, not local. Where [`wake`] queues a
/// rendezvous partner on the waker's own core (message in registers, cache warm), an interrupt
/// carries no such locality, and pinning the driver to the IRQ core re-concentrates the pipeline
/// (the `std_net` lesson). So place it on [`pick_wake_target`]'s choice. Returns `Some(target)` when
/// that is a *remote* core, so the caller sends the reschedule SGI after releasing `IPC_TABLES`; `None`
/// when it stayed local or the wake was parked. Caller holds the lock.
pub(super) fn wake_load_aware(sched: &mut IpcTables, tid: ThreadId) -> Option<usize> {
    let t = sched.threads.get_mut(tid)?;
    // The whole decision (not-blocked, the boot-8 undelivered-wake gate, the switch-out deferral)
    // is `thread_wake_handshake::Handshake::try_wake`, the extracted protocol loom searches on the host
    // (notes/interleaving.md). This function keeps what the crate cannot see: the trace ring, the
    // progress heartbeat, and §28.2's placement policy on the one verdict that queues.
    match t.handshake.try_wake() {
        WakeVerdict::NotBlocked => None,
        WakeVerdict::Refused => {
            trace::record(trace::Event::WakeRefused, tid, 0);
            None
        }
        WakeVerdict::Deferred => {
            // A device-IRQ wake is forward progress too (test builds only). A deferral in this
            // window is rare, and one non-load-aware completion in `finish_switch` is not worth
            // teaching that path a placement policy.
            #[cfg(any(test, feature = "system_tests"))]
            crate::testing::note_progress();
            trace::record(trace::Event::WakeDeferred, tid, 0);
            None
        }
        WakeVerdict::Queue => {
            #[cfg(any(test, feature = "system_tests"))]
            crate::testing::note_progress();
            // Blocked -> Ready: whatever unlinked it handed its token back (`hold_token`).
            let Some(token) = t.own_token.take() else {
                missing_token()
            };
            trace::record(trace::Event::Wake, tid, 0);
            // One placement decision, `place_on`'s: onto this core's own run queue, or into the
            // target's inbox with the inbox-len mirror kept under the inbox lock. Its return value
            // is what tells `irq_notify` to poke the target once IPC_TABLES drops, and it is the
            // only thing that may: a second `target == cpu::id()` comparison outside the lock can
            // disagree with the one that placed (see `place_on`).
            //
            // IPC_TABLES masks interrupts, which `with_runq` needs on the local side.
            place_on(pick_wake_target(), token)
        }
    }
}
