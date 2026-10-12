//! Tests of the status word, `$?`, and the status a line ends on.

use std::vec::Vec;

use super::*;
use crate::test_support::shown;
use crate::*;

/// `$?` is read where words are expanded, and its three values are the three things that can
/// happen to a line here.
#[test]
fn the_status_word_reads_the_last_commands_status() {
    for (s, digit) in [
        (Status::Ran, "0"),
        (Status::Failed, "1"),
        (Status::Refused, "2"),
    ] {
        let out = shown(|o| {
            echo(b"$?", s, &mut |_| Ok(NameSet::empty()), o);
        });
        assert_eq!(out, digit);
    }
}

/// **Quoting turns it off**, which is the escape hatch and also the honest statement of where
/// this shell is: both quote forms are literal today, so `"$?"` is two characters. When
/// variables arrive the two forms have to stop being the same thing.
#[test]
fn a_quoted_status_word_is_two_characters() {
    let out = shown(|o| {
        echo(b"'$?' $?", Status::Failed, &mut |_| Ok(NameSet::empty()), o);
    });
    assert_eq!(out, "$? 1");
}

/// The status a line ends on is not a name, a handle or a capability. This pins the shape
/// rather than a value: three variants, one small integer each, and nothing to designate with.
#[test]
fn a_status_carries_a_number_and_nothing_else() {
    assert_eq!(Status::default(), Status::Ran);
    assert!(Status::Ran.is_ok());
    assert!(!Status::Failed.is_ok() && !Status::Refused.is_ok());
    assert_eq!(
        (
            Status::Ran.code(),
            Status::Failed.code(),
            Status::Refused.code()
        ),
        (0, 1, 2),
    );
    for s in [Status::Ran, Status::Failed, Status::Refused] {
        let mut rendered = Vec::new();
        write_num(s.code(), &mut |b| rendered.extend_from_slice(b));
        assert_eq!(rendered, s.digits(), "the two spellings of {s:?} disagree");
    }
}

/// **A status crosses an atomic cell as a `u64`**, so the round trip is the contract and not
/// the two functions separately. Milestone 326's run replaced `from_code` with
/// `Default::default()` and deleted two of its arms, all three alive, because nothing in this
/// crate ever read a code back.
#[test]
fn a_status_survives_the_cell_it_is_carried_in() {
    for s in [Status::Ran, Status::Failed, Status::Refused] {
        assert_eq!(Status::from_code(s.code()), s, "{s:?}");
    }
    assert_eq!(Status::from_code(0), Status::Ran);
    assert_eq!(Status::from_code(1), Status::Failed);
    // Anything else is a refusal rather than a panic: the cell is shared and this side does
    // not get to assume what wrote it.
    assert_eq!(Status::from_code(2), Status::Refused);
    assert_eq!(Status::from_code(u64::MAX), Status::Refused);
}
