# Milestone 531 appendix: what was measured, and the mined CI baseline

The evidence behind [milestone 531 (no gate in this tree records when it
fires)](../0531-no-gate-records-when-it-fires.md): how the audit measured, why Actions is the firing
record, the per-check baseline mined from it on 2026-09-21, and what it could not establish. The
main page is complete without this one, which exists to verify or challenge it.

## What was measured, and how

- Firing evidence per gate, from the git history. Every `script/` entry point and every
  `script/lint` check was searched for commits that fix a real defect it found, as distinct from
  commits that add or repair the gate and commits that fix a cosmetic violation to satisfy it. The
  method works here only because `allow_squash_merge` is false on this repository, so a lane's
  own commits survive on `main` and a gate that fired pre-merge usually leaves a fix commit behind.
- `script/lint` end to end, from a lane worktree, warm cache: 84 seconds wall, exit 0. The
  record-parsing legs are the cheap end: `roadmap --check` 0.27 s, `decisions --check` 0.08 s,
  `citations --check` 1.06 s, `fatal-risks --check` 0.65 s, `names --check` 0.13 s,
  `falsifications --check` 0.61 s. The ten clippy passes are the other 80 seconds.
- The GitHub Actions failure record, mined: `gh api
  'repos/nifeos/nife/actions/runs?status=failure' --paginate`, then `/jobs` for each run, then
  `/logs` for each failed `clippy` and `bench` job.
- The live state of the machine, because two candidates are about state no file describes.

## Why the firing record is not silent

A blocking gate fires on a lane branch that is later force-pushed away, and git loses the evidence
with it. GitHub Actions does not: it keeps a run per commit whatever happens to the branch
afterwards. That is what the ledger has to buy over what is already there, and the mined record
below shortens the list of checks with no trail.

## The mined baseline, 2026-09-21

909 failed workflow runs are retained, the oldest from 2026-07-23, which is the day after CI was
added to this repository, against 14,295 runs in total. 894 of them name at least one failed job,
1,312 failed jobs in all. So the record reaches the whole of this project's CI history rather than a
recent window.

Failed jobs by name, whole history:

| failed jobs | check |
|---:|---|
| 327 | `build + test (host + QEMU)` |
| 309 | `clippy` (which is `script/lint`) |
| 213 | `cpu matrix (riscv64 across QEMU CPU models)` |
| 104 | `coverage (host crates)` |
| 99 | `bench (icount regression tripwire)` |
| 67 | `architect hold (needs-architect label)` |
| 55 | `rustfmt` |
| 33 | `fastpath footprint (the IPC path must stay L1i-sized)` |
| 14 | `supply chain`, and 14 `stack frames` |
| 5 | `verify (Kani proofs)`, and 5 `undefined-behavior check` |
| 4 | `re-falsify the harnesses this change can reach` |

And the logs still resolve which sub-check fired. All 309 `clippy` logs were fetched and none
had expired, so the last `==>` marker in each names the `script/lint` check that was running when
it exited. Nineteen distinct checks account for all 309:

| firings | first .. last | check |
|---:|---|---|
| 78 | 2026-08-04 .. 2026-09-20 | roadmap status |
| 50 | 2026-08-17 .. 2026-09-16 | markdown links and the notes index |
| 37 | 2026-08-04 .. 2026-09-16 | naming conventions |
| 27 | 2026-08-05 .. 2026-09-15 | em-dashes |
| 27 | 2026-08-02 .. 2026-09-09 | decisions |
| 21 | 2026-07-25 .. 2026-08-19 | clippy: host crates + xtask |
| 18 | 2026-08-17 .. 2026-09-01 | counted claims |
| 14 | 2026-08-05 .. 2026-09-03 | citations |
| 13 | 2026-07-23 .. 2026-09-19 | the aarch64 clippy pass, under its three successive names |
| 8 | 2026-09-20 .. 2026-09-21 | new citations say what they cite |
| 4 | 2026-08-13 .. 2026-08-18 | spelling |
| 2 each | | TODO markers cite a milestone; script docs; every fence names its counterpart |
| 1 each | | rustdoc; the separate-workspace clippy pass; the riscv64 shell-feature pass |

Thirty-two of the 47 checks have never failed a CI job. The list includes `conflict markers`,
`sh -n`, `module-wide dead-code suppression`, `unsafe fn contracts`, `environment names in prose`
and the Kani-harness clippy pass. Those are among the checks with the *best* documented defect
catches in git. That is not a contradiction, it is the structural bias stated as a measurement: a
check that fires locally, gets fixed, and is pushed green never reaches Actions at all. So the
Actions record systematically undercounts exactly the gates that work earliest.

The list of checks with no trail anywhere shortens from sixteen to fourteen. `script docs` and
`every fence names its counterpart` each failed a CI job and are no longer trace-free. Every other
check in the Actions list already had commit evidence.

The newest gate has the highest rate. `new citations say what they cite` was added 2026-09-19
and has fired eight times in two days, which is the birth spike this tree's whole record predicts
and the pattern `design/fatal-risks/README.md` records for proofs: a mechanism pays most on the
day it is written.

## What could not be established

Fourteen of the 47 `script/lint` checks have no trail in either record. They are ratchets: each
encodes a fix already made, and nothing says any of them later caught a new instance. That is not
evidence against them, because a ratchet that holds leaves no trace by design, and it is exactly the
case the first recommendation exists to settle.

`script/icount --check` fires, though no commit records it catching a regression. Of its 99 failed
`bench` jobs, 43 carry the tripwire's own `CHECK FAIL` line, across seven distinct days between
2026-08-14 and 2026-09-15, naming specific benchmarks: `spawn_el` 21 times, `map_new` 16,
`yield_switch` and `ctx_switch` 4 each. Absence of a commit message is not absence of a firing,
which is the reasoning this whole proposal argues against.

Two further facts are worth having, both measured rather than argued. The

other 56 failures are not the tripwire, so the majority failure mode of a required check named
`bench (icount regression tripwire)` is something other than the tripwire, which is a fact about
what its red means. And 43 firings across seven days is on the order of seven episodes, since the
clusters (13 on 2026-08-27, 10 the next day) are one condition re-failing rather than ten. The
record cannot tell a retry from a new event, a regression from a baseline that drifted under a
toolchain bump, or either from a lane rebasing. That distinction is what a ledger row can carry and
a job conclusion cannot.
