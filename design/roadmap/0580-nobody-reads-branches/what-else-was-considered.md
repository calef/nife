# Milestone 580 appendix: what else was considered

An appendix to [milestone 580 (nobody reads branches, so give the maintainer a command that
does)](../0580-nobody-reads-branches.md), moved out of that page on 2026-10-12 (UTC) under [§212 (a
prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md). The main page is complete
without it. This is the proposal's own list of the alternatives it weighed and refused, as filed on
2026-09-23, with the note on the one calef overruled. Long sentences were split in moving; the
argument is unchanged.

## Extend `helpers/merge-drain.sh`

Refused: its whole domain is pull requests, and these branches are defined by the absence of one.
Its `notify()` dedup logic posts a comment *on a pull request*; there is nowhere to attach that
comment when there is no pull request. It also already carries three distinct single-purpose checks
(admission, stuck workflow runs, stale drafts) for a reason. Each is one act with its own threshold,
and a fourth unrelated one blurs a script that earns its keep by staying narrow.

## Extend `helpers/lane-claim-check.sh`

The option: cover every branch prefix, and escalate past its 15-minute grace window into a days-long
sweep. This is the closest real contender and the one worth the longest refusal. Its own `BUGS`
section already explains why the `milestone/*` restriction is deliberate: widening it "would also
sweep in short-lived maintainer branches that are not claims and are not meant to be." Today's data
proves the restriction has teeth rather than being theoretical. `maintainer/capture-the-actions` and
`maintainer/subscription-stays` were both pushed by the maintainer *during the same session* that
produced the sweep this proposal answers. And `subscription-stays` had already resolved itself into
PR #1106 before this proposal was finished.

The two problems want thresholds three orders of magnitude apart: minutes to protect a lane between
push and draft, versus days to catch one that never will. They want different actions too: a nudge
to open a pull request, versus a person deciding whether to claim, land-and-delete, or just delete.
Folding the second into the first risks breaking the tuning that is currently working.

Overruled 2026-09-23; see "What calef decided" on the main page. The risk named here was real but
narrower than the refusal treated it: it is a risk to a *shared* threshold, not to folding the
checks into one file. Two separate grace windows in one script keep the 15-minute tuning exactly as
it was measured while adding the wider sweep beside it: `GRACE_MINUTES` untouched for `milestone/*`
and a new `GRACE_HOURS` for everything else.

## A new `helpers/` watcher of the same shape

The option: run on the existing five-minute `launchd` cadence and report once per stall the way
`merge-drain.sh`'s `notify()` already does. Refused on the strongest evidence available. This shape
already exists for the nearest analogous case (`stale_drafts`), and PR #1087 shows it firing exactly
on schedule and changing nothing. A fourth reporter, printing to a log on patagonia that nobody
tails, repeats a failure this tree has already measured rather than fixing it.

## A scheduled GitHub Actions workflow

The option: a workflow running independent of any session. `notes/merge-queue.md` records that
calef declined exactly this shape on 2026-08-26 for the analogous case, "an unattended scheduled
agent." He preferred something that "shuts down when the session driving it does" over "standing on
a timer with nobody watching." That reasoning was about an agent empowered to *resolve* a queue
problem, and this proposal's mechanism only reports, so the objection does not transfer
mechanically. But a pure report-only workflow still lands in a place nobody has an established habit
of reading (a workflow run's log, or a check summary). That is the same failure as the option above,
with GitHub Actions minutes spent to reach it. If it were extended to also *act*, deleting branches
unattended, that runs straight into AGENTS.md's own caution that deleting a branch destroys work and
needs a person's judgment first.

## Make it the steward's standing duty, in prose, with no new script

Refused for a reason this proposal does not have to argue for, because AGENTS.md already argues it.
The steward tried exactly this shape once, and the tree's own record of what happened is "it reported
and never acted." Assigning one more duty to a role that has already failed at prose-only duties, in
the same document that names the failure, is rung four dressed as an assignment.

## Do nothing; accept the cost

The honest baseline, and the one this proposal has to out-argue with numbers rather than assert
past. The oldest of today's 24 is nine days old (`maintainer/ratify-audits`). §90 (the claim is a draft pull request) has a claim
mechanism that assumes a lane reaches a draft pull request inside one sitting; the branch that built the
check measuring that assumption took three minutes. AGENTS.md names "somebody will notice" as rung
zero and says plainly it "belongs on no list." The backlog also does not self-correct. Nothing
currently shrinks it, and every missed claim, closed PR, or forgotten `git push origin --delete`
adds to it. Twenty-four today; the number has no reason to be smaller next month.
