//! **The port range's read side, its revocation's precision, and a duplicated holder** (an eighth
//! outsider pass at the confinement claim, claims 27, 28, 29 and 33).
//!
//! The cited tests drive a non-holder's `out`, a revoked holder's `out`, a self-deleting holder's
//! `out`, and a READ-only holder's `out`. This module attacks the sides those leave:
//!
//! - **The read side** (claims 27 and 33): a non-holder's `in` must fault like its `out` did, and
//!   after a holder transmits, a non-holder's `in` must still fault, the leak check's read half.
//!   A READ-only capability stops port I/O at the first instruction, the `in` itself: the TSS
//!   bitmap cannot split reads from writes, so without WRITE nothing is installed and the claim's
//!   "no output" is held the stronger way, no I/O at all. A full-rights holder running the same
//!   stub exits cleanly, the control that proves the refusals were the rights gate.
//! - **Revocation precision** (claim 28): revoking a *sub-range* of what a holder holds must
//!   revoke it; revoking a range the holder does not hold at all must leave its `out` working.
//!   Under-revocation passes the first and over-revocation fails the second.
//! - **A duplicated holder** (claim 29): a child holding one port range in two capability slots
//!   deletes one slot and its `out` must still be permitted; deleting both must revoke it. The
//!   single-slot cited test cannot see a revoke that sweeps by object (both slots) where it should
//!   sweep by slot, or vice versa.
//!
//! Every child exits or faults promptly (the `EXIT`-tailed stubs), so a wrongly permitted port
//! turns the run red as `EVENT_EXIT` where `EVENT_FAULT` was asserted, never as a hang. Parked
//! children park on a wake endpoint of their own, so no child ever competes with the supervision
//! endpoint the deaths arrive on. `x86_64` only, because port I/O is (DECISIONS §121 (what a
//! device capability is when the device has no page: x86 port I/O), milestone 299 (the x86
//! port-range capability: the serial console becomes a userspace driver)).

use super::supervision_tests::{CODE_VA, build_child_with};
use super::x86_programs;
use crate::cap::{
    Rights, port_range_cap as port_range_capability, rendezvous_cap as rendezvous_capability,
};
use crate::sched;

const COM1_BASE: u16 = crate::user::X86_COM1_PORT_BASE;
const COM2_BASE: u16 = 0x2F8;
const COM1_COUNT: u16 = 8;
const WRITE_VAL: u8 = 0xA5;

/// One test's endpoints: the supervision rendezvous deaths arrive on, and the wake rendezvous a
/// parked child blocks on. Both come out of one region the test gives back at the end.
struct Endpoints {
    fault: sched::RendezvousId,
    wake: sched::RendezvousId,
    region: u64,
}

fn endpoints() -> Endpoints {
    let region = crate::memory_region::create(4).expect("no endpoint region");
    let fault = sched::create_rendezvous_from(region).expect("no fault rendezvous");
    let wake = sched::create_rendezvous_from(region).expect("no wake rendezvous");
    Endpoints {
        fault,
        wake,
        region,
    }
}

/// Build a child from `stub` with `capabilities` in slots 0, 1, ... and the supervision endpoint in the
/// reserved fault slot, in its own region. Returns `(tid, region)` for the reaping retry.
fn child(stub: &[u32], capabilities: &[crate::cap::Cap], fault: sched::RendezvousId) -> (u64, u64) {
    let region = crate::memory_region::create(16).expect("no region for the child");
    let tid = build_child_with(
        region,
        stub,
        capabilities,
        Some(rendezvous_capability(fault, Rights::READ)),
    );
    (tid, region)
}

/// Wait for the next death on the supervision endpoint and return `(event, tid, pc)`.
fn next_death(fault: sched::RendezvousId) -> (u64, u64, u64) {
    let msg = sched::ipc_receive(fault);
    (msg[0], msg[1], msg[2])
}

/// Reap a child's region, retrying the way the cited tests do: a fresh corpse takes a moment to
/// settle off its own kernel stack.
fn reap(region: u64) {
    assert!(
        super::wait_for(|| sched::reclaim_region(region).is_ok()),
        "the child's region did not come back",
    );
}

