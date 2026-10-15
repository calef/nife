# §142 appendix: why the order against milestone 133 matters

An appendix to [§142 (what a spawner retains over a child after
`START`)](../0142-spawn-retention.md): the order against milestone 133 (ending a permanently blocked
thread), how to read the mapping, and what is out of scope. Moved out of that page on 2026-10-12
(UTC) under [§212 (a prose budget)](../0212-a-prose-budget-for-every-document.md). The main page is
complete without it; this file exists to verify or challenge it.

### Why this is upstream of milestone 133, and why the order matters

Milestone 133 (ending a permanently blocked thread, and deciding who may) has four proposals in
`notes/blocked-thread-teardown.md`. Proposal B is a terminate verb on the thread capability, seL4's
`suspend()` transliterated, and the note's own account of how B fails ends on this:

> That may be exactly right (it separates policy from memory) or exactly wrong (it produces a stopped
> thread nobody can free), and which one it is depends on a spawn-time endowment convention that does
> not exist yet.
>
> Source: `notes/blocked-thread-teardown.md`, Proposal B

So B cannot be evaluated against a convention nobody has stated. And proposal A would state it by
accident. A needs no new authority at all, which is most of its appeal; taking it settles the
retention question in the direction of "no lifetime handle, the region capability is the whole
answer", after which every spawn path grows around that and B gets harder rather than easier. The
sequencing is the reason this file exists.

## Reading the mapping

Four readings, and they are the useful part.

**A is compatible with every answer, which is why it is not the safe one.** A needs no retention at
all, so it neither requires nor forbids any row. What makes it a decision by accident is the
sentence one line down: taking A without answering this leaves R0 in place by default, and R0 is
the only row that forecloses B. The risk is not that A conflicts with a retention answer; it is that
A makes answering feel unnecessary.

B is the only proposal this decision gates, and it gates it in one direction: B requires
retention to be R2 or R3, and under R1 it becomes a different proposal wearing B's name. That is
the whole of the sequencing problem calef identified.

D is orthogonal, and is worth taking as evidence rather than as an escape. If milestone 133 ends
as `RECORDED`, this decision still wants an answer, because R0 is a convention with consequences and
today it is unwritten rather than chosen.

Nothing here decides milestone 133, and R2 is not a vote for B. R2 says a spawner may keep a
handle; whether a verb on that handle may end a running thread is 133's question and stays there.

## What this decision deliberately does not touch

- Milestone 133 itself. Options and a mapping; no winner.
- Detection. Deciding a child *is* stuck is milestone 106's timed wait and milestone 23's
  residual.
- The stranded caller, which is milestone 254 (a caller stranded by a server that died is stranded
  forever).
- Milestone 244's endowment table. Cited above because R4 shares its shape; the two are the same
  subject from opposite sides, and folding them together would put a proof question and an authority
  question in one lane.
