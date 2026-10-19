# Swapping the interactive stack

*Which of the interactive stack's three components can be swapped live, for milestone 23 (a
capability-routed component OS with live replacement). `line_editor` is swapped live by
`terminal_supervisor`. `display_terminal` and `compositor` are not, and each is a different
problem.*

## What the tree says, checked

| component | who builds it on a real boot | who could swap it | state it holds |
|---|---|---|---|
| `line_editor` | `system_initializer`, out of its own budget (`crates/system_initializer/src/lib.rs`, item 3) | `system_initializer`: it holds every object it routed | the edit line, the kill buffer, eight lines of history, up to four queued lines, raw or cooked mode, the interrupt count: under 4 KiB, measured (see BUGS) |
| `display_terminal` | `system_initializer`, at a `graphical_terminal` launch (`build_graphical_terminal_session`; no boot builds it, per milestone 632 (graphics on demand)) | nobody: no userspace process holds its endowment or its supervision endpoint | the character grid and scrollback, several hundred KiB (`video_terminal::Vt`) |
| `compositor` | no boot builds it. Only the tests of milestone 33 (a compositor: one screen, mutually distrusting clients) spawn it (`kernel/src/user/compositor_service.rs`, `#[cfg_attr(not(test), ...)]`); the graphical stack (option A of milestone 177 (wire the graphical terminal stack into the real interactive boot), built at a `graphical_terminal` launch) has "no compositor in this path" | nobody outside a test | nothing it could not rebuild: each client's surface is a frame the client owns |

All three run under the kernel test harness. Outside it, two are built and only `line_editor` has
a supervisor that could swap it.

`display_terminal` and `compositor` have no `component_plan` declaration: their endowments are
literals in their builders, the defect milestone 23's manifest work removed from `swapper` and
nowhere else. A declaration for either is written against the ELF-note manifest of milestone 597
(a program carries its manifest in an ELF note).

## line_editor: swapped live

Its supervisor exists, its state fits a page, and §209 (state handoff is an opaque blob over a
granted frame, and it is optional) is built (notes/state-handoff.md). The terminal contract is a
wire format eleven programs speak (`swish`, `rmle`, `mdr`, `window`, `input`, `keyboard_driver`,
`console`, `display_terminal`, `compositor`, `terminal_sink_caretaker` and `line_editor` itself),
so its two changes were calef's. He ruled them on 2026-09-26, and on 2026-09-27 option A of
milestone 661 (a terminal supervisor holds the line editor):

- **A quiesce request is an additive `OPERATION_QUIESCE`** in `line_editor::proto`, which changes no
  existing opcode and which only a supervisor sends. The swap's drain step rides the served
  endpoint so its FIFO does the waiting, per §41 (the endpoint is the broker, and a device is
  revoked by taking it back), as `hot_swap_contract` does. Not a second, supervisor-only control
  endpoint, because that loses the FIFO property and makes the drain racy.
- **A parked reader is told `FLAG_RETRY`**, meaning "ask again". At an idle prompt `swish` is always
  parked in `OPERATION_READLINE`, and its one-shot reply capability lives in `line_editor`'s
  capability table, where no one can move it (notes/hung-component.md, question 3), so the
  incumbent answers it before it quiesces. `swish` re-issues the same `OPERATION_READLINE`; the
  replacement, having absorbed the edit line, redraws it; the person sees nothing. Not
  `FLAG_INTERRUPTED`, because `swish` would redraw its prompt as if the person had pressed `^C`,
  which lies about why. Not deferring the swap until no read is parked, because at an idle prompt
  that is never. The flag name is provisional.

What is built:

- `line_editor::component` declares what a supervisor routes to it, with one declaration per
  output-sink shape. `line_editor::handoff` is its blob: the edit line and cursor, the kill buffer,
  the history ring, the prompt, the queued lines, raw mode and its queue, and the `^C` count. A
  supervisor mints the run with `MemoryRegion::RETYPE`'s page count, and `line_editor` declares one
  page.
