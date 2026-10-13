---
status: NOT-STARTED
raised: 2026-09-21
promoted_from: no-gate-records-when-it-fires
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 531. No gate in this tree records when it fires, so its own retirement rule cannot be applied

Promoted from the proposal `no-gate-records-when-it-fires`, filed 2026-09-21, on calef's instruction
of 2026-09-20 to give every proposal on `main` a number. The text below was the proposal's own at
promotion, unedited except for this paragraph: the argument is its author's and promotion is not the
moment to improve it. Raised by a research lane calef briefed to ask what gate practices this tree
lacks, with the instruction that any gate must prove its worth, existing ones included. The lane
read `script/lint`'s 47 named checks, the thirteen workflows, `notes/check-inventory.md`,
`design/fatal-risks/README.md` and the failure records in `notes/`, and measured the git history for
evidence that each gate has caught a real defect. The 47 is derived rather than copied (`grep -c
'^echo "==>' script/lint`), because the number written into `notes/check-inventory.md` has already
gone stale once and that file says to derive it.

Two of the three additions below change what every contributor runs, and the
third is a watcher rather than a gate. None should be built before calef rules on the first, because
the first is what would decide whether the others are worth keeping a year from now.

## The short version

**Nothing in this tree should be deleted on the evidence available, and that is the finding rather
than a reprieve.** This tree retires a check on a stated criterion, "a check that only ever rejects
valid work is measuring the wrong thing", which §61 (a lint is adopted on evidence from this tree,
not on its description) supplies the adoption half of and nothing supplies the retirement half of.
Applying it needs to know what a check has rejected. **No gate here records a single firing.** Three
checks have been retired under that criterion and each time the evidence was somebody's memory of
having been rejected, reconstructed after the fact.

A partial record does exist. GitHub Actions keeps a failed run per commit, and mining it gives a
per-check firing count for the whole CI history. That baseline shows `script/icount --check` fires,
and it expires: its logs begin disappearing on 2026-10-21. What it

cannot see is a check that fired on a lane's own machine and was fixed before anything was pushed,
which is the cheapest firing a gate achieves and, measurably, the commonest.

So the ranked recommendation is one cheap instrument, one convention already proven on proofs and
not yet on gates, and one watcher for hazards outside the gate surface. Two refusals are recorded
with their measurements, because a refusal with a number in it is worth more than an addition
without one.

The evidence and the full assessments are in three appendices, indexed in
[`0531-no-gate-records-when-it-fires/`](0531-no-gate-records-when-it-fires/README.md).

- [`ci-baseline.md`](0531-no-gate-records-when-it-fires/ci-baseline.md): the method, the mined CI
  baseline's tables and the `script/icount --check` record.
- [`additions.md`](0531-no-gate-records-when-it-fires/additions.md): additions A to D assessed in
  full.
- [`limits.md`](0531-no-gate-records-when-it-fires/limits.md): the `script/cadence-check` false
  positives, and the three things no gate can check.

## What was measured

The audit searched the git history for commits fixing a real defect each gate found (this works
because `allow_squash_merge` is false here). It timed `script/lint` at 84 seconds warm, mined the
Actions failure record, and read the live state of the machine. The method and tables are in
[`ci-baseline.md`](0531-no-gate-records-when-it-fires/ci-baseline.md). In brief: 909 failed runs are retained from 2026-07-23 on, against 14,295 in total,
with 1,312 failed jobs. All 309 failed `clippy` logs still resolve which `script/lint` sub-check
fired; nineteen checks account for them, led by roadmap status (78) and markdown links (50).
Thirty-two of the 47 checks have never failed a CI job, and those include some of the best
documented catches in git, because a check that fires locally and is fixed never reaches Actions.
Fourteen checks have no trail in either record. They are ratchets, and a ratchet that holds leaves
no trace by design.

`script/icount --check` fires: its logs show 43 `CHECK FAIL` lines across seven days, on the order
of seven episodes. The other 56 failures of the job named after it are not the tripwire.


## Finding 1: the deletion criterion has no record to read

`script/lint` is one required check carrying 47 sub-checks. It exits on the first failure, so a
slow or wrong one stalls every lane at once. Three have been deleted for the "only ever rejects
legitimate work" signature. The branch-prefix check is the documented case: four false rejections,
each discovered when something broke and fixed by widening an allowlist. Then calef asked what the
taxonomy was for, and the answer was that nothing consumed it.

Every one of those four was learned by being hit, and nothing counted them. The exact-count relations that became a merge hotspot were weakened twice, reactively,
each time after a lane paid. The unsafe-fn gate needed a trait-impl exclusion that was twelve false
positives out of thirteen without it, measured by hand at authoring time and never since.

The benefit side was no better instrumented. The em-dash check fires constantly: roughly twenty-odd commits since
2026-08-01 and 27 CI jobs between 2026-08-05 and 2026-09-15. It has never caught a defect, because
it is a house-style rule and not a correctness property.
Neither half was known without mining 3,000 commits and 408 logs.

### Addition A: a gate-firing ledger

