# Milestone 531 appendix: each addition, assessed in full

The full assessment of the four additions [milestone 531 (no gate in this tree records when it
fires)](../0531-no-gate-records-when-it-fires.md) proposes. Each answers the same questions: what it
is, which recorded failure it would have caught, its cost, its false-positive risk, and what it
cannot check. The main page is complete without this one, which carries each addition's summary and
the ranking.

## Addition A: a gate-firing ledger

What it is. Every gate that renders a verdict appends one line on failure: UTC timestamp, gate
name, sub-check name, branch. Local file, git-ignored, never read by any gate. `script/lint --fired`
(or its own entry point) reports firings per check over a window and, more usefully, the checks that
have never fired.

Which recorded failure it would have caught. None directly, and that is the honest statement: it
is an instrument, not a gate. What it would have prevented is the shape this whole audit ran into.
Milestone 191 (did the proofs catch the bugs? a retrospective of every real defect against the
harness that should have found it) had to assemble an eighteen-row corpus by hand. It did so to
establish that no Kani harness has ever caught a defect after the day it was written. That finding
is the single most load-bearing fact in `design/fatal-risks/README.md`.

The equivalent fact about the lint checks is obtainable from Actions (see [the mined
baseline](ci-baseline.md)), and the ledger is still worth building, for three reasons:

- The structural bias, which is the big one and which the mined data confirms rather than
  predicts. Actions sees only the failures that escaped local gating. Thirty-two of the 47 checks
  have never failed a CI job, and that set contains several of the checks with the best documented
  defect catches, because a lane ran `script/lint`, saw red, fixed it and pushed green. Those
  firings are the cheapest and earliest a gate ever achieves and they are exactly the ones the
  Actions record cannot count. Any ranking built on Actions alone rewards the gates that fail
  late.
- Retention. The mining worked because this repository is 60 days into a 90-day log retention and
  nothing has expired yet. That window closes: the logs behind the 2026-07 and 2026-08 firings
  begin disappearing from 2026-10-21, and after that the per-check attribution is gone for good.
  Run and job rows may outlive their logs, which is untested here; the `==>` marker will not.
- Cost. The baseline took 909 run queries plus 408 log fetches and about half an hour of wall
  clock. The ledger is one append on failure.

