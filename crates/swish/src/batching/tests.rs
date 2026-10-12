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

/// **The sweep's account, as a host test the coverage floor can read.** `write_sweep`'s doctest
/// asserts the same four shapes, but `script/coverage` counts no doctest, and while the function
/// sat in the crate root its file's average hid that nothing else ran it. A stop names the batch
/// that did not run; a sweep of one batch, or of none, prints nothing.
#[test]
fn a_sweep_names_its_total_or_the_batch_it_stopped_before() {
    let mut done = Sweep::default();
    done.ran(8);
    done.ran(3);
    assert_eq!(
        shown(|o| write_sweep(&done, o)),
        "  11 names, in 2 batches\n"
    );

    let mut one = Sweep::default();
    one.ran(3);
    assert_eq!(shown(|o| write_sweep(&one, o)), "");

    let mut stopped = Sweep::default();
    stopped.ran(8);
    stopped.stop();
    assert_eq!(
        shown(|o| write_sweep(&stopped, o)),
        "  batch 2 did not run: 8 names were handed over in 1 batch, and nothing after them was \
         attempted\n"
    );

    let mut none = Sweep::default();
    none.stop();
    assert_eq!(shown(|o| write_sweep(&none, o)), "");
}
