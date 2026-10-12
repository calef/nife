# Milestone 580 appendix: what this would cost, measured against the twenty-four

An appendix to [milestone 580 (nobody reads branches, so give the maintainer a command that
does)](../0580-nobody-reads-branches.md), moved out of that page on 2026-10-12 (UTC) under [§212 (a
prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md). The main page is complete
without it. This is the proposal's measurement of the grace window and of the judgment each flagged
branch costs, as filed on 2026-09-23. Long sentences were split in moving; the figures are
unchanged.

## The grace window

The false-alarm question is really a grace-window question, and that day's data answers it
directly. Of the 24, only `maintainer/capture-the-actions` was under a day old at measurement time.
Every other branch, including all seven holding real unmerged work, was at least two days old and
up to nine.

Consider a grace window set at 24 hours. That is one order of magnitude above
`lane-claim-check.sh`'s 15 minutes, which protects the gap between a push and a draft. It is
comfortably inside the gap between two maintainer sessions. It would have deferred exactly one
branch that day and flagged the other 23 correctly. `maintainer/subscription-stays` was pushed the
same session and was in the same position as `capture-the-actions`. It is the clean case for why the
window has to exist at all: it stopped being orphaned on its own, gaining PR #1106, before anyone had
to act on a report about it.

## The judgment each flagged branch costs

Judgment cost per flagged branch is small and case-dependent, which is the point of keeping the four
shapes separate rather than a single list. Six of the 24 (case 2) and five more (case 3) need no
judgment at all; delete is always correct. Two (case 4) need one look at the closed pull request's
own thread, which GitHub keeps regardless of the branch, to confirm the closure carries its
reasoning. The remaining eleven, including the seven detailed on the main page, are the ones
actually worth a maintainer's attention. That is eleven out of 24, not 24 out of 24, which is what
"one blanket rule" would have cost instead.
