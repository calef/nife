//! **The `Rendezvous` methods of the IPC round trip**: `SEND`, `RECEIVE`, `SEND_CAP`,
//! `RECEIVE_CAP` and `CALL`, with `SURVEY` beside them.
//!
//! Cut out of [`invoke`](super::invoke) on 2026-10-12 (UTC) by lane
//! maintainer/long-function-paydown-invoke, so that function falls under milestone 888 (a ratchet
//! on function length)'s 100 code lines. The bodies are unchanged; only the comment that sat above
//! each match arm became the doc comment of the function that holds it.
//!
//! **Every function here is `#[inline(always)]`, and that is the point of the file, not a tuning
//! guess.** These are the fastpath arms of the decoder: `SEND`, `RECEIVE`, `RECEIVE_CAP` and `CALL`
//! are steps of the round trip `script/fastpath-footprint` bounds, and `syscall::dispatch` is
//! counted flat as `syscall_entry`. The release profile builds this crate in several codegen units
//! with no LTO, and a child module can land in a different unit from its parent, where a plain
//! private function is not inlined at all. That would cost every IPC a `bl`/`ret` and move these
//! bytes out of `syscall_entry` under a new symbol the gate does not count, the under-measurement
//! `dispatch`'s own doc comment warns about. `#[inline(always)]` puts the bodies back inside
//! `invoke` in every unit, so the decoder compiles as it did when they were written in place.
//! `SURVEY` is not on the round trip; it is here because it was inline in `invoke` before the cut,
//! and moving it out of line would change what `syscall_entry` measures in a refactor that claims
//! to change nothing. The rare methods of this object (`BADGE`, `REAP`) were already out of line,
//! in `object_methods`, and stay there.
//!
//! Name: provisional (lane maintainer/long-function-paydown-invoke, 2026-10-12 UTC), for the
//! module, the file and each function in it. calef names things; expect these to change.

use super::*;
use crate::sched::RendezvousId;

/// `Rendezvous::SEND`. SEND takes WRITE, RECEIVE takes READ. The *same* endpoint, handed out with
/// different rights, is a one-way pipe in whichever direction each holder was trusted with.
#[inline(always)]
pub(super) fn rendezvous_send(
    rights: Rights,
    ep: RendezvousId,
    badge: u64,
    words: [u64; 3],
) -> Result<i64, Error> {
    if !rights.allows(Rights::WRITE) {
        return Err(Error::NotPermitted);
    }
    // The three words are already in registers. **Nothing is read from user memory**, so there
    // is no pointer to validate and no confused-deputy question to ask. That is the fastpath,
    // and it is why IPC carries control and not bulk data (§10 (process model: capability-based,
    // microkernel)). The badge rides in word 3 (milestone 613 (a system log service),
    // provisional): a byte-sink writer's identity is its capability's badge, never its own claim.
    sched::ipc_send_badged(ep, words, badge);
    // If the endpoint was revoked (stale, or reclaimed while we blocked), the send never
    // happened: report it rather than a silent success. Object revocation, notes/.
    //
    // `Gone`, not `NoSuchSlot`, since milestone 50 (pipes and redirection: one sink protocol).
    // The slot is not empty; a real capability names an object that has been destroyed, and a
    // writer branches on the difference in opposite directions (`abi::Error::Gone`,
    // notes/sink-protocol.md). This one line is what turns a dead pipe reader into something the
    // producer can act on, which is why the ABI grew a variant rather than the sink protocol
    // growing a heartbeat.
    //
    // **Or it was refused** (milestone 603 (provisional), DECISIONS §101 (notification objects)
    // ruling B): the endpoint carries a hardware interrupt, whose driver reads `w0 = 1` as "the
    // device fired", so no program may deposit anything there. `NotPermitted`, because it is the
    // answer to an operation the capability names but may not perform, and it is asked only once
    // the abort branch is taken, so a send that went through pays nothing for it.
    if sched::take_ipc_aborted() {
        return Err(aborted_send_error());
    }
    Ok(0)
}

