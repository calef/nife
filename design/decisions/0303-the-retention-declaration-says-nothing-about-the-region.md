---
status: PROPOSED
raised: 2026-10-15
---

# 303. The retention declaration says nothing about the region

Raised 2026-10-15 by the lane for milestone 652 (twelve `DECISION` gates cite only sections that
are already ruled). Milestone 356 (retention declares the thread capability and says nothing about
the region) has waited on this fork since it was filed while §142 was being built:
[§142 (what a spawner retains over a
child)](0142-spawn-retention.md) gave `ChildEndowment` a
`retention` field so one struct literal states what a child is given and what its spawner keeps,
and what it states is the disposal of the child's `ThreadControlBlock`, which the tree's own audit
showed is inert after `START`. The capability that actually decides whether a child can be ended is
the region it was built from, and the endowment says nothing about it. *(Section number provisional
until the merge queue lands it.)*

## What is being decided

Two questions, one about a struct and one that §40 has been owed for months:

1. Does the retention declaration widen to say what the spawner keeps of the child's region, or
   does it stay the TCB's disposal and say so?
2. The caveat [§40 (a supervisor's death is its subtree's death)](0040-no-reaper-of-last-resort.md) has been owed for months, in its own
   words: whether building a child from a *delegated* region is
   "forbidden or merely discouraged", which "is not yet decided, and it should be before someone
   writes a supervisor that does it".

## What this tree already does in the analogous case

The call sites have already chosen, each for itself, and they disagree. `components/src/spawner.rs`,
`fixtures/src/c_confiner.rs` and `components/src/timetable.rs` delete the region as soon as the
child runs, so they hold nothing that reaches a live instance's memory, and each says so in a
comment. `crates/system_initializer` keeps its boot regions for the boot's life and never destroys
them. All are correct for what they do, and none of it is stated where a reader of the endowment
can see it.

The declaration's one acting half is `start_child`'s: `Retention::Nothing` deletes the TCB
capability at `START`, and `Retention::ThreadControlBlock` keeps it. The field's own doc records
the trap in shipping declarations nothing enforces: the `reason` string "is not checked and cannot
be", and `Retention::ThreadControlBlock { reason: "" }` compiles. The one site that keeps a TCB
(`components/src/swapper.rs`) still deletes the region beside it.

`MemoryRegion::DESTROY` on the region a child was built from is the only thing in the system that
can end a live thread: §16's amendment arms the kill, and the scheduler converts the thread to a
corpse at its next preemption. That is the authority the declaration is silent about, and
[§16 (object revocation)](0016-object-revocation.md)'s amendment is what arms it.

## The options

| | shape | what it costs |
|---|---|---|
| **A** | Widen `Retention` into an enum over both objects, so one noun states what is kept of the TCB and of the region. | One thing to learn, and two lifetimes forced into one value: the TCB is disposed at `START`, the region at reap, which may be much later. Every spawn site writes the value, since the field has no default. |
| **B** | A second field beside it for the region. | Honest about the lifetimes, but it declares a fact about a capability the struct does not carry: the region reaches `build_child` as the `build_ut` argument, so the field and the thing it describes can disagree with nothing to catch it. |
| **C** | Leave the declaration the TCB's, and state the region's rule where the builder meets it: in `Retention`'s and `build_child`'s own docs, with §40's caveat answered in §40 itself. | The endowment still reads as a fuller account than it is, unless the docs say the omission out loud, which is the work. Nothing structural changes. |

Recommendation: C, with §40's caveat answered as *discouraged, and said so at the builder*. The
reason is enforcement, not effort, and the tree has already run the experiment: `Retention`'s
reason string is a declaration nothing can check, and its own doc records that it cannot be. The
region's disposal is the builder's bookkeeping across the child's whole life; no single moment
exists at which a field's claim about it could be acted on, so the field would be one more
unchecked declaration. The doc sentence, and the answer in §40, are checkable by the reader they
protect, which is rung three of the ladder rather than a field pretending to be rung one.

The effort test, stated because the skill asks: A's single noun would be worth more than C's two
sentences if the field could be enforced at a moment. It cannot, so the honest account is the doc.

## Whether the premise is true

Checked 2026-10-15. `crates/supervision_protocol/src/lib.rs` still declares `Retention` with
exactly two variants, the region is named only in the `Nothing` variant's doc, and the three drop
sites still drop while the boot keeps. §142 has been `DECIDED` since 2026-09-03 with nothing added
about the region, and §40's caveat is still open. Every load-bearing fact in milestone 356's block
still holds.

## How reversible it is

C is two documents. If a second consumer ever needs the declaration to carry the region, widening
then is additive: a new variant or field, and the call sites that already say what they do write
it. Starting from A and retracting would churn every spawn site for a declaration nothing reads,
which is the direction to avoid.

## What is blocked until this is answered

Milestone 356 itself, and the caveat §40 has carried since it was written: a supervisor that builds
a child from a delegated region breaks the subtree-death guarantee today, and the tree's rule
against it is nowhere.
