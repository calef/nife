---
status: NOT-STARTED
raised: 2026-09-03
milestone_dependencies: none
decision_dependencies: 302
machine_requirements: none
specific_machine: none
needs_person: no
---
# 334. Color and the pager: the spawn protocol's other two thirds

Filed 2026-09-03 as an unnumbered proposal by the milestone 247 sweep,
from milestone 40's block; numbered 2026-09-19 by milestone 433. **Premise re-checked 2026-09-19 and
it holds.** DECISIONS §106 is still the only narrowing that shipped, `components/src/mdr.rs` still
carries "No pager, and the reason is authority rather than effort" in its own module doc, and nothing
in the spawn wiring tells a stage it ends at a real screen. No program in this tree colors its
output.

Both halves widen a protocol two programs agree on, which is the same shape as
[DECISIONS §106 (the terminal_sink_caretaker narrowing)](../decisions/0106-tail-output-narrowing.md)
itself and the same reason §106 was calef's. The fork is decided:
[§302 (the screen fact, and one line of input)](../decisions/0302-the-screen-fact-and-one-line-of-input.md),
2026-10-15. It rules the screen fact is the presence of a `WRITE`-only terminal capability, and one
line of input is a one-shot badge served `OPERATION_READLINE` exactly once. Both commitments sit
on wire two programs agree on, so they cannot be un-shipped once a program is written against
them.

**In brief.** DECISIONS §106 narrowed the spawn protocol for a tail stage's *primary output*, which
is what lets `doc <page>` render at the prompt with no `| wc` in front of it. It built the narrowest
slice that unblocked that one command. The two untaken thirds of the original scope are decided by
[§302 (the screen fact, and one line of input)](../decisions/0302-the-screen-fact-and-one-line-of-input.md).
The screen fact is the presence of a `WRITE`-only terminal capability: the honest replacement for
`isatty`, and what color needs. One line of *input* without the keyboard is a one-shot badge, what
a pager needs. This milestone builds both.

## Why this matters

The first half is what stops this system growing a dishonest `isatty`. Unix programs decide about
color by asking the kernel what their file descriptor is attached to, which is ambient authority
answering a question about presentation. A capability system says it properly, as the presence of
a `WRITE`-only terminal capability ([§302 (the screen fact, and one line of
input)](../decisions/0302-the-screen-fact-and-one-line-of-input.md)). Every program that wants
to color output will need that grant. Until it ships,
the choices are no color anywhere or a program guessing, and a guess here becomes a convention
before anybody decides it is one.

The second half blocks the pager outright. A pager needs one line of input at a time and must not
hold the keyboard, because holding the keyboard is exactly the authority a confined viewer should
not have. There is no way to express that today, so `mdr` renders and cannot page. Milestone 40's
own refusal of `ratatui` said the tree needs its terminal contract first; this is a piece of that
contract.

Left alone, this is a decision nobody meets until they try to write the sixth program that wants a
screen, and by then there will be a habit rather than a protocol.

## Where it came from

Milestone 40's block: *"The other two thirds of §106's spawn-protocol narrowing: a bit telling a
tail stage it ends at a real screen (color, the honest `isatty` replacement), and a way to grant
one line of input without granting the keyboard (the pager). Both widen a protocol two programs
agree on, so both are an architect's call."*

`notes/documentation.md`'s "Where this goes next" is the only other record, and it heads its list with
these two: *"DECISIONS §106 took the narrowing for a tail stage's primary output; it did not extend
the same bit to 'tell this stage it ends at a real screen' ... or to granting one line of input
without granting the keyboard. Both still want the wiring bit this entry originally scoped for all
three."*

## Index row

DECISIONS §106 narrowed the spawn protocol for a tail stage's primary output, which is what lets
`doc <page>` render at the prompt with no `| wc` in front of it. It built the narrowest slice that
unblocked that one command. The two untaken thirds of the original scope are decided, in
[§302 (the screen fact, and one line of input)](../decisions/0302-the-screen-fact-and-one-line-of-input.md).
The screen fact is the presence of a `WRITE`-only terminal capability: the honest
capability-shaped replacement for `isatty`, and what color needs. One line of input without
granting the keyboard is a one-shot badge served `OPERATION_READLINE` exactly once, which is what
a pager needs. The first is what stops this system growing a dishonest `isatty`. Unix decides
about color by asking the kernel what a file descriptor is attached to; a capability system
says it properly as a capability the spawner grants. The second unblocks the pager outright.
Holding the keyboard is exactly the authority a confined viewer should not have, the one-shot
badge expresses the narrower thing, and `mdr` can stop rendering and start paging. Both widen a
protocol two programs agree on, so both are an architect's, and both were ruled by one.