/// `Rendezvous::RECEIVE`, which takes READ (see [`rendezvous_send`]).
#[inline(always)]
pub(super) fn rendezvous_receive(
    frame: &mut TrapFrame,
    rights: Rights,
    ep: RendezvousId,
) -> Result<i64, Error> {
    if !rights.allows(Rights::READ) {
        return Err(Error::NotPermitted);
    }
    let msg = sched::ipc_receive(ep);
    if sched::take_ipc_aborted() {
        return Err(Error::Gone); // endpoint revoked; the message is a placeholder
    }
    // Word 0 goes back the way every syscall result does, in x0 (dispatch writes it from our
    // return value). Words 1..4 we place directly, because a syscall return is one register and a
    // message is up to five: ordinary IPC fills three and leaves the top two zero, and a
    // fault/exit notification (DECISIONS §26 (the fault endpoint: thread death becomes a message a
    // supervisor holds)) fills all five.
    frame.set_arg(1, msg[1]);
    frame.set_arg(2, msg[2]);
    frame.set_arg(3, msg[3]);
    frame.set_arg(4, msg[4]);
    Ok(msg[0] as i64)
}

/// `Rendezvous::SEND_CAP`: delegation. `slot` is the slot of the capability to pass on, `narrow`
/// the rights to narrow it to, `word` one data word. Two rights are in play and they are different
/// questions: WRITE on *this* endpoint (may I send here?) and GRANT on the *delegated* capability
/// (was I trusted to pass it on?). Without GRANT you may use a thing and not lend it.
#[inline(always)]
pub(super) fn rendezvous_send_capability(
    rights: Rights,
    ep: RendezvousId,
    badge: u64,
    [slot, narrow, word]: [u64; 3],
) -> Result<i64, Error> {
    if !rights.allows(Rights::WRITE) {
        return Err(Error::NotPermitted);
    }
    // The source is read, checked (GRANT, narrow-only) and copied inside `sched`, under the hold
    // that files the copy, so no revocation sweep can fall between the read and the filing.
    // Reading it here first was that gap (`sched::Delegation`).
    #[cfg(feature = "system_tests")]
    crate::delegation_pause::here(); // no lock held
    sched::ipc_delegate_cap(
        ep,
        word,
        sched::Delegation {
            slot,
            rights: Rights::from_bits(narrow as u32),
        },
        badge, // the endpoint we send on may be badged (milestone 599 (a frame per filesystem client channel))
    )?;
    if sched::take_ipc_aborted() {
        // Revoked, or refused (§101 ruling B); either way the delegation did not happen and the
        // capability is still the sender's.
        return Err(aborted_send_error());
    }
    Ok(0)
}

/// `Rendezvous::RECEIVE_CAP`, which takes READ like `RECEIVE`.
#[inline(always)]
pub(super) fn rendezvous_receive_capability(
    frame: &mut TrapFrame,
    rights: Rights,
    ep: RendezvousId,
) -> Result<i64, Error> {
    if !rights.allows(Rights::READ) {
        return Err(Error::NotPermitted);
    }
    let msg = sched::ipc_receive_cap(ep);
    if sched::take_ipc_aborted() {
        return Err(Error::Gone); // endpoint revoked; the message is a placeholder
    }
    // x1 carries the slot the received capability landed in, or NO_CAP if the message brought
    // none; x2 the second data word (a CALL's, or 0); x3 the sender's badge (milestone 599), 0
    // when its capability was unbadged. x0 returns the first word. x4 is
    // `abi::notification::BOUND` when the bound notification ended this receive,
    // `abi::rendezvous::REPLY_DELIVERED` when x1 is a CALL's Reply (§245 (a `CALL` server tells a
    // Reply from a delegation)), and 0 otherwise (milestone 151 (notification objects)): the one
    // register here no sender can write, so the one a bound server, and every CALL server, tests.
    frame.set_arg(1, msg[1]);
    frame.set_arg(2, msg[2]);
    frame.set_arg(3, msg[3]);
    frame.set_arg(4, msg[4]);
    Ok(msg[0] as i64)
}

