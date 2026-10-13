# Milestone 118 appendix: the plan in three pieces

An appendix to [milestone 118 (CLAUDE.md has a budget)](../0118-constitution-budget.md), moved out
of that block on 2026-10-12 (UTC) under [§212 (a prose
budget)](../../decisions/0212-a-prose-budget-for-every-document.md). The block is complete without
it; this is the plan as written before the work, with the reasoning each piece was built on.

## Split, the way this tree has split three monoliths already

`DECISIONS.md` became `design/decisions/` (milestone 114), the roadmap became `design/roadmap/`, and
`notes/` is indexed. `CLAUDE.md` is the last monolith and the one loaded most often.

A core of the rules that change behavior on *every* task, with linked documents for the rest.
The roles section and the naming section are the obvious first moves at 46% of the file between
them. The test is not a line count but whether an agent will genuinely read the whole core, so the
lane should say how it judged that rather than picking a round number.

## The first cut: delete the eight rules a gate already enforces

[The applicable cut](applicable-cut.md) executed this and found four of the eight unenforced. The reasoning below stands and is why the cut is worth making at all; the
estimate of "on the order of 90 lines" does not, and the verified figure is about 15.

calef, 2026-08-18, asked whether a rule with a mechanism can simply be eliminated. Yes, and the
reasoning does not go with it, because the reasoning is already somewhere better.

The evidence is what the gates' own comments contain. `script/lint`'s rule-1 check carries twelve
lines. They hold the scar that produced it (a raw `SPSel` read in `user/tests.rs`), the three
spellings architecture code has in Rust, and why there is deliberately no allowlist. They also hold
the precedent for what to do instead (`current_sp` and `sync_icache`, where the register read moves
behind an `arch::` helper). `AGENTS.md`'s version of the same rule is three sentences with none of
that. The naming and dead-code checks carry eight and twelve lines respectively.

So this is deleting a duplicate, not an argument, which is the distinction that keeps it inside
this block's own warning that a lane deleting arguments has taken the wrong turn.

And the gate is the better home, not merely an equal one. A rule in `AGENTS.md` is read at session
start by an agent that cannot know which three rules will bite it that hour. A rule in a gate's
comment is read by somebody who has just tripped it. On 2026-08-18 `script/lint` refused a lane's
crate-level `allow(dead_code)`. That lane found the *right* fix (`icount = ["bench"]`, riding
conditions that already existed) because the reasoning of §38 (a suppression is scoped to an item
and carries a reason) was there when the check fired. That is rung three, and it is the shape of
milestone 115 (the names that were ratified, and the ones that were refused) exactly: the record
beside the thing rather than in a registry.

The test before deleting each one, and it is falsifiable. *Does the gate's failure message plus
its comment tell somebody enough to fix it correctly, including the case where the gate is wrong
about the tree?* The counted-claims check passes: it prints "fix the number, or fix the derivation
if the tree is right and the gate is asking the wrong question". The branch-prefix check failed it,
which is why it was widened four times instead of fixed once. Where the constitution holds
something the gate lacks, move it to the gate first; then delete.

No pointer left behind. A "see `script/lint`" line is a tax on every session for a rule that can
no longer be violated silently. The eight are listed in [the audit](audit.md); the paragraphs are
on the order of 90 lines, about a tenth of the file, and they are the tenth that needs the least
reading.

What this does not touch. The four budget rules have no mechanism, so deleting their prose deletes
the rule. They move *up* the ladder (next section) rather than out of the file. And the judgment
rules stay prose, because a gate over "push back when he is wrong" would be a gate about taste.

## The most-violated rules stop being prose

This is the ladder turned on itself. "Do not `pkill` QEMU" and "do not `reset --hard` to take a
measurement" are rung four, prose relying on memory, and they failed five times in one day. Both
have a higher rung available:

- a wrapper that finds and kills only the caller's own emulator, so the dangerous form is never the
  convenient one;
- `git show <sha>:<path>` as the read-only way to look at another revision, which is what every one
  of those four agents actually wanted.

A rule that is violated repeatedly is not stated too quietly. It is on the wrong rung.

## A budget, so this does not have to be done again

A one-time cut re-grows; this file added 336 lines in a day without anyone deciding to. So the lane
adds a gate on the core's size to `script/lint`. Crude, and that is the point: it converts "should
I add this rule?" into "what does this replace?", which is the question nobody asks unaided.

Pair it with the signal that actually matters, which is not size. A rule nobody breaks is cheap at
any length; a rule that gets broken is either mis-stated or on the wrong rung. So keep a short
ledger of times a documented rule was violated anyway, with the rule named. Three strikes and it
must move up the ladder or be deleted as unenforceable.

The evidence for that ledger already exists in lane reports, honestly self-declared: *"I killed one
of dev-97's QEMU processes by mistake"*, *"I clobbered my own working tree"*. Those reports are the
input; nothing currently reads them.
