# Capabilities: the appendices to notes/capabilities.md

[`notes/capabilities.md`](../capabilities.md) carries what a capability is, why the kernel has no
`open()`, the confused deputy, the cost argument and a summary of what milestones 7d and 7e built.
Both are parts of milestone 7 (user mode: EL0, capabilities, the ELF loader, and IPC). These files carry the full record of those two milestones, which its prose budget has no room for.

- [`syscall-surface.md`](syscall-surface.md): milestone 7d. The three calls, the ABI crate, the
  capability table and `derive`, a process that holds nothing, and the kernel refusing to be a
  confused deputy by asking the hardware.
- [`ipc-endpoints.md`](ipc-endpoints.md): milestone 7e. The endpoint as a rendezvous, rights as
  which end you hold, the `Blocked` state, where the message lives, and what single core made easy.

*Name: provisional, minted 2026-10-11 (UTC) by lane/fifth-ten-longest-notes, which split the parent
under §212 (a prose budget). The directory follows §212's siting default, `notes/X/` beside
`notes/X.md`. calef names directories.*

The stems below were minted the same day by the same lane. Each is named for its content, and calef
names files.

- `syscall-surface` Name: provisional, minted 2026-10-11 (UTC) by lane/fifth-ten-longest-notes,
  after the parent's own heading.
- `ipc-endpoints` Name: provisional, minted 2026-10-11 (UTC) by lane/fifth-ten-longest-notes.
