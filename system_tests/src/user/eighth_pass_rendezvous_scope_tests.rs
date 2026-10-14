//! **A supervisor's domain is its own rendezvous, on both the reap and the view side, and a
//! stranger's liveness is invisible through either** (an eighth outsider pass at the confinement
//! claim, claims 6, 7, 8 and 9).
//!
//! The four claims are proved on pure logic in the host crate (`reap_is_permitted_only_to_the_
//! supervising_rendezvous`, `a_stranger_reveals_nothing_about_its_liveness`, `a_survey_shows_
//! exactly_the_endpoints_own_children`, `the_view_and_the_reap_have_the_same_scope`) and driven
//! through the real dispatcher by the reap and survey tests. This module attacks them as one
//! cross-domain forger would: a *fully privileged* supervisor of its own domain (READ and
//! ENUMERATE, every right a legitimate supervisor holds) reaches sideways at another domain's
//! members, alive and dead, through `REAP` and `SURVEY`, and the attack's observable is whether
//! any answer differs from the same probe aimed at its own domain's members.
//!
//! The liveness attack is the one worth naming. A refusal that said `StillAlive` for a live
//! stranger and `NotSupervised` for a dead one would leak a stranger's liveness through an
//! endpoint the stranger's builder never named, which is claim 7 broken without a single right
//! being gained. The test asserts the two refusals are the same error word for word.
//!
//! Nothing blocks: every probe is non-blocking or aimed at a settled state, so a regression is red
//! at an assertion. Cross-ISA: the reap decision and the survey walk are portable kernel code
//! (DECISIONS §19 (architectural parity is a tenet)).

use abi::Error;

use super::supervision_tests::{FAULT_STUB, REPORT_STUB};
use super::survey_tests::{
    TEST_ROWS, arena, child_in, collect_all, drain, hold_supervisor, hold_view, rendezvous, tidy,
};
use crate::arch::exceptions::TrapFrame;
use crate::sched;

/// `invoke(cap, REAP, tid, _, _)`, through the real dispatcher.
fn reap(slot: u64, tid: u64) -> Result<i64, Error> {
    let mut frame = TrapFrame::for_user_entry(0, 0, [0, 0, 0]);
    crate::syscall::invoke(&mut frame, slot, abi::rendezvous::REAP, tid, 0, 0)
}

/// `invoke(cap, SURVEY, cursor, STATE, _)`, through the real dispatcher, returning the three words
/// a userspace caller would read out of its registers.
fn survey(slot: u64, cursor: u64) -> (i64, u64, u64) {
    let mut frame = TrapFrame::for_user_entry(0, 0, [0, 0, 0]);
    match crate::syscall::invoke(
        &mut frame,
        slot,
        abi::rendezvous::SURVEY,
        cursor,
        abi::survey::record::STATE,
        0,
    ) {
        Ok(next) => (next, frame.arg(1), frame.arg(2)),
        Err(e) => (e as i64, 0, 0),
    }
}

/// **A supervisor holding every right its own domain grants still cannot collect another domain's
/// corpse, and a stranger's liveness changes no answer.**
///
/// The attacker holds READ and ENUMERATE on its own rendezvous, the full legitimate supervisor's
/// grant. Through the real dispatcher it reaps at another domain's live member and at that domain's
/// dead one. Both must answer `NotSupervised`, the same error for the living stranger and the dead
/// one: a difference would be a liveness oracle through an endpoint the stranger's builder never
/// named (claim 7), and either one succeeding would be a corpse collected outside the supervising
/// rendezvous (claim 6). The positive control is the attacker reaping its own domain's corpse
/// through the same capability, which is what makes the sideways refusals the domain check
/// talking.
///
/// Falsification: unfalsified. Written by the eighth outsider pass's reviewer, who booted it
/// green but did not break the code it guards to watch it go red.
#[test_case]
fn a_supervisor_cannot_reap_across_domains_and_a_strangers_liveness_is_invisible() {
    let (budget, rendezvous_region) = arena();
    let mine = rendezvous(rendezvous_region);
    let theirs = rendezvous(rendezvous_region);
    let parking = rendezvous(rendezvous_region);

    let own_dead = child_in(budget, FAULT_STUB, None, mine);
    let stranger_alive = child_in(budget, REPORT_STUB, Some(parking), theirs);
    let stranger_dead = child_in(budget, FAULT_STUB, None, theirs);
    assert!(
        super::wait_for(|| sched::rendezvous_waiting_senders(mine) == 1
            && sched::rendezvous_waiting_senders(theirs) == 1
            && sched::rendezvous_waiting_senders(parking) == 1),
        "the children never reached their sends or deaths",
    );

    let attacker = hold_supervisor(mine);
    assert_eq!(
        reap(attacker, stranger_alive),
        Err(Error::NotSupervised),
        "a supervisor of one domain reaped another domain's live member",
    );
    let for_the_dead = reap(attacker, stranger_dead);
    assert_eq!(
        for_the_dead,
        Err(Error::NotSupervised),
        "a supervisor of one domain collected another domain's corpse",
    );
    assert_eq!(
        reap(attacker, stranger_alive),
        for_the_dead,
        "the refusal for a live stranger differed from the refusal for a dead one: liveness \
         leaked through an endpoint the stranger's builder never named",
    );

    // Positive control: the same capability reaps its own domain's corpse. Retried, because a
    // fresh corpse takes a moment to settle off its own kernel stack.
    assert!(
        super::wait_for(|| reap(attacker, own_dead) == Ok(0)),
        "the attacker could not reap its own domain's corpse: the sideways refusals above meant \
         nothing",
    );

    drain(parking, 1);
    let theirs_capability = hold_supervisor(theirs);
    collect_all(theirs_capability, &[stranger_alive, stranger_dead]);
    tidy(budget, rendezvous_region, &[attacker, theirs_capability]);
}

