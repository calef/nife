//! **The methods of the smaller objects**: `Notification`, `Reboot`, `Timer`, `Process`, the
//! interrupt and virtio capabilities, `PortRange`, the rendezvous badge and reap, and the two helpers
//! [`invoke`](super::invoke) leaves its rare Reply and abort paths to.
//!
//! Cut out of `syscall.rs` unchanged on 2026-10-12 (UTC) by lane maintainer/near-ceiling-splits,
//! so the parent stays under §266 (a Rust source file stays under 2,000 lines) and moves toward
//! §282 (when a file is split, the parent becomes an index). Each method keeps its
//! `#[inline(never)]`, so [`invoke`](super::invoke) reaches it through one call, as before, and
//! `script/fastpath-footprint` measures the same code under a longer symbol name. `pub(super)`
//! is the visibility a private item of `syscall` had: `syscall` and its children.
//!
//! Name: provisional (lane maintainer/near-ceiling-splits, 2026-10-12 UTC), for the module and the
//! file.

use super::*;

/// **A Reply method other than `REPLY`.** One exists: **`REPLY_CAPABILITY`, `REPLY` plus one
/// capability**, `carried` its slot in our table (§255 (each socket is its own capability)). `GRANT` on the carried capability is checked under the hold that
/// files it, and a refused source consumes nothing, so the caller still waits and this Reply can
/// still answer it. `Ok(1)` when the answer went but the copy did not reach the caller (a full
/// table, or a caller no longer waiting), so the server can undo what it minted it for.
///
/// Out of line on purpose, and handed the syscall's own registers so the call site is a jump. It
/// looks the Reply up again rather than taking it from [`invoke`](super::invoke), which costs a
/// cold method one table read. [`invoke`](super::invoke) inlines into `syscall::dispatch`, which
/// `script/fastpath-footprint` counts flat as `syscall_entry`; with this arm folded in, x86_64's
/// entry grew 144 bytes (8.6%) on every syscall for a method only the network stack uses. Plain
/// `REPLY` stays inline.
#[inline(never)]
pub(super) fn reply_other(
    slot: u64,
    method: u64,
    a0: u64,
    a1: u64,
    carried: u64,
) -> Result<i64, Error> {
    if method != abi::reply::REPLY_CAPABILITY {
        return Err(Error::BadMethod);
    }
    let reply = sched::current_cap(slot).map_err(|_| Error::NoSuchSlot)?;
    let Object::Reply(tid) = reply.object else {
        return Err(Error::BadMethod);
    };
    // As for `REPLY`: minted WRITE-only and without GRANT.
    if !reply.rights.allows(Rights::WRITE) {
        return Err(Error::NotPermitted);
    }
    let filed = sched::ipc_reply_capability(tid, [a0, a1], carried)?;
    // One-shot, as for `REPLY`.
    let _ = sched::delete_current_cap(slot);
    Ok(if filed { 0 } else { 1 })
}

/// **Which error an aborted `SEND`, `SEND_CAP` or `CALL` answers** (milestone 603 (provisional)).
/// `NotPermitted` when the endpoint carries an interrupt and refused the deposit (DECISIONS §101,
/// ruling B), `Gone` when it was stale or revoked. Out of line and `#[cold]`, because it runs only
/// on an abort and the three arms that call it are on the IPC fastpath.
///
/// Name: provisional (milestone 603 (provisional)): calef names public items.
#[cold]
#[inline(never)]
pub(super) fn aborted_send_error() -> Error {
    if sched::take_ipc_refused() {
        Error::NotPermitted
    } else {
        Error::Gone
    }
}

/// The four `Notification` methods (milestone 151, DECISIONS §101). Rights are §101's: `WRITE` to
/// signal and to bind, `READ` to wait and to poll. `BIND` also needs `WRITE` on the thread it names,
/// because binding changes what that thread's receives return. See `abi::notification`.
#[inline(never)]
pub(super) fn notification_invoke(
    rights: Rights,
    id: sched::NotificationId,
    method: u64,
    a0: u64,
) -> Result<i64, Error> {
    match method {
        abi::notification::SIGNAL => {
            if !rights.allows(Rights::WRITE) {
                return Err(Error::NotPermitted);
            }
            sched::notification_signal(id, a0)?;
            Ok(0)
        }
        abi::notification::WAIT => {
            if !rights.allows(Rights::READ) {
                return Err(Error::NotPermitted);
            }
            sched::notification_wait(id).map(|w| w as i64)
        }
        abi::notification::POLL => {
            if !rights.allows(Rights::READ) {
                return Err(Error::NotPermitted);
            }
            sched::notification_poll(id).map(|w| w as i64)
        }
        abi::notification::BIND => {
            if !rights.allows(Rights::WRITE) {
                return Err(Error::NotPermitted);
            }
            let thread = sched::current_cap(a0).map_err(|_| Error::NoSuchSlot)?;
            let Object::ThreadControlBlock(tid) = thread.object else {
                return Err(Error::WrongObject);
            };
            if !thread.rights.allows(Rights::WRITE) {
                return Err(Error::NotPermitted);
            }
            sched::notification_bind(id, tid)?;
            Ok(0)
        }
        _ => Err(Error::BadMethod),
    }
}

