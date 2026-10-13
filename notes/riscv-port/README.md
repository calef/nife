# Porting to RISC-V: the appendices to notes/riscv-port.md

[`notes/riscv-port.md`](../riscv-port.md) carries why RISC-V was the second port, the arch contract,
the leaks it found, the RISC-V specifics, the `tp` hazard and the build setup. These files carry the
detail its prose budget has no room for.

- [`hal-leaks.md`](hal-leaks.md): each HAL leak the port exposed and how it closed, with the
  user-entry frame placement that was wrong on RISC-V for a year.
- [`bring-up-log.md`](bring-up-log.md): the six steps of the port, each with the boot line that
  proved it and the bugs it surfaced.

*Name: provisional. The directory follows §212 (a prose budget)'s siting default, `notes/X/`
beside `notes/X.md`. calef names directories.*

Each stem below is named for its content, and calef names files.

- `hal-leaks` Name: provisional.
- `bring-up-log` Name: provisional. After the brief's own example of a name for content.
