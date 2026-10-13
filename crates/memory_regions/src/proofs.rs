use super::*;

/// A successful carve stays within budget, does not overflow, and strictly progresses (so the
/// run it hands out, `[parent_watermark, new)`, is fresh and disjoint from every earlier carve).
/// A refused carve is exactly: empty, overflowing, or beyond budget.
/// Falsification: replayable `crates/memory_regions/falsifications/proofs.split_stays_within_budget_and_progresses.patch`
#[kani::proof]
fn split_stays_within_budget_and_progresses() {
    let parent_pages: u64 = kani::any();
    let parent_watermark: u64 = kani::any();
    let want: u64 = kani::any();
    // The table's own invariant: a region's watermark never exceeds its page count.
    kani::assume(parent_watermark <= parent_pages);

    match split_new_watermark(parent_pages, parent_watermark, want) {
        Some(new) => {
            assert!(want > 0, "a zero carve must be refused");
            assert!(
                new == parent_watermark + want,
                "watermark advances by exactly want"
            );
            assert!(
                new <= parent_pages,
                "the carve stays within the parent's budget"
            );
            assert!(
                new > parent_watermark,
                "the carve strictly progresses, so it is fresh"
            );
        }
        None => {
            // Refused: it must be one of the three legitimate reasons.
            assert!(
                want == 0
                    || parent_watermark.checked_add(want).is_none()
                    || parent_watermark + want > parent_pages
            );
        }
    }
}

/// **A retyped run is exactly the pages asked for, all fresh, all within budget, and `0` is
/// one.** The `RETYPE` count's whole contract, over every watermark and every requested word.
/// A refusal is exactly "does not fit": the table does not move, which its own host test shows.
/// Falsification: replayable `crates/memory_regions/falsifications/proofs.a_retyped_run_is_the_pages_asked_for_and_zero_is_one.patch`
#[kani::proof]
fn a_retyped_run_is_the_pages_asked_for_and_zero_is_one() {
    let pages: u64 = kani::any();
    let watermark: u64 = kani::any();
    let requested: u64 = kani::any();
    kani::assume(watermark <= pages);
    let want = if requested == 0 { 1 } else { requested };

    match retype_new_watermark(pages, watermark, requested) {
        Some(new) => {
            assert!(
                new == watermark + want,
                "the watermark advances by exactly the run"
            );
            assert!(new <= pages, "the run stays within the region's budget");
            assert!(
                new > watermark,
                "the run is fresh: no page handed out twice"
            );
        }
        None => {
            assert!(
                watermark.checked_add(want).is_none() || watermark + want > pages,
                "a run that fits must not be refused"
            );
        }
    }
}

/// **The no-double-free crux:** a region with a live object or a live child refuses; a root frees
/// to the allocator; and a child NEVER frees to the allocator (its pages return to the parent).
/// So a page reaches the allocator only through the root that owns it, exactly once.
/// Falsification: replayable `crates/memory_regions/falsifications/proofs.destroy_never_frees_a_child_to_the_allocator.patch`
#[kani::proof]
fn destroy_never_frees_a_child_to_the_allocator() {
    let pinned: bool = kani::any();
    let children: u32 = kani::any();
    let is_root: bool = kani::any();
    let is_lifo_top: bool = kani::any();
    let child_pages: u64 = kani::any();

    let outcome = destroy_outcome(pinned, children, is_root, is_lifo_top, child_pages);

    if pinned || children > 0 {
        assert!(
            outcome == DestroyOutcome::Refused,
            "a pinned or parent region must refuse"
        );
    } else if is_root {
        assert!(
            outcome == DestroyOutcome::FreeToGlobal,
            "a root frees to the allocator"
        );
    } else {
        // A child returns to its parent, never to the allocator: this is what prevents the
        // double-free when the parent's run is later freed.
        match outcome {
            DestroyOutcome::ReturnToParent { unbump } => {
                assert!(unbump == if is_lifo_top { child_pages } else { 0 });
            }
            _ => panic!("a child must not free to the allocator"),
        }
    }
}

/// **Reclaiming holes never lowers the watermark under anything still held, and never raises
/// it.** Under it stay the region's own retypes and every live child; an in-flight sibling
/// (not quiescent) leaves it where the LIFO rule put it. Together with
/// `split_stays_within_budget_and_progresses`, that is why a page carved after a reclaim is one
/// nothing else holds.
/// Falsification: replayable `crates/memory_regions/falsifications/proofs.coalescing_never_reclaims_a_held_page.patch`
#[kani::proof]
fn coalescing_never_reclaims_a_held_page() {
    let watermark: u64 = kani::any();
    let retyped_to: u64 = kani::any();
    let live_to: u64 = kani::any();
    let quiescent: bool = kani::any();
    // What the table guarantees: both floors are pages under the watermark.
    kani::assume(retyped_to <= watermark && live_to <= watermark);

    let after = coalesced_watermark(watermark, retyped_to, live_to, quiescent);

    assert!(after <= watermark, "reclaiming must never mint budget");
    assert!(after >= retyped_to, "the region's own retypes stay spent");
    assert!(after >= live_to, "a live child's pages stay spent");
    if !quiescent {
        assert!(
            after == watermark,
            "an in-flight sibling blocks the reclaim"
        );
    }
}
