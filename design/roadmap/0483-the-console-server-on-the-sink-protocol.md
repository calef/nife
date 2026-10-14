---
status: REFUSED
raised: 2026-09-20
refused_by: 50, 448
---
# 483. The console server on the sink protocol

Refused by milestone 50 (design/roadmap/0050-pipes-and-redirection.md), which records it in its
Follow-on. Milestone 448 (design/roadmap/0448-a-refusal-gets-a-number.md) gives a refusal that names
work a number, a status and a condition that would change it.

## The refusal, in its own words

From '50. Pipes and redirection: one sink protocol, and `|` turns out to be an endpoint', under `##
Follow-on`:

> Converting the console server to the sink protocol. `line_editor` is its only client and now
> speaks for two writers, so once the terminal adapter existed the page-plus-ack channel looked like
> the right answer rather than a gap. A second client of the console would hit the same one-page
> wall one layer down with nothing gained. `notes/sink-protocol.md` has the reasoning.
>
> -- design/roadmap/0050-pipes-and-redirection.md

## Why it is here rather than only there

`line_editor` is the console server's only client and now speaks for two writers, so once the
terminal adapter existed the page-plus-ack channel looked like the right answer rather than a gap.
Converting the console server to the sink protocol would move a second client's problem one layer
down without solving it: it would hit the same one-page wall with nothing gained.

## Revisit

- **Condition.** A second client of the console server. The refusal's reasoning is entirely about
  there being one, and `notes/sink-protocol.md` carries the argument, so a second client is both what
  would reopen this and what would decide it.

## Index row

Two protocols for getting bytes out of a program, and a deliberate decision not to unify them while
one of them has a single client. The condition is a second client, which the refusal names as the
thing that would change both the answer and the reasoning.
