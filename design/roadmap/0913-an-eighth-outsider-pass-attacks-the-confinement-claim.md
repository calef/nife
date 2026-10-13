---
status: NOT-STARTED
raised: 2026-10-12
milestone_dependencies: 824
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 913. An eighth outsider pass attacks the confinement claim

*(Minted 2026-10-12 (UTC) by lane `maintainer/mint-eighth-outsider-pass`, the way milestone
903 (a seventh outsider pass attacks the confinement claim) was minted. The number 913 is
provisional; other lanes are minting nearby, so expect renumbering at merge. Title, slug and every
name below are drafts.)*

Reuse: the brief is `briefs/confinement-outsider-pass.md`, unchanged, and through it milestone 800
(a non-Anthropic model attacks the confinement claim)'s counting rule, report format and refusal
log, so the passes compare row for row. The isolation is milestone 633 (an outside agent attacks
the confinement claim)'s: this pass is blind. Which model runs it is calef's call.

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

## The attack

Run by a fresh agent under `briefs/confinement-outsider-pass.md`. The lane briefs, collects,
reproduces each claimed escape by test or boot, and records. It does not attack and does not hint.

- Posture: Blind, milestone 633's: the reviewer gets a scratch copy of the claims table alone
  and the source tree, and the brief's list of what not to open.
- The reviewer's model: calef's call, made when he launches the pass. If this pass is to be criterion (c)'s non-Anthropic half,
  the reviewer is a non-Anthropic model or a human; an Anthropic reviewer can be only the other
  half.
- The floor: Every row a boot can reach, 27 of 34 (calef on #2010: "27 seems right"). A row
  counts as booted when a hostile test written for this pass ran under QEMU on at least one ISA it
  applies to. Rows 1 to 3 may carry pass 7's tests (`seventh_pass_delegation_tests`,
  `seventh_pass_retype_tests`) when the reviewer accepts or strengthens each in writing. Row 3's
  test is a held assertion since 824, not the characterization pass 7 left.
- Reproduction: The lane reruns every booted row's test itself, one ISA's QEMU at a time.

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