/// `Reboot::REBOOT` (milestone 805, DECISIONS §251): restart the machine, or answer why not, one of
/// the four `abi::Error::Reset…` reasons. See kernel/src/reboot.rs.
#[inline(never)]
pub(super) fn reboot_invoke(method: u64) -> Result<i64, Error> {
    match method {
        abi::reboot::REBOOT => Err(crate::reboot::restart()),
        _ => Err(Error::BadMethod),
    }
}

/// The two `Timer` methods (milestone 106, DECISIONS §147). `WRITE` on the timer for both, and
/// for `ARM` also `WRITE` on the notification it names, because arming is signalling later and
/// `SIGNAL` takes `WRITE`. See `abi::timer`.
#[inline(never)]
pub(super) fn timer_invoke(
    rights: Rights,
    id: sched::TimerId,
    method: u64,
    a0: u64,
    a1: u64,
    a2: u64,
) -> Result<i64, Error> {
    if !rights.allows(Rights::WRITE) {
        return Err(Error::NotPermitted);
    }
    match method {
        abi::timer::ARM => {
            let target = sched::current_cap(a1).map_err(|_| Error::NoSuchSlot)?;
            let Object::Notification(notification) = target.object else {
                return Err(Error::WrongObject);
            };
            if !target.rights.allows(Rights::WRITE) {
                return Err(Error::NotPermitted);
            }
            sched::timer_arm(id, a0, notification, a2)?;
            Ok(0)
        }
        abi::timer::CANCEL => sched::timer_cancel(id).map(i64::from),
        _ => Err(Error::BadMethod),
    }
}

/// `Rendezvous::BADGE`: **mint a badged copy of an endpoint** (milestone 599 (a frame per
/// filesystem client channel), provisional). The new capability names the same endpoint `ep` with
/// the holder's own `rights`, plus `new_badge`, in a free slot of the caller's table, and the slot
/// is the result, the way `RETYPE_OBJ` answers with the slot it minted.
///
/// GRANT-gated, because minting a delegatable view is a delegation-class power (the gate `SEND_CAP`
/// and `CAP_INSERT` use), and refused for a zero badge (the unbadged value) or an already-badged
/// source (`held_badge != 0`), so a badge is set once and never changed: seL4's rule, and what lets
/// a server trust the badge it is delivered. Out of line for `memory_region_map`'s reason: it is
/// spawn-time administration, and `script/fastpath-footprint` measures `invoke`'s own bytes.
#[inline(never)]
pub(super) fn rendezvous_badge(
    ep: sched::RendezvousId,
    rights: Rights,
    held_badge: u64,
    new_badge: u64,
) -> Result<i64, Error> {
    if !rights.allows(Rights::GRANT) || new_badge == 0 || held_badge != 0 {
        return Err(Error::NotPermitted);
    }
    let slot = sched::grant(crate::cap::rendezvous_cap_badged(ep, rights, new_badge))
        .map_err(|_| Error::OutOfMemory)?;
    Ok(slot as i64)
}

/// `Rendezvous::REAP`: collect a corpse this rendezvous supervises (DECISIONS §32), the control
/// half `rendezvous::SURVEY` is the view half of. Administrative, not the IPC round trip: rare
/// enough (one call per child death, not per message) that it moves out of `invoke`'s own bytes
/// with the rest of this arm's non-fastpath methods. `#[inline(never)]` for the reason
/// `memory_region_map` gives.
#[inline(never)]
pub(super) fn rendezvous_reap(ep: crate::sched::RendezvousId, tid: u64) -> Result<i64, Error> {
    sched::reap_supervised(ep, tid)?;
    Ok(0)
}

