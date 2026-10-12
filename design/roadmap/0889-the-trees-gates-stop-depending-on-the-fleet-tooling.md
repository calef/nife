---
status: PARTIAL
raised: 2026-10-12
promoted_from: the-trees-gates-stop-depending-on-the-fleet-tooling
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 889. The tree's gates stop depending on the fleet tooling

*(Promoted 2026-10-12 (UTC) by lane `milestone/889-the-trees-gates-stop-depending-on-the-fleet-tooling`
from the proposal `the-trees-gates-stop-depending-on-the-fleet-tooling`, at the next free number.
The number is provisional until the merge queue lands it; the title, the slug and every new name
below are drafts for calef's naming pass.)*

Raised 2026-10-12 (UTC) by lane `maintainer/tooling-inventory`. It is the first step calef ruled
on pull request #1976: Fork 4 of
[nife's tooling, sorted by who needs it](proposals/tooling-sorted-by-who-needs-it.md), answered
[*"A"*](https://github.com/nifeos/nife/pull/1976#issuecomment-6117241317) on 2026-10-12 (UTC). The
edges are cut in place, in one nife lane, before any fleet tooling leaves. The title, the slug and
every new name below are provisional, for calef's naming pass.

Reuse: nothing new is written beyond a split of existing files. `toolchain-bump.yml` already skips
its App token step when the App is absent, and the other four workflows take that pattern as it is.

## Why

The inventory found six places where a build, proof or standards item cannot run without the
agent-fleet tooling (class 3). Until they are cut, moving class 3 to its own repository (Fork 2,
ruled A) is a rewrite rather than a move. Cutting them also gives a stranger a `script/lint` and a
`pre-push` that never touch the merge queue or GitHub.

## The work

Each item is one edge from the inventory. Nothing moves to another repository in this milestone.

1. `script/lint` stops running class-3 checks. The 22 checks (19 selftests of the queue, label and
   identity machinery, rules 4 and 4b, and the agent-instruction index) move into a new entry point,
   provisionally `script/lint-fleet`. A new `ci.yml` job runs it on every pull request and merge
   group, so no check stops gating. `script/claim` keeps calling `helpers/branch-name-check.sh`.
2. `.githooks/pre-push` runs `helpers/push-queued-check.sh` only on a checkout that opts in,
   provisionally with `git config nife.fleet true`. `script/bootstrap` sets it where it already
   installs the hooks for a lane. A stranger's push then asks GitHub nothing.
3. `falsifications`, `mutation`, `vendor-watch` and `verify-shard-refresh` gate their
   `create-github-app-token` step on the App's id being present, as `toolchain-bump.yml` does. They
   fall back to `GITHUB_TOKEN`, so a fork runs them without the `nife-smelter` App.
4. `metrics.yml` splits in two. Its method collectors, `merge_queue_share.py` and
   `debt_paydown_share.py`, move to a new scheduled workflow, provisionally `fleet-metrics.yml`,
   that writes the same CSV files. Everything else, CI and proof times included, stays.
   `script/metrics` already reads those files and needs no change.
5. `script/cadence-check` stops calling `script/effort --check` and `helpers/watcher_watch.py`
   itself. Both checks move into `fleet-metrics.yml`.
6. `notes/scripts.md` lists the fleet entry points in their own section. `AGENTS.md`'s skills index
   is left alone: it is calef's to edit, and the lane proposes the wording in its report.

Two edges are kept on purpose, and the lane records why where a reader meets each:

- `ci.yml`'s draft gate. Skipping CI on a draft costs a contributor nothing and saves runner time,
  so it is a sensible default for any repository, not a fleet dependency.
- `helpers/scope-merge-base.sh` and `helpers/job-budget.py`. Both are class-1 CI plumbing that the
  fleet also reads. A fleet item reading a class-1 one is the normal direction and needs no cut.

## Exit

- A throwaway copy of the tree with every class-3 helper, script and workflow deleted passes
  `script/lint --no-cargo`. The appendix of the inventory lists which files those are.
- `script/lint-fleet` runs green in CI on the same commit, and each of its 22 checks fails when the
  file it tests is broken, as each did inside `script/lint`.
- `git push` from a checkout without `nife.fleet` runs no `gh` command, checked with a stub `gh` on
  `PATH` that fails if called.
- Each of the four workflows runs to completion on a dispatch with the App secrets unset.
- `fleet-metrics.yml` writes byte-identical CSV rows to `metrics.yml`'s last run for the same week.

## Where it stands, 2026-10-12 (UTC)

PARTIAL. Every cut is made, and four of the five exit checks were run and pass. The fifth needs a
dispatch this branch cannot make. Each tie from the inventory, checked against the code first:

| Tie | What the code did | What happened |
|---|---|---|
| 1, lint | 22 fleet checks ran inside `script/lint`, about 170 lines | Moved whole to `script/lint-fleet`, a step of CI's `lint` job |
| 2, pre-push | Every push asked GitHub twice, queued and ready | Behind `nife.fleet`, in `helpers/pre-push-fleet.sh` |
| 3, App token | Four workflows could not run without the App | Token step skipped without it; `github.token` falls back |
| 4, draft gate | Reads GitHub's own draft flag only | Kept; the reason is written in `ci.yml` |
| 5, metrics | `metrics.yml` ran two fleet collectors | Moved to `fleet-metrics.yml`, same CSV files |
| 5, cadence | `cadence-check` called `script/effort` and `watcher_watch.py` | Both asked by `helpers/trunk-health.sh` instead |
| 6, shared helpers | `ci_job_times.py`'s selftest read two fleet files | That check moved to the fleet's selftest; the rest kept |

Four places differ from the text above, each for a reason found in the code:

- The fleet checks run as a step of the `lint` job, not a new job. A new job gates nothing until
  the ruleset requires it, and `lint`'s verdict is already required.
- `script/claim` sets `nife.fleet`, not `script/bootstrap`. Bootstrap runs on every clone,
  a stranger's included, and the hooks are installed by `script/setup`.
- The effort and watcher checks went to `helpers/trunk-health.sh`, not `fleet-metrics.yml`. The
  watcher rule promises a stop is reported within three five-minute intervals, and a daily
  workflow would break that.
- `script/lint-fleet` has 23 checks. The 23rd tests the hook's fleet half, which is new.

Exit checks, as run:

- **Stranger clone.** With all 72 class-3 helpers, scripts and workflows deleted, every gate in
  `script/lint --no-cargo` passes. Two record checks still fail, each section run alone to see
  them all. `script/roadmap --check` fails on six `Recorded.` paths in five old blocks. The
  prose environment-name check fails on names that only class-3 notes and briefs mention. Both are
  records citing tooling that has not moved yet, which is §201 (one roadmap until a citation has
  to cross)'s event, not a gate calling the fleet.
- **The checks still fire.** 39 breaks, one at a time: a real defect in each tested file where one
  was cheap to plant (18), and the check's own file failing (21). All 39 turn
  `script/lint-fleet` red at the right section. The first sweep missed one, a parser that exits
  while being sourced; `helpers/blocked-by-selftest.sh` now loads it in a child first.
- **No token.** A fresh clone, no `GH_TOKEN`, GitHub behind a dead proxy and a `gh` that logs and
  fails: `script/lint --no-cargo` passed, a real `git push` passed through the hook, and `gh` was
  called 0 times. `main` before this, in the same setup, called `gh` once and failed lint: the
  `smelter-token` selftest sent its loopback stub through the proxy. It now sets `NO_PROXY`.
- **Same CSV rows.** Both collectors, run locally, wrote rows byte-identical to `metrics.yml`'s
  last run (2026-10-11 12:25 UTC) for every finished day.
- **Not run.** The four workflows have not been dispatched with the App absent. A dispatch needs
  the file on `main`, and the secrets are scoped to `main`.

## BUGS

- A gate split in two can lose a check in the gap. The 39-break sweep above guards the split once;
  nothing guards it afterwards.
- The opt-in is per clone. A fleet machine that never ran `script/claim` pushes without the queued
  check until `git config nife.fleet true` is set.
- Three current-tense mentions of "`script/lint` 4b" sit in files a lane may not edit:
  `design/naming.md`, `design/tenets/roles-and-the-queue.md` and
  `notes/skills/developer-lane/SKILL.md`. They now mean `script/lint-fleet`.
- The new names are provisional: `script/lint-fleet`, `nife.fleet`, `fleet-metrics.yml`,
  `helpers/pre-push-fleet.sh` and its two selftests.

## Follow-on

- **Outstanding.** Dispatch `falsifications`, `mutation`, `vendor-watch` and
  `verify-shard-refresh` with the App absent, on a fork or with the secrets unset, and record the
  runs. Checked 2026-10-12: each skips its token step when `AUTOMATION_APP_ID` is empty, as
  `toolchain-bump.yml` already did.
- **Proposed.** Moving class 3 as a unit, with the six `Recorded.` paths and the class-3 notes
  that make the stranger clone's two record checks fail, is the inventory's next step:
  `design/roadmap/proposals/tooling-sorted-by-who-needs-it.md`.
- **Recorded.** The three stale "`script/lint` 4b" mentions, in BUGS above.

## Index row

The tree's own gates stop calling the agent-fleet tooling. script/lint's 22 fleet checks move to
script/lint-fleet, which CI's lint job still runs. The pre-push hook asks GitHub nothing unless a
clone opts in, four scheduled workflows run without the automation App, and the method-metric
collectors leave metrics.yml. It is the first step of calef's Fork 4 ruling on #1976, and what
makes moving the fleet tooling to its own repository a move rather than a rewrite.