/// **A domain's view is exactly its own members, dead ones included, and every member the view
/// shows is a member the reap may collect, and nothing else is.**
///
/// One domain holds a live member (blocked in a send) and a dead one (a settled corpse), another
/// holds a live stranger, a third holds nothing. The view of the first domain, driven by the real
/// program's real loop (`ps::collect`) through the real dispatcher, must report exactly its two:
/// the corpse may not vanish early and the stranger may not appear. The empty domain must answer
/// with an empty, complete walk rather than a refusal. Then the domain equality (claim 9): each
/// tid the view showed is one the same rendezvous's `REAP` answers for, with the live member
/// answering `StillAlive` and the dead one collectable, while the stranger, which no view showed,
/// is refused.
///
/// Falsification: unfalsified. Written by the eighth outsider pass's reviewer, who booted it
/// green but did not break the code it guards to watch it go red.
#[test_case]
fn the_view_and_the_reap_are_one_domain_dead_members_included() {
    let (budget, rendezvous_region) = arena();
    let mine = rendezvous(rendezvous_region);
    let theirs = rendezvous(rendezvous_region);
    let empty = rendezvous(rendezvous_region);
    let parking = rendezvous(rendezvous_region);

    let member_alive = child_in(budget, REPORT_STUB, Some(parking), mine);
    let member_dead = child_in(budget, FAULT_STUB, None, mine);
    let stranger = child_in(budget, REPORT_STUB, Some(parking), theirs);
    assert!(
        super::wait_for(|| sched::rendezvous_waiting_senders(mine) == 1
            && sched::rendezvous_waiting_senders(parking) == 2),
        "the members never reached their sends or deaths",
    );

    let view = hold_view(mine);
    let mut rows = [ps::Row::default(); TEST_ROWS];
    let walked = ps::collect(&mut rows, &mut |cursor| survey(view, cursor));
    assert!(
        walked.is_complete(),
        "the walk of a three-member world was refused or truncated",
    );
    let reports = |tid: u64| walked.rows().iter().any(|r| r.tid == tid);
    assert!(
        reports(member_alive) && reports(member_dead),
        "the domain's own view omitted one of its members (wanted {member_alive} and \
         {member_dead}): the corpse vanished from its own domain",
    );
    assert_eq!(
        walked.rows().len(),
        2,
        "the domain's view reported more than its own two members",
    );
    assert!(
        !reports(stranger),
        "the domain's view reported another domain's member",
    );
    let dead_row = walked
        .rows()
        .iter()
        .find(|r| r.tid == member_dead)
        .expect("the dead member was reported but then lost");
    assert_eq!(
        dead_row.state,
        abi::survey::DEAD,
        "the corpse's state was misreported as a live one",
    );

    // An empty domain answers with an empty walk, not a refusal.
    let empty_view = hold_view(empty);
    let mut empty_rows = [ps::Row::default(); TEST_ROWS];
    let empty_walked = ps::collect(&mut empty_rows, &mut |cursor| survey(empty_view, cursor));
    assert!(
        empty_walked.is_complete() && empty_walked.rows().is_empty(),
        "an empty domain refused the walk or reported members it never had",
    );

    // The reap side of the same domain: exactly the tids the view showed.
    let supervisor = hold_supervisor(mine);
    assert_eq!(
        reap(supervisor, member_alive),
        Err(Error::StillAlive),
        "the reap side did not recognize the view side's live member as its own",
    );
    assert!(
        super::wait_for(|| reap(supervisor, member_dead) == Ok(0)),
        "the reap side could not collect the view side's dead member",
    );
    assert_eq!(
        reap(supervisor, stranger),
        Err(Error::NotSupervised),
        "a member no view of this domain showed was reapable through it: the two scopes differ",
    );

    drain(parking, 2);
    collect_all(supervisor, &[member_alive]);
    let theirs_capability = hold_supervisor(theirs);
    collect_all(theirs_capability, &[stranger]);
    tidy(
        budget,
        rendezvous_region,
        &[view, empty_view, supervisor, theirs_capability],
    );
}
