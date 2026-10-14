---
status: BUILT
raised: 2026-08-03
built: 2026-08-04
---
# 85. Mutation testing over the host crates

Built 2026-08-04 (PR #76). Raised 2026-08-03, same survey as 79.

Everything below was done and is on `main`. `script/mutation` ran **5,551 mutants over 38 host
crates** (92.4% of the viable ones killed). Every one of the 391 survivors was triaged with a ledger
recording whether it was killed by a new test, proved equivalent, or confirmed a hang.
`.cargo/mutants.toml` carries each exclusion's reason, `.cargo/mutants-baseline.txt` as the
machine-readable baseline, `notes/mutation-testing.md`. `.github/workflows/mutation.yml` runs four
shards weekly and reporting against that baseline. It is a report, not a gate, as intended.

The coverage job answers "did this line run under a test". It cannot answer "would any test notice
if this line were wrong", and the second question is the one a test suite exists for. cargo-mutants
answers it by mutating the code and re-running the tests. The survivors, mutations no test caught,
are a worklist sorted by exactly the property this project cares about. The exhaustive
suites (`ntp_proto`, `gpt`) should score near-perfectly, which is itself a calibration check on the
tool. The interesting results will be in the middle of the tree.

The work: one full, time-boxed run over the host crates. Triage every survivor into either a test
worth writing or an exclusion recorded in `.cargo/mutants.toml` with a reason (config, not a code
dependency, per §46 (thin primitives or whole subsystems)). A note recording the baseline. Then a
weekly scheduled workflow that reports
against that baseline. A report, not a gate, until the weekly numbers prove stable enough that a
new survivor deserves to fail something.

## Follow-on

- **Recorded.** `notes/mutation-testing.md`: it is a report and not a gate, so a new survivor fails
  nothing. Deliberate until the weekly numbers prove stable enough to be worth blocking on.
- **Milestone 238.** The weekly workflow this milestone shipped never once succeeded, which
  milestone 238 (two scheduled checks have never once succeeded) took up. Four scheduled
  runs on 2026-08-10, -17, -24 and -31 gave four failures and zero reports. That left
  `design/fatal-risks/README.md` risk 3 reading green on a number nothing was refreshing.

## Index row

Coverage reports what ran; cargo-mutants reports whether a test would notice a change, which is
the claim the suite actually makes. A weekly, time-boxed job with a recorded baseline, not a PR
gate
