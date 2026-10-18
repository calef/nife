//! Tests of the byte helpers the crate root keeps.

use crate::*;

#[test]
fn a_grant_name_limit_matches_the_filesystem_contract() {
    // `grant_plan` deliberately does not depend on `filesystem_contract` (the shell's parser must not need the
    // filesystem contract to check a command line), so the constant is duplicated. That is only
    // safe if a change to one without the other fails here, on the host, in milliseconds.
    assert_eq!(MAX_FILE_NAME, filesystem_contract::grant::MAX_NAME);
    assert!(file_name_fits(b"sixteen-bytes!!!"));
    assert!(!file_name_fits(b"seventeen-bytes!!"));
    assert!(!file_name_fits(b""));
}

#[test]
fn parse_u64_rejects_junk() {
    assert_eq!(parse_u64(b"123"), Some(123));
    assert_eq!(parse_u64(b""), None);
    assert_eq!(parse_u64(b"1a"), None);
    assert_eq!(parse_u64(b"-1"), None);
}
