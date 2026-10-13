# Milestone 118 appendix: the size gate's ceiling in September

An appendix to [milestone 118 (CLAUDE.md has a budget)](../0118-constitution-budget.md), moved out
of that block on 2026-10-12 (UTC) under [§212 (a prose
budget)](../../decisions/0212-a-prose-budget-for-every-document.md). The block is complete without
it. This continues [the size gate's ledger](size-gate.md) through September 2026, oldest first:
every raise and lowering of the `agents-md-lines` ceiling, with the reason each was worth it.

## 1032 to 1035, 2026-09-01

Three lines, and the cheapest raise this file has taken. The developer role said a lane never
edits `design/`. `script/lint` check 4b requires a `milestone/N-*` branch to edit
`design/roadmap/N-*.md`, and calef had made that exception on 2026-08-23, but it was written only
in a comment above the check. So the rule a failing lane could read forbade the only edit that
clears the gate. Two lanes hit it on one day and both reported the gate as impossible, correctly.
The three lines are the exception itself; the reasoning stays beside the check, which is where the
next person meets it.

## 1035 to 1037, 2026-09-05

Two lines, and they buy a correction rather than an addition. The paragraph of §90 (the claim is a
draft pull request) justified the draft-pull-request convention with *"a draft cannot be stuck in
the merge queue because a draft cannot be merged"*, and that is false. GitHub closes a draft as
merged when its head holds nothing its base does not. That happens whenever a lane bases on a
`maintainer/mint-*` branch that lands before the lane's first commit. The board then reads empty
while a lane is working, which is the single property the board exists to provide, and it had done
so four times since 2026-09-02.

The growth is two lines because the evidence went elsewhere, which is this budget working as
designed rather than being worked around. The instruction (`git commit --allow-empty` before the
draft) is in `AGENTS.md` because that is where a lane reads it. The four cases, the window, and why
a rate-dependent failure hid for three days are in [§90 (the claim is a draft pull
request)](../../decisions/0090-claiming-and-closing.md)'s amendment, because that is where evidence
belongs. The constitution gained the rule and not the argument.

## 1037 to 1043, 2026-09-05

Six lines, and they replace a list with a test. The naming section protected eight standard terms
by enumeration, which told a reader nothing about the ninth. It now carries calef's rule that an
acronym is spelled out unless its expansion teaches nothing, which can be applied to a name nobody
has coined yet. It also overturns a ratification of his own (`dma_validator`, 2026-08-01) and says
so, because a reader who remembers that decision should see it reconsidered rather than quietly
dropped.

The growth is six lines and not thirty because the argument went to `design/naming.md`. The same
clause was also stated there and would otherwise have contradicted the constitution the moment this
landed. That duplication is itself the subject of a milestone now: the constitution should carry
the tests a lane applies and the note should carry the history. Today the naming section is 12% of
`AGENTS.md`, with most of that being argument.

## 1043 to 1048, 2026-09-05

The third raise that day. Five lines add a seventh question to the list a fork must arrive answered:
*would we still choose this if both options cost the same?* It is not new reasoning. It is the test
of §92 (a caretaker is supervised by the client it serves), which already existed and lived inside a
tenet, so it fired only when somebody remembered it. calef asked for it as a standing question after
it had gone unasked for a month on milestone 51 (wall-clock time)'s "three problems, one addition".
That block counted effort and read as design until he applied the test himself.

Three raises in one day is the argument for milestone 262 (the naming rule is written twice), not
against the raises. Each was a considered act with its sentence, which is what this budget asks. But
the file grew fifteen lines between 1033 and 1048 in a morning, and the naming section alone is 12%
of it and mostly argument. The constitution should carry the tests a lane applies and the notes
should carry the cases; until that split is made, every good rule costs the budget the same as a bad
one.

## 1048 to 988, 2026-09-05, lowered by milestone 262

A lowering deserves the sentence as much as a raise does. The naming section went 137 lines to 77
without losing a rule. The argument moved to `design/naming.md`, which now says in its own first
paragraph that it is the case and not the authority. Where the two disagree, this file is the rule
and the note is the bug.

Most of the 60 lines were not moved; they were deleted as duplicates. The two-tier rejection, the
`wc` example, the "0 of 57" measurement, the standard-terms guard rail and the `NAME_LEN` history
were each already written out in full in both files. That is the defect milestone 262 was minted
for, found four more times while fixing it once. It answers the question this budget could not
ask: the file was not only growing, it was growing things it already had.

The ceiling is set at the new exact value with zero headroom, per this block's own rule, so the
next addition argues for itself the way the three on 2026-09-05 did.

## 988 to 992, 2026-09-13

Four lines: the rename procedure in the naming section. The rule came out of a rename performed the
same day that missed three live proposals and nearly rewrote a quotation. The detail lives in
`design/naming.md`. What is here is the four-line rule itself, because `AGENTS.md` is the rule and
the note is the argument. Nothing was removed to pay for it; that is the cost, stated.

## 992 to 1001, 2026-09-13

Nine lines: a new section saying every date in this tree is UTC. Provenance blocks, roadmap rows,
`design/decisions/` sections, notes and commit messages all carry dates. `script/names` *fails* a
ratification that lacks one, and nothing anywhere said what zone a date meant. The agents writing
most of them run UTC and calef does not. So a name he ruled in his evening was already being filed
under the next day, silently, in a record nobody can correct later. This is the *facts that leave
the machine* category of the move-fast-on-what-can-be-undone tenet, which is why it earned lines
rather than a note. Nothing was removed to pay for it; that is the cost, stated.

Deliberately not gated. `script/names` could compare a ratification date against the commit's
author date, but a record honestly written the morning after a ruling would fail it. The check
would be wrong about the tree in the way §77 (the branch-prefix list now describes the tree)
already is. Rung three, at the thing itself.

## 1001 to 1009, 2026-09-14

Net of a consolidation; the eleven added lines are a third lane-count ceiling, disk. The file
already bounded concurrent lanes by collision surface and by memory, and neither of those warns
about the third. Five lanes at roughly 3 GB of `target/` each, plus 7.2 GB in the main checkout's
own, took the volume to 1.9 GB free and then to a command failing mid-write. The maintainer session
met it four times in one night and cleaned up four times without ever writing the limit down, which
is rung zero wearing diligence. It earned lines rather than a note for the reason named in the
text: disk is the only pressure in this system that destroys work rather than delaying it. A lane
pruned under it loses uncommitted work outright.

Three lines were removed to pay part of it, the first time this ledger records a raise partly
funded rather than simply taken. calef asked whether the prune rule could be consolidated; it stood
in four places. Two of those are role duty lists (the maintainer's and the steward's hygiene
clauses), where naming the duty is the point rather than a restatement. The other two were genuine
duplicates, each stating one rule with different evidence. One had eight worktrees with one holding
3.3 GB; the other had 2026-07-31's zero bytes free with 42 worktrees holding 78 GB. They are now
one paragraph carrying both measurements and the push-before-removing clause, sited beside the disk
ceiling where a reader now meets the subject.

What was not added, and the near-miss is worth the sentence. The same session proposed also adding
"prune the worktree at merge", which this file already says in four places (323, 367, 496, 615).
The rule was not missing and the session had simply not followed it. A fifth copy would have been
the defect of milestone 262 (the naming rule is written twice), the file growing something it
already had.

## 1009 to 922, 2026-09-19

The ceiling read 1009 until §155 (naming conventions move out of the constitution) moved the naming
conventions to `design/naming.md` and the file fell to 922. Milestone 262 had asked for 988 against
its own smaller diff, which §155 made moot.

## 922 to 924, 2026-09-21

Recorded at the time as the first time this ceiling had moved. It was the first move since the drop
to 922 two days earlier; the marker's history shows eighteen moves before it, from 942 on 2026-08-22.
It is the case this budget exists to permit rather than the case it exists to refuse. The file's
advice on avoiding a shared `git stash` recommended `/tmp/x.patch` by name. That is a fixed path
shared machine-wide exactly like the stash stack it was protecting against, and two lanes duly
collided on `/tmp/pr-body.md` that day. Two lines bought a correction to a hazard the constitution
was itself teaching. Compressed first: the addition was five lines before being cut to three.

## 924 to 925, 2026-09-22

One line, because the disk ceiling it repriced was not merely stale but actively misleading. It
told a reader the lane count was three or four for a reason that had stopped being true. The
paragraph that replaces it has to carry both the retired number and the lever that replaced it
(lanes gate in CI). Compressed first, from an eleven-line original to twelve.

## 925 to 929, 2026-09-23

Four lines, and the considered act is that a session's own context had become the same shape of
unrecorded fact this milestone exists to catch. It was real: 233 million tokens against 5 million
for twenty dispatched lanes, measured 2026-09-22. It was consequential: it inverts why delegation
is worth doing here, context isolation before cost per token. And it lived only in one session's
head. The full argument and the practices it earns went to `notes/what-a-session-carries.md`, per
this block's own rule that the note carries the argument and the constitution carries the rule.
What is here is three lines pointing at it. Nothing was removed to pay for it; that is the cost,
stated.

## 929 to 949, 2026-09-23

Measured from the merged tree after two branches raised it independently from the same base of
925, one to 929 and one to 945, arriving at 949. The new tenet: calef stated on 2026-09-23 that
everyone owns a problem they notice, not only the ones with a mechanism already pointed at them. It
is the complement to the existing "Nobody remembers, so build the mechanism that does not need them
to" tenet (2026-08-04). A load-sensitive test flake was correctly written into
`notes/load-sensitive-assertions.md` twice and left, since every pull request that hit it was not
the one that caused it. Nothing was removed to pay for it. The section earned its lines by staying
to one paragraph of quote, one of relation to the existing tenet, and one of the failure it names.

## 949 to 935, 2026-09-23

Recorded at the time as the first time this ceiling ever moved down. It was the fourth: 1048 to 988
on 2026-09-05, 1012 to 1009 on 2026-09-14 and 1009 to 922 on 2026-09-19 came first, each in the
marker's history. The fourteen lines went to
`briefs/merge-and-cleanup.md` and `briefs/session-start.md` rather than being deleted. The number
was taken with `wc -l` from the finished file, not by arithmetic against the diff. Two branches
earlier the same day had each raised this ceiling honestly from the same base and produced a wrong
number between them.

The considered act is a different one from every raise. Milestone 579 (which of the constitution
must be carried, and which is a brief) classified every passage of the file by whether it has to
fire when nobody is looking for it. It found 85% must be carried, so this is a targeted extraction
rather than the split this block had been asking for. The three passages that moved all fire at one
event performed by one role: the merge checklist's one line, the prune-the-worktree paragraph, and
the `nife-dev` relink. The watcher pair moved because most of its nine lines already duplicated
`briefs/survey-the-queue.md`. What stayed behind in each case is the ambient clause with no task to
trigger it: prune promptly and never prune uncommitted work, expect every lane to take `nife-dev`,
a queue reports and does not resolve.

## 935 to 584, 2026-09-24, and a ratchet

#1189 (calef, 2026-09-24) split the file into imperatives plus `design/tenets/` appendices, and a
ceiling left at 935 would let it regrow 351 lines. So `agents-md-lines` went into `script/lint`'s
`RATCHET_CEILINGS`: the gate also fails when the file falls below this number, and the fix is to
lower it, so every shrink is banked. A raise was still an edit here with its reason beside it, as
every raise before it was.

## 584 to 573, 2026-09-25

Lowered by calef's seven cuts.
