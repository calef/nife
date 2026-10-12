//! **A spend-only (GRANT-less) budget narrows a SPLIT child but mints full-rights objects on every
//! RETYPE** (milestone 903 (a seventh outsider pass attacks the confinement claim), fatal risk 7's
//! confinement claim, claim 3).
//!
//! Claim 3 ("a budget that cannot delegate cannot split itself a child that can") is proved on its
//! SPLIT side by `capability::split_never_widens_rights` and characterized on its RETYPE side, for a
//! page frame only, by `confinement_attack_tests::a_grant_less_budget_mints_a_grant_bearing_frame`.
//! This module drives the real dispatcher (`syscall::invoke`) and does two things that test does
//! not:
//!
//! 1. It puts the two sides next to each other in one boot: the same WRITE-only budget shape is
//!    SPLIT (the child comes back WRITE-only, the claim holding) and RETYPEd (the frame comes back
//!    `Rights::ALL`, GRANT included, the gap).
//! 2. It shows the gap is not confined to page frames. `MemoryRegion::RETYPE_OBJ` mints a
//!    `Rights::ALL` capability for a rendezvous, a notification and a timer alike, so a holder of a
//!    non-delegating budget obtains a GRANT-bearing capability to a kernel object it made, which it
//!    can then pass on (GRANT is exactly the right `SEND_CAP`/`SLICE`/`CAP_INSERT` consult). The
//!    page-frame characterization does not reach `RETYPE_OBJ`.
//!
//! This is a near miss, not an escape: every object is made in the holder's own region, so the
//! holder reaches no object it was not granted. What leaks is the *delegability* a GRANT-less budget
//! was supposed to withhold. The gap is known and ruled: calef's ruling (a) of 2026-10-07 (recorded
//! in `kernel/src/cap.rs` at `memory_region_cap`) is that a retype mints the intersection, not
//! `Rights::ALL`, and milestone 824 (a retype mints no right its budget lacks) is the fix. When that
//! lands, the GRANT assertions below flip and this module is what announces it: the characterization
//! becomes a held assertion that a GRANT-less budget mints GRANT-less objects.
//!
//! Name: provisional (this pass's lane, 2026-10-12 UTC), for the module and the file; a rename is an
//! architect's call.
//!
//! Cross-ISA: `invoke`, the retype paths and the rights model are portable kernel code, so the same
//! body runs on each architecture (DECISIONS §19, architectural parity is a tenet).

use abi::Error;

use crate::arch::exceptions::TrapFrame;
use crate::cap::{Object, Rights};
use crate::sched;
use crate::syscall::invoke;

/// `invoke` through the real dispatcher, the way a program's `svc`/`ecall` reaches it. Copied in
/// shape from `confinement_attack_tests::call`.
fn call(slot: u64, method: u64, a0: u64, a1: u64, a2: u64) -> Result<i64, Error> {
    let mut frame = TrapFrame::for_user_entry(0, 0, [0, 0, 0]);
    invoke(&mut frame, slot, method, a0, a1, a2)
}

/// The rights on the capability at `slot`, or a panic naming the test's own mistake if the slot is
/// empty (the attack body always files something first).
fn rights_at(slot: u64) -> Rights {
    sched::current_cap(slot)
        .expect("no capability at the slot the attack just filled")
        .rights
}