So the ledger's first task is to absorb the baseline rather than start from zero, and that
should happen before the logs expire. The mining commands are in this proposal's own history and in
[the measurement section](ci-baseline.md#what-was-measured-and-how).

What it costs a contributor per run. One append on failure, zero on success. Nothing on the
success path, which is the path that runs.

False-positive risk. None available: it renders no verdict and cannot fail a build. The risk is
the opposite one, a misread. Two biases have to be written at the report rather than discovered.
`script/lint` exits on first failure, so a check late in the file is under-counted relative to an
early one. And a lane that re-runs the same failure five times produces five rows.

What it cannot check. It counts firings, never value. On this ledger the em-dash check would sit
near the top. The `sh -n` check, which caught a genuine bash 3.2 parser bug that ShellCheck had
already passed, would sit near the bottom with one row. A firing count is the cheap half of §61 (a
lint is adopted on evidence from this tree, not on its description)'s question and not the whole of
it, and it should be labeled that way where it prints. It also sees only local runs, since CI
runners are ephemeral and this proposal deliberately does not ask the workflows to upload their rows
in a first version. That is the exact complement of what Actions holds and not a duplicate of it, so
the two together cover the surface and neither alone does. The ledger sees what was fixed before a
push, Actions sees what escaped, and a check quiet in both is the only one that has genuinely never
fired.

## Addition B: a gate carries a selftest, or it is not evidence

What it is. §134 (a harness carries a machine-replayable falsification record, or it is not
evidence)'s convention, applied to checks as it is to proofs. Each `script/` entry point that
renders a verdict grows a `--selftest` that runs it against fixtures which must fail and fixtures
which must stay quiet. And `script/lint` gains one check: an entry point with a verdict and no
`--selftest` is refused. The shape is already in the tree and does not need designing;
`script/fatal-risks --selftest` is the worked example and it costs 0.65 s combined with its
`--check`.

Which recorded failures it would have caught. `script/fmt` ignoring `--check`, which the record
calls the same defect as the `tee` fail-open and which went unnoticed for months. The `--shard 4/4`
argument error, which is a fixture that must produce a shard and did not. `script/citations`' input
and reading patterns disagreeing about lettered citations, which is precisely a "must fail" fixture
nobody wrote: three files in the tree are unreachable to that gate today and it was found by
accident. `script/swish-check` red on `main` on both architectures with nothing saying so. And
prospectively, the class `notes/check-inventory.md` closes on: *"Milestone 233's `login` was found by
somebody asking what a passing check proved, and that remains the only known way to find the next
one."* A selftest is that question, asked once, in a form that keeps being asked.

What it costs a contributor per run. A fixture sweep is milliseconds; the measured combined cost
of the one that exists is 0.65 s. The real cost is at authoring time, and it is the point rather than
a side effect: a gate is roughly a third more work to write, and the third buys the evidence that it
works.

False-positive risk, and the method. Near zero by construction, since a failing selftest means
the gate is broken rather than the tree. The residual risk is a selftest that rots into a nag, and
the estimate is from the one instance: `script/fatal-risks --selftest` has eleven fixtures and no
recorded false rejection since 2026-09-11. One data point, stated as one.

What it cannot check. That the fixtures cover the failure space. A selftest proves a gate *can*
fire, never that it fires on everything it claims, and the record already holds the sharper version
of this: a harness that restates the reader's own inequality cannot detect that the inequality is
wrong. A selftest written by the same person in the same hour as the gate inherits the same blind
spot, which is what milestone 191's survivorship finding says about proofs and says equally here. It
also cannot reach a gate whose subject is the live machine: `script/crate-probes` needs the network,
`script/qemu-check` needs an emulator, and a fixture for either is a fixture about a stand-in.

And it wants a bounded adoption, not a sweep. §61's corollary applies exactly: nothing goes in
"to see what it finds", because adding this check is a commitment to write every missing selftest
first. The defensible shape is the tree's own ratchet: the gate refuses a new or modified entry
point without one, the existing surface is a worklist, and the worklist is `script/`'s own listing.

## Addition C: one watcher over the lane fleet

What it is. One script, run from `helpers/trunk-health.sh` the way `script/cadence-check` already
is. It reports three readings: worktrees with uncommitted work and no commit inside a window; free
space on the volume against a floor; worktree count and total `target/` footprint including the main
checkout's.

Explicitly not a gate, and the precedent decides it rather than taste.
`helpers/lane-claim-check.sh` faced the identical question and answered it in its own header: nothing
should fail a build over this, because the lane that most needs telling is mid-work, and `script/lint`
was refused for exactly that reason. The same applies here twice over, since the subject is another
worktree's state and a gate that fails a lane over a sibling's dirt would be the "only ever rejects
legitimate work" signature acquired on purpose.

Why one script and not three: they are one question, "is any lane's state at risk right now", and
they share their whole mechanism. One `git worktree list`, one `status --porcelain` per entry, one
`df`. Three scripts would be three cadences to keep alive and three headers to keep true.

What it costs. A poll, on a watcher that already polls. The `du` leg is the only expensive
reading and it is the one that can be sampled rather than run every pass.

False-positive risk, estimated with a stated method. Run today against the live fleet, the
uncommitted-work leg names 2 of 35 worktrees and both are true positives on inspection. The
shape to design out is the one `lane-claim-check.sh` already priced: a lane legitimately mid-edit.
A window measured from the last commit rather than from the last write handles it, and the same
script's five-times-measured grace period is the precedent for choosing the number.

What it cannot check. Whether uncommitted work is *wanted*: generated artifacts a lane has
deliberately not committed look identical to work about to be lost, and one of today's two hits is
regenerated SVG output. It cannot see growth during a long run, so a `df` reading is a snapshot and a
build that fills the volume in ten minutes passes it and then fails. It reports only the machine it
runs on. And it dies the way every watcher here dies, which `notes/merge-queue.md` records as the
unfixed defect one level up: neither existing watcher reports its own death.

## Addition D, smaller: refuse `git stash` in a shared checkout

`AGENTS.md` records the 2026-08-26 case where one session's stash entry was popped into another
lane's tree, and prescribes a patch file instead. That is rung four for a hazard with a rung-two
mechanism available, and the mechanism was verified here rather than recalled: git's
`reference-transaction` hook fires on `refs/stash` and can abort the transaction. Tested in a scratch
repository: the stash is refused, the working tree is left exactly as it was, and ordinary commits
are unaffected. Cost measured over 20 commits with and without the hook: 0.364 s against 0.809 s,
so about 22 ms per ref transaction, which is process spawn rather than the check. A fetch is one
transaction and one invocation, not one per ref.

It is ranked last because the recorded incident lost nothing, and it is included because it is the
only place in this audit where a rung-four rule can move up a rung for ten lines of shell. It cannot
reach the other two clobbers in the record, `git reset --hard` and `git checkout <file>`, because
both are legitimate commands whose damage is indistinguishable from their intent.
