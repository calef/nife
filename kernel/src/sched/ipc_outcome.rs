//! **How a blocking IPC learned it failed**: resolving a rendezvous name, and the aborted and
//! refused flags the syscall layer reads and clears after the primitive returns.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// The rendezvous behind a name, or `None` if the name no longer resolves. Caller holds `IPC_TABLES`.
///
/// This used to panic on a miss, because endpoints could not be destroyed (their regions stayed
/// pinned), so a miss was kernel corruption. Object revocation made destruction real: a stale
/// `Rendezvous` capability (its rendezvous reclaimed out from under a holder) is now ordinary user
/// input, so this returns `None` and the callers turn that into a clean error rather than a panic.
///
/// The `'static` is the page's pinned-ness made into a lifetime: while the name resolves the page is
/// pinned and direct-mapped, and `IPC_TABLES` serializes every access to what it holds.
pub(super) fn rendezvous_of(
    sched: &IpcTables,
    ep: RendezvousId,
) -> Option<&'static mut Rendezvous> {
    let phys = *sched.rendezvous_table.get(ep)?;
    // SAFETY: retyped exclusively for this rendezvous, its region pinned while the name resolves,
    // direct-mapped, and serialized by IPC_TABLES, which every caller holds.
    Some(unsafe { &mut *(crate::arch::mmu::phys_to_virt(phys) as *mut Rendezvous) })
}

/// Mark the current thread's blocking IPC as aborted (a stale rendezvous, or one revoked while it
/// blocked): the syscall layer reads-and-clears this after the primitive returns and hands back an
/// error. A helper because several IPC paths set it. Caller holds `IPC_TABLES`.
///
/// **`#[cold]`, and it is a claim about the callers rather than about this body.** Every call site
/// is the `else` of a `rendezvous_of` that returned `None`, which means the name a program invoked
/// no longer resolves. A healthy IPC never reaches it, and it is reached from four functions that
/// are all on `script/fastpath-footprint`'s closures, so inlining it put the abort path's bytes on
/// the fast path four times over (milestone 188 phase 3).
#[cold]
#[inline(never)]
pub(super) fn set_ipc_aborted(sched: &mut IpcTables, tid: ThreadId) {
    if let Some(t) = sched.threads.get_mut(tid) {
        t.handshake.abort();
        // **The capability staged for the aborted send goes with it** (the 2026-10-03 security
        // audit's follow-up). A `SEND_CAP` or `CALL` that parked put its delegation, or the Reply
        // the kernel minted, in `outgoing_cap` for the receiver to take. An abort means no receiver
        // ever will: the rendezvous is gone. Left in place, the next plain `SEND` this thread
        // parked on a *different* rendezvous would hand that capability to whoever `RECEIVE_CAP`s
        // there, a delegation the sender made to one endpoint delivered to another. The sender
        // still holds its own copy (`SEND_CAP` narrows a copy, it never moves the source), so
        // nothing is lost by dropping this one.
        t.outgoing_cap = None;
    }
}

/// **Refuse the current thread's send, because the rendezvous carries an interrupt** (milestone 603
/// (provisional), DECISIONS §101 ruling B). An abort, so the syscall layer's existing
/// `take_ipc_aborted` branch is the only one the common path pays for, plus the reason, which only
/// that branch reads ([`take_ipc_refused`]). The sender never parked, so the abort does not weaken
/// the boot-8 gate for anything: it is taken immediately, as a stale endpoint's is.
///
/// `#[cold]` for [`set_ipc_aborted`]'s reason: no healthy IPC reaches it, and it is reached from
/// three functions on `script/fastpath-footprint`'s closures.
///
/// Name: provisional (milestone 603 (provisional)): calef names public items.
#[cold]
#[inline(never)]
pub(super) fn set_ipc_refused(sched: &mut IpcTables, tid: ThreadId) {
    if let Some(t) = sched.threads.get_mut(tid) {
        t.handshake.abort();
        t.ipc_refused = true;
    }
}

/// **Read and clear why the current thread's aborted send was aborted**: `true` when the rendezvous
/// carries an interrupt and refused it ([`set_ipc_refused`]), `false` when it was stale or revoked.
/// Called by the syscall layer only after [`take_ipc_aborted`] returned `true`, so an IPC that was
/// not aborted never pays for it.
///
/// Name: provisional (milestone 603 (provisional)): calef names public items.
#[cold]
#[inline(never)]
pub fn take_ipc_refused() -> bool {
    let mut guard = IPC_TABLES.lock();
    let Some(sched) = guard.as_mut() else {
        return false;
    };
    let tid = current_thread_id();
    sched
        .threads
        .get_mut(tid)
        .map(|t| core::mem::take(&mut t.ipc_refused))
        .unwrap_or(false)
}

/// **Read and clear the current thread's IPC-aborted flag** (object revocation). The syscall layer
/// calls this right after an rendezvous IPC primitive returns: `true` means the rendezvous was stale, or
/// revoked while the thread blocked on it, so the caller gets an error instead of the primitive's
/// placeholder result. Kernel-side IPC callers never set it (their endpoints are never revoked), so
/// they need not check it.
pub fn take_ipc_aborted() -> bool {
    let mut guard = IPC_TABLES.lock();
    let Some(sched) = guard.as_mut() else {
        return false;
    };
    let tid = current_thread_id();
    sched
        .threads
        .get_mut(tid)
        .map(|t| t.handshake.take_aborted())
        .unwrap_or(false)
}
