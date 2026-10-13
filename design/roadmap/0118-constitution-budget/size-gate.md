# Milestone 118 appendix: the size gate and every move of its ceiling

An appendix to [milestone 118 (CLAUDE.md has a budget)](../0118-constitution-budget.md), moved out
of that block on 2026-10-12 (UTC) under [§212 (a prose
budget)](../../decisions/0212-a-prose-budget-for-every-document.md). The block is complete without
it. This is the gate's design and the ledger of its ceiling, from 2026-08-22 until calef dropped it
on 2026-10-06; September's entries are in [size-gate-september.md](size-gate-september.md). The
entries were reordered oldest first when they moved here; before that they were appended in no fixed
order.

## The gate, 2026-08-22

`script/lint`'s existing `count-at-most` relation (built for the unsafe-density ceiling of milestone
134 (the register of measures)) is exactly the mechanism this block had asked for without naming it.
It is a claimed number that fires when the tree exceeds it and stays silent when the tree falls
below. `script/lint` gained an `agents-md-lines` registry entry (the file's own `wc -l`-equivalent
line count), and the claim lived in this block. See `notes/counted-claims.md` for the mechanism, and
`notes/rule-violations.md` for the same pattern applied to a different number.

It was set at the tree's exact value with zero headroom. The point, per this block's own words, is
that every line added should replace one removed, or be a considered act that says why the growth
was worth it. That is the same choice `unsafe-thread-safety-claims` made for a different reason (a
population small and consequential enough that every addition deserves the stop). Here the reason
is this milestone's own diagnosis: the file grows in whole deliberate steps rather than by diffuse
creep. So a lane adding one of those steps is exactly the lane that should also write the sentence
justifying it.

It targeted the whole file, not "the core", because the core did not exist yet. Once the split
happened, the ceiling was to move to whatever the core became and stop counting the linked
documents. That was a known adjustment for whoever did the split, not a defect.

Raising it is the correct response to a deliberate addition, and the mechanism is unchanged from
every other ceiling in the registry. Raise the number in the same commit that grows the file, and
say beside it why the growth was worth it. A developer lane cannot make that edit either, so raising
the ceiling is the integrator's or calef's act, exactly like applying the proposed cut already is.

## August

### 942 to 947, 2026-08-23

Milestone 160 (review the public function names across the kernel's dependency crates) extended the
naming tenet. Three lines record that calef's crate/program/module naming authority now also covers
public function names, plus a blank line and citation. That is exactly the considered act this
block asks for: a durable rule change belongs in the file the rule lives in, not only in the
milestone that decided it.

### 947 to 957, 2026-08-26

A real, previously unrecorded footgun was found live. The `git stash` of the two-core-crash lane of
milestone 161 (the x86_64 kernel port) collided with another session's stash, since the stash stack
is per-`.git` rather than per-worktree. It was recorded beside the existing
squash-against-`origin/main` trap it shares a family with, so the next lane to reach for `git stash`
in a worktree finds the warning before it needs it.

### 957 to 965, 2026-08-26

"A lane continues until it needs a human or it is done" (calef, launching the second-round lanes of
milestone 47 (navigation and naming) and milestone 49 (users, login, and attribution) the same day)
moved out of the briefs. It had been a per-brief instruction, typed into every lane's prompt by
hand. It is now the Developer role's own standing default, beside its sibling rule ("a developer
polls its own background work to completion"). The same shape of fix as the entry above: a rule a
maintainer had to restate every time is now one every brief inherits.

### 965 to 975, 2026-08-26

The watcher-start instruction was rewritten after it failed exactly the way it was written to
prevent: a maintainer session read it and did not act on it. The addition records what replaced it
(`launchd` on patagonia, rung one for the part of the gap a session can close). It also records
the gap calef accepted rather than solved (asleep or shut down, neither watcher runs). It is the
same "durable rule change belongs in the file the rule lives in" case as the entries above.

### 975 to 990, 2026-08-26

calef asked, mid-session, whether clearing a stalled PR's conflicts needed new machinery or a human
pointing at it each time. The honest answer surfaced that `notify()` posts once and goes quiet, so
nothing re-announces a stall to a session that opens later. The addition records the standing check
this became: a maintainer session reads the queue for `DIRTY`/`FAILURE` each pass. It also records
the alternative considered and declined, an unattended scheduled agent, which calef turned down in
favor of work that shuts down with the session driving it.

### 990 to 1012, 2026-08-30

The addition is principle 1. The customer path it had named since 2026-08-05 ended that day, when
calef reported the family's backups running on borg over SSH on cordoba. They were built with the
existing Linux ecosystem while nife was not ready. A ranking function that still named milestone 55
(Time Machine) would have been ordering work by a workload nobody runs, the one failure the
principle exists to prevent. So the correction is not optional and it does not compress. It has to
say what happened, and that the principle was confirmed rather than refuted. It has to say what
replaces the ordering while the path is empty, and give the sizing lesson: the first customer was
among the largest things a home system can be asked to be. Principle 2's figures were refreshed in
the same commit and cost two lines net, since every number in them had drifted since 2026-08-05.

Twenty-two of the twenty-seven lines proposed were kept and five were cut before raising this,
which is the order the rule intends. The stale figures went to git, two bullets saying one
thing became one, and a paragraph lost a clause. A ceiling raised without that pass is a ceiling
that is not doing anything.

### 1012 to 1019, 2026-08-30

Seven lines, and the rule's own test is why they were worth it. calef: *"I don't think we expose
nife to third parties (aka other customers) until we have a package manager and a trivial install
process."* That is a precondition on principle 1's ranking function, not an item ranked by it. A
roadmap that ranks by the shortest path to a customer while being unable to accept one is ranking
against a door it has not built. The seven lines say that, say it is ours rather than the
customer's constraint, and record that he wants package management early for the builders' sake
and not only for a stranger's. Nothing was cut this time: the bullet it extends is the shortest in
the section and had nothing spare in it.

### 1019 to 1032, 2026-08-31

Thirteen lines, and a cost the file did not know it had. The lane-count section bounds concurrency
by *collision surface*, which is correct and incomplete: it says nothing about memory. So a
maintainer obeying it exactly ran two lanes whose gates were eight parallel SAT solvers, against a
budget `script/verify`'s own header tunes at four. That cost an out-of-memory kill which took the
session with it, a `ci-build` invalidated by 2.7x oversubscription, and two proof runs killed by
SIGTERM. All of it happened in one day and was read as three unrelated problems. The addition is
thirteen lines because the failure is easy to misread. It never arrives as "out of memory"; it
arrives as a cancellation or a timing failure, and naming that tell is most of what the lines buy.

## September

September's raises and lowerings, from 1032 to 573, are in
[size-gate-september.md](size-gate-september.md): the 2026-09-05 lowering by milestone 262 (the
naming rule is written twice), the disk ceiling, the first extraction to `briefs/`, and the split
that made it a ratchet.

## October: dropped, 2026-10-06

calef dropped the ceiling: "drop 118's limit and rely on §212's cap". One budget in one unit,
because words track context cost and lines track wrapping. `RATCHET_CEILINGS` in `script/lint` has
been empty since; the mechanism stays for the next ceiling that must sit at the tree's exact value.
