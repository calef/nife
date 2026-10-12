//! Tests of batching at the bound.

use super::*;
use crate::test_support::{listing, shown};

/// **The per-batch set, which is the grant that batch is handed.** `write_batch` had no caller
/// in this crate's tests at all, so replacing its whole body with `()` survived (milestone
/// 326): a sweep that printed nothing would have shown a person no set while handing each
/// batch a real one, which is the exact property the batching lane exists to make visible.
#[test]
fn a_batch_prints_the_set_that_batch_is_handed() {
    let set = listing(&[b"a.txt", b"b.txt"]);
    assert_eq!(
        shown(|o| write_batch(2, &set, o)),
        "  batch 2: a.txt b.txt\n"
    );
}
