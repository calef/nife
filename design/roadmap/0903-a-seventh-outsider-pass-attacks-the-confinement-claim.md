---
status: IN-PROGRESS
branch: milestone/903-a-seventh-outsider-pass-attacks-the-confinement-claim
raised: 2026-10-12
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 903. A seventh outsider pass attacks the confinement claim

*(Minted 2026-10-12 (UTC) by this lane as the maintainer's delegate, the way milestone 871 (a sixth
outsider pass attacks the confinement claim) was minted. The number 903 is provisional; other lanes
are minting nearby, so expect renumbering at merge. Title, slug and every name below are drafts.)*

**Reuse:** the counting rule, the report format and the refusal log are milestone 800 (a
non-Anthropic model attacks the confinement claim)'s, unchanged, so the passes compare row for row.
The isolation is milestone 633 (an outside agent attacks the confinement claim)'s, not 800's: this
pass is blind.

## Index row

The seventh outsider pass at risk 7's confinement claim, by a fresh Claude agent briefed blind with
only the published claims table and the source. It counts an attack only when it boots, and keeps
the standing refusal log.

## Why

Risk 7 (the confinement claim is false) is AMBER. Its criterion (c) for green is two consecutive
independent attacks with no escape on a shipped path, at least one by a non-Anthropic model or a
human. The sixth pass (milestone 871) booted DNS rebinding in the package fetch, fixed in #1901, and
the count restarted at zero. calef asked on 2026-10-12 (UTC) for the seventh pass, with the
independence rule as the point: a fresh reviewer, briefed the way milestone 633 allows.

## The attack

Blind, milestone 633's posture. The reviewer is a fresh agent spawned by this lane and briefed with
two things: the claims table in `notes/confinement-claims.md` and the source tree. The brief names
no audit report and no earlier pass note, and asks the reviewer not to open the claims file's
narrative, `design/audit-reports/`, `design/fatal-risks/`, `design/roadmap/`, any earlier outsider
or falsification note, or git history. The lane does not attack and does not hint. It briefs,
collects, reproduces each claimed escape by test or boot, and records.

## Result

Running.

## Done means

- Every claim has an attack, or a written reason it was not attacked.
- Every escape is a failing test committed before any fix.
- The pass-7 note exists with its refusal log, and is indexed in `notes/README/` and its area page.
- Risk 7's appendix cites the pass through the maintainer under §216 (fatal-risk facts are
  correctable, and verdicts are the architect's). Moving the color stays calef's.

## Follow-on

- **Milestone 198.** A human review, or a public bounty once a stranger can install nife, is the
  stronger form and waits on milestone 198 (a package manager, and the trivial install that makes a
  second customer possible).