/// `Process::DESTROY` (milestone 812): end the process and every thread in it. `WRITE`.
#[inline(never)]
pub(super) fn process_invoke(
    rights: Rights,
    pid: crate::sched::ProcessId,
    method: u64,
) -> Result<i64, Error> {
    match method {
        abi::process::DESTROY => {
            if !rights.allows(Rights::WRITE) {
                return Err(Error::NotPermitted);
            }
            sched::destroy_process(pid)?;
            Ok(0)
        }
        _ => Err(Error::BadMethod),
    }
}

/// `Irq::WAIT`: block on the endpoint the kernel routed this interrupt to. The interrupt arrives
/// as a message (`sched::irq_notify`), exactly like any other. `#[inline(never)]` for the reason
/// `memory_region_map` gives.
#[inline(never)]
pub(super) fn irq_wait(frame: &mut TrapFrame, intid: u32) -> Result<i64, Error> {
    let ep = sched::irq_route(intid).ok_or(Error::WrongObject)?;
    let m = sched::ipc_receive(ep);
    // A driver with a bound notification (milestone 151) is woken here by either the interrupt
    // (`1` in x0, x1 and x4 zero) or the notification (`abi::notification::BOUND` in x0 and x4, the
    // word in x1). This is what lets a driver wait on its device and a deadline at once, which is
    // the complaint milestone 106 (a wait that ends on either the interrupt or the deadline) makes about `net_stack`. Unconditional stores: an interrupt's mailbox has
    // zeros in both, so a driver that never bound anything reads the same zeros it would have.
    frame.set_arg(1, m[1]);
    frame.set_arg(4, m[4]);
    Ok(m[0] as i64)
}

/// `Irq::ACK`: re-enable the interrupt at the controller. The kernel masked it when it fired; now
/// that the driver has serviced the device, it is safe to let it fire again. This names
/// `arch::irq`, not a specific controller: the GIC on aarch64, the PLIC on RISC-V.
/// `#[inline(never)]` for the reason `memory_region_map` gives.
#[inline(never)]
pub(super) fn irq_ack(intid: u32) -> Result<i64, Error> {
    crate::arch::irq::enable(intid);
    Ok(0)
}

/// `Virtio`'s four register-level methods (`READ_REG`, `WRITE_REG`, `SETUP_QUEUE`, `NOTIFY`): a
/// driver's transport-level plumbing, not a step of the IPC round trip. `#[inline(never)]` for
/// the reason `memory_region_map` gives.
#[inline(never)]
pub(super) fn virtio_invoke(id: usize, method: u64, a0: u64, a1: u64) -> Result<i64, Error> {
    use crate::virtio::TransportError;
    let map = |e: TransportError| match e {
        TransportError::DmaEscape => Error::DeviceRefused,
        _ => Error::WrongObject,
    };
    match method {
        abi::virtio::READ_REG => crate::virtio::read_register(id, a0)
            .map(|v| v as i64)
            .ok_or(Error::WrongObject),
        abi::virtio::WRITE_REG => crate::virtio::write_register(id, a0, a1 as u32)
            .map(|_| 0)
            .map_err(map),
        abi::virtio::SETUP_QUEUE => crate::virtio::setup_queue(id, a0 as u16, a1 as u16)
            .map(|_| 0)
            .map_err(map),
        abi::virtio::NOTIFY => crate::virtio::notify(id, a0 as u16).map(|_| 0).map_err(map),
        _ => Err(Error::BadMethod),
    }
}

/// `PortRange::REVOKE` (milestone 299): take the port range back from every other holder. The only
/// method a port capability answers; enforcement is at the switch, not here.
///
/// **`#[cold]`, not just `#[inline(never)]`.** A live-driver-replacement take-back is among the
/// rarest things a program does, and `syscall_entry`'s footprint bound is measured over the
/// *non-cold* calls reachable from the dispatcher (`script/fastpath-footprint`). Marking this cold
/// keeps it and `revoke_port_range_from_others` out of that closure, so a new capability method does
/// not spend the IPC round trip's L1i budget on a path the round trip never takes.
#[cfg(target_arch = "x86_64")]
#[cold]
#[inline(never)]
pub(super) fn port_range_invoke(
    rights: Rights,
    base: u16,
    count: u16,
    method: u64,
) -> Result<i64, Error> {
    match method {
        abi::port_range::REVOKE => {
            if !rights.allows(Rights::GRANT) {
                return Err(Error::NotPermitted);
            }
            crate::revoke::revoke_port_range_from_others(base, count);
            Ok(0)
        }
        _ => Err(Error::BadMethod),
    }
}
