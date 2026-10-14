---
status: BUILT
raised: 2026-08-03
built: 2026-08-16
---
# 93. Documentation audits as a mechanism: the docs stay true to the tree

Built 2026-08-16. The mechanism is one row in the cadence table of milestone 92 (security audits as
a mechanism), not a second machine.
`documentation` is a kind `script/audits` evaluates. `script/audits --worklist` answers the scope
question this block left open (a heuristic, not a signal, and the reasoning is in
[notes/documentation-audit.md](../../notes/documentation-audit.md)). The procedure is written down
where a stranger can run it. The first sweep ran in the same lane, because a cadence whose first run
has never happened is a plan. It found thirteen environment variables in prose that name nothing,
nine backticked paths that resolve to nothing, and a rotted line count. Most instructively, it found
a rotted *justification* (`script/lint`'s recorded reason for not gating backticked paths had grown
from 8-of-95 to 31-of-379). Part 3's compounding half is real rather than aspirational. The first
class is now gated by `script/lint`'s `==> environment names in prose` and cannot rot again. Report:
[design/audit-reports/2026-08-16-docs-versus-reality.md](../audit-reports/2026-08-16-docs-versus-reality.md).
Raised 2026-08-03 by calef, as milestone 92's sibling. 92 keeps the security story from rotting;
this keeps every other documented claim from rotting. A demonstrator's docs are part of the
deliverable (CLAUDE.md). So a doc describing a system that no longer exists is a defect in the
deliverable, not a cosmetic lag.

The cadence was decided 2026-08-16 (§74 (audits run on change, not on the calendar)) and this
milestone shares it rather than proposing its own.
The same triggers read the same tree, because a documentation sweep's input is what changed since
the last one. (This block carried **Gate: NONE** until it was built. The gate line is dropped rather
than kept, because `script/roadmap` is right that nothing gates finished work.)

The evidence that rot is real here, all found in one day (2026-08-03). notes/verification.md said
the proof suite ran in "a few minutes" a month after that stopped being true. Two roadmap status
rows claimed more remained than their own blocks said. The in-brief of milestone 47 (navigation and
naming) listed work as
pending that had shipped. notes/cpu-models.md's closing line prescribed a fix a later milestone
had replaced. Every one was found by accident, while looking for something else. The mechanism
exists so finding them stops being luck.

What already exists, so this builds on it rather than beside it. The structural gates catch
structural rot: script/roadmap and script/decisions for citations and status vocabulary, the lint
link check for dead paths, typos for spelling. What nothing catches is claim rot: a number, a count,
a "currently", a "still open" that the tree has moved past. The tree's own habit points at the fix.
Its best documents already say "counted by grepping, not by memory" next to their
numbers.

The mechanism, mirroring 92 where the shape is the same:

1. Cadence and tripwire. Same pattern as 92: a recorded schedule, an index file, an overdue signal
   in the toolchain-drift style. If 92 lands first, share its index and tripwire machinery rather
   than growing a twin. One audit index with a type column beats two files.
2. Each audit is a sweep with a scope: a set of notes read against the tree as it is, plus against
   the roadmap as it stands. "Proposed state" rots too. A note describing planned work as built, or
   a plan a later decision overrode, is the same defect in the other direction. The sweep's output
   is per-claim. Still true, corrected (loudly, per the house rule), or moved to the honest tense.
3. **Shrink the auditable surface over time**: each audit should convert some checkable claims into
   checked ones. The harness count moved from prose memory to a grep that way. Counts, file paths,
   statuses, and dates are mechanically re-derivable. A claim a gate re-derives can never
   rot again. This is the compounding half of the mechanism. It is the reason audits should get
   cheaper each round rather than staying constant.
4. **Findings end in one of three states**, exactly 92's rule. Fixed in the audit lane (most doc
   corrections are one lane's work). Minted as a milestone (when a doc gap reveals a system gap, the
   84-to-90 path). Or recorded-accepted where a reader meets the claim. "Noted" is not a
   state.

## Scope note

This audits prose against reality. It does not write missing docs (the remainder of milestone 68
(code-quality gates) owns doc
examples and item docs). It does not restructure them: milestone 40 (documentation as a system
service) owns docs as a service, and
milestone 91 (A glossary, and every acronym linked to it) owns the glossary. (91's every-use links
add surface this mechanism then keeps true.)
The staleness triggers worth building beyond cadence are recorded for the design rather than
promised. A note whose cited code has changed substantially since the note's last edit is the
highest-value candidate for the next sweep, and git can compute that. Whether it becomes a signal or
stays a sweep heuristic is a decision for whoever builds this. The index and the reports share
92's home, `design/audit-reports/`, decided 2026-08-04.

## Follow-on

- **Recorded.** `notes/documentation-audit.md` answers the one question this block left open: which
  staleness triggers to build beyond the cadence. `script/audits --worklist` derives a note's
  candidacy from how much its cited code has moved since the note's last edit. The reasoning there
  says why that stayed a sweep heuristic instead of becoming a signal that fires.

## Index row

92's sibling for claim rot: four stale claims were found in one day, every one by accident. Shares
92's index, tripwire and dispositions by adding one cadence row. `script/audits --worklist` ranks
documents by how much of the code they cite has moved, and `notes/documentation-audit.md` is the
procedure. The first sweep ran in the same lane. It found 13 stale environment names, 9 stale paths
and a rotted justification, then closed the first class with a gate
