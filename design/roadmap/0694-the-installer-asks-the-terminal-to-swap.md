---
status: NOT-STARTED
raised: 2026-09-27
promoted_from: the-installer-asks-the-terminal-to-swap
milestone_dependencies: none
decision_dependencies: 308
machine_requirements: none
specific_machine: none
needs_person: no
---
# 694. The installer asks the terminal to swap

*Title and slug are drafts.*
Raised by the lane for milestone 23 (a capability-routed component
OS with live replacement) after building `terminal_supervisor`, which can replace `line_editor` live
but which nothing on a real boot asks to.

## What exists

`terminal_supervisor` serves a swap endpoint when it is started with one, and answers `SWAP` with
`SWAPPED` or `ROLLED_BACK` (`line_editor::component::supervisor`). The kernel's guest test holds that
endpoint. On a boot, `system_initializer` passes none, so the supervisor builds the terminal and
waits. And a replacement today is the same image the supervisor was handed at birth.

## The decision

[§308 (the installer asks the terminal to swap)](../decisions/0308-the-installer-asks-the-terminal-to-swap.md)
rules the shape. `system_initializer` holds one swap endpoint per `terminal_supervisor`, and the
activation path sends. The new `line_editor` image travels as a frame run sent with `SWAP` and comes
back on the answer. Every running terminal swaps, the boot terminal first. The activation waits for
every answer, and the first `ROLLED_BACK` fails it and swaps the others back.

The capability table is 64 slots with a measured peak of 37
(`kernel::cap::CAPABILITY_TABLE_PEAK_MEASURED`), so the endpoint each terminal costs
`system_initializer` fits without freeing another. Members of the `terminal` package other than
`line_editor` take effect at the next build, not live.

## What it unblocks

A person installing a new `line_editor` and the terminal they are typing in changing under them,
losing nothing: milestone 23's claim on a component a person uses, end to end.

## Index row

`terminal_supervisor` can replace `line_editor` live but nothing on a real boot asks it to. §308 rules the shape: `system_initializer` holds one swap endpoint per terminal supervisor, the new image travels as a frame run with `SWAP`, and every terminal swaps.
