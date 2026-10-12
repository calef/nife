---
status: PROPOSED
raised: 2026-10-12
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# The tree's gates stop depending on the fleet tooling

Raised 2026-10-12 (UTC) by lane `maintainer/tooling-inventory`. It is the first step calef ruled
on pull request #1976: Fork 4 of
[nife's tooling, sorted by who needs it](tooling-sorted-by-who-needs-it.md), answered
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

## BUGS

- A gate split in two can lose a check in the gap. The exit's 22-check comparison is what guards
  against that; nothing else does.
- The opt-in for the queued-push check means a lane cut without `script/bootstrap` is unguarded
  until it runs it.
- The new names are provisional.
