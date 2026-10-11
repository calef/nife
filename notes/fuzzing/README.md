# Fuzzing the parse surface: the appendices to notes/fuzzing.md

[`notes/fuzzing.md`](../fuzzing.md) carries what fuzzing finds that Kani does not, which parsers
are fuzzed, the three bugs in summary, the CI job, corpus discipline, examples and every limitation.
These files carry the evidence its prose budget has no room for.

- [`kani-and-fuzzing.md`](kani-and-fuzzing.md): three crates where the bound is the whole story.
- [`not-fuzzed.md`](not-fuzzed.md): the parsers deliberately not fuzzed, each with its reason.
- [`why-cargo-fuzz.md`](why-cargo-fuzz.md): the three candidates, and the one license exception.
- [`three-bugs.md`](three-bugs.md): each bug, its fix and how it was found.
- [`ci-budget.md`](ci-budget.md): the exec rates behind sixty seconds, and the revert test that
  showed the job is a sweep.
- [`elf-seeds.md`](elf-seeds.md): why there is one ELF seed per machine, and what a seed buys.

*Name: provisional, minted 2026-10-11 (UTC) by lane/fourth-ten-longest-notes, which split the parent
under §212 (a prose budget). The directory follows §212's siting default, `notes/X/` beside
`notes/X.md`. calef names directories.*

The stems below were minted the same day by the same lane. Each is named for its content, and calef
names files.

- `kani-and-fuzzing` Name: provisional, minted 2026-10-11 (UTC) by lane/fourth-ten-longest-notes.
- `not-fuzzed` Name: provisional, minted 2026-10-11 (UTC) by lane/fourth-ten-longest-notes.
- `why-cargo-fuzz` Name: provisional, minted 2026-10-11 (UTC) by lane/fourth-ten-longest-notes,
  after the main page's own heading.
- `three-bugs` Name: provisional, minted 2026-10-11 (UTC) by lane/fourth-ten-longest-notes, after
  the main page's own heading.
- `ci-budget` Name: provisional, minted 2026-10-11 (UTC) by lane/fourth-ten-longest-notes.
- `elf-seeds` Name: provisional, minted 2026-10-11 (UTC) by lane/fourth-ten-longest-notes.
