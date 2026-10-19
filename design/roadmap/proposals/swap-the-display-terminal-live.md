---
status: PROPOSED
raised: 2026-10-19
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# Swap `display_terminal` live, and leave the compositor until a boot runs it

Raised 2026-10-19 (UTC) by the lane for milestone 23 (a capability-routed component OS with live
replacement), after it bounded `terminal_supervisor`'s quiesce. Milestone 23's last two items are
swapping `display_terminal` and `compositor` the way `line_editor` is swapped (§232 (the line
editor swap contract)). Neither can be built without three answers that are an architect's: one is
a page budget two programs share, one a taste call the tree already names as such, and one a
contract change.

Reuse: `line_editor`'s swap, which this follows step for step: its declaration, its §209 (state
handoff is an opaque blob over a granted frame, and it is optional) blob and `terminal_supervisor`'s
two bays. tmux and GNU screen keep a grid in a server process so the program in front of it
can be replaced; they are recalled prior art for fork 2, not code to take.

## What the tree says, checked

- `display_terminal` already serves `line_editor::proto` (`components/src/display_terminal.rs`),
  so `OPERATION_QUIESCE`, `CTL_RESUME`, `CTL_QUIT` and the `START_*` flags exist for it. Answering
  them adds no opcode.
- One instance holds a `video_terminal::Vt` in `.bss`: 717,312 bytes of cells, which
  `crates/video_terminal/src/geometry.rs` counts as 177 page frames. A live swap holds two instances at once.
- A `graphical_terminal` session's region is 528 pages, of which 490 to 495 were measured in use
  (`GRAPHICAL_TERMINAL_SESSION_PAGES`). It comes from the job pool, `JOBS_BUDGET_PAGES` (672), which
  has 144 pages left while a session runs.
- So a second instance does not fit. The session would need about 180 more pages, more than the
  pool has free.
- A handoff, by what it carries, at sixteen bytes a cell: the visible grid at the virtio scanout's
  132 by 43 is 23 pages; the grid budget `MAX_CELLS` is 72; the whole `Vt` with its scrollback is 176.
  These are computed from the constants, not measured on a boot.
- No boot builds the compositor. Only `kernel/src/user/compositor_service.rs`, under the tests of
  milestone 33 (a compositor: one screen, mutually distrusting clients), spawns it.

## The three forks, each with a recommendation

**1. Where the replacement's pages come from.**

- A, recommended: grow the session region and the job pool by one instance and its supervisor's
  overhead, about 180 pages (720 KiB) per session, held for the session's life. Fewest moving parts:
  the supervisor's two bays, as `terminal_supervisor` already has.
- B: split the replacement from the job pool at swap time and refuse the swap when the pool cannot
  cover it. Costs nothing between swaps, but the supervisor then holds authority over a pool other
  jobs draw from, and a swap can fail for reasons unrelated to the terminal.
- C: shrink the scrollback until two instances fit in today's session. It trades a feature a person
  uses every day for an event that happens at an upgrade.

**2. What a replacement inherits.** The tree names this a taste call (notes/interactive-stack-swap.md).

- None: declare no handoff. The replacement starts on a blank grid over a surface that still shows
  the old text, which looks right until the first scroll and then loses the screen.
- The visible grid, recommended: a run sized to `MAX_CELLS` (72 pages), with the cursor, the
  attributes and the parser state. The screen does not change; the scrollback is lost, and that is
  recorded. §232 chose `FLAG_RETRY` over `FLAG_INTERRUPTED` so that the person sees nothing, and this
  holds the display to the same bar.
- The whole grid and scrollback (176 pages): nothing is lost, at the cost of a second instance's worth of pages
  again, on top of fork 1.

**3. Who holds it.** Recommended: `terminal_supervisor`, in display mode, holds `display_terminal`
beside `line_editor`, and `SWAP`'s second word names which one. In a session it is already built from
the same region, right beside `display_terminal`, and milestone 694 (the installer asks the terminal
to swap) wants one swap endpoint per terminal. The alternative is a second supervisor program, which
is a new name and a second endpoint for the installer to find. `SWAP`'s words are provisional, so
either is cheap to change.

The compositor waits. Its swap needs a quiesce on `compositor::proto`, a new opcode on a
contract every windowed client speaks, and a replacement has to republish each client's control page.
Swapping it under milestone 33's harness alone would measure the harness. Recommended: no compositor
swap until a boot runs one, which no milestone yet schedules.

## The other questions

- Prior art, recalled rather than read: MINIX 3's reincarnation server restarts a driver without its
  state, and Fuchsia's component framework restarts a component empty unless it escrows state itself.
  Both are fork 2's "none".
- Reversible: every number here is a constant and every word provisional, and nothing outside the
  tree has acted on them. Fork 1 is the one with a cost a reader of the session's budget meets.
- Effort: none of the three recommendations is about effort. Fork 2's visible grid is more work than
  no handoff and is recommended for what the person sees.

## What it unblocks

Milestone 23's `display_terminal` item. With the three answers it is a declaration in
`video_terminal`, a quiesce and an absorb in `display_terminal`, a second component in
`terminal_supervisor`, and a test in the shape of `system_tests::user::terminal_swap_tests` over
`display_service`'s wiring.