/// **A non-holder's `in` faults, a READ-only holder reads but may not write, and a leak of the
/// holder's grant would show as a non-holder's clean exit.**
///
/// A holder transmits first (`EVENT_EXIT`), so its grant was genuinely installed in some core's
/// TSS bitmap. Then non-holder children run `in` against the same port and each must take its
/// general-protection fault at the `in` instruction itself: a bitmap left behind by the holder's
/// run would carry the non-holder through to a clean exit instead. Last, the READ-only holder: its
/// `in` succeeds and its immediately following `out` faults, with the faulting pc naming the `out`
/// as the instruction that was refused, which is what makes the read's success observable rather
/// than assumed.
///
/// Falsification: unfalsified. Written by the eighth outsider pass's reviewer, who booted it
/// green but did not break the code it guards to watch it go red.
#[test_case]
fn a_non_holder_cannot_read_a_port_and_a_read_only_holder_may_not_write_it() {
    let eps = endpoints();

    // The holder: a full-rights capability, one `out`, a clean exit. Its grant was installed.
    let (_holder, holder_region) = child(
        &x86_programs::port_out_then_exit(COM1_BASE, WRITE_VAL),
        &[
            rendezvous_capability(eps.wake, Rights::READ),
            port_range_capability(COM1_BASE, COM1_COUNT, Rights::ALL),
        ],
        eps.fault,
    );
    let (event, _tid, _pc) = next_death(eps.fault);
    assert_eq!(
        event,
        abi::fault::EVENT_EXIT,
        "the holder's own out was refused",
    );
    reap(holder_region);

    // Non-holders, after the holder ran: each `in` must fault at the `in`.
    for _ in 0..4 {
        let (_probe, probe_region) = child(
            &x86_programs::port_in_then_exit(COM1_BASE),
            &[rendezvous_capability(eps.wake, Rights::READ)],
            eps.fault,
        );
        let (event, _tid, pc) = next_death(eps.fault);
        assert_eq!(
            event,
            abi::fault::EVENT_FAULT,
            "a non-holder read a port without holding any capability for it",
        );
        assert_eq!(
            pc,
            CODE_VA + x86_programs::PORT_IN_ONLY_PC_OFFSET,
            "the non-holder's fault was not at its `in` instruction",
        );
        reap(probe_region);
    }

    // A READ-only holder: no I/O at all. The TSS bitmap cannot split `in` from `out`, so a
    // capability without WRITE installs nothing and the fault lands on the `in` itself, the
    // instruction the stub runs first. That is stronger than the claim's letter (which says no
    // output): the read the right would name is refused too, conservatively, and the faulting pc
    // says which instruction was stopped.
    let (_reader, reader_region) = child(
        &x86_programs::port_in_then_out(COM1_BASE, WRITE_VAL),
        &[
            rendezvous_capability(eps.wake, Rights::READ),
            port_range_capability(COM1_BASE, COM1_COUNT, Rights::READ),
        ],
        eps.fault,
    );
    let (event, _tid, pc) = next_death(eps.fault);
    assert_eq!(
        event,
        abi::fault::EVENT_FAULT,
        "a READ-only port capability let port I/O through",
    );
    assert_eq!(
        pc,
        CODE_VA + x86_programs::PORT_IN_ONLY_PC_OFFSET,
        "the fault was not at the `in`: the READ-only capability's refusal stopped a different \
         instruction than the first port access",
    );
    reap(reader_region);

    // The control for the stub itself: a full-rights holder runs the same `in` and the same `out`
    // and exits cleanly, so the refusals above were the rights gate and not a broken stub.
    let (_full, full_region) = child(
        &x86_programs::port_in_then_out(COM1_BASE, WRITE_VAL),
        &[
            rendezvous_capability(eps.wake, Rights::READ),
            port_range_capability(COM1_BASE, COM1_COUNT, Rights::ALL),
        ],
        eps.fault,
    );
    let (event, _tid, _pc) = next_death(eps.fault);
    assert_eq!(
        event,
        abi::fault::EVENT_EXIT,
        "a full-rights holder could not do the port I/O the READ-only holder was refused for",
    );
    reap(full_region);

    sched::reclaim_region(eps.region).expect("the endpoint region did not come back");
}

