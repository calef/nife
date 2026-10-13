# A machine with no serial port: the appendices to notes/serial-less-output.md

[`notes/serial-less-output.md`](../serial-less-output.md) carries the two halves of the problem,
what was chosen, the gate, the bench procedure with its triage table, the early-boot window's table
and BUGS. These files carry the arguments and detail its prose budget has no room for.

- [`the-handoff-design.md`](the-handoff-design.md): why the screen rides PVH's command line, and why
  the console is not `video_terminal`.
- [`the-alternatives.md`](the-alternatives.md): each candidate priced against the human and the gate.
- [`the-shell-on-the-screen.md`](the-shell-on-the-screen.md): how milestone 400 (the shell on the
  firmware's screen) put the prompt on the screen.
- [`ramfb.md`](ramfb.md): the boards' screen under QEMU, its gate, what the first red CI run
  settled, and its own BUGS.
- [`the-early-boot-window.md`](the-early-boot-window.md): why the window cannot be narrated, the
  banner that bounds it, and the halted-loader experiment.
- [`the-xenon-grid.md`](the-xenon-grid.md): the grid xenon's screen shows instead of text, and
  the experiment that would measure it.

*Name: provisional, minted 2026-10-11 (UTC) by lane/seventh-ten-longest-notes, which split the
parent under §212 (a prose budget). The directory follows §212's siting default, `notes/X/` beside
`notes/X.md`. calef names directories.*

The stems below were minted the same day by the same lane. Each is named for its content, and calef
names files.

- `the-handoff-design` Name: provisional, minted 2026-10-11 (UTC) by
  lane/seventh-ten-longest-notes, for the two design choices in the handoff it holds.
- `the-alternatives` Name: provisional, minted 2026-10-11 (UTC) by lane/seventh-ten-longest-notes,
  after the parent's "What the alternatives cost".
- `the-shell-on-the-screen` Name: provisional, minted 2026-10-11 (UTC) by
  lane/seventh-ten-longest-notes, after the parent's "The shell on the screen, too".
- `ramfb` Name: provisional, minted 2026-10-11 (UTC) by lane/seventh-ten-longest-notes, for the
  QEMU device it describes.
- `the-early-boot-window` Name: provisional, minted 2026-10-11 (UTC) by
  lane/seventh-ten-longest-notes, after the parent's "The window between the firmware and the kernel".
- `the-xenon-grid` Name: provisional, minted 2026-10-13 (UTC) by lane/comment-blocks-zero-1, for
  the finding it holds.