- `terminal_supervisor` (name provisional, calef's to name) is what `system_initializer` builds. It
  builds `line_editor` from the declaration and runs the §209 swap when asked on its swap endpoint.
  Instances are built in two reusable bays, because a replacement split straight from the budget
  sits above the incumbent and leaves a hole when it dies, so a second swap fails for want of
  budget.
- `kernel::user::terminal_swap_tests` swaps it twice under a typist and a parked reader on all
  three architectures: a line half typed before the swap finishes after it, and history typed
  before it is recalled after it. `script/swish-check`, plain and graphical, boots through it.
- The quiesce is bounded. A stand-in built from the supervisor's own image makes the `CALL` and
  signals the answer on a notification the supervisor's timer also signals. A `line_editor` that does not answer within
  `QUIESCE_BOUND_MS` is replaced without its state and its region destroyed, which answers its
  callers `Gone`. The second test in `terminal_swap_tests` hangs one behind a console that stops
  acknowledging. A spawner that passes a swap endpoint copies the supervisor's image and grants a
  stand-in's budget; one that passes none, which is every boot today, grants nothing extra.

What is left is the trigger. [§308 (the installer asks the terminal to swap)](../design/decisions/0308-the-installer-asks-the-terminal-to-swap.md)
rules it, and `design/roadmap/0694-the-installer-asks-the-terminal-to-swap.md` builds it.

## display_terminal: a budget, a handoff and a holder

`system_initializer` builds `display_terminal` (milestone 600 (the graphical terminal stack is built
in userspace)), so a supervisor could hold it, and it already serves `line_editor::proto`, so its
quiesce needs no new opcode. Three things are left, and each is an architect's:

- One instance is 177 pages of `.bss`, a swap holds two, and a `graphical_terminal` session has 144
  pages of its job pool to spare.
- Its state is far past a page. A replacement that starts with an empty grid over a surface that
  still shows the old text would look right until the first scroll. Carrying the visible grid is
  72 pages; the scrollback too, 176. Which to carry is a taste call.
- Something has to hold it: `terminal_supervisor` beside `line_editor`, or a second supervisor.

`design/roadmap/proposals/swap-the-display-terminal-live.md` has the options, their costs and a
recommendation for each.

## compositor: not before it has a boot path

It is the easiest of the three on paper: no state worth moving, and every surface a client owns is a
capability its supervisor routed, so a replacement wired from the same declaration gets the same
surfaces. But no boot runs it. Swapping it under the milestone 33 test harness alone would measure
the harness.

## Milestones

- Milestone 669 (swap `line_editor` live under `system_initializer`) is built, under a terminal
  supervisor rather than `system_initializer`, as ruled.
- Milestone 600 (the graphical terminal stack is built in userspace) is built, so
  `system_initializer` builds `display_terminal`.
- The compositor gets no milestone until it has a boot path; that is recorded in the block.

## BUGS

`terminal_supervisor`'s, which its module doc lists:

- **Nothing on a real boot asks it to swap.** `system_initializer` passes no swap endpoint, so the
  supervisor builds the terminal and waits; only the kernel's test swaps. Milestone 694 (the
  installer asks the terminal to swap) builds the trigger, and §308's frame run, sent with `SWAP`,
  is how a new image arrives. Until then a replacement is the same image.
- **A hung `line_editor`'s state is lost, and its callers are told `Gone`.** Only the incumbent can
  write the blob, so the half-typed line, the history and any queued lines are gone. `swish` and
  `rmle` handle `FLAG_RETRY` but not `Gone`, and the terminal contract does not say which of its
  requests may be repeated, so on a real boot the shell would read a hang as a failed read. That is
  a contract question for an architect; only the kernel's test typist asks again today.
- **A hang caused by the sink can answer the replacement late.** In console mode a `line_editor`
  blocked in its sink's reply is hung by the console. If the console recovers, its ack for the dead
  incumbent's flush reaches the replacement's first `RECEIVE`, one flush early.
- **A slow `line_editor` is convicted as a hung one.** Nothing distinguishes an answer that arrives
  after the bound from none. The bound is about thirty times the slowest healthy answer measured.
- **A `line_editor` that faults is not restarted.** Its death is collected at the next swap, and
  until then the terminal is gone. Restarting it fresh is the hung path without the quiesce, and is
  not built because nothing has asked for it.

The note's own:

`line_editor`'s blob is smaller than `LineDisc`'s buffers, because it does not carry the
completed-line snapshot or the browse stash. Measured by `line_editor`'s host test on 2026-09-27: a
full ring of near-maximal history lines serializes to under 2 KiB, and with four queued lines the
whole blob is under 4 KiB, so it fits the one page it declares. `MemoryRegion::RETYPE` takes a page
count anyway, DECISIONS §233 (`MemoryRegion::RETYPE` takes a page count), because `redoxfs_server`
will not fit a page and `swapper`'s fixture uses two.
`display_terminal`'s size is still read from `video_terminal::Vt`'s own doc comment.

## See also

- notes/state-handoff.md, notes/live-replacement.md, notes/terminal-contract.md
- design/roadmap/0177-graphical-interactive-boot.md for why the graphical stack is built kernel-side
