---
status: DECIDED
raised: 2026-10-15
decided: 2026-10-15
ratified_by: calef
---

# 303. The retention declaration says nothing about the region

Raised 2026-10-15 by the lane for milestone 652 (twelve `DECISION` gates cite only sections that
are already ruled). Milestone 356 (retention declares the thread capability and says nothing about
the region) has waited on this fork since it was filed while §142 was being built.
[§142 (what a spawner retains over a
child)](0142-spawn-retention.md) gave `ChildEndowment` a
`retention` field, so one struct literal states what a child is given and what its spawner keeps.
What it states is the disposal of the child's `ThreadControlBlock`, which the tree's own audit
showed is inert after `START`. The capability that actually decides whether a child can be ended is
the region it was built from. The endowment says nothing about it. Ruled the day it was proposed,
the fourth of five asked one at a time, with the enforcement shape confirmed at the ruling.
*(Section number provisional until the merge queue lands it.)*

## The decision

Three things, ruled together:

1. **The declaration stays the TCB's.** `Retention` keeps exactly two variants. It does not widen
   into an enum over both objects, and no second field is added beside it. What the endowment
   covers is what the spawn moment acts on, and that is the TCB at `START`.
2. **The region's rule is stated where the builder meets it**, in `Retention`'s and
   `build_child`'s docs. The endowment does not declare the region: the region's disposal is the
   builder's bookkeeping across the child's whole life. No moment exists at which a field's claim
   about it could be acted on.
