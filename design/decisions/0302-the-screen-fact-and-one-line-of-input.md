---
status: DECIDED
raised: 2026-10-15
decided: 2026-10-15
ratified_by: calef
---

# 302. The screen fact and one line of input: the spawn protocol's other two thirds

Raised by [milestone 652 (twelve `DECISION` gates cite only sections that are already ruled)](../roadmap/0652-twelve-decision-gates-cite-only-ruled-sections.md). Milestone 334 (color and the pager: the spawn protocol's other two thirds) has
waited on this fork since
[milestone 40 (documentation as a system service)](../roadmap/0040-documentation-service.md)'s block named
it. [§106 (the terminal_sink_caretaker narrowing)](0106-tail-output-narrowing.md) took the spawn protocol's narrowing for a tail stage's *primary output*.
It did not extend
it to telling a stage that it ends at a real screen, or to granting one line of input without
granting the keyboard. Both widen a protocol two programs agree on, so both are an architect's.

## The decision

**A stage ends at a screen exactly when it holds a `WRITE`-only terminal capability, and a pager
reads one line with a one-shot badge the terminal refuses to serve twice.**

The screen fact:

- The grant is the fact. A stage holds a terminal capability narrowed to `WRITE`, no read side,
  exactly when its output ends at a real screen. Color means holding it: the documentation
  renderer's `Style.color` derives from the capability check instead of arriving as a
  passed-in boolean, which is what
  [§106 (the terminal_sink_caretaker narrowing)](0106-tail-output-narrowing.md) left the spawn
  path unable to say. Presence *is* the passing the renderer asked for.
- The grant §106 routes through the caretaker today is re-routed to the stage's own holding. The
  caretaker keeps its completion-signal role: `Wiring::screen` and the spawn-protocol bit between
  shell and progenitor are unchanged, and they stay invisible to the stage. The stage's knowledge
  and its connection become one channel, so a stage cannot believe it ends at a screen while its
  output lands in a file.
- The read side never travels. The capability is `WRITE` only, which is what keeps the keyboard
  out of every stage and the one-line half necessary at all.
- Naming: the new right is not called "screen". `SCREEN_BIT` is taken in
  `crates/grant_plan/src/spawnproto.rs`, and `components/src/graphical_terminal.rs` already
  refused that name once.

One line of input:

- A new badge class on the terminal protocol, served `OPERATION_READLINE` exactly once and
  refused afterward. The shell mints one per page; the pager holds one at a time and can never
  hold the keyboard, which stays behind the full terminal endpoint's read side.
- No new opcode. What is committed is badge semantics on wire two programs already agree on: this
  class is single-use, and a refused second `READLINE` is the contract, not a queue. The
  terminal tracks one-shot badges; composing N of them replaces any counter-per-badge scheme.
- The tree's own precedent for a capability good for exactly one use is
  [§12 (call/reply IPC)](0012-call-reply-ipc.md)'s one-shot `Reply` capability. This is the same
  idea on the terminal's badge axis.

## What this tree already does in the analogous case

§106's narrowing is shipped and encoded. `crates/grant_plan/src/spawnproto.rs` carries
`SINK_BIT` (word 2, bit 33) and §106's own `SCREEN_BIT` (bit 37), and `Wiring::screen` records the
completion-signal fact they set.

The tree has also already written down what the color half wants to say.
`crates/documentation/src/render.rs` carries `Style.color` and the reason it is not sniffed:
"this is a capability question wearing a boolean's clothes. `doc` writes to whatever endpoint it
was handed and has no way to ask what is behind it... the *shell* knows whether it wired this
program to a terminal or to a pipe, and passes the answer in."

On the input half, the terminal protocol already has the narrowed-capability shape. A capability
carrying `RAW_ONLY_BADGE` is served `OPERATION_RAWMODE` and `OPERATION_READRAW` and refused
everything else: no `OPERATION_BYTES`, so its holder cannot type a line somebody else reads, and no
`OPERATION_READLINE`, so it cannot take one. The full terminal endpoint's read side is the
keyboard, which is why `mdr` renders and cannot page: "a keypress needs the terminal endpoint,
whose read side is the keyboard, and the spawn protocol cannot hand over one line of input without
it" (`components/src/mdr.rs`).

## The prior art, and the shapes the ruling declines

The screen fact's prior art splits into two families. The first queries the object: Unix's
`isatty(1)`, Windows' `GetFileType` and `GetConsoleMode`, Fuchsia's fd-is-a-tty query (recalled).
That is a metadata opcode on the terminal capability, declined here. It is a new opcode on wire
two programs agree on, the narrowed endpoint §106 built was not meant to be interrogated, and the
renderer's own words say the answer is passed, not asked. The second family is the spawner passing
the fact: `TERM`, `NO_COLOR`, `CLICOLOR_FORCE`, Windows Terminal's `WT_SESSION`, GNU's
`--color=auto|always|never` (all recalled). Those are start-time data, exactly like the option
that lost. The attested form carries one truth on two channels: the bit can say screen while the
wiring delivers a file, and color escapes land in the file. The ruling keeps one channel.

The input half's declined shapes:

- *Lease the keyboard*: the pager holds the read side for the duration. It is Unix's controlling
  terminal plus `tcsetpgrp` (recalled), and it fails the premise: a pager that holds the keyboard
  can take the next one.
- *Push*: the terminal sends each line to an endpoint the pager owns. It inverts direction, and
  the terminal becomes a client holding state about foreign endpoints.
- *A bounded-count badge*, `READLINE` × N. It buys what N one-shot badges compose, with a counter
  the terminal must keep.
- *Poll with narrow rights*: a standing read right is the keyboard minus write. That is why
  `RAW_ONLY` holders are refused `READLINE` today.

## The options

For the screen fact:

| | shape | what it costs |
|---|---|---|
| **A** | A word at start: the spawn path already passes the child a word in its second argument register, so a second word or a bit in it says "you end at a screen". | Declined. One truth on two channels: the bit and the wiring can disagree, and nothing a reader of `caps` can see. |
| **B** | Presence of a capability: a stage ends at a screen exactly when it holds a `WRITE`-only terminal capability, the grant §106 currently routes through the caretaker instead. Chosen. | The stage holds a piece of the terminal again, which is what §106's narrowing moved away from; the rights are narrow enough (`WRITE`, no read side) that the keyboard never travels, and presence is the connection. |
| **C** | Refuse: no stage learns anything, and no program colors its output. | Today's state. The guess this leaves is `isatty`'s, and a guess becomes a convention before anybody decides it is one. |

For one line of input:

| | shape | what it costs |
|---|---|---|
| **D** | A one-shot badge on the terminal protocol: a new badge class the terminal serves `OPERATION_READLINE` on exactly once, then refuses. The shell mints one per page; the pager holds one at a time and can never hold the keyboard. Chosen. | New badge semantics on wire two programs agree on, and the terminal tracks one-shot badges. No new opcode. |
| **E** | Composition: the pager asks an endpoint the shell endows per line, and the shell forwards each line from its own terminal holding. | Declined: less work than D and still the wrong shape, because the shell stands in the loop for every line a pager ever shows. The whole point of a capability is that the holder acts without a broker waking up. |
| **F** | Refuse: `mdr` renders and cannot page. | Today's state, and the sixth program that wants a screen meets the gap with a habit instead of a protocol. |

## Whether the premise is true

Checked 2026-10-15. `Wiring::screen` and `SCREEN_BIT` exist, and they carry §106's completion
signal for a narrowed tail stage, not a fact the stage itself can read; the encoding is between
shell and progenitor. `components/src/mdr.rs` still says "plain text only, until the shell can say
a stage ends at the terminal", and `crates/documentation/src/render.rs` still takes color as a
passed-in boolean because there is nothing to derive it from. No program in the tree colors its
output, and no pager exists. Both halves of the block's premise held, and the ruling answers them.

## How reversible it is

Both halves are wire. A badge class, a rights answer and a derivation rule are cheap to write and
expensive to change once a program is written against them, which is the same reason §106 was
calef's and the same slice of caution. Two programs already want it, `doc` and `mdr`, so nothing
here waits on a hypothetical consumer.

## What this unblocks

Milestone 334. Behind it: every program that wants color, starting with the documentation
service's own renderer, and the pager that `mdr`'s module doc has promised nobody. Both halves
land as one milestone's work because one consumer, the paging `mdr`, needs both at once.
