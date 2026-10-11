# Porting to RISC-V: the appendices to notes/riscv-port.md

[`notes/riscv-port.md`](../riscv-port.md) carries why RISC-V was the second port, the arch contract,
the leaks it found, the RISC-V specifics, the `tp` hazard and the build setup. These files carry the
detail its prose budget has no room for.

- [`hal-leaks.md`](hal-leaks.md): each HAL leak the port exposed and how it closed, with the
  user-entry frame placement that was wrong on RISC-V for a year.
- [`bring-up-log.md`](bring-up-log.md): the six steps of the port, each with the boot line that
  proved it and the bugs it surfaced.

*Name: provisional, minted 2026-10-11 (UTC) by lane/third-ten-longest-notes, which split the parent
under §212 (a prose budget). The directory follows §212's siting default, `notes/X/` beside
`notes/X.md`. calef names directories.*

The stems below were minted the same day by the same lane. Each is named for its content, and calef
names files.

- `hal-leaks` Name: provisional, minted 2026-10-11 (UTC) by lane/third-ten-longest-notes.
- `bring-up-log` Name: provisional, minted 2026-10-11 (UTC) by lane/third-ten-longest-notes, after
  the brief's own example of a name for content.
