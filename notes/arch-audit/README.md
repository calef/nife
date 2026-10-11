# The arch audit: the appendices to notes/arch-audit.md

[`notes/arch-audit.md`](../arch-audit.md) carries the bug class, what was read, the three findings
and their dispositions, the x86_64 gap and the audit's limit. These files carry the evidence its
prose budget has no room for.

- [`candidates-cleared.md`](candidates-cleared.md): every candidate the audit cleared on aarch64 and
  riscv64, and the reasoning that makes each one safe.
- [`plic-lock.md`](plic-lock.md): how finding 3 was closed, why the lock is IRQ-safe, at that rank
  and scoped to that register, and what the tests can and cannot prove.

*Name: provisional, minted 2026-10-11 (UTC) by lane/fifth-ten-longest-notes, which split the parent
under §212 (a prose budget). The directory follows §212's siting default, `notes/X/` beside
`notes/X.md`. calef names directories.*

The stems below were minted the same day by the same lane. Each is named for its content, and calef
names files.

- `candidates-cleared` Name: provisional, minted 2026-10-11 (UTC) by lane/fifth-ten-longest-notes,
  after the parent's own heading.
- `plic-lock` Name: provisional, minted 2026-10-11 (UTC) by lane/fifth-ten-longest-notes.
