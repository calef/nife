---
status: DECIDED
raised: 2026-10-18
decided: 2026-10-18
ratified_by: calef
---

# 308. The installer asks the terminal to swap: who holds the swap endpoint, and how a new image travels

[Milestone 694 (the installer asks the terminal to swap)](../roadmap/0694-the-installer-asks-the-terminal-to-swap.md)
builds what this section rules. Milestone 23 (a capability-routed component OS with live
replacement) built `terminal_supervisor`, which replaces `line_editor` live and answers `SWAP` with
`SWAPPED` or `ROLLED_BACK` (`line_editor::component::supervisor`). Nothing on a real boot sends it.
`system_initializer` passes no swap endpoint (`children.rs`, the terminal builder), and a
replacement is the image the supervisor was handed at birth. Two programs agree on a message and
one image travels between them, so the shape is an architect's.

## What the code says

- The capability table is 64 slots, with a measured peak of 34 (`kernel::cap`, milestone 754 (the
  capability table grows to 64 slots), ruled 2026-10-04). A permanent capability per terminal fits without freeing another.
- There is more than one terminal. `system_initializer` builds a `terminal_supervisor` for the boot
  terminal and another for every `graphical_terminal` session (`SessionDiscipline`: "built twice
  from one measurement"). So "the" swap endpoint is one per running supervisor.
- `line_editor` ships in the `terminal` package with `console`, `input`, `terminal_sink_caretaker`
  and `terminal_supervisor` itself. An install of a new `terminal` can change members that have no
  swap contract, including the supervisor.

## The forks

### 1. Who holds the swap endpoints

- 1a. `system_initializer` holds them, and the activation path sends. When it builds a supervisor
  it retypes a swap endpoint, hands the supervisor `READ`, and keeps `WRITE` in a list beside the
  session it belongs to. The activation path (`activation::activate`, rung 3a of milestone 198 (a
  package manager)) is already in this process, so the trigger needs no new route. Cost: one slot
  per running terminal in `system_initializer`'s table, against 30 free. *Ruled.*
- 1b. The package manager holds them. `jig` would then hold the power to replace the terminal of
  every session. §270 (a package manager holds an installer endpoint, not the spawn endpoint) gave
  it the right to ask for an install and nothing wider. *Refused.*

### 2. How the new image reaches the supervisor

- 2a. A frame run sent with the request. The activation path copies the vouched `line_editor` bytes
  into a frame run it retypes by page count, under §233 (`MemoryRegion::RETYPE` takes a page
  count). It sends that capability with `SWAP` (`SEND_CAP`), with the length in the message. The
  supervisor maps it where it maps its own image (`ELF_VA`) and builds the replacement from it.
  The supervisor returns the run on `SWAPPED` or `ROLLED_BACK`, so it carries one image at rest,
  not two. An image larger than the window or than `INSTANCE_PAGES` is answered `ROLLED_BACK`
  before anything is quiesced. *Ruled.*
- 2b. The supervisor reads the package store. It would need filesystem authority it does not have
  today, to fetch one file. *Refused.*
- 2c. A second copy at build time. The supervisor would carry every future build from birth.
  *Refused.*

### 3. Which terminals swap

- 3a. Every running terminal swaps, the boot terminal first. A person installing a new
  `line_editor` sees it in the terminal they are typing in, whichever session that is. That is
  milestone 23's claim end to end. *Ruled.*
- 3b. Only the boot terminal swaps; a session picks the new build up when it is next built. This is
  simpler, and it is the Unix answer (a running process keeps its binary). But then "active" in the
  installed-programs table means "what new terminals are built from", not "what runs". The session
  a person is typing in does not change. *Refused.*

### 4. What an activation does when a swap rolls back

The new build refused the old state, so the old instance serves on.

- 4a. The activation waits for every answer, and the first `ROLLED_BACK` fails it. Terminals
  already swapped are swapped back to the previous generation's image (the store keeps it;
  `Rollback` exists). The activation is recorded as failed, a verdict the installer already has.
  The table never says a build is active while another runs. *Ruled.*
  - Known hole: a swap back can itself be refused if the old build cannot read the new build's
    state blob. Under §209 (state handoff is an opaque blob over a granted frame, and it is
    optional), the fallback is a fresh start of the old build.
    That loses the half-typed line and the history in that one terminal, and the activation's
    answer says so.
- 4b. The boot terminal decides. If it swaps, the activation stands, and a session that refuses
  keeps the old build until it is rebuilt. Each refusal is reported. This is simpler, but the table
  can then be false for a session. *Refused.*

### 5. Members with no swap contract (stated, not a fork)

Only `line_editor` has a swap contract, §232 (the `line_editor` swap contract). An install that changes `console`, `input`,
`terminal_sink_caretaker` or `terminal_supervisor` activates them for the next build, which is the
next session or the next boot. The supervisor is the swapper and is not swapped this way, as
milestone 23 says of the root supervisor. The activation's answer names the members that took
effect live and the members that wait.

## The wire

On the swap endpoint, from `system_initializer` to `terminal_supervisor`: `SWAP`, with the image
frame run as the sent capability and its length in bytes. Answers are unchanged: `SWAPPED` (with
the instance count) or `ROLLED_BACK` (with the reason). One new refusal reason, "image too large",
comes before any quiesce. `STOP` stays test-only. Both ends are in
`line_editor::component::supervisor`, which is already the contract crate the two programs share.

## What it unblocks

Milestone 694's build: one lane for 1a and 2a (the endpoints, the frame run and the wire), then 3
and 4 on top. The test is a typist in a terminal while a package install activates a new
`line_editor`, on all three architectures, with the half-typed line and the history carried over.
That is the kernel guest test's swap, driven by the real activation path in place of the test.

## The ruling

calef, 2026-10-18 (UTC):

- Fork 1: *"1a: system_initializer holds the swap endpoints and the activation path sends."*
- Fork 2: *"2a: the image travels as a frame run sent with SWAP, returned on the answer."*
- Fork 3: *"3a: every running terminal swaps, the boot terminal first."*
- Fork 4: *"4a: the activation waits for every answer; the first ROLLED_BACK fails it and swaps the others back."*
