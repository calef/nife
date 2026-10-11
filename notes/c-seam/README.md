# The C seam: the appendices to notes/c-seam.md

[`notes/c-seam.md`](../c-seam.md) carries why C is the demonstration, the shape of the Rust shell,
the libc tiers, the toolchain and the confinement test in brief, what the supervisor had to hold,
and the caveats. These files carry the detail its prose budget has no room for.

- [`the-toolchain.md`](the-toolchain.md): how clang is found, why one compiler must carry both
  backends, and every flag with its reason.
- [`the-confinement-test.md`](the-confinement-test.md): the layout and its two witnesses, the five
  assertions and the control, and the fault transcripts from both ISAs.

*Name: provisional, minted 2026-10-11 (UTC) by lane/sixth-ten-longest-notes, which split the parent
under §212 (a prose budget). The directory follows §212's siting default, `notes/X/` beside
`notes/X.md`. calef names directories.*

The stems below were minted the same day by the same lane. Each is named for its content, and calef
names files.

- `the-toolchain` Name: provisional, minted 2026-10-11 (UTC) by lane/sixth-ten-longest-notes,
  after the parent's own heading.
- `the-confinement-test` Name: provisional, minted 2026-10-11 (UTC) by
  lane/sixth-ten-longest-notes, after the parent's own heading.