3. **Building a child from a retained, delegated region is forbidden.** [§40 (a supervisor's death
   is its subtree's death)](0040-no-reaper-of-last-resort.md)'s first caveat asked whether that is
   forbidden or merely discouraged; the answer is forbidden. Children are built from budget you
   own: a region you created, a `SPLIT` child of your own region, or a root handed to you as your
   whole budget. The answer lives in §40's caveat, which this ruling closes.

**The enforcement, named.** The rule is prose until the check lands, and the record says so. The
check, when it is built: `build_child` requires the caller to present the owning region's
capability beside `build_ut`, and the kernel refuses when `build_ut`'s recorded parent in the
region table is not the region presented. `split` already writes that parent link
(`kernel/src/memory_region.rs`), so the check is one comparison on data that exists. It fires
before a single page is retyped, which converts the failure from a silent orphan at
supervisor-death time to a loud refusal at build time. Nothing current breaks: every call site
builds from its own budget and can present it, and a genuinely transferred root is presented
directly. The check is sized as its own piece of work; it can ride milestone 356 or stand alone.

## Why forbidden, and why the declaration does not widen

**Forbidden over discouraged**, on a reversibility asymmetry calef named at the ruling. A
permissive rule accretes users, so permitting today makes forbidding later a breaking change.
Forbidding today breaks nobody: nothing in the tree builds a child from a retained delegation. And
it permits later only when a named consumer argues for it, additively, through the endowment this
decision declined to widen. The same logic that keeps §26 death-only keeps this forbidden.

**C over A and B**, on an acting-moment test. A field earns its place when code can act on it at
a moment. The TCB field earns it: `start_child` reads `Retention::Nothing` at `START` and deletes
the capability. Region disposal has no such moment; `spawner` drops the region after `START`,
`system_initializer` keeps it until reap, and the only thing that ever acts is the builder
itself, holding its own capability, at a time of its own choosing. The authority is not hidden
either: the builder holds the region capability or it does not, visible at the call site. What
was hidden is the rule about delegation, and that is this decision's job, not a per-spawn
field's.

- **A** (widen `Retention` over both objects) forces two lifetimes into one value: the TCB is
  disposed at an instant, the region at reap or never. Every spawn site writes the combined
  value, since the field has no default by §142's design, and the variant set goes combinatorial.
- **B** (a second field) describes a capability the struct does not carry: the region reaches
  `build_child` as the `build_ut` argument, so the field and the fact can drift with nothing to
  catch it. The tree already ran this experiment: `Retention`'s `reason` string is a declaration
  nothing can check, its own doc says it cannot be, and `Retention::ThreadControlBlock { reason:
  "" }` compiles. A and B are that again, wearing a type.
- **C** (this ruling) is also the reversible direction: widening later is additive, while
  retracting A or B would churn every spawn site to remove a field nothing could act on.

## What this tree already does in the analogous case

The call sites have already chosen, each for itself, and they disagree. `components/src/spawner.rs`,
`fixtures/src/c_confiner.rs` and `components/src/timetable.rs` delete the region as soon as the
child runs, so they hold nothing that reaches a live instance's memory, and each says so in a
comment. `crates/system_initializer` keeps its boot regions for the boot's life and never destroys
them. All are correct for what they do, and none of it is stated where a reader of the endowment
can see it.

`MemoryRegion::DESTROY` on the region a child was built from is the only thing in the system that
can end a live thread. [§16 (object revocation)](0016-object-revocation.md)'s amendment arms the
kill, and the scheduler converts the thread to a corpse at its next preemption. §16 also records
the ownership shape the enforcement stands on: a region belongs to its owner, not to the thread
that occupies it, and `split` gives each child its own independently reclaimable region.

## Prior art

Read and recalled, per family. **seL4** (mechanics recalled) has no process object and no
declaration anywhere: what a spawner retains is what it has not dropped, and memory authority runs
through the untyped lineage, the same shape as this tree's region. **Fuchsia** (recalled) takes no
retention parameter; what the parent keeps is the handles it does not drop, and the cascade is an
object, the job, with `zx_task_kill` as the one-act reclamation. **Erlang** (recalled) has the one
genuine retention declaration, `spawn` / `spawn_link` / `spawn_monitor`, a per-spawn checked
choice of death coupling, and it covers exactly what §142 covers, the coupling at the spawn
moment, with nothing to say about memory. The art's dividing line is consistent: declare what the
spawn moment acts on.

On delegation itself, the art is permissive because its cascades are structural, not
ownership-based: seL4 hands untyped memory through CNodes freely, and Fuchsia hands VMOs
everywhere, because neither loses a guarantee when it does. This tree's cascade is
ownership-based, which is why the delegation question arises here and not there, and why
[§40 (a supervisor's death is its subtree's death)](0040-no-reaper-of-last-resort.md) flagged it.
SysV shared memory (recalled) is the cautionary tale: segments that outlive their creator until an
external reaper collects them.

## The options

| | shape | what it costs |
|---|---|---|
| **A** | Widen `Retention` into an enum over both objects, so one noun states what is kept of the TCB and of the region. | Declined: two lifetimes in one value, every spawn site taxed, nothing to act on the region half. |
| **B** | A second field beside it for the region. | Declined: a declaration about a capability the struct does not carry, the `reason` string again. |
| **C** | Leave the declaration the TCB's, state the region's rule where the builder meets it, and answer §40's caveat in §40 itself. Chosen, with the caveat answered *forbidden* and the presentation check named as enforcement. | The endowment still reads as a fuller account than it is, unless the docs say the omission out loud, which is the work. Nothing structural changes. |

## Whether the premise is true

Checked 2026-10-15. `crates/supervision_protocol/src/lib.rs` still declares `Retention` with
exactly two variants, the region is named only in the `Nothing` variant's doc, and the three drop
sites still drop while the boot keeps. §142 has been `DECIDED` since 2026-09-03 with nothing added
about the region, and §40's caveat was still open at the ruling. Every load-bearing fact in
milestone 356's block held.

## How reversible it is

The declaration can widen later, additively, if a consumer arrives that needs the endowment to
carry region rights; the forbidden rule can relax the same way, through the same field. The
reverse directions are migrations: retracting A or B churns every spawn site, and forbidding after
permitting breaks whatever was built on the permission. Starting minimal is the reversible side of
both forks.

## What this leaves open

The enforcement check itself: present the owning region beside `build_ut`, compare the recorded
parent, refuse before retype. Named here, sized as its own work, marked as the thing that makes
"forbidden" true rather than stated. Until it lands, the docs carry the rule and say that nothing
enforces it yet.
