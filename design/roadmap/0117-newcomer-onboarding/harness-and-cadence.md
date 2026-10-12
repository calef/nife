# Milestone 117 appendix: the harness and the cadence

*An appendix to [milestone 117 (the stranger test)](../0117-newcomer-onboarding.md), moved out of
that page on 2026-10-12 (UTC) under [§212 (a prose
budget)](../../decisions/0212-a-prose-budget-for-every-document.md). The main page is complete
without it. This is the record of the two lanes that built `script/stranger-test` and its monthly
tripwire, both on 2026-08-18.*

## The recurrence lane, 2026-08-18

Status stayed `PARTIAL`, and the gate was no longer `NONE`. This lane built `script/stranger-test`
and ran nothing through it, on purpose: conducting run 5 in the lane that wrote the instrument would
make the harness's author the harness's only evidence. For the first time, running the test cost
one command rather than an afternoon. A lane that wants to run it cuts nothing by hand, so "a lane
could start today" became true in the sense the token means. What the milestone still owed was a
run conducted through the harness and recorded in notes/stranger-test.md, plus whatever that run
found. No run had used it yet, so the block did not get to count the instrument as the measurement.

### What it holds

The clone goes inside the stranger's working directory rather than being it. That is the isolation
run 3 established, and the only reason the mental-model rows are scorable. The answer key is
withheld the way runs 2 through 4 withheld it, extended to cover markdown links to the note. That
extension exists because `script/lint` fails on a relative link that does not resolve, and a red
gate the harness caused is a worse contamination than the edit. Logs go in a sibling directory with
no run number in any path element. `pkill` and `killall` are shadowed to this clone's QEMU, which
run 4 did by hand and recommended keeping. The isolation is probed before the run, and the run stops
if the probe does not answer NONE. The stranger is told it is being measured, which is run 4's
handoff 5 acted on. The account-wide `nife-dev` link is recorded before the run and restored after,
since a stranger's `script/test` takes it and its tree is disposable in a way a lane's is not.

### Three defects found by testing it

Three defects in the harness were found by testing it rather than by reading it, which is the
argument for `--smoke` being part of the deliverable rather than scaffolding. The first clone leaked
this project's live branch names, including this lane's, to `git branch -a`. The reflog was cleared
before the withholding amend instead of after, so it carried a line reading `commit (amend)` beside
the tip's subject. And the rubric extractor matched every `| M<n> |` row in the note, which handed a
stranger runs 3 and 4's *scores* as part of its own question list. The smoke run's reply quoted them
back, which is how it was caught.

### What it does not do

These are in its own `BUGS` section, where a reader meets them. It cannot make the operator un-read
`AGENTS.md`. It cannot stop the tree leaking that a test exists, and it adds one more leak by being
a `script/` entry point. It does not schedule itself, which is a cadence decision and calef's. And
the withheld note is still one `git show HEAD~1:<path>` away. That last one is measured rather than
closed, since the summary reports whether the transcript ever mentions it.

The name `stranger-test` is provisional, as `script/` names are an architect's. It takes its noun
from the note that named the instrument first, so the command and the record share a word.

## The cadence lane, 2026-08-18

Status stayed `PARTIAL`, and for the first time the reason was not "nothing schedules a run".
calef decided the cadence (monthly) and this lane made it mechanical. It used the shape §74 (audits
run on change, not on the calendar) and `script/audits` already established for security audits,
rather than a second one invented for the same job.

The block had earlier said "a gate would be the thing that moves this, and writing one is not a
lane's call either, because it would be a gate on somebody's attention rather than on the tree."
The first half was right and the second was overtaken. A gate on somebody's attention is exactly
what a cadence is, and deciding how much attention the answer is worth was calef's call rather than
a reason no gate could exist. The sentence stayed true for four days and was answered in one.

### What it is

`script/stranger-test --due` exits 1 when a run is owed, and prints the command to run and what a
run costs. `.github/workflows/stranger-cadence.yml` asks it at 08:00 UTC on Mondays, in its own
workflow rather than in `script/lint`. A run coming due is information about the tree, and not a
defect in whichever commit happened to be pushed that week. `script/stranger-test --check` is the
structural half and is in `script/lint`. It checks that the cadence sentence appears exactly once,
the run headings are numbered from 1 without a gap, and their dates are in order. A malformed record
is a defect in the commit that malformed it. It does not fail loudly; it silently moves the date
the tripwire measures from.

Where the numbers come from, which is the part worth copying. Both live in
notes/stranger-test.md and nowhere else. The interval is the sentence a reader meets
(`**A run is due every 30 days.**`, matched literally). The date is the newest
`### Run <n>, <date>:` heading, which every run already writes because that is how the note records
a run. Nothing is maintained for the tripwire's benefit, so the schedule and the record cannot
disagree. A hand-maintained date in a workflow or a script would have been a fact kept in two
places, which is the failure this tree has watched often enough to have named.

Both cadence modes are handled above every line that clones or spawns anything, which is a
guard rather than tidiness. This script's *default* mode spends money, unlike `script/audits`, whose
every mode is a report. So `--due` needs no `claude`, no clone and no toolchain (the weekly job runs
it in seconds on a bare checkout), and a workflow that loses its flag cannot fall through into a
run.

### Notification, never execution

The limit is stated in the script, the workflow, the note and the block. A run spawns a `claude`
process, spends real budget, needs a machine with the pinned toolchain and both QEMUs, and ends in
a debrief a person scores against the rubric. Nothing in CI can do any of that. Red means run the
test.

### Watched firing and watched staying quiet

A mechanism nobody has seen fire is not a mechanism. Against the real record (run 5, 2026-08-18)
it exits 0 and says "not due for another 30 days". Against a copy of the note with the run headings
shifted back, it exits 1 at 30 and 31 days and 0 at 28 and 29, so the boundary is where the sentence
says it is. Six malformations of the record were fed to `--check`, and each was rejected with its
reason. They were the cadence sentence deleted, the cadence sentence written twice, a run
number skipped, the runs out of date order, no run heading at all, and `2026-02-31`.

### What this does not do

It has the residual every record-derived signal has: the tripwire believes the headings. A run
conducted and never written up leaves the check red. That is the right direction to be wrong in,
since the note is the only place a run exists. A heading nobody earned turns it green, which is the
same hole `script/audits` names in the same words. Closing a cadence by editing the record instead
of doing the work is available, cheap, and the one thing that makes the whole mechanism a lie.

What would make this `BUILT`, as the lane wrote it: run 5's handoffs landed, and run 6 conducted
under the cadence rather than because somebody thought of it. The second was the one that had never
happened.
