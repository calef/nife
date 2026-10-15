---
status: PROPOSED
raised: 2026-10-15
---

# 302. The screen fact and one line of input: the spawn protocol's other two thirds

Raised 2026-10-15 by the lane for milestone 652 (twelve `DECISION` gates cite only sections that
are already ruled). Milestone 334 (color and the pager: the spawn protocol's other two thirds) has
waited on this fork since
[milestone 40 (documentation as a system service)](../roadmap/0040-documentation-service.md)'s block named
it: [§106 (the terminal_sink_caretaker narrowing)](0106-tail-output-narrowing.md) took the spawn protocol's narrowing for a tail stage's *primary output* and did not extend
it to telling a stage that it ends at a real screen, or to granting one line of input without
granting the keyboard. Both widen a protocol two programs agree on, so both are an architect's.
*(Section number provisional until the merge queue lands it.)*

## What is being decided

Two facts, each its own choice, joined by one mechanism:

1. How a stage learns that its output ends at a real screen, which is the honest replacement for
   `isatty` and the thing color needs.
2. How a program reads one line of input while holding nothing that could take the next one, which
   is what a pager needs and what `mdr` lacks today.

## What this tree already does in the analogous case

§106's narrowing is shipped and encoded. `crates/grant_plan/src/spawnproto.rs` carries
`SINK_BIT` (word 2, bit 33) and §106's own `SCREEN_BIT` (bit 37), and `Wiring::screen` records the
completion-signal fact they set. One naming caution falls out of this: `SCREEN_BIT` is taken, and
`components/src/graphical_terminal.rs` already refused the name `screen` for that reason. Whatever
this decision adds needs its own word.

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

Prior art, recalled and marked as such: every Unix pager lives inside this exact tension and
resolves it with a controlling terminal and `tcsetpgrp`, the machinery block 48 shows this kernel
does not need. The capability-native version of "one line, no keyboard" has no standard teacher.

## The options

For the screen fact:

| | shape | what it costs |
|---|---|---|
| **A** | A word at start: the spawn path already passes the child a word in its second argument register, so a second word or a bit in it says "you end at a screen". | One more convention in the start path, and nothing a reader of `caps` can see. |
| **B** | Presence of a capability: a stage ends at a screen exactly when it holds a `WRITE`-only terminal capability, the grant §106 currently routes through the caretaker instead. Color means holding it, and `Style.color` derives from the check. | The stage holds a piece of the terminal again, which is what §106's narrowing moved away from; the rights are narrow enough (`WRITE`, no read side) that the keyboard never travels, but the decision to re-widen the stage's holding is the real cost. |
| **C** | Refuse: no stage learns anything, and no program colors its output. | Today's state. The guess this leaves is `isatty`'s, and a guess becomes a convention before anybody decides it is one. |

For one line of input:

| | shape | what it costs |
|---|---|---|
| **D** | A one-shot badge on the terminal protocol: a new badge class the terminal serves `OPERATION_READLINE` on exactly once, then refuses. The shell mints one per page; the pager holds one at a time and can never hold the keyboard. | New badge semantics on wire two programs agree on, and the terminal tracks one-shot badges. No new opcode. |
| **E** | Composition: the pager asks an endpoint the shell endows per line, and the shell forwards each line from its own terminal holding. | No protocol change, and the shell stands in the loop for every line a pager ever shows, which is a permanent context switch and a reason the shell can never be replaced while a pager runs. |
| **F** | Refuse: `mdr` renders and cannot page. | Today's state, and the sixth program that wants a screen meets the gap with a habit instead of a protocol. |

Recommendation: B and D together. B is the capability-native form of the fact the documentation
crate already asked for ("the shell knows and passes the answer in": presence of the narrowed
capability *is* the passing), it needs no new wire word, and it extends §106's slice in the
direction §106 was already going. D is the input half's direct answer, and the badge mechanism is
the tree's own precedent for saying "this capability may do exactly this much" on the terminal.
The effort test, stated because the skill asks: E is less work than D and would still lose, because
it puts the shell inside every page turn forever, and the whole point of a capability is that the
holder can act without a broker waking up.

## Whether the premise is true

Checked 2026-10-15. `Wiring::screen` and `SCREEN_BIT` exist, and they carry §106's completion
signal for a narrowed tail stage, not a fact the stage itself can read; the encoding is between
shell and progenitor. `components/src/mdr.rs` still says "plain text only, until the shell can say
a stage ends at the terminal", and `crates/documentation/src/render.rs` still takes color as a
passed-in boolean because there is nothing to derive it from. No program in the tree colors its
output, and no pager exists. Both halves of the block's premise hold.

## How reversible it is

Both halves are wire. A badge class, a rights answer and a derivation rule are cheap to write and
expensive to change once a program is written against them, which is the same reason §106 was
calef's and the same slice of caution. Nothing here is un-shippable before a second program wants
it; two programs already want it, `doc` and `mdr`.

## What is blocked until this is answered

Milestone 334. Behind it: every program that wants color, starting with the documentation
service's own renderer, and the pager that `mdr`'s module doc has promised nobody.
