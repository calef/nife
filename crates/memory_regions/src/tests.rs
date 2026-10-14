use super::*;

#[test]
fn split_refuses_zero_overflow_and_over_budget() {
    assert_eq!(split_new_watermark(8, 0, 0), None); // zero carve
    assert_eq!(split_new_watermark(8, 4, 5), None); // 4+5 > 8, over budget
    assert_eq!(split_new_watermark(8, u64::MAX, 1), None); // overflow
    assert_eq!(split_new_watermark(8, 4, 4), Some(8)); // exact fit
    assert_eq!(split_new_watermark(8, 0, 4), Some(4)); // first carve
}

#[test]
fn destroy_decides_by_ownership() {
    assert_eq!(
        destroy_outcome(true, 0, true, false, 0),
        DestroyOutcome::Refused
    );
    assert_eq!(
        destroy_outcome(false, 1, true, false, 0),
        DestroyOutcome::Refused
    );
    assert_eq!(
        destroy_outcome(false, 0, true, false, 0),
        DestroyOutcome::FreeToGlobal
    );
    assert_eq!(
        destroy_outcome(false, 0, false, true, 4),
        DestroyOutcome::ReturnToParent { unbump: 4 }
    );
    assert_eq!(
        destroy_outcome(false, 0, false, false, 4),
        DestroyOutcome::ReturnToParent { unbump: 0 }
    );
}
