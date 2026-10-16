---
status: IN-PROGRESS
branch: milestone/422-a-cadence-job-whose-healthy-state-is-red
raised: 2026-09-17
promoted_from: a-cadence-job-whose-healthy-state-is-red
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 422. A cadence job whose healthy state is red, and nothing that notices it repeating

Promoted from the proposal `a-cadence-job-whose-healthy-state-is-red`,
filed 2026-09-17 by milestone 311, which was minted to fix a path and found that the path was the
smaller half. *(Number provisional until the merge queue lands it.)*

It reads GitHub's run history, the same source `script/cadence-check` already reads.

Premise re-checked 2026-09-19 and still true, and the overdue run is now five weeks long.
`helpers/trunk-health.sh` still reports nothing about a repeated failure, and `script/cadence-check`
still asks only when a workflow last succeeded. `script/audits --due` still names `documentation`,
whose last sweep is 2026-08-17 and whose milestone trigger now reads +122 against a threshold of 10.
The security sibling is the one thing that moved: it was audited on 2026-09-17 and is no longer due,
so `documentation` is now the only standing example.

## In brief

`.github/workflows/audit-cadence.yml` reports "an audit is due" by failing. That is deliberate
and correct: its own header argues, at length, that an audit coming due is information about the tree
rather than a defect in a commit, which is why it is a scheduled workflow instead of a gate.

The consequence nobody priced is that red is now ambiguous. These two rows look identical in the
Actions tab:

```
2026-09-07  audits: 7 on record, DUE: documentation, security   exit 1     <- working perfectly
2026-09-14  FileNotFoundError: .../user/Cargo.toml              exit 1     <- broken for four days
```

And these four look identical to each other, which is the part that actually cost something:

```
2026-08-17  audits: 5 on record, DUE: documentation, security   exit 1
2026-08-24  audits: 7 on record, DUE: documentation, security   exit 1
2026-08-31  audits: 7 on record, DUE: documentation, security   exit 1
2026-09-07  audits: 7 on record, DUE: documentation, security   exit 1
```

An audit was overdue every week for a month, the mechanism said so on schedule every time, and no
audit was run. Milestone 92 built this tripwire to stop auditing depending on somebody remembering
to ask. The tripwire fired four times and auditing still depended on somebody remembering.

## Why `script/cadence-check` does not cover it

Milestone 238's check asks when a workflow last succeeded, and reports one whose last success is
more than fifteen days old. That is the right question for a job whose healthy state is green, and it
is why the mutation and undefined-behavior workflows were caught.

For `audit-cadence` it is the wrong question in both directions. A job that is *supposed* to be red
has no green to be stale against, so a firing tripwire is indistinguishable from a dead one; and a
job that has **never** succeeded is reported forever on a condition it can never clear, which
generates exactly the standing alarm that teaches a reader to stop looking. It caught milestone 311's
path bug by coincidence: this workflow had never succeeded, so it tripped the never-succeeded arm,
for a reason unrelated to the defect.

## What it reports

`helpers/trunk-health.sh` answers the question, from the read-only source `script/cadence-check`
already reads. On every `--once` pass it takes the last six scheduled runs of each scheduled
workflow through the per-workflow runs endpoint, because the all-workflows listing caps at 100
runs and no longer reaches a weekly's own last six. It groups the failed ones by the final line of
their failing step and prints two verdicts beside the DEAD and DUE rows. REPEAT names a line that
took three or more of those runs, with the line and the span of dates (UTC). CHURN is a separate
and less urgent finding, never folded in, for failures that disagree with each other. A single
line failing exactly twice stays silent until a third occurrence settles it. The watching loop
does not call the section, because each failing run examined costs two API reads;
`trunk-health.yml` relays the rows every five minutes as a warning annotation, the same treatment
its own BUGS gives a dead cadence. Reading the line is best effort, because `gh run view
--log-failed` is a screen rather than an API, and the script's BUGS says where that breaks first;
`helpers/trunk-health-repeat-selftest.sh` proves the thresholds, the strip and the watcher
exclusions offline.

Against the live specimen, on 2026-10-16 (UTC), the row read:

```
audit-cadence                    5 of last 6 failed       REPEAT: design/audit-reports/README.md says how, and which lens the last one lacked. (2026-09-07..2026-10-12 UTC)
```

Five of the six runs ended in `script/audits`' own epilogue, identical for six weeks, and the
sixth, on 2026-09-14, ended in a `FileNotFoundError` that fell out of the group on its own: the
two rows the Actions tab renders identically, separated by the only witness that saw both.

## The open question

Who is the report for, and what closes it? A repeat report on `audit-cadence` says "run the
audit", and running an audit is a lane and a day, not a fix somebody applies between tasks. Reported
to a maintainer session it becomes rung four again, a message somebody has to read; reported into a
pull request there is no pull request to hang it on. AGENTS.md's answer for work waiting on a person
is the `needs-architect` label and a `## What I need from you` comment, and neither has a home when
the finding belongs to no branch.

That is an architect's call, not a lane's, and it is the same question the steward's own "it
reported and never acted" failure of 2026-08-04 raised without settling. It stands on the
milestone's pull request as a `## What I need from you` comment carrying `needs-architect`, and the
pull request does not merge until it is answered; the mechanism above reports either way.

## What this does not propose

Not making the job green when an audit is due. A cadence checker that cannot report "overdue" is
decoration, and the workflow's own header and `design/audit-reports/README.md`'s `BUGS` both already
say that closing this by editing the index is the one thing that makes the mechanism a lie. The
ambiguity is the cost of a correct design, and the fix is a second signal rather than a quieter
first one.

## Index row

`.github/workflows/audit-cadence.yml` reports that an audit is due by failing, which is deliberate
and correct, and the consequence nobody priced is that red is ambiguous: a firing tripwire and a
`FileNotFoundError` are the same row in the Actions tab, and so are four consecutive weeks of the
same true report. An audit was overdue every week for a month, the mechanism said so on schedule
every time, and no audit ran, which is milestone 92's tripwire firing four times into a process that
still depended on somebody remembering. `script/cadence-check` cannot cover it, because a job whose
healthy state is red has no green to be stale against and a job that never succeeded is reported for
ever. `helpers/trunk-health.sh` now reports a repeat rather than a color: each scheduled workflow's
last six runs grouped by the failing step's final line. A REPEAT row fires when one line took three
or more, a CHURN row when the failures disagree; an offline selftest proves both against the real
audit-cadence history. What still makes this a block rather than a finished brief is who the report
is for and what closes it. Running an audit is a lane and a day, and there is no pull request to
hang the finding on.
