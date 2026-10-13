# No gate records when it fires: the appendices to milestone 531

[`0531-no-gate-records-when-it-fires.md`](../0531-no-gate-records-when-it-fires.md) carries the
three findings, a summary of each addition with its cost and limits, the baseline in brief, what to
weaken, what this lane refuses, what cannot be gated, and the ranking. This directory carries the
evidence and the full assessments its prose budget has no room for.

- [`ci-baseline.md`](ci-baseline.md): how the audit measured, why the GitHub Actions record is the
  firing record, the mined per-check baseline of 2026-09-21 with its tables, and the
  `script/icount --check` record.

- [`additions.md`](additions.md): additions A to D, each with what it would have caught, its cost,
  its false-positive risk and what it cannot check.
- [`limits.md`](limits.md): the `script/cadence-check` false positives workflow by workflow, and
  the three things no gate can check, argued in full.

*Name: provisional. The directory follows §212 (a prose budget)'s siting default. calef names
directories.*

- `ci-baseline` Name: provisional. It holds the parent's "What was measured, and how" and the mined
  baseline. calef names files.
- `additions` Name: provisional. It holds the parent's four "Addition" sections in full. calef names
  files.
- `limits` Name: provisional. It holds the parent's "What to weaken, with evidence" and "What cannot
  be gated" in full. calef names files.