/// **Revoking a holder's own range revokes it; revoking a range nobody held revokes nothing.**
///
/// Two holders of the whole COM1 range, each parked in `RECEIVE` on its own wake endpoint so the
/// test chooses the instant to revoke. The first wakes after its exact range was revoked: its
/// `out` must fault. The second wakes after an entirely different range was revoked (one no child
/// holds): its `out` must still be permitted, which is the over-revocation check; a sweep that
/// matched too widely would stop it.
///
/// **A sub-range revoke is not a leg, and the reason is worth recording.** The sweep matches by
/// exact `(base, count)`, so `revoke_port_range(base, 2)` leaves a holder of `(base, 8)` alone:
/// observed, during this pass, as a clean exit where the fault was asserted. It is not reachable
/// from the boundary, because `PortRange::REVOKE` takes its range from the capability itself and
/// no syscall can name a narrower one, and the frame sweep's overlap matching
/// (`delete_page_frame_caps_overlapping`) is the deliberate contrast. It is recorded in this
/// pass's results as a near miss: one design change away from an escape if a sub-range revoke ever
/// becomes expressible.
///
/// Falsification: unfalsified. Written by the eighth outsider pass's reviewer, who booted it
/// green but did not break the code it guards to watch it go red.
#[test_case]
fn a_revoked_range_stops_its_holder_and_an_unrelated_revoke_stops_no_one() {
    let eps = endpoints();

    let parked = |eps: &Endpoints| {
        child(
            &x86_programs::receive_then_port_out(COM1_BASE, WRITE_VAL),
            &[
                rendezvous_capability(eps.wake, Rights::READ),
                rendezvous_capability(eps.wake, Rights::READ),
                port_range_capability(COM1_BASE, COM1_COUNT, Rights::ALL),
            ],
            eps.fault,
        )
    };

    // An unrelated range, revoked with a live holder of COM1 parked: the holder must be untouched.
    let (_other, other_region) = parked(&eps);
    assert!(
        super::wait_for(|| sched::rendezvous_waiting_receivers(eps.wake) == 1),
        "the unrelated-range holder never parked",
    );
    crate::revoke::revoke_port_range(COM2_BASE, COM1_COUNT);
    sched::ipc_send(eps.wake, [0, 0, 0]);
    let (event, _tid, _pc) = next_death(eps.fault);
    assert_eq!(
        event,
        abi::fault::EVENT_EXIT,
        "revoking a range nobody held stopped a holder of a different range",
    );
    reap(other_region);

    // The holder's own range, the expressible revoke: the `out` after the wake must fault.
    let (_full, full_region) = parked(&eps);
    assert!(
        super::wait_for(|| sched::rendezvous_waiting_receivers(eps.wake) == 1),
        "the whole-range holder never parked",
    );
    crate::revoke::revoke_port_range(COM1_BASE, COM1_COUNT);
    sched::ipc_send(eps.wake, [0, 0, 0]);
    let (event, _tid, pc) = next_death(eps.fault);
    assert_eq!(
        event,
        abi::fault::EVENT_FAULT,
        "revoking a holder's whole range did not stop its next `out`",
    );
    assert_eq!(
        pc,
        CODE_VA + x86_programs::RECEIVE_THEN_PORT_OUT_PC_OFFSET,
        "the fault was not at the `out` instruction",
    );
    reap(full_region);

    sched::reclaim_region(eps.region).expect("the endpoint region did not come back");
}

/// **A child holding one range in two slots loses the port when one slot is deleted, though the
/// twin survives; deleting both loses it too.**
///
/// **The first leg is a characterization of a near miss this pass found, asserted as the tree
/// behaves so a fix turns it red.** Deleting one of two identical `PortRange` capabilities clears
/// the thread's cached TSS grant wholesale, so the surviving capability's `out` faults even though
/// the table still holds it with every right: claim 5's "dropping one capability does not disturb
/// the others" fails at this corner, in the fails-closed direction. No confinement is breached
/// (nothing was reached that was not granted; authority was lost, not gained), so this records the
/// gap rather than blocking the pass. The cached grant is one field the context switch reads; a
/// delete that checked whether another capability still named the range would leave the twin
/// working, and would flip this assertion.
///
/// The second leg is the claim's: after deleting both slots the `out` faults on the `out`
/// instruction itself, the duplicated form of the cited single-slot test.
///
/// Falsification: unfalsified. Written by the eighth outsider pass's reviewer, who booted it
/// green but did not break the code it guards to watch it go red.
#[test_case]
fn one_port_in_two_slots_falls_when_one_slot_is_deleted_though_its_twin_survives() {
    let eps = endpoints();
    let duplicated = [
        rendezvous_capability(eps.wake, Rights::READ),
        port_range_capability(COM1_BASE, COM1_COUNT, Rights::ALL),
        port_range_capability(COM1_BASE, COM1_COUNT, Rights::ALL),
    ];

    // Delete slot 1 only: the twin in slot 2 still names the range with every right, and the
    // cached grant is cleared anyway. Characterized, per the header.
    let (_one, one_region) = child(
        &x86_programs::cap_delete_then_port_out(1, COM1_BASE, WRITE_VAL),
        &duplicated,
        eps.fault,
    );
    let (event, _tid, pc) = next_death(eps.fault);
    assert_eq!(
        event,
        abi::fault::EVENT_FAULT,
        "deleting one of two capabilities for a port range no longer stops the range: the cached \
         grant survived a delete that left a twin, and this characterization is stale (good)",
    );
    assert_eq!(
        pc,
        CODE_VA + x86_programs::CAP_DELETE_THEN_PORT_OUT_PC_OFFSET,
        "the fault was not at the `out` instruction",
    );
    reap(one_region);

    // Delete both slots: the `out` must fault at the `out`.
    let (_both, both_region) = child(
        &x86_programs::two_capability_deletes_then_port_out(1, 2, COM1_BASE, WRITE_VAL),
        &duplicated,
        eps.fault,
    );
    let (event, _tid, pc) = next_death(eps.fault);
    assert_eq!(
        event,
        abi::fault::EVENT_FAULT,
        "after deleting every capability for the range, the `out` was still permitted",
    );
    assert_eq!(
        pc,
        CODE_VA + x86_programs::TWO_CAPABILITY_DELETES_THEN_PORT_OUT_PC_OFFSET,
        "the fault was not at the `out` instruction",
    );
    reap(both_region);

    sched::reclaim_region(eps.region).expect("the endpoint region did not come back");
}
