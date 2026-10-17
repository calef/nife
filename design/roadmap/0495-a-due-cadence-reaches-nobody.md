---
status: BUILT
raised: 2026-09-19
built: 2026-10-15
promoted_from: a-due-cadence-reaches-nobody
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 495. A cadence that says "due" reaches nobody, and the watcher cannot tell due from dead

Two scheduled cadences exist to say "a person must now do something": the stranger cadence
(`script/stranger-test --due`) and the audit cadence (`script/audits --due`). Both say it by going
red in the Actions tab. That red is their correct output, not a failure. The one watcher a person
reads, `helpers/trunk-health.sh`, asked `script/cadence-check`. That script reports a workflow only
as DEAD, once it has gone fifteen days without a success. A job correctly reporting due and a job
that is broken therefore arrived as the same verdict, a fortnight late. The audits sat that way for
five Mondays (2026-08-17 to 2026-09-19) before anyone acted.

## What was built

`helpers/trunk-health.sh` gained a `due()` section beside its `DEAD:` lines. It asks the scripts
themselves, not the workflows that relay them. The function name is provisional, like the script's
own. Both `--due` modes read records already on disk: the stranger note's run headings, and the
audit index. They need no network, no toolchain and no credential. They answer in seconds on the
checkout the watcher already holds. No new scheduled workflow, no `issues: write`, no second
channel. The two shapes this declines stay declined for their recorded reasons. Teaching
`script/cadence-check` a third verdict would parse step summaries and still inherit the weekly
lag. An issue from the workflow needs `issues: write` and is a second channel to the watcher. This
is deliberately not a gate and not in `script/lint`. The reason is the one
`script/cadence-check`'s header records: a due cadence is not a defect in the commit that happens
to trip it.

Rows keep the watcher's existing three-column shape, beside the DEAD lines:

```
audits                           2 overdue                DUE: documentation, security
stranger-test                    last run 2026-08-19      DUE: Run 6 was 57 days ago and the cadence is 30.
```

A `--due` mode that exits nonzero without a DUE sentence is a record that would not read. That is
a malformed or missing one, and it prints nothing on stdout. It gets its own row instead, state
`cannot ask`, verdict `DUE: unknown`, naming the `--check` that explains it. Silence there would
recreate the gap this closes, in miniature. `--once` prints the rows whenever they stand, being a
snapshot rather than a watcher.

### What a transition means

The verdict speaks on transition, like the watcher's RED/GREEN and DEAD lines. It speaks when
something becomes due, when it clears, and not on the ninety-second polls between. A cadence
standing due for weeks is one fact, not a standing alarm. Clearing prints `nothing is due any
more` once. It means the work was done, or that the record moved underneath. A stranger run was
written up, or the audits ran and their rows were added. Both scripts' own BUGS sections name the
edit-to-silence hole. This inherits it rather than policing it.

The watcher keeps no state between runs. `prev_due` lives in the watching shell and dies with it.
So a transition means newly observed, not newly due. A restart announces whatever stands due on
its first pass. A due that clears while nothing is watching is never reported. That is the honest
direction to be wrong in. It has been true of the RED/GREEN and cadence halves all along. It is
now stated once, in the script's BUGS list.

## How it was proven

Against real state in the lane's worktree, 2026-10-15 (UTC): `--once` printed the audits row.
Documentation and security were overdue. No stranger row printed, run 6 being 26 days old with the
cadence at 30, due 2026-10-19. The transition half ran in a throwaway clone, against moved
records, with the watch loop's `sleep` shimmed to two seconds. The first pass announced the
standing due state once, and the next passes stayed silent. Restoring run 6's real date cleared
the stranger due. The next pass announced only the audits row, with no recovery line. Answering
both audit kinds the way a person answers them emptied the list: dated rows and baselines from
`script/audits --baseline`. The next pass printed `nothing is due any more` exactly once, and
further passes stayed silent. The two cannot-ask shapes, a missing run record and a malformed
index row, each produced their `DUE: unknown` row.

## BUGS

- The audits row inherits `script/audits`' own defects. It says which kinds are due because the
  script says so. A drifted count (see Follow-on) or a report that found nothing reads the same as
  a real one. The judgment triggers, the `?` lines, are not consulted: a kind due on judgment
  alone produces no row.
- `DUE: unknown` sits in a column a reader skims for `DUE:`. The state column carries
  `cannot ask`, but a prefix-based skim reads the row as due. This is deliberate: an unreadable
  record must reach the same stream as a due one, and the row names the `--check` that says which
  it is.
- A machine asleep is a watcher not watching, inherited from the watcher's own delivery. The
  ninety-second notice is a line in the watcher's output, which a maintainer session reads. It is
  not a page and not an issue.

## Follow-on

- **Done.** The cadence stream cannot empty on a pass that could not ask. `helpers/trunk-health.sh`
  captures the `script/cadence-check --quiet` call and prints a `cannot ask` row when it exits
  nonzero with no rows, in the DUE section's row shape. The `prev_cadence` compare therefore reads
  cannot-ask as a state of the stream: it speaks on the transition into it and stays silent while
  it stands. Its recovery line fires only when a real answer empties the stream.
  `helpers/trunk-health-cadence-selftest.sh` (name provisional) holds the failure cases and runs in
  `script/lint`.
- **Done.** The restart caveat is a BUGS bullet in `helpers/trunk-health.sh`, stated once for all
  three transition memories.

## Index row

The trunk watcher now asks `script/stranger-test --due` and `script/audits --due` directly on every
pass, and says `DUE:` on transition beside its `DEAD:` lines. A cadence saying "a person must now
do something" reaches the watcher's output within one poll. It no longer arrives as DEAD, a
fortnight after the workflow went red.