/// `Rendezvous::CALL`: send two words and block until replied. The kernel mints a one-shot Reply
/// cap naming us into the server (delivered by its `RECEIVE_CAP`); we return here only when the
/// server invokes it. See §12 (Call/Reply IPC: a one-shot reply capability) and
/// notes/ipc-naming.md. Sending needs WRITE, like SEND.
#[inline(always)]
pub(super) fn rendezvous_call(
    frame: &mut TrapFrame,
    rights: Rights,
    ep: RendezvousId,
    badge: u64,
    words: [u64; 2],
) -> Result<i64, Error> {
    if !rights.allows(Rights::WRITE) {
        return Err(Error::NotPermitted);
    }
    let reply = sched::ipc_call_badged(ep, words, badge);
    if sched::take_ipc_aborted() {
        return Err(aborted_send_error()); // revoked or refused; no call, no reply
    }
    frame.set_arg(1, reply[1]); // r1; r0 returns in x0 below
    // x2: the slot a `REPLY_CAPABILITY` filed in our table, or `NO_CAP` (§255 (each socket is its
    // own capability)). Written on every CALL so a caller reads the kernel's word, never what it
    // left in x2 itself.
    frame.set_arg(2, reply[2]);
    Ok(reply[0] as i64)
}

/// `Rendezvous::SURVEY`: **read one entry of the domain this endpoint supervises** (milestone 126
/// (the `procps` package)). The view half of what REAP is the control half of, and scoped by the
/// same relationship, so a supervisor sees exactly the children whose deaths would arrive here.
///
/// READ for the same reason REAP takes READ: the authority to see who may die here is the
/// authority to receive deaths here. **A send-only holder is refused rather than shown an empty
/// domain**, which is the whole point of the method; a monitor that reports nothing because it
/// could not look is the worst failure this tool has, and an empty answer is reserved for a domain
/// that really is empty.
///
/// `cursor` is 0 to start, then whatever the last call returned, until a `survey::DONE` comes
/// back. `record` is the RECORD: which per-thread fact the caller wants. x1 carries the tid and x2
/// that record's word.
///
/// **The selector is calef's 2026-09-21 ruling, and it replaces growing this row.** The earlier
/// plan put each new fact in a further return register; he expects a third and a fourth fact, and
/// a mechanism that has to be redesigned at the sixth field is the wrong mechanism at the fourth.
/// Nobody grows a register row: Linux reached 52 fields through a pseudo-file and Zircon 41 topics
/// through a selector. So x0 and x1 are the frame, the same for every record, and only x2 belongs
/// to the record.
///
/// **`record::STATE` is 0 so that every caller written before the selector keeps working**, having
/// passed 0 into an argument that was then unused. That is a claim about a wire rather than a
/// hope, and it is asserted in `abi`'s own tests.
///
/// **Why the placement record needs no authority beyond the `ENUMERATE` below**, said here rather
/// than in a decisions file the reader would have to go and find. §150 (how does a thread's CPU
/// time reach userspace?) already weighed that a viewer holding `ENUMERATE` learns something
/// aggregate about threads it cannot otherwise name, and accepted it for a CPU-time counter. A
/// placement is strictly less than what was accepted: one bounded value out of at most 64, written
/// once when the thread started and never again, against a counter that moves continuously and can
/// therefore be differenced into a timing channel. A viewer that can already see a thread's tid
/// and run state learns which of a handful of cores it was put on. **It can never name a thread
/// outside the domain it was handed**; what it can learn about them is one noisy bit per spawn
/// (2026-09-24 security audit): `pick_spawn_target` samples two cores and takes the one with the
/// shorter run queue, and that queue counts every domain's threads, so a placement says which of
/// two random cores was lighter at that instant. That is the counting channel the 2026-08-17 audit
/// recorded, at lower bandwidth, and it is accepted on the same reasoning. The right to look is
/// still the capability, and nothing here widens it.
#[inline(always)]
pub(super) fn rendezvous_survey(
    frame: &mut TrapFrame,
    rights: Rights,
    ep: RendezvousId,
    cursor: u64,
    record: u64,
) -> Result<i64, Error> {
    // `ENUMERATE`, not `READ`, and the distinction is the method's whole safety argument: `READ`
    // here also unlocks `RECEIVE` and `REAP`, so a viewer granted it could reap a child. A domain
    // names its members and does not act on them, and one bit for three operations cannot say
    // that. See `Rights::ENUMERATE`.
    if !rights.allows(Rights::ENUMERATE) {
        return Err(Error::NotPermitted);
    }
    let (next, tid, word) = sched::survey_supervised(ep, cursor, record)?;
    frame.set_arg(1, tid);
    frame.set_arg(2, word);
    Ok(next as i64)
}
