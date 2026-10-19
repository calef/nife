---
status: BUILT
raised: 2026-08-05
built: 2026-10-19
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 118. CLAUDE.md has a budget, and the rules that get violated move up the ladder

Minted 2026-08-05 by calef, who
noticed the file had gotten huge and asked what that costs. Re-checked against the tree on
2026-10-03 (UTC): the audit (#309), the cut and the split are done. `AGENTS.md` fell from 942 to
573 lines (calef's seven cuts of 2026-09-25, #1189's split into imperatives plus appendices): the
reasons moved to `design/tenets/`, the procedures to `briefs/`, and §212 (a prose budget) caps that
core. All four budget rules have a mechanism; `## Follow-on` names each.

The size measurement is no longer hand-entered. Until 2026-08-22 it was `wc -lwc AGENTS.md`,
run by nobody, which was the milestone's own budget argument for itself: this block carried three
different sizes in thirteen days, each hand-entered by whoever last looked. `script/lint` derived
it from then until 2026-10-06; see "The size gate and the ledger, 2026-08-22".

This block is the summary. The working moved on 2026-10-12 (UTC) to
[`0118-constitution-budget/`](0118-constitution-budget/README.md) under §212 (a prose budget):

- [`audit.md`](0118-constitution-budget/audit.md): the 2026-08-18 audit's tables, the two stale
  rules, the four budget rules in full, and the clause the tenet needs.
- [`applicable-cut.md`](0118-constitution-budget/applicable-cut.md): the second lane's growth
  measurement, each of the four cuts with its verification, and the five rules declined.
- [`plan.md`](0118-constitution-budget/plan.md): the three pieces as planned, with the reasoning
  for deleting a rule a gate already carries.
- [`size-gate.md`](0118-constitution-budget/size-gate.md) and
  [`size-gate-september.md`](0118-constitution-budget/size-gate-september.md): the gate's design
  and every raise and lowering of its ceiling, with the reason each was worth it.

## What it costs, measured 2026-08-05

738 lines, 8,306 words, roughly 11,000 tokens, loaded into every session and into every lane
brief, because every brief says "read `CLAUDE.md` first, in full". About a dozen lanes ran on
2026-08-04 alone.

Re-measured 2026-08-17: 868 lines, 10,001 words (`wc -lw AGENTS.md`), so it has grown another 130
lines and 1,695 words in the twelve days this milestone has been open. The premise got stronger, not
weaker, which is the argument for the budget rather than against it. The 2026-08-05 figures above
are left as the measurement calef acted on.

It grew 336 lines in one day, across 11 commits. Two sections are 46% of it: the roles section at
224 lines and the naming section at 118.

And the cost that matters is not tokens. `CLAUDE.md` warns against `pkill`-ing QEMU and against
`git reset --hard` to take a measurement, once each. In one day one lane killed another lane's
emulator mid-test with `pkill -f`, and four agents clobbered work with `reset --hard`, `checkout`
or `stash`. Every one of them had been told to read the file in full.

So the warnings are present and were skipped. Length is already costing compliance, which is a
measurement rather than an aesthetic complaint.

## The diagnosis: not too long, wrongly stratified

Everything loads at once with equal weight, so "never `pkill` QEMU" competes for attention with the
`snake_case` conventions table. A reader skimming it cannot tell which three rules will bite them
this hour, and there are 130 more lines of it than when that was written.

The reasoning is why the rules stick. "The `sed` that rewrote the very row recording that the name
had been refused" is what makes that rule memorable, and compressing it to a bullet would produce a
style guide nobody obeys. So this milestone stratifies rather than compresses, and a lane that
finds itself deleting arguments has taken the wrong turn.

There is also an uncomfortable connection to the third principle: a stranger's first encounter with
this project is a document of this length addressed to an agent. Milestone 117 (the stranger
test) found that independently, in both of its runs.

## The audit, 2026-08-18

calef asked for the count: the rules, how many have a mechanism behind them, and the rest as the
budget. The file then held 868 lines, 10,001 words, 16 sections and roughly 33 rules. `script/lint`
alone ran 32 named checks, and six other gate scripts ran more. So the file is not mostly
unmechanised prose, which is not what "the file has gotten huge" implies.

The audit sorted the rules three ways ([audit.md](0118-constitution-budget/audit.md) has each):

- Eight rules with a mechanism, whose paragraphs could shrink to a sentence. Four did not survive
  verification (next section).
- Two rules whose prose had gone stale. The citations paragraph told agents to check by hand what
  `script/citations` now checks. "Never squash-merge" stood against a setting that allowed it,
  since turned off (#334).
- Four budget rules that could have a mechanism and did not, two of which failed on 2026-08-18.
  They are the ones `## Follow-on` tracks: (1) starting the two watchers, (2) pruning a lane's
  worktree at merge, (3) squashing against the recorded base commit, never `origin/main`, and (4)
  enforcing `needs-architect`.

Everything else unmechanised is judgment and stays prose. calef, the same day: prefer gates over
prose in this file, but a gate is only better when it can be right about the tree, so record what
it measured when you built it.

## The applicable cut, 2026-08-18

A second lane checked each of the eight claims by reading the gate's body, and four of the eight
did not survive. A banner in `script/lint` names a check's subject, not its coverage.
[applicable-cut.md](0118-constitution-budget/applicable-cut.md) has the line-by-line working.

- The file had reached 955 lines and 11,062 words, +29% on lines since minting. The growth steps
  rather than creeps: two commits (`1da0d68b`, `77017f71`) added 133 of the 217 lines, each a
  whole new tenet section.
- Four cuts were verified and applied by the integrator on 2026-08-18: the stale citations
  paragraph, the branch-deletion rule (`delete_branch_on_merge` does it), rule 7's second reason
  (check 5's comment carries it), and a reword of the squash-merge prohibition. `AGENTS.md` went
  955 lines / 11,062 words to 942 / 10,880, 1,212 bytes in all.
- Five rules were examined and declined, four of them from the audit's eight: rule 1, `snake_case`,
  names are calef's call, benchmarks, and the `nifefs` 32-byte cap. In each, the gate checks less
  than the prose claims, or the prose is judgment.

The verified cut is about 1.7% of the file, against the audit's estimate of a tenth. So the growth
problem is real and the duplication problem is mostly not. The budget and the split are where the
value is, and this lane and #309 have covered the re-audit ground.

## Three pieces

The plan as written, with its reasoning, is in [plan.md](0118-constitution-budget/plan.md).

- Split, the way `DECISIONS.md` and the roadmap were split: a core of the rules that change
  behavior on every task, with linked documents for the rest. The test is whether an agent will
  genuinely read the whole core, not a line count.
- Delete a rule a gate already enforces, because the gate's comment is the better home: it is read
  by somebody who has just tripped it. Where the constitution holds something the gate lacks, move
  it to the gate first. The verified figure was about 15 lines, not 90.
- Move the most-violated rules up the ladder. A rule violated repeatedly is not stated too quietly;
  it is on the wrong rung.
- Add a size budget, so a one-time cut does not simply re-grow, and pair it with a ledger of rules
  violated anyway: three strikes and a rule moves up the ladder or is deleted as unenforceable.

## The size gate and the ledger, 2026-08-22

A third lane built both, and neither could be a mechanical extension of the earlier work. The split
that would produce a "core" had not happened, and neither the cut nor the budget rules bear on what
a size gate or a ledger need.

### The size gate

`script/lint` gained an `agents-md-lines` ceiling, built on the `count-at-most` relation of
milestone 134 (the register of measures). It sat at the tree's exact value with zero headroom, so
every line added had to replace one removed or be a raise with its reason written in this block.
It moved from 942 up to 1048 by 2026-09-05. Milestone 262 (the naming rule is written twice)
lowered it to 988, and it peaked again at 1009. After §155 (the naming conventions move out of the
constitution) it fell to 922. It then rose to 949 and was lowered to 935, and #1189's split took
it to 584 on 2026-09-24. That day it became a ratchet that also fails when the file shrinks, and
calef's seven cuts took it to 573 on 2026-09-25. On 2026-10-06 (UTC) calef dropped it: "drop
118's limit and rely on §212's cap". One budget in one unit, because words track context cost and
lines track wrapping. [size-gate.md](0118-constitution-budget/size-gate.md) and
[size-gate-september.md](0118-constitution-budget/size-gate-september.md) record every move and
its reason.

### The violation ledger

`notes/rule-violations.md` and `script/rule-violations` (both provisional names). One row per
incident, with an `instances` count per row. A source that reports an aggregate without naming
individual incidents gets one row with that count, honestly, rather than invented distinct rows. A
status keeps a row from re-triggering once a rule has already been addressed. `--check` fails when
an `open` rule reaches three strikes.

It was seeded from the incidents this milestone and `AGENTS.md` already recorded against themselves,
not invented for the occasion. They are the squash-against-`origin/main` scar (`AGENTS.md`,
"Commits"), and the `pkill`/mid-test-emulator incident and the reset-`--hard`/`checkout`/`stash`
clobbers that "What it costs, measured 2026-08-05" quotes. The git-clobber rule crossed the
threshold at four open strikes, one past three, exactly the finding this ledger exists to surface.
[DECISIONS §128 (what enforces the git-clobber rule)](../decisions/0128-git-clobber-enforcement.md)
priced the real enforcement options. calef's own call, on the evidence of zero repeats in the three
weeks since, was to accept rather than add a mechanism: the existing `AGENTS.md` prose already
appears to be doing the job. The ledger row is marked `resolved` on that basis. See
notes/rule-violations.md, including its own honest limits (self-reporting only, exact-text matching,
not wired into any mandatory gate).

Deliberately not wired into `script/lint` or CI. The git-clobber rule's threshold was already
crossed by history this lane did not create. Wiring `--check` into the mandatory suite would fail
every lane's pull request over a decision that is not a lane's to make, the same restraint
DECISIONS §61 (a lint is adopted on evidence) already states for an ordinary lint. Whether and
when to wire it in is left as an open question in the note rather than answered here.

### What was left on 2026-08-22

The split was then the only untouched piece. It requires editing `AGENTS.md`, so no developer lane
could start it; it waited on calef or the integrator, as the proposed cut and the ceiling raises
did. It has since been done (`## Follow-on`).

## Scope note

Not a rewrite, and not a trim. No argument in that file is deleted; the reasoning is the asset.
Text moves, and only the rules that need to change rung change form.

The budget number is provisional and belongs to whoever builds this, informed by what the core
actually needs rather than chosen first and filled to.

The honest limit: a size gate measures the wrong thing on purpose. It cannot tell a rule that
earns its lines from one that does not, and a lane that games it by moving text into a linked file
nobody reads has satisfied the gate and defeated the milestone. The ledger is the counterweight and
it is weaker than a gate, because it depends on lanes continuing to report their own mistakes
honestly, which is a culture rather than a mechanism. Say so where the reader meets it.

## Follow-on

- **Done.** The split into a core plus linked documents: `AGENTS.md` is 573 lines, with the evidence
  in `design/tenets/` and the task procedures in `briefs/` (#1189, 2026-09-24). The line ceiling
  counts the core. Verified 2026-10-03.
- **Done.** Budget rule 1, the watchers nobody starts, has a mechanism: `AGENTS.md` records
  `com.nife.merge-drain` and its sibling running unattended on patagonia under `launchd`, with the
  asleep-or-shut-down gap named as accepted rather than closed.
- **Done.** Budget rule 2, pruning a lane's worktree at merge: `helpers/prune_worktrees.py
  --remove` (name provisional) runs hourly from launchd, without `--force`. It removes a worktree
  whose pull request merged, that `git status` calls clean, whose HEAD origin holds and that
  nothing wrote to for an hour. Its dry run on 2026-10-19 (UTC) found 13 such worktrees on
  the fleet machine. `script/lint-fleet` runs its selftest.
- **Done.** Budget rule 3 is "a pushed commit is never rewritten", which replaced squashing against
  the recorded base, plus never `git stash`. The stash half is unrepresentable on a fleet clone:
  `helpers/stash-block.sh` holds `refs/stash/no-git-stash-see-worktree-commits-skill`, and git
  cannot create `refs/stash` beside it, so `git stash` fails in every worktree before touching the
  tree. `script/claim` and the fleet pre-push install it. The rewrite half is a check that fails
  loudly: `helpers/push-rewrite-check.sh`, from the fleet pre-push, refuses a push whose remote sha
  is not an ancestor of the pushed one. Both names provisional; `script/lint-fleet` runs both
  selftests. A server-side refusal of force-pushes would be the stronger rung and is not taken,
  because the bot workflows force-push their own branches.
- **Done.** Budget rule 4 is mechanized: DECISIONS §88 was ratified 2026-08-25
  (`design/decisions/0088-needs-architect-as-a-check.md`) and `.github/workflows/architect-hold.yml`
  is a required check that fails any pull request carrying the `needs-architect` label.
- **Done.** The unresolved "every fence names its counterpart" row is answered, and the answer is
  that the row named a rule which is not in the file: `AGENTS.md` contains the word "fence" zero
  times, and only the gate in `script/lint` carries it.
- **Refused.** Rule 1's gate past the kernel and its prose cut: milestone 489 (the rule-1 lint, past
  the kernel) was refused by milestone 83 (a mechanical rule-1 lint), and `kernel/src/main.rs` no
  longer reads an `aarch64_cpu` register outside `arch/`. Checked 2026-10-03.
- **Done.** The `#[path]` gate's scope: check 5 in `script/lint` scans `components/src` and
  `fixtures/src` and exits if either directory is missing (milestone 401 (a gate that selects the
  set it judges)). Checked 2026-10-03.
- **Recorded.** The naming-conventions table is still ungated apart from check 6, which covers
  `notes/` and `design/` markdown filenames; no case convention for crates, programs, modules or
  `script/` entry points is enforced anywhere.
- **Recorded.** The ledger stays out of `script/lint` and CI: `notes/rule-violations.md` and
  `script/rule-violations` exist and nothing in `script/lint` references either, with the
  git-clobber row marked resolved on §128's basis exactly as this block says.

## Index row

`AGENTS.md` (942 lines today) warns against `pkill` and `git reset --hard` once each, and both had
already been violated when that was written. One lane killed another's emulator mid-test, and four
agents clobbered work with a destructive git operation in one day. All had been told to read it in
full. Audited (#309), and the cut and the split applied (the file is 573 lines, 2026-09-25). A size
gate (`script/lint`'s `agents-md-lines` ceiling) plus a violation ledger were built (2026-08-22).
The ledger's first run found the git-clobber rule already four strikes over the three-strike
threshold, a decision for calef or the integrator. Budget rules 2 (pruning at merge) and 3 (never
rewrite a pushed commit, never `git stash`) are mechanized by an hourly worktree prune, a ref that
blocks `git stash` and a pre-push refusal.
