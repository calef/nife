//! **Waking a blocked thread.**
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

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
pub(super) fn wake(sched: &mut IpcTables, tid: ThreadId) {
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
