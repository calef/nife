---
status: PARTIAL
raised: 2026-10-12
milestone_dependencies: 824
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 913. An eighth outsider pass attacks the confinement claim

*(Minted 2026-10-13 (UTC), at 04:03, by lane `maintainer/mint-eighth-outsider-pass`, the way
milestone 903 (a seventh outsider pass attacks the confinement claim) was minted. This line said
2026-10-12 until lane `maintainer/outsider-passes-eight-and-nine` corrected it to the commit's UTC
date. The number 913 is
provisional; other lanes are minting nearby, so expect renumbering at merge. Title, slug and every
name below are drafts.)*

Reuse: the brief is `briefs/confinement-outsider-pass.md`, unchanged, and through it milestone 800
(a non-Anthropic model attacks the confinement claim)'s counting rule, report format and refusal
log, so the passes compare row for row. The isolation is milestone 633 (an outside agent attacks
the confinement claim)'s: this pass is blind. The reviewer is GLM 5.3, by calef's ruling below.

## Index row

The eighth outsider pass at risk 7's confinement claim, held to the booted-row floor calef ruled on
#2010: 27 of 34 rows booted, rows 12 to 18 exempt as host proofs. It counts an attack only when it
boots, and keeps the standing refusal log.

## Why

Risk 7 (the confinement claim is false) is AMBER. Its criterion (c) for green is two consecutive
independent attacks with no escape on a shipped path, at least one by a non-Anthropic model or a
human. The count is 0: the seventh pass found no escape but booted 3 of 34 rows, and calef ruled on
#2010 (2026-10-12 UTC), option (b), that it does not count. The same ruling set the floor this pass
must meet. Its one near miss, the retype gap, is milestone 824 (a retype mints no right its budget
lacks), so this pass runs on a tree where 824 has landed.

## calef's ruling, 2026-10-13 (UTC)

Asked how criterion (c) is to be met, calef ruled shortly before 05:30 UTC: "Yes, record it and prepare
both briefs. We will likely run GLM 5.3 here and Fable in Claude's cloud runner." The terms below
are the maintainer's relay of what he agreed to; lane `maintainer/outsider-passes-eight-and-nine`
recorded them.

- Two passes. This one, by GLM 5.3, launched by calef on patagonia; and
  milestone 914 (a ninth outsider pass attacks the confinement claim), by Claude Fable in Claude's
  cloud runner, started by the maintainer. GLM is the non-Anthropic half.
- One commit. Both attack the first main commit after #2035 (milestone 824) lands, the merge
  commit that brings 824 to main. It is pinned at launch and named in each pass's note. Both
  milestones depend on 824 for that reason.
- Consecutive. Nothing changes between the two, so they are consecutive even if they run at
  the same time. Running them together against one commit is allowed, and it keeps each blind to
  the other's findings. The brief's "never in parallel" is about test runs within one pass.
- Reset. If either finds an escape on a shipped path, the count resets: fix it, then both run
  again.
- Contention. If this pass runs on patagonia while another pass or lane loads it,
  timing-sensitive results are suspect. The cloud runner spares pass 9 that.

The launch, with each pass's one prompt, is `briefs/confinement-outsider-pass-launch.md` (name
provisional).

## The attack

Run by a fresh agent under `briefs/confinement-outsider-pass.md`. The lane briefs, collects,
reproduces each claimed escape by test or boot, and records. It does not attack and does not hint.

- Posture: Blind, milestone 633's: the reviewer gets a scratch copy of the claims table alone
  and the source tree, and the brief's list of what not to open.
- The reviewer's model: GLM 5.3 on patagonia, ruled above; the harness is calef's (pass 4 used
  opencode). calef starts it with the one prompt the launch brief assembles. The reviewer works in
  a standalone clone at the pinned commit, with no remote and one branch.
- The floor: Every row a boot can reach, 27 of 34 (calef on #2010: "27 seems right"). A row
  counts as booted when a hostile test written for this pass ran under QEMU on at least one ISA it
  applies to. Rows 1 to 3 may carry pass 7's tests (`seventh_pass_delegation_tests`,
  `seventh_pass_retype_tests`) when the reviewer accepts or strengthens each in writing. Row 3's
  test is a held assertion since 824, not the characterization pass 7 left.
- Reproduction: The lane reruns every booted row's test itself, one ISA's QEMU at a time. A
  timeout seen while patagonia was loaded is inconclusive until rerun on an idle machine; the
  reviewer records `uptime` beside each leg so the lane can tell.
- The lane's role: pin the commit, assemble the prompt, hand it to calef, then collect the
  clone's commits and results, reproduce each claimed escape by test or boot, and record. It
  does not attack and does not hint.

## Result

PARTIAL. The pass ran on 2026-10-13 (UTC) and is recorded in `notes/confinement-outsider-pass-8.md`
by lane `lane/pass-8-collect`, with the per-row table, the reproduction and the refusal log. What
calef needs to rule on whether it counts toward criterion (c):

- No escape. Four near misses: two unreachable or fail-closed port corners, a `component_plan`
  provisioning check that is missing, and one that is a deliberate design. Each has a `BUGS` entry
  or a doc line at its code.
- The breach. calef started the reviewer with "Do milestone 913" in the maintainers' workspace. It
  read this block, 914's block and the launch brief before its prompt, and also ran a `git log`
  and a `gh pr view`. Its report disclosed the first two. Against what the prompt and the allowed
  source already held, it added that a concurrent pass existed and nothing about a weakness.
- The floor. The report counts 27 of 27 bootable rows booted. Its own table grades rows 10 and 11
  `host`, so the brief's count is 25. Without the carry-over rows 1 and 3, fresh attacks reach 23.
  Row 32's test censuses a spawned child rather than the boot shell.
- Reproduction. Every booted row's test passed again on all three ISAs it applies to, one ISA at
  a time, under another lane's load; no leg timed out. The host rows passed too.

Outstanding:

- calef's ruling on whether pass 8 counts toward criterion (c).
- Risk 7's appendix cites the pass, through the maintainer under §216.
- Pass 8's tests land on main with their `Co-Authored-By: GLM 5.3` trailers, in a pull request of
  their own after this record.

## Done means

- Every bootable row booted, or the pass recorded in full and marked as not counting, as the brief
  says.
- Every escape is a failing test committed before any fix.
- A pass-8 note with the per-row table and its refusal log, indexed in `notes/README/` and its area
  page.
- Risk 7's appendix cites the pass through the maintainer under §216 (fatal-risk facts are
  correctable, and verdicts are the architect's). Moving the color stays calef's.

## Follow-on

- **Milestone 198.** A human review, or a public bounty once a stranger can install nife, is the
  stronger form and waits on milestone 198 (a package manager, and the trivial install that makes a
  second customer possible).
