---
status: BUILT
raised: 2026-09-23
built: 2026-10-20
promoted_from: constitution-carry-vs-brief
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 579. Which of the constitution must be carried, and which is a brief

BUILT, 2026-10-20 (UTC). Shipped: two of the five briefs, as files in `briefs/` (commit cc49c2acc),
and three more passages carried by skills in `notes/skills/` rather than by briefs (pull request
#1726). calef ruled that the last `uncertain` passage, the agent-attribution line, stays in
`AGENTS.md` (Follow-on).

*(Number minted at promotion.)* Promoted from the proposal `constitution-carry-vs-brief`, filed
2026-09-23. calef's instruction of 2026-09-23 was to give the two unnumbered proposals on `main` a
number. The argument is its author's, and promotion was not the moment to improve it. On 2026-10-12
(UTC) maintainer/prose-budget-paydown tightened the block under §212 (a prose budget) and moved its
passage-by-passage table to appendices. Every verdict, figure and finding is kept.

As filed: calef asked whether the roles in `AGENTS.md` should become skills, and whether that would
shrink the constitution. The answer worked out in conversation was no to skills and yes to the
underlying instinct. Some of what `AGENTS.md` carries is only needed at a specific, known task, not
on every turn of every session. `briefs/` (built for milestone 118's still-open "split" piece) is
already the right place for that. The proposing lane ran the test end to end against the whole file
and reported what it found; it did not build the briefs.

A developer lane cannot edit `AGENTS.md`, and this file alone does not authorize populating
`briefs/`. calef decides whether to build the five briefs below and cut the matching prose. He also
settles one open question: where the largest of the five belongs. Nothing here blocks on hardware or
on another milestone.

The appendices, in [`0579-constitution-carry-vs-brief/`](0579-constitution-carry-vs-brief/README.md):

- [`classification.md`](0579-constitution-carry-vs-brief/classification.md): the verdict and reason
  for every passage outside the three roles section.
- [`roles.md`](0579-constitution-carry-vs-brief/roles.md): the same for the three roles section,
  the largest and the one with most of the `brief` and `uncertain` rows.
- [`skills.md`](0579-constitution-carry-vs-brief/skills.md): the three reasons for refusing skills,
  in full.

## Since this was filed

Verified against the tree on 2026-10-12 (UTC). The block's status still reads NOT-STARTED, but much
of what it proposed has happened, in a different form than it recommended:

- Two of the five briefs were built on 2026-09-23 (commit cc49c2acc): `briefs/merge-and-cleanup.md`
  and `briefs/session-start.md`. Both cite this milestone. The `nife-dev` relink the first one
  carried was retired on 2026-10-10, when each worktree got a toolchain name of its own. Milestone 118 (CLAUDE.md has a budget, and
  the rules that get violated move up the ladder) banked the shrink, 949 to 935 lines.
- On 2026-10-06 (UTC) calef approved moving `AGENTS.md`'s one-kind-of-work rules to `notes/skills/`
  (commit 8f8aeddf1, pull request #1726). The claim recipe is now in the `developer-lane` skill, the
  seven questions in `decisions`, and the leaked-QEMU recipe in `qemu-hygiene`. These are plain
  Markdown in the open Agent Skills format, linked from `AGENTS.md`. `.claude/skills/` and
  `.agents/skills/` hold symlinks to them. calef ruled the split must be agent-agnostic.
- That answers this block's refusal of skills differently than it argued. The block's third reason
  (on-demand loading is wrong for what must fire unprompted) still matches what stayed in
  `AGENTS.md`. Its first two (vendor lock-in, principle 3) were met by keeping the files plain and
  linked rather than by refusing the format.

calef ruled on 2026-10-12 (UTC) that the block is PARTIAL, on these facts.

## Why this differs from milestone 118's budget

Milestone 118 already asked "does a gate already enforce this." It found the file is mostly not
duplicate (a verified cut of about 1.7%). It left one piece undone: the split into a core plus linked
documents. This proposal is that piece, run against a different test.

118's test was mechanism: does a script already check this, so the prose is redundant. This test is
timing. Does the rule have to already be in an agent's head, or can it be looked up when the task
that needs it begins? The two overlap. The prune-worktree and `nife-dev`-relink passages (lines
508-517 and 587-597) turn out to be duties a role already performs on a schedule. But a passage can
fail 118's test (no gate exists) and still pass this one (it is only needed at one nameable task).

The line count is not the argument. `AGENTS.md` is loaded into every session and every lane brief.
Measured 2026-09-22, one maintainer session spent roughly 233M tokens against roughly 5M for twenty
developer lanes combined. Carrying context dominated that, not thinking on any single turn. That cost
is real, and it decided no row. A row moved to `brief` because it is genuinely task-triggered. A row
that would save lines but is genuinely ambient stayed. Using the budget as the reason would be
circular. It is the temptation 118's own "prefer gates over prose" clause had to correct for,
extended to a different lever.

## The test

Does this passage need to fire when nobody is looking for it?

- Yes: carry. The rule has to already be in an agent's head at the moment it would otherwise be
  violated, because nothing prompts a lookup. Most of the file is this shape: the three principles,
  the ladder, the naming authority rule, the landmines.
- No: brief. The passage is consulted while performing a task an agent already knows it is doing:
  rebasing, opening a lane, cleaning up after a merge, writing a proposal for calef. The task itself
  triggers the lookup.

When in doubt, two sub-questions must both hold before a passage moves:

1. Is the trigger a specific, nameable, recurring task, not "any moment of any lane's work"? `git
   stash` is unsafe at any point a lane reaches for it. No task called "using git stash" exists that a
   lane would recognize and consult a brief for, so it stays carry even though it reads like a
   procedure. Rebasing onto `main` is a task a lane names to itself first. That is why
   `briefs/rebase-onto-main.md` already exists and works.
2. Is the failure mode still caught if the passage moves? Either the ambient trigger survives as a
   short carried stub ("after a QEMU session, verify ownership before you kill anything, see
   `briefs/...`"), or a role carries the duty elsewhere. The steward's "cleans up behind finished
   work" already commits to worktree pruning, so the mechanics of *how* can move.

If either sub-question is unclear, the row is marked `uncertain` rather than forced to a verdict, as
the proposing lane's brief asked.

## What the table found

The full table is in the appendices, grouped by `AGENTS.md`'s own sections in file order. Line ranges
are against the file at the proposing lane's branch point (925 lines). The shape of it:

- Carry: the preamble, the three principles, the mechanism ladder and both tenets, the codebase rules
  and naming, testing, commits, dates, comments, style, and almost all of the roles section. These
  are judgments that recur unpredictably, not at one named task.
- Carry, flagged as landmines. These read like procedures but must be known before acting: the
  memory ceiling on `VERIFY_JOBS`, the review request that silently does nothing, squashing against
  the base commit, `git stash`, `wfi` rather than `wfe`, and `perl alarm` failing on QEMU.
- Brief: five candidates, listed below.
- Uncertain, three passages. The opening line every agent-written pull request and comment carries
  (344-351) fits a claim brief, but it also binds ad hoc comments that no brief would reach. Work
  waiting on calef (427-445) is both an ambient judgment (when to hold) and a task with a checkable
  shape (writing the ask). The FreeBSD documentation standard (554-570) governs notes, decisions and
  `BUGS` entries too, a trigger broader than one task.

## Totals

Classified: 909 of 925 lines (the remainder is section headers and blank separators, not separately
tallied).

| Verdict | Lines | Share |
|---|---|---|
| carry | 769 | 85% |
| brief (five candidates) | 96 | 10% |
| uncertain (three passages) | 44 | 5% |

This is worth doing, but as a targeted extraction, not a rewrite, and it should be described to
calef that way. Ten percent is not nothing at roughly 11,000 tokens a session. The honest headline is
the 85%. Examined passage by passage against "does this need to fire when nobody is looking for it,"
most of `AGENTS.md` says yes. That matches milestone 118's own finding on a different axis: the file
is not mostly unmechanised duplicate prose either. This proposal extends that conclusion along the
timing axis and gets a similarly modest number.

Recommendation: do the five `brief` extractions. Two are nearly free. One mostly already exists as
`briefs/survey-the-queue.md`, and one is the proposing lane's own launch procedure, typed out again
rather than pointed to. The other three match the shape of briefs already in the directory. Do not
force the three `uncertain` passages into briefs without calef's call. Each has a genuine reason the
split did not resolve, given in the table.

## The five new briefs (provisional names)

1. `claim-a-lane.md` (lines 352-360, 9 lines). Cut the branch, make one empty commit (`git commit
   --allow-empty -m "claim: milestone N"`), push it, and open a draft pull request before any work.
   The proposing lane's launch instructions restated this recipe inline. That is the evidence it
   belongs in a file rather than retyped into every brief that starts a lane.
2. `merge-and-cleanup.md` (lines 411-413, 508-517, 587-597, roughly 24 lines). What the maintainer
   or steward does the moment a pull request merges. Delete the branch (already automatic,
   `delete_branch_on_merge`), prune the worktree, and relink `nife-dev` from the main checkout with
   the exact command. Then check off any identified work from the lane's report against a home. The
   safety clause carries forward verbatim: a blocked lane's work is committed and pushed before
   anything is removed. This is the one failure mode in the whole system that destroys rather than
   delays. The one ambient clause of the relink stays carried: "every lane will take `nife-dev`;
   that is expected, do not try to stop it."
3. `session-start.md` (lines 518-531, roughly 14 lines). Confirm the two `launchd` watchers are
   alive (`launchctl list | grep nife`) and start them if not. Then run
   `briefs/survey-the-queue.md` (already built) and treat what it reports as a standing task, not a
   status check to skim. Most of the nine lines on what the watchers found duplicate that brief, so
   the prose shrinks to a pointer rather than moving wholesale.
4. `hunt-leaked-qemu.md` (lines 910-925, 16 lines). Find a leaked `qemu-system-aarch64` (or
   `-riscv64`, `-x86_64`) process and kill it safely. `lsof` the image file it holds. Walk the process
   tree in both directions: a QEMU whose ancestry ends in a live harness is someone's gate in flight,
   not a leak. Kill the tree at its root rather than the leaf. Line 908's one-line trigger stays
   carried.
5. `raise-a-fork-for-calef.md` (provisional name and location; lines 641-673, 33 lines). The seven
   questions a fork must answer before it reaches calef, and the tell that one has not been answered
   (reaching for an adjective where a command would do). A lane already recognizes this task once
   the "stop and bring it to calef" criterion (lines 546-549) has fired.

The open question is the fifth. `briefs/README.md` scopes that directory to "work that recurs, whose
failure modes are already known, and whose correctness a gate can check... not design, not prose a
reader will trust," aimed at rented mechanical-work models. This passage is prose a reader will
trust, written by whichever agent is presenting to calef. It may belong in `briefs/` with that scope
note widened by one sentence, or somewhere else (a `design/` reference doc). That choice is calef's.

## What could go wrong

The QEMU-hunting cut is the one most likely to be forgotten, and the highest-consequence one.
Suppose line 908's stub stays as written ("check `pgrep` and clean up") and the safe-killing
mechanics move out wholesale. A lane that has not internalized "verify ownership before killing"
might `pkill` on sight. That is the failure already recorded: a maintainer killed a lane's mid-suite
emulator this way, and that lane's run failed for a reason no one could see from inside it.
Mitigation, to apply at cut time and not skip: reword the carried stub to hold the landmine in one
clause. For example, "check `pgrep`, but do not kill on sight; walk the process tree in both
directions first, see `briefs/hunt-leaked-qemu.md`." The warning then survives for a reader who
never opens the brief.

`merge-and-cleanup.md` carries the file's worst recorded failure: zero bytes free, twice, and two
lanes died mid-work. The risk is lower than the QEMU one. The ambient duty survives in the carried
steward definition ("cleans up behind finished work"), which commits to the outcome even if the
exact commands live only in the brief. A lapse is also visible (a failing write, a full volume),
unlike the QEMU case, which fails silently as someone else's confusing test failure.

`raise-a-fork-for-calef.md` has no mechanical catch, same as today. A degraded fork (options with no
cost, no prior-art check) is caught only by calef noticing and pushing back. That is how this section
of `AGENTS.md` came to be written (2026-08-18, a maintainer asked seven questions calef had to ask
him). Moving the checklist changes nothing either way; this is judgment, not a gate.

## The refusal of skills

calef asked whether the roles should become Claude Code skills, loaded on demand from
`.claude/skills/`. As filed, the answer was no, for three reasons given in full in
[`skills.md`](0579-constitution-carry-vs-brief/skills.md). `AGENTS.md` is deliberately the
cross-tool convention, and `.claude/skills/` is one vendor's format. A skill that loads only inside
one CLI fails principle 3 (a newcomer must succeed alone). And on-demand loading is the wrong
direction for anything that must fire unprompted, which is most of the carry column. `briefs/`
already does the job in plain Markdown. [Since this was filed](#since-this-was-filed) says how the
tree later resolved this.

## Follow-on

- **Done.** Briefs 2 and 3, `briefs/merge-and-cleanup.md` and `briefs/session-start.md`, built on
  2026-09-23 in commit cc49c2acc.
- **Done.** Briefs 1, 4 and 5 were carried by skills rather than briefs: the claim recipe in
  `notes/skills/developer-lane/SKILL.md`, the leaked-QEMU recipe in `notes/skills/qemu-hygiene/SKILL.md`,
  and the seven questions in `notes/skills/decisions/SKILL.md` (commit 8f8aeddf1, pull request #1726).
  That also settles where the fifth belonged.
- **Done.** The three `uncertain` passages. Work waiting on calef is in the `decisions` skill, and
  the documentation standard is in `design/tenets/documentation-standard.md`. The opening line every
  agent-written pull request and comment carries stays in `AGENTS.md`, under "Pull requests and
  comments an agent writes", and names no person. calef, 2026-10-20 (UTC): *"It shouldn't have my
  name because AGENTS.md is checked into the repo. It could be used by anybody with their agents."*
  On the line `**Lane:** <branch or milestone>, written by an agent; the author GitHub shows is the
  person running it.`, the same day: *"Wording is fine."* Every writer of the line uses it:
  `script/claim`, `script/record-ruling`, `script/preflight-queue`, the pull request template and
  `toolchain-bump.yml`, and a session on the project's App names `nife-smelter[bot]` instead.

## Index row

Classifies every passage of `AGENTS.md` by whether an agent must already be carrying it or can look
it up when the task arrives, and finds five briefs' worth of prose that fails the carry test. It is
the still-open "split" half of milestone 118 (CLAUDE.md has a budget, and the rules that get
violated move up the ladder), run against a timing test rather than 118's duplication test.