Every gate that renders a verdict appends one line on failure: UTC timestamp, gate name, sub-check
name, branch. The file is local, git-ignored and never read by any gate. `script/lint --fired` (or
its own entry point) reports firings per check over a window and the checks that have never fired.
It costs one append on failure and nothing on success.

It is an instrument, not a gate, and would have caught no recorded failure directly. It would have
spared milestone 191 (did the proofs catch the bugs?) its eighteen-row corpus built by hand. Mining
Actions does not replace it. Actions sees only failures that escaped local gating, so a ranking
built on it rewards the gates that fail late. The logs behind the 2026-07 and 2026-08 firings begin expiring on
2026-10-21. And the mining took 909 run queries and 408 log fetches. So the ledger's first task is
to absorb the baseline before the logs expire.

It counts firings, never value: the em-dash check would rank near the top. A firing count is the
cheap half of §61 (a lint is adopted on evidence from this tree, not on its description)'s question,
and should be labeled that way where it prints. It sees local runs only, the exact complement of
Actions. A check quiet in both is the only one that has genuinely never fired.

## Finding 2: the tree proves that a proof can fail, and does not prove that a gate can

The most frequently recurring failure in this tree's record, by a wide margin, is an absent failure
signal read as a pass. The instances are the same defect in twelve places:

- milestone 214 (a test that prints "skipping" and returns is counted as passed);
- a vacuous Kani harness reporting `SUCCESSFUL`;
- an assertion that answered "U-mode cannot read the kernel" by refusing to look, green through
  every gate since milestone 41 (dead code: triage the suppressions, and un-blindfold the gate);
- confinement tests that hang instead of going red;
- every `uefi_loader` mutant recorded MISSED in "0s build + 0s test";
- `cargo kani -p kernel` selecting `arch/` by the host's `target_arch` and so proving one
  architecture on all three, which two `assert!(false)` probes exposed as "4 successfully verified
  harnesses, 0 failures";
- `script/fmt` silently ignoring `--check` for months;
- a scheduled step piped through `tee` and exiting with tee's status;
- `--shard 4/4` dying as an argument error in twenty seconds every week for a month;
- `script/citations` reading exit 0 on a file `git ls-files` could not see;
- `script/decisions` silently skipping a binary file;
- `script/roadmap`'s merged-branch check spending a day unable to fail.

The last three are told together in `notes/citations/bugs.md`.

This tree already has the answer to that shape and applies it in one direction only. §134 (a
harness carries a machine-replayable falsification record, or it is not evidence) makes a proof
carry a demonstration that it can come back red, and `script/falsifications --check` gates it. The
same discipline has been applied to gates three times, each time by hand:

- `script/fatal-risks --selftest` runs eleven fixtures of which eight must go red and three must
  stay quiet;
- `script/image-permissions` was proved both ways before it was wired up, green on three images and
  red on the pre-fix x86_64 one;
- milestone 233 (`login` dies on every boot, and the boot says it is ready) proved its
  killed-thread assertion able to fail before believing it.

All three are among the newest checks in the tree. The good practice is arriving as an instinct
rather than as a rule.

### Addition B: a gate carries a selftest, or it is not evidence

§134's convention, applied to checks as it is to proofs. Each `script/` entry point that renders a
verdict grows a `--selftest` that runs it against fixtures which must fail and fixtures which must
stay quiet. `script/lint` gains one check refusing an entry point with a verdict and no
`--selftest`. `script/fatal-risks --selftest` is the worked example, at 0.65 s combined with its
`--check`.

It would have caught `script/fmt` ignoring `--check`, the `--shard 4/4` error, the
`script/citations` pattern disagreement, and `script/swish-check` red on `main` unnoticed. It costs
milliseconds per run; the real cost is a third more work to write each gate, which buys the evidence
that it works. A failing selftest means the gate is broken, not the tree, so false positives are
near zero.

It proves a gate *can* fire, never that it fires on everything it claims, and it cannot reach a gate
whose subject is the live machine. It wants a bounded adoption: the gate refuses a new or modified
entry point without a selftest, and the existing surface is a worklist.

## Finding 3: the hazards that destroy work are all outside the gate surface

