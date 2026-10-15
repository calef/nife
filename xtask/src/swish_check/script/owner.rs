//! **The owner's console** (DECISIONS §221): suspending and resuming an identity, vouching
//! for a local build, and the rollback that takes the vouch away.
//!
//! Cut from the table in `script.rs` verbatim by milestone 900 (the swish-check script
//! splits by topic before it crosses the ceiling).
//!
//! Name: provisional (milestone 900's lane, 2026-10-15 UTC).

use super::*;

/// The owner's-console rows of [`SWISH_CHECK_SCRIPT`], in table order.
pub(super) const LINES: &[Line] = &[
    // **The owner suspends and resumes an identity** (milestone 152 (durable delegation), calef's
    // §108 (disabling credentials kills the durable session) ruling of 2026-09-26). `user` edits `login_protocol::SUSPENDED_LIST` at this prompt's
    // root, the file service's, in the format `login` reads at a login and at start-up; the kernel
    // suite's `login_tests` prove what it does with it. Here: the
    // two verbs answer, the file is where `login` looks, and a malformed line changes nothing.
    line(0, "user suspend alice", &["suspended alice"]),
    line(0, "ls", &["suspended"]),
    line(0, "user resume alice", &["resumed alice"]),
    line(
        0,
        "user delete alice",
        &["user suspend <name> | user resume <name>"],
    ),
    // **The owner vouches for a local build** (DECISIONS §221 (the boot prompt is the owner's
    // console), ruling 1). `installed/unvouched` is the fresh build the D2 lines above ran on the
    // ruling's endowment (slots 0, 1 and 2). Vouching writes a generation that lists its digest,
    // so the same bytes now run vouched, endowed from the manifest note they carry (milestone 597
    // (a program carries its manifest in an ELF note)). The witness's note asks for the process
    // domain, entropy and the network and no clock, so the census moves from D2's `0 1 2` to
    // `0 7 9 10` (10 is the network stack's endpoint): the grant changed with the
    // vouch, and nothing else on the line did. The probe lines then read as reached, which their
    // wording ("declared no ...") was not written for. A rollback takes the vouch away and the
    // census is D2's again.
    line(
        0,
        "vouch installed/unvouched",
        &["vouched; generation 4 is live"],
    ),
    line(
        0,
        "caps installed/unvouched",
        &["provenance: vouched by the owner in activation generation 4 (digest sha256:"],
    ),
    // **A vouch claims no name** (§229 B2): the entry is found by the bytes' digest and never by
    // the name it was recorded under, so the bare word reaches nothing.
    line(0, "unvouched", &["no such program"]),
    line(1, crate::disk::INSTALLED_UNVOUCHED, &["slots held: 0 7 9"]),
    line(1, "jig rollback", &["rolled back; generation 3 is live"]),
    line(
        0,
        "caps installed/unvouched",
        &["provenance: unvouched (digest sha256:"],
    ),
    line(
        1,
        crate::disk::INSTALLED_UNVOUCHED,
        &["slots held: 0 1 2\n"],
    ),
];
