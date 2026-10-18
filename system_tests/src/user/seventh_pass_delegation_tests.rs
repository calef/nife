//! **A `SEND_CAP` cannot widen a right out of its rights register, and cannot lend a GRANT-less
//! capability** (milestone 903 (a seventh outsider pass attacks the confinement claim), fatal risk
//! 7's confinement claim, claims 1 and 2).
//!
//! Claim 1 ("a derive holds no more than its source") and claim 2 ("userspace cannot forge a right
//! out of a syscall register") are proved in the host crate (`capability::derive_never_widens_rights`,
//! `from_bits_cannot_forge_a_right`). This module drives the live kernel delegation path instead:
//! `Rendezvous::SEND_CAP` through the real dispatcher (`syscall::invoke`), the way a program's
//! `svc`/`ecall` reaches it. `SEND_CAP`'s second argument register (`a1`) is the rights the copy is
//! narrowed to, so asking for a right the source lacks is literally forging a right out of a syscall
//! register, which is claim 2 in its wire form, and widening on delegation, which is claim 1.
//!
//! `confinement_attack_tests` drives the refusal of `SEND` by a READ-only holder and of an ungranted
//! slot. It does not drive a `SEND_CAP` whose rights mask asks to widen, nor a `SEND_CAP` of a
//! source that lacks GRANT, which is what this adds.
//!
//! **Why nothing blocks.** `ipc_send_cap_from` runs the source derivation first, under the hold, and
//! returns its error before the rendezvous is ever touched (`kernel/src/sched/rendezvous.rs`). So a
//! refused delegation never parks a sender. A regression does: with the subset check dropped, the
//! widening `SEND_CAP` derives a copy and parks the test thread on the receiverless rendezvous, and
//! the run fails by the watchdog's thread dump, not at an assertion (the lane's falsification,
//! aarch64, 2026-10-12 UTC). Red either way, but a reader of the failure sees a hang. The
//! endpoint's waiting-sender count is asserted to stay zero after each refusal, the other half of
//! "the rendezvous was not touched."
//!
//! Name: provisional (this pass's lane, 2026-10-12 UTC). Cross-ISA: `invoke`, the delegation rule
//! (`Delegation::derive`) and the rendezvous are portable kernel code (DECISIONS §19 (architectural parity is a tenet)).

use abi::Error;

use crate::arch::exceptions::TrapFrame;
// The `cap` module and these names still carry their pre-sweep spelling
// (design/naming/capability-spelling-worklist.md renames them when the tree is quiet); aliased here so
// this file spells `capability` out everywhere a reader looks (calef, 2026-10-06).
use crate::cap::{Rights, rendezvous_cap as rendezvous_capability};
use crate::sched::{self, delete_current_cap as delete_current_capability};

const SEND_CAPABILITY: u64 = abi::rendezvous::SEND_CAP;
use crate::syscall::invoke;

/// `invoke` through the real dispatcher. Copied in shape from `confinement_attack_tests::call`.
fn call(slot: u64, method: u64, a0: u64, a1: u64, a2: u64) -> Result<i64, Error> {
    let mut frame = TrapFrame::for_user_entry(0, 0, [0, 0, 0]);
    invoke(&mut frame, slot, method, a0, a1, a2)
}

/// **A holder cannot use `SEND_CAP`'s rights register to hand itself, or anyone, a right the source
/// capability did not carry; and a capability without GRANT cannot be lent at all.**
///
/// Two refusals, each driven through the real dispatcher, each returning before the rendezvous is
/// touched:
///
/// 1. **Widening.** The source holds `READ | GRANT` (it may be read and it may be passed on, but it
///    may not be written). `SEND_CAP` is invoked with a rights mask of `READ | WRITE | GRANT`,
///    asking the kernel to mint a copy that holds `WRITE`, a right the source never had. The answer
///    is `NotPermitted`: `Delegation::derive`'s subset check refuses, so the `WRITE` bit set in the
///    `a1` register buys nothing. This is claim 2 in its wire form and claim 1's monotonicity.
///
/// 2. **Lending without GRANT.** The source holds `READ | WRITE` but not `GRANT`. `SEND_CAP` is
///    invoked with a mask of `READ` alone, a strict subset, so the only thing that can refuse it is
///    the GRANT gate. The answer is `NotPermitted`: you may use a thing and not lend it.
///
/// The endpoint cap itself carries `WRITE`, so each refusal is the delegation rule talking, not the
/// endpoint gate. After each, the endpoint's waiting-sender count is still zero, which is the
/// kernel's own record that the refused delegation never parked anything.
///
/// Falsification: attested 2026-10-12. On patagonia, aarch64, with the subset check in
/// `Delegation::derive` patched to `if false && ...`, the run went red by the watchdog, since the
/// derived copy parks the sender (see the module header). The `GRANT` check was not patched. Not
/// recorded as a replayable patch.
#[test_case]
fn a_delegation_cannot_widen_a_right_or_lend_a_grant_less_capability() {
    let region = crate::memory_region::create(4).expect("no region for the endpoint");
    let ep = sched::create_rendezvous_from(region).expect("no rendezvous");

    // The endpoint capability the delegations are sent over: WRITE, so SEND_CAP clears the endpoint
    // gate and whatever refuses below is the delegation rule, not "you may not send here".
    let writer =
        sched::grant(rendezvous_capability(ep, Rights::WRITE)).expect("grant the WRITE endpoint");

    // --- 1. Widening: forge WRITE out of the a1 rights register. --------------------------------
    // The source: READ and GRANT, but no WRITE. Using the same endpoint object keeps the fixture to
    // one rendezvous; what is under test is the rights on the delegated capability, not which object
    // it names.
    let src_no_write = sched::grant(rendezvous_capability(ep, Rights::READ.union(Rights::GRANT)))
        .expect("grant the READ|GRANT source");
    let widen_mask = Rights::READ
        .union(Rights::WRITE)
        .union(Rights::GRANT)
        .bits() as u64;
    assert_eq!(
        call(writer, SEND_CAPABILITY, src_no_write, widen_mask, 0),
        Err(Error::NotPermitted),
        "SEND_CAP minted a copy carrying WRITE from a source that held only READ|GRANT: a right was \
         forged out of the a1 rights register (claims 1 and 2)",
    );
    assert_eq!(
        sched::rendezvous_waiting_senders(ep),
        0,
        "a refused widening SEND_CAP still parked a sender: the rendezvous was touched before the \
         derivation refused",
    );

    // --- 2. Lending a GRANT-less capability, even narrowed to a strict subset. ------------------
    let src_no_grant = sched::grant(rendezvous_capability(ep, Rights::READ.union(Rights::WRITE)))
        .expect("grant the READ|WRITE (GRANT-less) source");
    let subset_mask = Rights::READ.bits() as u64;
    assert_eq!(
        call(writer, SEND_CAPABILITY, src_no_grant, subset_mask, 0),
        Err(Error::NotPermitted),
        "SEND_CAP lent a GRANT-less capability (even narrowed to READ): a holder passed on a thing \
         it was trusted to use but not to lend",
    );
    assert_eq!(
        sched::rendezvous_waiting_senders(ep),
        0,
        "a refused GRANT-less SEND_CAP still parked a sender: the rendezvous was touched before the \
         derivation refused",
    );

    let _ = delete_current_capability(writer);
    let _ = delete_current_capability(src_no_write);
    let _ = delete_current_capability(src_no_grant);
    sched::reclaim_region(region).expect("the endpoint's region did not come back");
}
