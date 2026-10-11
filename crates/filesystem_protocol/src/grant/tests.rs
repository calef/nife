use crate::*;

#[test]
fn a_granted_name_survives_the_two_argument_words() {
    // The name rides in START arguments rather than a frame, so a per-file grant costs no page.
    // Every length up to the limit has to come back exactly, including the 8-byte boundary where
    // it splits across the two words.
    for name in [
        &b"a"[..],
        b"motd",
        b"scratch",
        b"12345678",
        b"123456789",
        b"sixteen-bytes!!!",
    ] {
        assert!(grant::fits(name), "{name:?} should fit");
        let (lo, hi) = grant::pack_name(name);
        let mut buf = [0u8; grant::MAX_NAME];
        let n = grant::unpack_name(lo, hi, name.len(), &mut buf);
        assert_eq!(&buf[..n], name, "{name:?} did not survive the round trip");
    }
    assert!(!grant::fits(b""), "an empty name designates nothing");
    assert!(
        !grant::fits(b"seventeen-bytes!!"),
        "a name past the limit must be refused where it is packed, not truncated silently",
    );
}

/// **`pack_name` stops at the limit even when the caller did not.** [`grant::fits`] is the
/// caller's check and `pack_name` does not repeat it, so it must stay total on input it was
/// told not to receive: a 17-byte name packs as its 16-byte prefix rather than indexing `hi`
/// off its end.
#[test]
fn pack_name_stops_at_the_limit_even_when_the_caller_did_not() {
    assert!(!grant::fits(b"sixteen-bytes!!!X"));
    assert_eq!(
        grant::pack_name(b"sixteen-bytes!!!X"),
        grant::pack_name(b"sixteen-bytes!!!"),
    );
}

#[test]
fn a_grant_spec_carries_its_length_and_its_direction_apart() {
    // A read grant and a write grant of the same name differ only here, so the two fields must
    // not bleed: a 16-byte name must not look like a write bit, and write must not lengthen it.
    let ro = grant::spec(16, grant::READ);
    let rw = grant::spec(16, grant::READ | grant::WRITE);
    assert_eq!(grant::spec_len(ro), 16);
    assert_eq!(grant::spec_len(rw), 16);
    assert!(!grant::is_writable(ro));
    assert!(grant::is_writable(rw));
    // And a zero-rights spec grants nothing, rather than defaulting to something.
    assert!(!grant::is_writable(grant::spec(4, 0)));
}

// The tests from here down each pin something the run of milestone 85 (mutation testing over the
// host crates) showed nothing pinned. The equivalences (every `|` over disjoint masked fields,
// where `^` cannot differ) are recorded in notes/mutation-testing.md rather than tested.

/// The per-file grant words, byte for byte: the rights bits, the spec word, and a name packed
/// across the two argument words. The seventeen-byte case is the one that matters: the copy
/// loop's bound is `i < MAX_NAME`, and `<=` there is an out-of-bounds write the caller's
/// `fits` check never reaches.
#[test]
fn grant_words_pack_exactly() {
    assert_eq!([grant::READ, grant::WRITE], [1, 2]);
    let w = grant::spec(5, grant::WRITE);
    assert_eq!(w, 5 | (2 << 8));
    assert_eq!(grant::spec_len(w), 5);
    assert_eq!(grant::spec_rights(w), grant::WRITE);

    let (lo, hi) = grant::pack_name(b"abcdefghij");
    assert_eq!(lo, u64::from_le_bytes(*b"abcdefgh"));
    assert_eq!(hi, u64::from_le_bytes(*b"ij\0\0\0\0\0\0"));
    // Seventeen bytes: the packer stops at MAX_NAME rather than indexing past hi.
    let (lo17, hi17) = grant::pack_name(b"abcdefghijklmnopq");
    assert_eq!(lo17, u64::from_le_bytes(*b"abcdefgh"));
    assert_eq!(hi17, u64::from_le_bytes(*b"ijklmnop"));
}
