---
status: BUILT
raised: 2026-09-21
built: 2026-09-21
---
# 517. What fraction of survivor growth arrives on lines a pull request touched

Built 2026-09-21 (all dates UTC; the measurements ran through the evening of 2026-09-20 UTC and the
block landed just after midnight). Minted provisionally by the maintainer after calef asked for the
number milestone 438 (would a diff-scoped mutation check have caught the 55) named as his to ask
rather than a lane's to answer.

The answer is 81.6%, and the shape of the other 18.4% is the part that decides anything. Of the
771 survivors standing in the 2026-09-14 census, 629 sit on a line a pull request in the window
rewrote or added, and a diff-scoped check running on that pull request would have reported them.
The remaining 142 sit on lines older than the window, and every one of them was checked against
a mutation run of the tree as it stood on 2026-08-03: 141 were already survivors then, and one was
not.

So the inflow hypothesis holds on the measurement: survivors arrive with new code, not by old code
quietly losing its tests. One counter-example exists and is named below, which is what keeps this
a measurement rather than a slogan.

## Nothing here is switched on

No gate, no workflow, no `script/ci-build` row, no ruleset entry, no edit to
`design/fatal-risks/README.md`, `.cargo/mutants.toml` or `notes/mutation-testing.md`. This block is three
numbers and a proposed wording. Milestone 479 (a blocking `--in-diff` mutation gate) carries the
standing refusal and the condition that would change it; the last section is written for whoever
answers that condition, and this lane does not answer it.

## The method, because the method is what the number rests on

Milestone 438's own `BUGS` said the experiment needed pull requests its clone did not contain, and
that this tree could not replay them. That is no longer true for two reasons, and neither of them
is a deeper fetch.

The main checkout has the whole history. 4,957 commits back to 2026-07-12, so every pull request
in the window is present and `git archive` reaches any of them.