`AGENTS.md` names three hazards that destroy work rather than delaying it: uncommitted work in a lane
worktree ("the one thing no part of this system protects"), disk ("the only pressure here that
destroys work rather than delaying it"), and the shared stash stack. All three are at rung four
today. Nothing in `script/` or `helpers/` runs `git worktree list`, and nothing anywhere reads
`df` outside one CI resource trace; both were grepped rather than assumed.

Measured on patagonia while writing this, 2026-09-21:

- 35 worktrees. The 2026-07-31 incident that took the volume to zero bytes free was 42.
- The main checkout's `target/` is 30 GB. `AGENTS.md` records it at 7.2 GB, as the one nobody
  watches because it is not a lane. It has quadrupled since, and nothing reported that.
- 134 GiB free of 460. Fine today, and no mechanism would say otherwise on the day it was not.
- Two worktrees carry uncommitted work. `maintainer/metrics-2026-09-19` holds seven modified
  files, its last commit a day old: the 2026-08-04 failure verbatim, found only because this lane
  ran the command.

### Addition C: one watcher over the lane fleet

One script, run from `helpers/trunk-health.sh` like `script/cadence-check`. It reports worktrees
with uncommitted work and no recent commit, free space against a floor, and worktree count with the
total `target/` footprint including the main checkout's. The three readings are one question with
one mechanism, so they are one script.

It is explicitly not a gate. `helpers/lane-claim-check.sh` refused `script/lint` for the same
question, because the lane that most needs telling is mid-work. Run against the live fleet,
its uncommitted-work leg names 2 of 35 worktrees, both true positives. It cannot tell wanted from
unwanted uncommitted work (one of the two hits is regenerated SVG output), a `df` reading is a
snapshot, and like every watcher here it does not report its own death.

### Addition D, smaller: refuse `git stash` in a shared checkout

`AGENTS.md` records the 2026-08-26 case where one session's stash entry was popped into another
lane's tree, and prescribes a patch file instead. That is rung four for a hazard with a rung-two
mechanism available: git's `reference-transaction` hook fires on `refs/stash` and can abort the
transaction. Tested, it refuses the stash and leaves ordinary commits alone, at about 22 ms
per ref transaction. It is ranked last because the recorded incident lost nothing. It cannot reach `git reset --hard` or `git checkout <file>`, whose damage is
indistinguishable from their intent.

## What to weaken, with evidence

`script/cadence-check`'s DEAD verdict is 67% false positive, measured by running it on 2026-09-21.
Of three workflows reported dead, only `undefined-behavior check` is truly dead: it has never had a
successful scheduled run since 2026-08-10. `audit-cadence`'s red is its signal, and `vendor-watch`
moved from monthly to weekly on 2026-09-17, so its gap is a transition. This is the tree's own
"cries wolf" diagnosis applied to the watcher built to cure it. Both fixes are already owed in the
script's header: derive each workflow's expected interval from its cron, and let a workflow declare
that red is its signal, so the tripwire asks "did it run" rather than "did it pass". Nothing else in
this audit should be weakened. The case for each workflow is in
[`limits.md`](0531-no-gate-records-when-it-fires/limits.md#what-to-weaken-with-evidence).

## What this lane refuses to add, and the measurements that refuse it

A gate on `AGENTS.md`'s lane line, which is rung four until milestone 128 (the automation gets its
own identity, and the agents get their own voice) delivers a mechanism. Compliance was measured over 400 pull requests since 2026-08-16: 398 carry it, and the two that
do not are dependabot's, which are not agent-written and correctly should not. A gate would have zero true positives and two
false ones in four hundred, which is the retirement signature acquired at birth.

A gate requiring a gloss on every citation. `script/citations` already refused it on a sweep of
2,911 sites, and the 2026-09-19 census is worse: 505 of 9,483 scheme-number pairs are named, 5.3%.
The ratchet is the right shape.

More checks generally. The tree considered this question on 2026-09-02 after four findings in one
day and wrote the answer down: the answer to four bad checks is not six more. This lane agrees, and
two of its three recommendations are not checks.

## What cannot be gated

The tree has already written this list: a grep for the declarative forms of "nothing checks this"
returns 509 lines across `notes/`, `design/` and `script/`, each where a reader meets the thing it
is about. Three bound this proposal, argued in full in
[`limits.md`](0531-no-gate-records-when-it-fires/limits.md#what-cannot-be-gated):

- A premise being overtaken. A green `script/fatal-risks` is not a warrant that the arguments still
  hold, and the larger half of that question has no mechanism.
- Whether a comment, a gloss or a name is *true*. A lint asks whether a comment is present, never
  whether it is about the thing underneath it.
- Whether a green result means anything. Addition B asks whether a check *can* fail, not whether it
  fails on the thing its name promises. That gap is where the next finding will come from.

## The ranking

1. The ledger (A). It is the only item that changes what anyone can know, it costs nothing on the
   success path, and it lets calef ask this question again in three months and get an answer
   instead of a lane. It alone has a deadline: the Actions logs behind the per-check baseline start
   expiring on 2026-10-21, so absorbing that baseline is worth doing whatever else is taken.
2. The selftest convention (B). The highest-value by defects prevented and the most expensive,
   which is why it goes second and wants the ratchet shape. It is a decision rather than a build:
   the code is already in `script/fatal-risks`.
3. The lane-fleet watcher (C). The only one with a live failure standing in the tree as this was
   written. Third because it protects against loss rather than wrongness, and a watcher nobody
   reads is this tree's most repeated disappointment.
4. The stash refusal (D). Ten lines, verified, and honestly the smallest thing here.

If only one is taken, take the first. The other three are opinions about where the next defect
will come from. The first is the instrument that would tell us whether this proposal was right.

## Index row

Nothing in this tree should be deleted on the evidence available, and that is the finding rather
than a reprieve. This tree retires a check on a stated criterion, "a check that only ever rejects
valid work is measuring the wrong thing",...
