use super::address_space::run_end_va;
use super::*;

/// The page size the run arithmetic steps by, spelled once so the harnesses below and
/// `run_end_va` cannot drift apart.
const PAGE: u128 = paging::PAGE_SIZE as u128;

/// **`run_end_va` is exact, and it refuses exactly the runs that do not fit.**
///
/// This is milestone 142's MAJOR 4 written as a property. The code it replaced computed
/// `va.checked_add((count - 1) * paging::PAGE_SIZE)` and checked only the addition, so a
/// `count` big enough to wrap the *multiply* produced a `last_va` sitting a few pages above
/// `va`, and the guard at both call sites cheerfully admitted a run that spans the address
/// space. A unit test cannot find that: the witnesses are a measure-zero set of `count`s near
/// multiples of 2^52, and nobody writes them down. The solver enumerates the whole domain.
///
/// The claim is stated against `u128` arithmetic, which cannot wrap in this range, so the
/// harness does not repeat the implementation's own expression back to it.
///
/// Falsification: replayable `kernel/falsifications/syscall.proofs.the_run_end_is_exact_and_refuses_exactly_what_does_not_fit.patch`
#[kani::proof]
fn the_run_end_is_exact_and_refuses_exactly_what_does_not_fit() {
    let va: u64 = kani::any();
    let count: u64 = kani::any();
    // `count` rides on the capability as a `NonZeroU64` (cap.rs, DECISIONS §102), so zero is
    // not a state a caller can reach. Assuming it here rather than proving it is honest: the
    // type is the mechanism, and this harness is about the arithmetic above it.
    kani::assume(count >= 1);

    let want = va as u128 + (count as u128 - 1) * PAGE;

    match run_end_va(va, count) {
        Some(last) => {
            assert!(
                last as u128 == want,
                "the last page of the run is va + (count - 1) * PAGE_SIZE, with no wrap"
            );
            assert!(last >= va, "the run cannot end below where it starts");
        }
        None => assert!(
            want > u64::MAX as u128,
            "a run that fits in the address space must not be refused"
        ),
    }
}

/// **Checking the two ends of a run is enough to check every page in it.**
///
/// Both mapping paths (`page_frame_map` and `MAP_INTO`) guard `va` and `run_end_va(va, count)`
/// and then walk `va + k * PAGE_SIZE` for `k` in `0..count` with **nothing re-checking the
/// pages in between**. That is milestone 142's MAJOR 3 in its fixed form, and it is only sound
/// because the user half is a contiguous prefix of the address space: on aarch64
/// `is_user_page_va` is `va & 0xfff == 0 && va >> 48 == 0`, so an aligned address between two
/// user addresses is itself a user address.
///
/// Sound, but not obviously so, and the loop is where a future widening of `Half::Low` would
/// break it silently. `k` is chosen by the solver rather than iterated, so this covers every
/// page of every run without an unwind bound.
///
/// **The guard below is a model of two call sites, and a model can go stale.** Milestone
/// 213's sweep read `page_frame_map` and `MAP_INTO` on 2026-09-02 and both apply exactly
/// this: `run_end_va(va, count)`, then `is_user_page_va` on `va` and on `last_va`, then the
/// loop. So the model is faithful today, checked rather than assumed. Nothing checks it
/// tomorrow: a call site that added an alignment condition, or dropped one of the two ends,
/// would leave this harness green while proving a claim about a guard nobody applies. Unlike
/// the harnesses 213 rewrote, the duplication here cannot be removed by calling something,
/// because what is duplicated is a *caller's* control flow rather than a function.
///
/// Falsification: replayable `kernel/falsifications/syscall.proofs.every_page_between_the_checked_ends_is_itself_a_user_page.patch`
#[kani::proof]
fn every_page_between_the_checked_ends_is_itself_a_user_page() {
    let va: u64 = kani::any();
    let count: u64 = kani::any();
    let k: u64 = kani::any();
    kani::assume(count >= 1);
    kani::assume(k < count);

    // The guard the call sites apply, verbatim.
    let Some(last_va) = run_end_va(va, count) else {
        return;
    };
    kani::assume(paging::is_user_page_va::<crate::arch::mmu::Format>(va));
    kani::assume(paging::is_user_page_va::<crate::arch::mmu::Format>(last_va));

    // What the loop body computes. In a release build this is wrapping arithmetic; the claim
    // is that it never gets the chance to wrap.
    let offset = k as u128 * PAGE;
    assert!(
        va as u128 + offset <= last_va as u128,
        "no page of the run is computed past the end the guard checked"
    );
    let page_va = va + k * paging::PAGE_SIZE;
    assert!(
        paging::is_user_page_va::<crate::arch::mmu::Format>(page_va),
        "every page the map loop touches is an aligned user-half page"
    );
}
