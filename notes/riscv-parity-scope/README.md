# RISC-V parity: the appendices to notes/riscv-parity-scope.md

[`notes/riscv-parity-scope.md`](../riscv-parity-scope.md) carries each parity workstream as built,
the gaps in the shared suite, and the post-parity correction. These files carry the records its
prose budget has no room for.

- [`user-tests-port.md`](user-tests-port.md): how `kernel::sched::tests` and `kernel::user::tests`
  came to run on both ISAs, the defects that surfaced, and what stays aarch64-only.
- [`suite-gaps.md`](suite-gaps.md): the ASID gap, the 24-test overlap still open, the leak police,
  and two test failures first misread as host load.
- [`original-scope.md`](original-scope.md): each workstream's scope as written before it was built,
  the superseded "blocked" record for C, and the recommended sequence.

*Name: provisional, minted 2026-10-11 (UTC) by lane/third-ten-longest-notes, which split the parent
under §212 (a prose budget). The directory follows §212's siting default, `notes/X/` beside
`notes/X.md`. calef names directories.*

The stems below were minted the same day by the same lane. Each is named for its content, and calef
names files.

- `user-tests-port` Name: provisional, minted 2026-10-11 (UTC) by lane/third-ten-longest-notes.
- `suite-gaps` Name: provisional, minted 2026-10-11 (UTC) by lane/third-ten-longest-notes.
- `original-scope` Name: provisional, minted 2026-10-11 (UTC) by lane/third-ten-longest-notes,
  after the parent's "(original scope)" headings.
