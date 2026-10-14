---
status: PARTIAL
raised: 2026-10-14
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 920. An outsider pass is launched pinned and floor-checked

*Name: provisional. The number 920 is provisional until the merge queue lands it, and the title,
the slug and both script names below are drafts. Branch
`milestone/920-an-outsider-pass-is-launched-pinned-and-floor-checked`.*

calef, 2026-10-14 (UTC), on outsider passes at risk 7 (the confinement claim is false): *"mint a
milestone for the launcher, the floor checker and the brief change that allows a completion round.
get rid of the blind passes."*

Reuse: `briefs/confinement-outsider-pass-launch.md`'s shell (a standalone clone at the pinned
commit, the prompt assembly) is the launcher's starting point. The test-run output `script/test`
already prints is the checker's run evidence. Nothing outside the tree counts claims-table rows
against a boot, so the checker is written.

## Index row

An outsider pass at risk 7 should fail only on what the reviewer found, not on procedure it could
not see. A launcher pins the model and the commit, a checker counts booted rows against the
27-of-34 floor, and a pass below it gets one completion round.

## Why

Criterion (c) needs two consecutive clean outsider passes, at least one non-Anthropic or human,
each meeting the booted-row floor in `briefs/confinement-outsider-pass.md`. The standard stays.
The last three clean passes missed it on procedure:

- Pass 7 booted 3 rows.
- Pass 8 (GLM 5.3) reported 27 booted, and its own table showed 25: rows 10 and 11 were host
  only. Nobody counted before the report was written. Its prompt was a milestone number, so it
  read the lane's instructions as its own.
- Pass 9 was meant for Fable and was served by `claude-opus-4-8`, and it booted 20.

A count the reviewer can run, a launch that cannot send the wrong prompt or the wrong model, and a
way to finish a short pass instead of discarding it close all three.

## The brief change

The brief change is calef's ruling and is in force in both briefs:

- Every pass is informed, milestone 800 (a non-Anthropic model attacks the confinement claim)'s
  posture. The blind instructions are gone from both briefs.
- A completion round. A pass below the floor goes back, once, to the same reviewer in the same
  session with the checker's list of short rows, and counts if the floor is then met. One round,
  because a second turns the lane into the reviewer's supervisor, and a reviewer that needs two has
  not attacked those rows.

## Still to build

### The checker: `script/booted-rows` (name provisional)

A report, so a noun. Given a pass's checkout, the pinned commit and the run logs, it prints each
claims-table row with its grade. A row that does not count gets its reason: `row 10: host only`,
`row 19: rerun of the row's cited test`, `row 31: no pass-written test`, `row 22: timed out on
every ISA (inconclusive)`. Its last line is `booted N of 27, floor met` or `floor short by K`, and
its exit status says the same.

- Pass-written tests are the test functions added between the pinned commit and the pass's head
  under `system_tests/` (and any other test tree the diff touches). A test already at the pinned
  commit is a rerun, whatever it attacks.
- Rows come from the test's module doc comment. The checker reads one machine-readable form; this
  milestone picks it (for example `Attacks rows: 4, 5, 9`) and adds it to the reviewer brief's
  Mechanics. A test with no row reference counts for nothing and is listed.
- Run evidence is `script/test` output, one log per ISA, from the reviewer, the lane's
  reproduction or CI. A row is booted when one of its pass-written tests ran to a pass or an
  assertion on at least one ISA the row applies to. A timeout is inconclusive.
- Rows 12 to 18 are exempt and graded `host`. Rows 1 to 3 count when the results file holds the
  written acceptance of pass 7's test, or a strengthened test booted.
- The floor and the exempt rows are read from one place both briefs cite, so a growing table
  moves the count without an edit here.

The reviewer runs it before it reports. The lane runs it on its own reproduction logs before it
records, and the pass note quotes its output.

### The launcher: `script/launch-pass` (name provisional)

An action, so a verb. It starts one reviewer and records what actually ran.

- The prompt is the reviewer's brief from `briefs/confinement-outsider-pass.md` and the runner
  section, nothing else. It refuses a prompt that is a milestone number or names a roadmap block.
- It makes the checkout: a standalone clone with no remote, on one branch at the pinned commit,
  as the launch brief does by hand. Since passes are informed, it scrubs nothing.
- It pins the model on the harness's command line: opencode for GLM 5.3, Claude Code for Fable.
  It reads the model id the session reports (Claude Code's `init` message and per-model usage;
  opencode's session record) and stops the session before the first attack if that id differs
  from the pin.
- It writes a launch record outside the repository: harness and version, model pinned, model
  reported, session id, pinned commit, the prompt's SHA-256 and the start time. The pass note
  copies it.
- `--complete <launch record>` resumes the same session with the checker's short-row lines as the
  whole message, and refuses a second completion for the same record.

Where a harness cannot resume a session or report its model (a cloud runner the launcher does not
start), the launcher says so and refuses, rather than recording a guess. Whether Fable runs on
patagonia or in a cloud runner is this lane's call, and the block records which and why.

## Done means

- Both scripts exist, with selftests run by `script/lint`, and each has its `notes/scripts.md`
  entry with `EXAMPLES` and `BUGS`.
- The checker, run on pass 8's and pass 9's results at their recorded commits, reproduces the
  counts their notes give (25 and 20) and names the short rows.
- Pass 10 (GLM 5.3) and pass 11 (Fable) can both be launched through the launcher at one pinned
  commit, and the checker reports both floors. Those passes follow this milestone; each is minted
  by its own lane, as milestone 913 (an eighth outsider pass attacks the confinement claim) was,
  with `milestone_dependencies: 920`.
- `briefs/confinement-outsider-pass-launch.md` is reduced to what the launcher does not do.

## BUGS

- Until the checker lands, the floor is counted by hand, and a hand count can disagree with the
  reviewer's own table. The brief tells the lane to count from the per-row table, not the report's
  total.
- The completion round's message is short rows only, but a reviewer told "row 10 is short" learns
  where the lane looked. It learns nothing the brief did not already demand, since every bootable
  row must boot.

## Follow-on

- **Recorded.** Passes 10 and 11 are named under *Done means* and wait on this milestone.
