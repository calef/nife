---
status: NOT-STARTED
raised: 2026-09-03
milestone_dependencies: none
decision_dependencies: 303
machine_requirements: none
specific_machine: none
needs_person: no
---
# 356. Retention declares the thread capability and says nothing about the region

Filed as a proposal on 2026-09-03 by the
`maintainer/spawn-retention-field` lane, while building DECISIONS §142's answer; promoted by
milestone 433 on 2026-09-19. Premise checked against the tree that day and unchanged:
`crates/supervision_protocol/src/lib.rs` still declares `Retention` with exactly two variants
(`Nothing` and `ThreadControlBlock { reason }`), the region is named only in prose there ("after
`START` the only authority over this child is the region it was built from"), and §142 has been
DECIDED since 2026-09-03 with nothing added about the region. The three spawn paths this file names
as disagreeing still disagree.

The question is decided:
[§303 (the region)](../decisions/0303-the-retention-declaration-says-nothing-about-the-region.md),
2026-10-15. It keeps the declaration the TCB's, and it forbids building a child from a retained
delegated region. The enforcement is named as work for this milestone or its own: present the
owning region beside `build_ut`, and the kernel refuses when `build_ut`'s recorded parent is not
it.

**In brief.** §142 gave `ChildEndowment` a `retention` field, so one struct literal now states both
what a child is given and what its spawner keeps. What it states is the disposal of one capability,
the child's `ThreadControlBlock`, which the tree's own audit showed is inert after `START`: all
three of its methods refuse anything but an `Embryo`. The capability that is *not* inert, and that
actually decides whether a child can be ended, is the region it was built from, and the endowment
says nothing about that one.

## Why this is not tidiness

The declaration's whole claim is that reading a `ChildEndowment` tells you the authority in play in
both directions. It does not, yet, and the half it omits is the powerful half. `MemoryRegion::DESTROY`
on the region a child was built from is the only thing in this system that can end a live thread
(DECISIONS §16's amendment). Whether a spawner keeps that capability is decided today exactly the
way TCB retention was decided before §142: one call site at a time, by whether it happens to call
`cap_delete(region)` afterwards.

The sites disagree, and the disagreement is real rather than accidental. `components/src/spawner.rs`,
`fixtures/src/c_confiner.rs` and `components/src/timetable.rs` drop the region as soon as the child runs, so
they hold nothing that reaches a live instance's memory and say so in comments. `crates/system_initializer`'s
job path keeps its region across the child's life and reclaims it on death. Both are correct for
what they do. Neither is stated anywhere a reader of the endowment can see.

## What §40 already says about it

§40's first recorded caveat was this question in other words: the ownership chain that makes a
reap work "holds only if children are built from the supervisor's own region," and whether building
a child from a *delegated* region is forbidden or merely discouraged "is not yet decided, and it
should be before someone writes a supervisor that does it."
[§303 (the region)](../decisions/0303-the-retention-declaration-says-nothing-about-the-region.md)
answered it in §40 itself, 2026-10-15: forbidden. The rule lives in the docs at the builder and in
the named enforcement check, not in a declared field.

## The fork, so it is not decided by accident

## The fork, decided

The three shapes are decided as the third. The declaration stays the TCB's. The region's rule is
stated in `Retention`'s and `build_child`'s docs, so the endowment no longer reads as a fuller
account than it is. Building a child from a retained delegated region is forbidden, with the
presentation check named in
[§303 (the region)](../decisions/0303-the-retention-declaration-says-nothing-about-the-region.md)
as the work that makes the refusal real. What remains of this milestone is those doc sentences and
that check.

## Index row

DECISIONS §142 gave `ChildEndowment` a `retention` field so one struct literal states both what a
child is given and what its spawner keeps. What it states is the disposal of the child's
`ThreadControlBlock`, which the tree's own audit showed is inert after `START`. The capability that
actually decides whether a child can be ended is the region it was built from, and the endowment
says nothing about it: `components/src/spawner.rs`, `fixtures/src/c_confiner.rs` and
`components/src/timetable.rs` drop the region when the child runs while `crates/system_initializer`
keeps its across the child's life, both correct for what they do and neither stated where a reader
of the endowment can see it. §303 decided it, 2026-10-15: the declaration stays the TCB's, the
region's rule is stated at the builder, building from a retained delegated region is forbidden,
and the enforcement check is named. What remains here is the doc sentences and the check.
