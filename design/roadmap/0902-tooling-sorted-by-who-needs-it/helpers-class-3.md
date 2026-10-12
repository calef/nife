# Helpers in class 3

An appendix of [nife's tooling, sorted by who needs it](../0902-tooling-sorted-by-who-needs-it.md),
measured 2026-10-12 (UTC) against `e3cc3f128`.
Every class-3 file in `helpers/`, with recorded fixtures as one row per directory.

Columns. Class is 1 (build and prove), 2 (standards), 3 (the fleet) or 4 (one machine). Two
numbers mean the row straddles both. "Called or read by" lists the files that name this path,
comments included. The search covers `script/`, `helpers/`, `.github/`, `.githooks/`, `.claude/`,
`.cargo/` and `xtask/`. "Calls or reads" lists the inventoried paths this file names. Both show four and a
count. `s/` is `script/`, `h/` is `helpers/`, and a bare `.yml` is a workflow. The last
column counts the times the file names calef.

| Path | Class | Why | Called or read by | Calls or reads | Lines | calef |
|---|---|---|---|---|---|---|
| `helpers/agents-index-check.py` | 3 | agent-instruction index checks | `s/lint`, `s/names` | `s/lint`, `notes/skills/README.md` | 125 | 3 |
| `helpers/agents-md-split-check.py` | 3 | agent-instruction index checks | none | none | 129 | 4 |
| `helpers/architect-label-comment.sh` | 3 | architect label, hold and ruled bots | `architect-label.yml`, `h/architect-label-rules.py` | `s/record-ruling`, `h/architect-label-rules.py`, `h/merge-drain.sh`, `architect-label.yml`, +2 | 81 | 4 |
| `helpers/architect-label-rules.py` | 3 | architect label, hold and ruled bots | `architect-label.yml`, `h/architect-label-comment.sh`, `h/merge-drain.sh`, `h/needs-maintainer.jq`, +3 | `s/lint`, `h/architect-label-comment.sh`, `h/merge-drain.sh`, `architect-label.yml`, +1 | 1496 | 11 |
| `helpers/architect-ruled-selftest.sh` | 3 | architect label, hold and ruled bots | `h/architect-ruled.sh`, `s/lint` | `h/architect-ruled.sh` | 32 |  |
| `helpers/architect-ruled.sh` | 3 | architect label, hold and ruled bots | `architect-label.yml`, `architect-ruled-clears-hold.yml`, `coe-architect-label.yml`, `h/architect-ruled-selftest.sh` | `s/record-ruling`, `h/architect-ruled-selftest.sh`, `notes/merge-queue.md` | 47 | 2 |
| `helpers/at-risk-check.sh` | 3 | lane claim, branch shape, lane-block rule | `trunk-health.yml`, `h/merge-drain.sh`, `h/trunk-health.sh` | `s/lint`, `s/qemu-check`, `s/stack-frame-check`, `h/lane-claim-check.sh`, +4 | 204 | 1 |
| `helpers/blocked-by-resolution-selftest.sh` | 3 | merge drain and its predicates | `h/blocked-by-resolution.jq`, `h/merge-drain.sh`, `s/lint` | `s/lint`, `h/blocked-by-resolution.jq` | 67 |  |
| `helpers/blocked-by-resolution.jq` | 3 | merge drain and its predicates | `h/blocked-by-resolution-selftest.sh`, `h/merge-drain.sh`, `s/lint` | `s/lint`, `h/blocked-by-resolution-selftest.sh`, `h/merge-drain.sh`, `h/queue-eligible.jq` | 33 |  |
| `helpers/blocked-by-selftest.sh` | 3 | merge drain and its predicates | `s/lint` | `h/blocked-by.sh`, `h/queue-eligible-selftest.sh` | 60 |  |
| `helpers/blocked-by.sh` | 3 | merge drain and its predicates | `h/blocked-by-selftest.sh`, `h/merge-drain.sh`, `s/lint` | `h/branch-name-check.sh`, `h/merge-drain.sh`, `notes/merge-queue.md` | 41 | 1 |
| `helpers/branch-name-check.sh` | 3 | lane claim, branch shape, lane-block rule | `h/blocked-by.sh`, `s/claim`, `s/lint` | `s/claim`, `s/lint`, `h/qemu-path.sh` | 58 | 1 |
| `helpers/cancelled-duplicate-selftest.sh` | 3 | merge drain and its predicates | `h/cancelled-duplicate.jq`, `s/lint` | `s/lint`, `h/cancelled-duplicate.jq`, `h/missing-check.jq` | 82 |  |
| `helpers/cancelled-duplicate.jq` | 3 | merge drain and its predicates | `h/cancelled-duplicate-selftest.sh`, `h/merge-drain.sh`, `h/missing-check.jq` | `h/cancelled-duplicate-selftest.sh`, `notes/merge-queue.md` | 33 |  |
| `helpers/ci-failing-fixtures/` | 3 | fixtures (1 files) | none | none | 1 |  |
| `helpers/ci-failing-selftest.sh` | 3 | failing-CI flag on a ready PR | `s/lint` | `s/lint`, `h/ci-failing.jq`, `h/ci-failing.sh` | 98 |  |
| `helpers/ci-failing.jq` | 3 | failing-CI flag on a ready PR | `ci-failing.yml`, `h/ci-failing-selftest.sh`, `h/ci-failing.sh` | `h/ci-failing.sh`, `notes/merge-queue.md` | 34 |  |
| `helpers/ci-failing.sh` | 3 | failing-CI flag on a ready PR | `ci-failing.yml`, `h/ci-failing-selftest.sh`, `h/ci-failing.jq`, `h/merge-drain.sh`, +3 | `h/ci-failing.jq`, `h/needs-maintainer.jq`, `ci-failing.yml`, `notes/merge-queue.md` | 125 | 3 |
| `helpers/claim-selftest.sh` | 3 | lane claim, branch shape, lane-block rule | `s/claim`, `s/lint` | `s/claim`, `s/lint`, `notes/debt-paydown.md` | 72 | 1 |
| `helpers/debt_paydown_share.py` | 3 | method metric | `metrics.yml`, `s/lint`, `s/metrics` | `s/metrics`, `s/names`, `h/merge_queue_share.py`, `notes/debt-paydown.md` | 173 | 1 |
| `helpers/handoff-check.py` | 3 | Claude Code hook | `claude-settings` | `claude-settings`, `notes/rule-violations.md` | 96 | 1 |
| `helpers/hold-rerun-selftest.sh` | 3 | architect label, hold and ruled bots | `architect-ruled-clears-hold.yml`, `h/hold-rerun.sh`, `s/lint` | `h/hold-rerun.sh` | 63 |  |
| `helpers/hold-rerun.sh` | 3 | architect label, hold and ruled bots | `architect-hold.yml`, `architect-ruled-clears-hold.yml`, `h/hold-rerun-selftest.sh` | `h/hold-rerun-selftest.sh`, `notes/merge-queue.md` | 93 |  |
| `helpers/hooks/refuse-ci-polling` | 3 | Claude Code hook | `s/lint`, `claude-settings`, `h/ci-failing.sh` | `s/lint`, `h/nanny.py`, `h/runwatch.py`, `briefs/gate-in-ci.md`, +2 | 207 | 2 |
| `helpers/lane-claim-check.sh` | 3 | lane claim, branch shape, lane-block rule | `merge-drain.yml`, `h/at-risk-check.sh`, `h/merge-drain.sh`, `s/claim` | `s/lint`, `s/qemu-check`, `s/stack-frame-check` | 214 | 3 |
| `helpers/merge-drain.sh` | 3 | merge drain and its predicates | `merge-drain.yml`, `metrics.yml`, `h/architect-label-comment.sh`, `h/architect-label-rules.py`, +17 | `s/architect-queue`, `s/lint`, `s/metrics`, `s/record-ruling`, +25 | 948 | 28 |
| `helpers/merge_queue_share.py` | 3 | method metric | `metrics.yml`, `trunk-health.yml`, `h/debt_paydown_share.py`, `s/lint`, +1 | `s/metrics` | 331 | 2 |
| `helpers/milestone-branch-block-selftest.sh` | 3 | lane claim, branch shape, lane-block rule | `h/milestone-branch-block.sh`, `s/lint` | `s/lint`, `h/milestone-branch-block.sh` | 92 |  |
| `helpers/milestone-branch-block.sh` | 3 | lane claim, branch shape, lane-block rule | `h/milestone-branch-block-selftest.sh`, `s/lint` | `s/lint`, `h/milestone-branch-block-selftest.sh`, `notes/skills/developer-lane/SKILL.md` | 95 | 2 |
| `helpers/missing-check.jq` | 3 | merge drain and its predicates | `h/cancelled-duplicate-selftest.sh`, `h/merge-drain.sh`, `h/needs-maintainer-selftest.sh`, `h/needs-maintainer.jq` | `h/cancelled-duplicate.jq`, `h/ci-failing.sh`, `h/merge-drain.sh` | 37 | 1 |
| `helpers/nanny.py` | 3 | queue and CI watchers for a maintainer session | `h/hooks/refuse-ci-polling`, `h/merge-drain.sh`, `h/prune_toolchains.py` | `h/runwatch.py`, `briefs/session-start.md` | 129 | 1 |
| `helpers/needs-maintainer-fixtures/` | 3 | fixtures (8 files) | none | none | 4481 |  |
| `helpers/needs-maintainer-selftest.sh` | 3 | needs-maintainer detector | `h/ci_job_times.py`, `h/merge-drain.sh`, `h/needs-maintainer.jq`, `s/lint` | `s/citations`, `s/lint`, `s/record-ruling`, `h/merge-drain.sh`, +3 | 386 | 9 |
| `helpers/needs-maintainer.jq` | 3 | needs-maintainer detector | `h/ci-failing.sh`, `h/ci_job_times.py`, `h/merge-drain.sh`, `h/needs-maintainer-selftest.sh`, +4 | `s/lint`, `h/architect-label-rules.py`, `h/ci-failing.sh`, `h/ci_job_times.py`, +7 | 411 | 18 |
| `helpers/open-lane.sh` | 3 | rented-model lanes and delegated review trials | `h/at-risk-check.sh`, `s/redo-rate`, `s/nanny` | `s/citations`, `s/lint`, `notes/open-model-lanes.md`, `notes/effort-levels.md` | 208 | 1 |
| `helpers/open-question.jq` | 3 | architect label, hold and ruled bots | `h/needs-maintainer.jq`, `h/record-ruling-selftest.sh`, `s/architect-queue`, `s/lint`, +3 | `s/architect-queue`, `s/record-ruling`, `h/merge-drain.sh`, `h/needs-maintainer.jq`, +1 | 75 | 2 |
| `helpers/prompt-reminder.sh` | 3 | Claude Code hook | `claude-settings` | `claude-settings` | 29 | 4 |
| `helpers/push-queued-check.sh` | 3 | refuse a push to a queued branch | `hook:pre-push`, `h/push-queued-selftest.sh`, `s/lint` | `notes/merge-queue.md` | 61 |  |
| `helpers/push-queued-selftest.sh` | 3 | refuse a push to a queued branch | `s/lint` | `h/push-queued-check.sh` | 57 |  |
| `helpers/queue-eligible-selftest.sh` | 3 | merge drain and its predicates | `h/blocked-by-selftest.sh`, `h/merge-drain.sh`, `h/queue-eligible.jq`, `s/lint` | `s/lint`, `h/queue-eligible.jq` | 60 |  |
| `helpers/queue-eligible.jq` | 3 | merge drain and its predicates | `h/blocked-by-resolution.jq`, `h/merge-drain.sh`, `h/needs-maintainer.jq`, `h/open-question.jq`, +4 | `s/lint`, `h/queue-eligible-selftest.sh` | 27 |  |
| `helpers/queue-hold.sh` | 3 | merge drain and its predicates | `h/merge-drain.sh`, `h/trunk-health.sh`, `h/queue-eligible-selftest.sh`, `h/queue-eligible.jq` | `s/lint`, `h/merge-drain.sh`, `h/queue-eligible.jq`, `h/trunk-health.sh`, +4 | 303 | 3 |
| `helpers/record-ruling-selftest.sh` | 3 | architect label, hold and ruled bots | `s/lint` | `s/lint`, `s/record-ruling`, `h/open-question.jq` | 88 |  |
| `helpers/review-bundle.sh` | 3 | rented-model lanes and delegated review trials | none | none | 43 |  |
| `helpers/review-matrix.sh` | 3 | rented-model lanes and delegated review trials | `h/review-trial.sh` | `notes/delegated-review.md` | 45 | 1 |
| `helpers/review-trial.sh` | 3 | rented-model lanes and delegated review trials | `h/review-matrix.sh` | `h/review-matrix.sh` | 69 |  |
| `helpers/runwatch.py` | 3 | queue and CI watchers for a maintainer session | `h/hooks/refuse-ci-polling`, `h/nanny.py`, `h/prune_toolchains.py` | `briefs/gate-in-ci.md`, `briefs/session-start.md` | 61 | 1 |
| `helpers/session_steps.py` | 3 | session and smelter identity | `s/lint` | none | 184 | 13 |
| `helpers/smelter-bin/gh` | 3 | session and smelter identity | `hook:pre-push`, `architect-hold.yml`, `architect-label.yml`, `architect-ruled-clears-hold.yml`, +71 | `h/smelter-token`, `notes/session-identity.md` | 22 | 1 |
| `helpers/smelter-token` | 3 | session and smelter identity | `h/smelter-bin/gh`, `s/claim`, `s/lint`, `s/preflight-queue` | `s/lint`, `notes/session-identity.md` | 670 | 9 |
| `helpers/stuck-head.jq` | 3 | merge drain and its predicates | `h/merge-drain.sh`, `h/needs-maintainer-selftest.sh`, `h/needs-maintainer.jq` | `h/needs-maintainer.jq`, `notes/queue-ejection.md` | 19 |  |
| `helpers/trunk-health.sh` | 3 | trunk health and watcher liveness | `trunk-health.yml`, `undefined-behavior-check.yml`, `h/queue-hold.sh`, `s/cadence-check`, +2 | `s/cadence-check`, `s/lint`, `h/at-risk-check.sh`, `h/merge-drain.sh`, +4 | 229 | 4 |
| `helpers/watcher_watch.py` | 3 | trunk health and watcher liveness | `merge-drain.yml`, `trunk-health.yml`, `s/cadence-check`, `s/lint` | `s/cadence-check` | 424 | 5 |
| `helpers/prune_toolchains.py` | 3/4 | uninstalls nightly toolchains nothing on this machine pins | none | `s/toolchain-bump`, `briefs/merge-and-cleanup.md` | 151 | 2 |
