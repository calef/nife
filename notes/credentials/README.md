# Credentials: the appendices to notes/credentials.md

[`notes/credentials.md`](../credentials.md) carries why the service hands out the operation and not
the secret, its two phases, and the Argon2id choice in brief. It also carries what a naive verifier
gets wrong, the reply and the shared page, EXAMPLES, a summary of what is proven, and BUGS. These
files carry the evidence its prose budget has no room for.

- [`argon2id.md`](argon2id.md): why the service depends rather than writes, the known-answer
  vectors, the overflow found in `argon2`, and the cost parameters with their measurements.
- [`what-is-proven.md`](what-is-proven.md): every host test, Kani harness and guest test, and what
  each pins.

*Name: provisional, minted 2026-10-11 (UTC) by lane/fifth-ten-longest-notes, which split the parent
under §212 (a prose budget). The directory follows §212's siting default, `notes/X/` beside
`notes/X.md`. calef names directories.*

The stems below were minted the same day by the same lane. Each is named for its content, and calef
names files.

- `argon2id` Name: provisional, minted 2026-10-11 (UTC) by lane/fifth-ten-longest-notes, after the
  algorithm it records.
- `what-is-proven` Name: provisional, minted 2026-10-11 (UTC) by lane/fifth-ten-longest-notes,
  after the parent's own heading.
