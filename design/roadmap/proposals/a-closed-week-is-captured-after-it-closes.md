---
status: PROPOSED
raised: 2026-10-19
milestone_dependencies: 519
decision_dependencies: none
machine_requirements: the machine whose session records `script/effort` reads
specific_machine: none
needs_person: yes
---
# A closed week's effort row is captured after the week closes

Raised by the lane `maintainer/effort-w42-capture`, while repairing the 2026W42 row of
`notes/project-metrics/effort.csv`. Name: provisional.

Reuse: the existing `script/effort --update`, `script/metrics --backfill-measure`, `script/claim`
and the snapshot job already on the machine. Nothing new is written except a wrapper and a check.

## The failure

`effort.csv` gains a row only when a person runs `script/effort --update` on the machine that ran
the lanes and commits the result. Nothing prompts that run at the right time. Every capture so far
has happened when someone noticed a gap.

The case that raised this: W42 was captured on 2026-10-13, two days into the week, and not again.
The committed row held 18,663 turns. The full week, read on 2026-10-19, holds 35,118. The mean
context per turn moved from 197,648 to 215,892 tokens and the peak from 643,048 to 966,683. Two
days of a week read as a quiet week with no session near the context wall, and neither was true.

`script/effort --check` did not catch it, and as written it cannot. It asks whether the newest
committed week is the current one. A row written on a Tuesday satisfies that for the rest of the
week. After the week closes, the check fires because the new week has no row, which is the same
alarm as "nothing was ever captured". It cannot tell a complete row from a partial one, because
the committed file does not say when a row was read.

## Options weighed

### A `script/metrics --check` failure when the newest closed week has no effort row

Refused.
It reads the same committed file as `script/effort --check` and has the same blindness: W42 had a
row the whole time. `script/metrics --check` also has no caller, by its own header, because every
merge moves the current week. Adding a condition to a check nothing runs is rung four.

### A scheduled local job that opens a pull request each Monday

Recommended. Only the machine
with the session records can produce the number, so the only mechanism that removes the person is
one that runs there. It fires without being remembered, which is rung two's property, though it
is not a gate.

### Recording when each row was read, and checking it

Recommended alongside, as the backstop for
the job failing. Without it, a Monday job that silently stops looks the same as one that ran.

## The proposal

### 1. A weekly capture job on the machine that ran the lanes

A launchd agent beside the existing six-hourly snapshot job, firing early each Monday UTC.
launchd runs a missed calendar job on wake, so a sleeping laptop delays the run rather than
skipping it. The job:

1. Cuts a throwaway worktree from `origin/main` with
   `script/claim maintainer/effort-<week> --debt-paydown`, so it never writes into a checkout a
   lane or a person is using.
2. Runs `script/effort --update`, then
   `script/metrics --backfill-measure context-per-turn` and
   `script/metrics --backfill-measure cost`. Those are the two measures that read `effort.csv`.
   `--update` alone recomputes only the current week, so a closed week's row needs the backfill.
3. Commits only `effort.csv`, the two CSVs and `context-per-turn.svg`, pushes, and marks the draft
   ready. The commit body states the closed week's turns, mean and peak.
4. Exits nonzero, with the reason in its log, on any step that fails.

It does not merge. The pull request goes through the queue like any other, and whoever arms
ruled, green, independent pull requests arms this one.

The job needs `gh` credentials and a Python with `tomllib` on its `PATH`, as the snapshot job
already has. A Monday with nothing new (no lanes ran) still opens the pull request, because
"zero requests" and "the job did not run" must not look alike.

Installing it is a change to the machine, so calef approves it. Its shape and the reasoning live
in `notes/project-metrics.md` beside the snapshot job's; unit-specific details are kept off-tree
by the maintainers.

### 2. `script/effort --check` asks whether the newest closed week was read after it closed

The check fails when the newest closed week's row was written before that week ended. Two ways to
know when a row was written:

- A `captured_at` column in `effort.csv`, the UTC time of the `--update` that last raised the
  row. Exact, and it survives any clone. It is a format change to a file `script/metrics` also
  reads, so it is an architect's call. `script/metrics` reads by header name, so the new column
  would not break it.
- The commit time of the last change to `effort.csv`, from git. No format change, but a
  shallow checkout answers with the head commit's time, and `trunk-health.yml` may check out
  shallow. It would pass exactly where it is needed most.

The first is the stronger one and is recommended. Where the snapshot cache is present, `--check`
can also compare the cache against the committed row and fail when the cache is larger, which
catches a job that ran on Monday before the week's last records arrived.

`helpers/trunk-health.sh` already relays `--check`. With this change its row would say which week
is partial instead of only that the current week has no row.

## What this does not fix

The job runs on one laptop. A week the laptop is away for longer than the transcripts survive is
still lost, and the six-hourly snapshot remains what makes that a delay rather than a loss.

## Open questions for calef

- Approve the launchd job's shape, and where its plist lives.
- Whether `effort.csv` takes a `captured_at` column.