/// **A WRITE-only budget's SPLIT child is WRITE-only (claim 3 holds), but every RETYPE off the same
/// budget mints a GRANT-bearing capability (the gap), for a page frame and for three kernel object
/// kinds alike.**
///
/// The SPLIT step is the positive control and the claim's cited property: a non-delegating budget
/// cannot carve a delegating child, because `MemoryRegion::SPLIT` mints through `Cap::mint_child`,
/// which inherits the parent's rights. The RETYPE and `RETYPE_OBJ` steps are the characterization: the
/// same budget, through the same dispatcher, hands back `Rights::ALL`.
///
/// Shaped so a regression is observable at an assertion, not a hang: nothing here blocks. Each
/// minted capability is deleted and each region reclaimed, so the suite's frame and region ledgers
/// are flat across the test.
///
/// Falsification: unfalsified. It characterizes behavior the tree has today. When milestone 824
/// lands (a retype intersecting rights with its budget), the `allows(Rights::GRANT)` assertions
/// below go red, which is the change this module exists to announce; the SPLIT assertion stays
/// green.
#[test_case]
fn a_grant_less_budget_narrows_a_split_child_but_mints_full_rights_on_every_retype() {
    // --- The SPLIT side: claim 3's cited property, which holds. ---------------------------------
    let region_b = crate::memory_region::create(8).expect("no region for the split side");
    let budget_b =
        sched::grant(crate::cap::memory_region_cap(region_b)).expect("grant the WRITE-only budget");
    assert_eq!(
        rights_at(budget_b),
        Rights::WRITE,
        "the leaf budget should be WRITE-only, with no GRANT to inherit",
    );
    let child_slot = call(budget_b, abi::memory_region::SPLIT, 2, 0, 0)
        .expect("SPLIT refused a WRITE-only budget") as u64;
    assert_eq!(
        rights_at(child_slot),
        Rights::WRITE,
        "a GRANT-less budget SPLIT itself a child carrying more than WRITE: claim 3's SPLIT side \
         broke, a non-delegating budget manufactured a delegating child",
    );
    // Teardown the split side: destroy the child region first (a parent with children refuses
    // reclaim), then the parent.
    let Object::MemoryRegion(child_region) = sched::current_cap(child_slot).unwrap().object else {
        panic!("the SPLIT child was not a MemoryRegion");
    };
    sched::reclaim_region(child_region).expect("the split child region did not come back");
    let _ = sched::delete_current_cap(child_slot);
    let _ = sched::delete_current_cap(budget_b);
    sched::reclaim_region(region_b).expect("the split-side region did not come back");

    // --- The RETYPE side: the gap, for a page frame and three object kinds. ---------------------
    let region_a = crate::memory_region::create(16).expect("no region for the retype side");
    let budget_a =
        sched::grant(crate::cap::memory_region_cap(region_a)).expect("grant the WRITE-only budget");
    assert_eq!(
        rights_at(budget_a),
        Rights::WRITE,
        "the leaf budget should be WRITE-only"
    );

    // A page frame: the case the existing characterization covers, replayed here for the contrast.
    let frame_slot = call(budget_a, abi::memory_region::RETYPE, 0, 0, 0)
        .expect("RETYPE refused a WRITE-only budget") as u64;
    assert!(
        rights_at(frame_slot).allows(Rights::GRANT),
        "a WRITE-only budget RETYPEd a frame WITHOUT GRANT: milestone 824 has landed and this \
         characterization should become a held assertion",
    );
    assert_eq!(
        rights_at(frame_slot),
        Rights::ALL,
        "the retyped frame should carry the full Rights::ALL the retype path documents today",
    );

    // Three kernel object kinds via RETYPE_OBJ: the case no existing test reaches. Each must come
    // back GRANT-bearing for the gap to generalize; a holder of a non-delegating budget thus holds a
    // delegatable capability to every object it carves out of its own memory.
    for &(kind, name) in &[
        (abi::objtype::RENDEZVOUS, "rendezvous"),
        (abi::objtype::NOTIFICATION, "notification"),
        (abi::objtype::TIMER, "timer"),
    ] {
        let obj_slot = call(budget_a, abi::memory_region::RETYPE_OBJ, kind, 0, 0)
            .unwrap_or_else(|e| panic!("RETYPE_OBJ {name} refused a WRITE-only budget: {e:?}"))
            as u64;
        assert!(
            rights_at(obj_slot).allows(Rights::GRANT),
            "a WRITE-only budget RETYPE_OBJ'd a {name} WITHOUT GRANT: milestone 824 has landed for \
             objects too and this should become a held assertion",
        );
        assert_eq!(
            rights_at(obj_slot),
            Rights::ALL,
            "the retyped {name} should carry Rights::ALL: a GRANT-less budget minted a fully \
             delegatable {name} in its own memory",
        );
        let _ = sched::delete_current_cap(obj_slot);
    }

    let _ = sched::delete_current_cap(frame_slot);
    let _ = sched::delete_current_cap(budget_a);
    sched::reclaim_region(region_a).expect("the retype-side region did not come back");
}
