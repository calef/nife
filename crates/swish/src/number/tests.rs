//! Tests of how a number is printed.

use super::*;
use crate::test_support::shown;

#[test]
fn numbers_render_in_base_ten_including_the_ends() {
    assert_eq!(shown(|o| write_num(0, o)), "0");
    assert_eq!(shown(|o| write_num(7, o)), "7");
    assert_eq!(shown(|o| write_num(128, o)), "128");
    // Twenty digits, which is exactly the buffer. One more and this would have been a panic
    // nobody could have provoked from a prompt.
    assert_eq!(shown(|o| write_num(u64::MAX, o)), "18446744073709551615");
}
