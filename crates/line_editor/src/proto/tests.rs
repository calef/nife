use crate::proto;

/// `FLAG_RETRY` is distinguishable from every other read reply, in both shapes: a line reply
/// of length zero with no flags is an empty line, and a raw reply always carries 1..=8 bytes.
#[test]
fn a_retry_is_never_mistaken_for_data() {
    assert!(proto::is_retry(0, proto::FLAG_RETRY));
    assert!(!proto::is_retry(0, 0), "an empty line is not a retry");
    assert!(!proto::is_retry(0, proto::FLAG_INTERRUPTED));
    assert!(
        !proto::is_retry(1, proto::FLAG_RETRY),
        "a one-byte raw read of 0x04 is data"
    );
    assert_eq!(
        proto::FLAG_RETRY & (proto::FLAG_EOF | proto::FLAG_INTERRUPTED),
        0
    );
    assert_eq!(proto::OPERATION_QUIESCE, 8);
}

/// The proto word packing round-trips, and the reserved bits stay zero.
#[test]
fn proto_words_round_trip() {
    let w = proto::req(proto::OPERATION_READLINE, 42);
    assert_eq!(proto::operation(w), proto::OPERATION_READLINE);
    assert_eq!(proto::len(w), 42);
    assert_eq!(w & 0x00ff_ffff_0000_0000, 0);
    // The reply flags are wire ABI shared with every client: pin the values, because a
    // drifted FLAG_INTERRUPTED makes an interrupted read look like a normal empty line.
    assert_eq!(proto::FLAG_EOF, 1);
    assert_eq!(proto::FLAG_INTERRUPTED, 2);
    // Pinned for the same reason: OPERATION_RAWMODE and OPERATION_READRAW are wire ABI too, and a drifted
    // opcode number would silently reassign OPERATION_PRINT's or a future opcode's traffic.
    assert_eq!(proto::OPERATION_RAWMODE, 6);
    assert_eq!(proto::OPERATION_READRAW, 7);
}

// ---- the survivors of the 2026-10-03 mutation census ----

/// The two start bits are the two lowest bits, one each.
#[test]
fn the_two_start_bits_are_distinct_low_bits() {
    assert_eq!(proto::START_HANDOFF, 1);
    assert_eq!(proto::START_ABSORB, 2);
}
