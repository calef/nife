# A TLS crypto provider: the appendices to notes/cryptography-provider.md

[`notes/cryptography-provider.md`](../cryptography-provider.md) carries the build table, the two
build failures and the ring-3 hazard, what runs, how to run it, the refusal of `rustls-rustcrypto`,
what the pricing of the glue path found, and BUGS. These files carry the evidence its prose budget
has no room for.

- [`the-getrandom-gap.md`](the-getrandom-gap.md): why `getrandom` 0.2 needed a second symbol in
  `entropy_backend`, and where `ring` and `aws-lc-rs` fail instead.
- [`pricing-the-glue-path.md`](pricing-the-glue-path.md): the graph measured the same way for both
  paths, quality per primitive, the `rsa` argument and the `deny.toml` suppression, and the test
  of §46 (thin primitives or whole subsystems).

*Name: provisional, minted 2026-10-11 (UTC) by lane/sixth-ten-longest-notes, which split the parent
under §212 (a prose budget). The directory follows §212's siting default, `notes/X/` beside
`notes/X.md`. calef names directories.*

The stems below were minted the same day by the same lane. Each is named for its content, and calef
names files.

- `the-getrandom-gap` Name: provisional, minted 2026-10-11 (UTC) by lane/sixth-ten-longest-notes.
- `pricing-the-glue-path` Name: provisional, minted 2026-10-11 (UTC) by
  lane/sixth-ten-longest-notes, after the parent's own heading.