And the census does not have to be re-run, because it kept its receipts. The weekly workflow
uploads one artifact per shard, each with `missed.txt` naming every survivor by
`path:line:col: description`. The runs of
[2026-09-14](https://github.com/nifeos/nife/actions/runs/34833498873) and
[2026-09-19](https://github.com/nifeos/nife/actions/runs/35421192143) are both still
downloadable. `design/fatal-risks/README.md`'s risk 3 says the 2026-09-14 census's per-crate numbers
"were never written into the tree", and that is true of the tree; the artifacts carry something
stronger than per-crate numbers, one line per survivor, and have been sitting there the whole time. The 2026-09-14 run's 8 shards hold exactly 771 lines, the
census's own number, at head `e25f519a`.

So the measurement is two steps and no census:

1. Blame every survivor line at the census commit. A survivor whose line's last touch is inside
   the window is a line some pull request in the window wrote; one whose last touch predates
   2026-08-03 is a line no in-window diff contained, so no diff-scoped check running in the window
   could ever have seen it.
2. Replay `cargo mutants --in-diff` against the pull requests blame names, to find out whether
   the proxy tells the truth. It is a proxy in a specific direction: blame reports the *last* touch,
   so a rename or a lint fix that rewrites a line re-blames a survivor that is much older than the
   commit it now points at, and the gate at that commit would not have reported it.

## The fraction

| | survivors | share |
|---|---|---|
| census, 2026-09-14, all shards | 771 | |
| on a line last touched in the window (after 2026-08-03) | 629 | 81.6% |
| on a line older than the window | 142 | 18.4% |

The 629 are concentrated, which is the finding under the finding. They are blamed to 80
commits in 62 pull requests, out of 761 pull requests merged in the window. The median
such pull request carries 6 survivors; the largest carries 79.

| pull request | what landed | survivors |
|---|---|---|
| [#207](https://github.com/nifeos/nife/pull/207) | the mDNS/DNS-SD wire format | 79 |
| [#545](https://github.com/nifeos/nife/pull/545) | milestone 142 (a text display good enough that people use it instead of a GUI), with UTF-8 decoding and scrollback | 66 |
| [#326](https://github.com/nifeos/nife/pull/326) | milestone 129 (scheduled execution: a cron whose every entry is a grant), the `timetable` crate | 48 |
| [#130](https://github.com/nifeos/nife/pull/130) | milestone 40 (documentation as a system service), renderer and all | 43 |
| [#451](https://github.com/nifeos/nife/pull/451) | milestone 161 (the x86_64 kernel port), ACPI parsing included | 42 |

A whole new crate landing is what this instrument sees, which is the same thing
`design/fatal-risks/README.md`'s risk 3 already says from the corpus side when it reports that the
like-for-like gap is "exactly the 26 crates that did not exist at baseline". The two records agree,
measured from opposite ends.

### The bound on 81.6%, stated as a range

It is 68% to 85% across the plausible boundaries, and 81.6% at the right one. The baseline run
is dated 2026-08-03 with no hour recorded, so the cut is a day rather than an instant.

## Does the proxy tell the truth: six replays, 197 of 197

Each replay ran `cargo mutants --in-diff` against one commit blame names, and asked whether the
survivors blame attributes to that commit are among those it reports. Every attributed survivor was
reported, including in two lint-only commits picked because they should have broken the proxy. One
lesson travels: mutants have to be matched on the function and the mutation, not on the path,
because the tree renames.

- [replays.md](0517-survivor-growth-on-the-lines-a-diff-touched/replays.md) holds the six replays,
  the one that nearly read as a miss, and why a replay reports more than blame attributes.

## The other 18.4%: pre-existing, with exactly one exception

The 142 survivors on old lines are the population an inflow gate is structurally blind to. Whether
that matters depends on whether they are old survivors or regressions, code that was once tested
and quietly stopped being.
 Each was checked against a mutation run of the tree as it stood when the
window opened, and every one but one was already a survivor then.

The one regression is `compositor`'s `replace * with + in Rect::area`, a survivor at the census on
a line nobody in the window touched. It is exactly the case a diff-scoped inflow gate cannot see, it
is real, and out of 771 survivors it is one.

- [bounds.md](0517-survivor-growth-on-the-lines-a-diff-touched/bounds.md) holds the table of cut
  dates and why the share moves with it, and the old-line check: the tree it ran against, its
  table, and the three renames that read as regressions.

## What it would take to put `kernel/**` and `components/**` in the corpus

This is milestone 438's third objection and the one that decides whether an inflow gate can carry
risk 3 at all: a kernel change passes by construction today, because `.cargo/mutants.toml` excludes
both trees. It is mechanically possible, and the price is between two and three orders of
magnitude, about 500 hours of runner time per census across three architectures.

- [kernel-pricing.md](0517-survivor-growth-on-the-lines-a-diff-touched/kernel-pricing.md) holds the
  mutant counts, the per-mutant cost and where it comes from, the table per tree, and what makes
  the price worse or better.

So the honest verdict has two halves. A kernel *census* at the current cadence is not feasible
and should not be attempted: 500 hours a week to measure a suite is a worse use of the machine than
anything it would find. A kernel diff-scoped check is feasible, costs minutes on a kernel pull
request, and is the only version of "put the kernel in the corpus" that this project can pay for.
Nobody has priced the scaffolding half, which is the piece that would need a lane.

## What the green condition should say

A proposal for calef, not an edit. `design/fatal-risks/README.md` is his and this lane does not touch
it. The condition below is offered against the ruling of 2026-09-20 that the green condition should
be inflow, with the corpus rate as a lagging indicator.

The measurement supports the inflow framing, and it says how strongly. Over six weeks the tree
gained 629 survivors on lines pull requests wrote and one on a line nobody touched. Decay of
old code is real and it is running about three orders of magnitude behind inflow, so inflow is not
merely most of the problem, it is nearly all of it, and a condition written about it is a condition
written about the thing that is happening.

That one is still why the condition needs a second clause. An inflow-only condition cannot see
`Rect::area` at all, and the failure it would miss is silent by construction: nobody is editing the
code, so nothing prompts anyone to look. A trailing clause costs nothing, because the census already
runs weekly, and it is what stops this entry repeating the defect risk 3's own text names in its
predecessor, "a condition written for one quantity being applied to another".

Proposed wording, two clauses, and the second is what makes the first safe:

> Green when both hold. (a) Inflow: the survivors a merged pull request adds on its own
> lines, measured by `cargo mutants --in-diff` on the merged diff, are zero or triaged into a test,
> an exclusion with a reason, or a recorded gap, under milestone 85's rule, for every pull request
> since the last census. (b) Trailing: the like-for-like census rate has not fallen between the
> two most recent censuses. Amber if (a) holds and (b) does not, because that is coverage decaying
> on code nobody is editing, which is a different defect and wants a different repair.

Why (a) rather than "the gate is on". A condition that requires a blocking gate makes the fatal
risk hostage to a decision milestone 479 refused on evidence that has not changed. The measurement
is available without the gate: the replay costs four seconds on a documentation change and under two
minutes on the largest crate landing in six weeks, and it can run after the merge, weekly, over the
window since the last census, which is exactly what this lane did for six weeks in an afternoon.

Why (b) is a floor and not a target. A percentage target can be met by excluding awkward crates,
which is the argument milestone 326 (nobody has been assigned to turn a mutation score upward) makes;
"has not fallen" cannot be, because the exclusions change both sides of the comparison.

What this costs if it is adopted, priced rather than asserted. An inflow condition measured over
the window would have named 62 pull requests of 761 (8.1%) as carrying untriaged survivors, with
a median of 6 each. That is a real worklist and it is the honest shape of the claim: "new code does
not arrive less tested" is a promise about roughly one pull request in twelve.

And it does not carry the kernel, which is the caveat that has to travel with it. The kernel pricing above
prices that; until the scaffolding half is built, any inflow condition is a claim about the host
crates only, and risk 3's text should say so where the reader meets the verdict rather than in a
follow-on.

## What could not be replayed, and what was not attempted

- Every pull request in the window can now be replayed; 6 were. The limit is wall clock, not
  history. The 6 cover 197 of the 629 attributed survivors.
- The baseline's own survivor list does not exist. `.cargo/mutants-baseline.txt` is per-crate
  counts, and the 2026-08-03 run's output was never kept, which is why the old-line check above re-ran the tree at
  `c87c1575` instead of comparing against it. Those counts are also pre-triage, as the file's own
  header says, so the re-run finds fewer survivors than the file records and the re-run is the better
  reference for "what was already a survivor when the window opened".
- The census was not re-run. Every number here comes from the 2026-09-14 artifacts, from blame,
  or from a scoped replay.

## Follow-on

- **Milestone 479.** milestone 479 (a blocking `--in-diff` mutation gate), `design/roadmap/0479-a-blocking-mutation-gate-on-the-diff.md`,
  whose `## Revisit` asks for "a second measurement, on the experiment measurement 1 could not run".
  This is that measurement. The refusal is still calef's to lift or keep, and this lane does not edit
  that block.
- **Recorded.** It wants a lane, and the maintainer mints the number. The kernel test suite cannot run
  from a cargo-mutants build directory: the scaffolding `xtask/src/suite.rs` builds before the boot
  is what stops `cargo test -p kernel` standing alone, and nothing else in the kernel pricing is
  blocked on judgment. It is worth its own lane whether or not any gate follows, because
  "the kernel suite runs from one command" is a newcomer-facing property.
- **Recorded.** It wants a lane, and the maintainer mints the number. Nothing keeps the census's
  per-survivor list in the tree. The workflow's artifacts expire, and every question in this block
  was answerable only because two runs' artifacts happened to still be downloadable.
  `design/fatal-risks/README.md`'s risk 3 already calls the missing per-crate record "the first thing to
  close".
- **Recorded.** `compositor`'s `replace * with + in Rect::area`, a mutant caught on 2026-08-03 and
  surviving at the census on an untouched line. It belongs in `notes/mutation-testing.md`'s triage,
  which milestone 326's part-3 lane owns; this lane does not edit that file and the maintainer
  should route it.
- **Recorded.** Every name in this block is an existing one. Nothing was named.

## BUGS

- Blame is a proxy and it was checked on 31% of the population. 197 of 629, all confirmed. The
  residual risk is a commit that rewrote a line cosmetically *and* was not sampled; the two sampled
  cosmetic commits both confirmed, so the correction is small, but it is not zero and the fraction
  should be read as "about 82%" rather than to the tenth.
- The cut is a day, not an instant. The baseline run is dated 2026-08-03 with no hour, so any
  survivor blamed to a 2026-08-03 commit is assigned to the window whether or not the run had
  already happened. At most 25 survivors ride on that choice.
- The old-line check matches mutants by function and mutation text, not by path or line. It has
  to, because the tree renames: three of the 142 would otherwise have been scored as regressions on
  a type rename alone. The residual risk runs the other way, that a rename makes a genuine
  regression look pre-existing, and nothing here rules that out beyond reading the four that did not
  match.
- A replay is not a live run, milestone 438's own caveat and it still holds: `--in-diff` against
  a merged diff sees the code as it landed, not as it was proposed.
- The 55-second kernel mutant is derived, not measured here. It is `xtask/src/suite.rs`'s 53
  seconds plus `kernel/build.rs`'s 2.3-second relink, both recorded in the tree by the milestones
  that measured them, on this hardware. A mutation run's rebuild is not always a relink, and a hung
  boot costs the timeout instead, so the real figure is a floor.
- Nothing here measures whether the survivors matter. Milestone 85's rule is triage into a test,
  an exclusion with a reason, or a recorded gap, and a count of survivors is not a count of defects.

## Index row

Milestone 438 ended at the measurement that refused a diff-scoped gate and named the one nobody had:
what share of the mutation corpus's survivor growth arrives on lines a pull request touched. It is
81.6%. Of the 771 survivors in the 2026-09-14 census, 629 sit on lines last touched inside the
window that opened at the 2026-08-03 baseline, blamed to 80 commits in 62 pull requests out of 761
merged; all 142 on older lines were checked against a mutation run of the tree as it stood on
2026-08-03, and 141 were already survivors then. The exception is `compositor`'s
`replace * with + in Rect::area`, caught in August and surviving now on a line nobody edited. The blame proxy was validated by replaying `cargo mutants --in-diff` against six
commits: 197 attributed survivors, 197 reported, including two lint-only commits picked because they
should have broken it. Neither the census nor a deeper clone was needed, because the weekly
workflow's artifacts carry the per-survivor list the tree does not. Putting `kernel/**` and
`components/**` in the corpus is mechanically possible and costs about 500 hours of runner time per
census across three architectures, against 52 minutes today, so a kernel census is refused on price
and a kernel diff-scoped check is the affordable half. The proposed green condition is inflow plus a
trailing clause, because one regression on an untouched line in six weeks is small enough to make
inflow the right primary term and not small enough to leave unwatched.
