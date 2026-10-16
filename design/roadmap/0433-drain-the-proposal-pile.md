---
status: BUILT
raised: 2026-09-19
built: 2026-09-19
---
# 433. Drain the proposal pile to zero, and keep it there

Built 2026-09-19, in one evening by four lanes. All 106 files are numbered milestones
and `design/roadmap/proposals/` is empty.

**Empty, not abolished, and the difference took a round trip to establish.** The maintainer read
this milestone's ruling as a case against the directory and retired it (milestone 434), which calef
reversed the same evening: *"There will be future proposals. Deleting support for them would be
short sighted. We just don't want anything to change for future proposals. The mechanism was working
fine. We just needed to see them into milestones."* So the mechanism below stands unchanged and what
this milestone bought is the drain, not a rule change. 434 carries the two costs the maintainer
failed to price.

Minted the same day by calef, who ruled the pile should not accumulate:
*"We should promote them all to milestones and then close them versus leave them as proposals. We
want to drive proposals to zero."* *(Number provisional until the merge queue lands it.)*

**It carried `Gate: NONE` while it was open**, on the argument that everything it needed was a
number and the number is the integrator's to assign. The line is gone because a finished block's
gate can only be stale. The assignment it pointed at is below, and it stands: 327 to 432, once, for
all 106.

## Why the pile should not exist, in calef's own earlier words

`design/roadmap/README.md` records why a lane writes a proposal rather than a milestone, ratified by
him on 2026-09-03: lanes were barred from minting because concurrent lanes cannot see each other and
two reaching for the same number collide, and **"the collision is in the number, not in the
authority"**. So the directory is a number-assignment queue and nothing else.

A queue waiting on one cheap operation should not be 106 deep and sixteen days old. The README asked
this of itself, under the heading **the graveyard question**, and answered that a proposal nobody
promotes is the same burial in a new location. That is what happened.

**The measurement that decides it.** Of the 106, **66 carry gate `NONE`**, meaning a lane could start
them today. `script/roadmap --ready` prints 69 milestones and reads no proposals at all. So
roughly half the tree's startable work is invisible to the command a lane picks work from, which
is a throughput problem wearing a filing problem's clothes.

Why promote and then close, rather than close some as proposals. calef's ruling, and it is the
better shape: a numbered block marked `REMOVED`, `SUPERSEDED` or `BUILT` is a record a reader can
find, while a deleted proposal is gone. The maintainer's own recommendation had been to close the
dead ones in place, which would have destroyed exactly the account a later reader needs.

## The numbering, assigned once, because it cannot be assigned twice

327 to 432, oldest filed date first, then slug. A number is never reused in this tree, so this
mint is irreversible and is written down rather than left to be re-derived from a pile that is about
to stop existing.

The filename keeps the slug: `proposals/<slug>.md` becomes `<N>-<slug>.md`. That is what lets any
of the 114 citations of a proposal path in numbered blocks be resolved by a reader with one
`ls design/roadmap/ | grep <slug>`, with no lookup table to consult and nothing to keep in sync.

The whole assignment, one line per proposal with its filed date and slug, is moved verbatim to
[`numbering.md`](0433-drain-the-proposal-pile/numbering.md).

## What promoting one actually costs, measured rather than guessed

It is not a `git mv`, and this block exists partly to say so before four lanes discover it.

- Zero of the 106 carry a `## Index row` section, which `script/roadmap --check` requires of every
  non-lettered numbered block. Each needs one written: a paragraph saying what the milestone is, in
  the generated table's voice.
- **`Status: PROPOSED` is not in the milestone vocabulary**, so every status line is rewritten. The
  filed date stays in the prose, because that is what makes the pile's age measurable after the pile
  is gone.
- **92 `**Proposed.` follow-on bullets across 71 blocks name a proposal path, and the roadmap gate
  requires that path to exist. Every one becomes `**Milestone N.**` as its file moves. This is the
  half that makes the pass large, and it is also the half that cannot be missed: the gate fails on
  each until it is fixed.
- **The premise has to be re-read, not assumed.** Five proposals were read on 2026-09-18 and
  2026-09-19 and all five had decayed premises; four were fully answered by the tree before anyone
  opened them, two by work that landed after filing and before reading. 64 of the 106 were filed in
  a three-day window, 2026-09-03 to 09-05, and 32 came from one sweep, so the decay is correlated
  rather than spread out. A block promoted to `NOT-STARTED` without that check is a false claim in
  the roadmap, which is the thing this tree objects to hardest.

## The method, per proposal

1. `git mv design/roadmap/proposals/<slug>.md design/roadmap/<N>-<slug>.md`.
2. Retitle line 1 to `# <N>. <title>`, keeping the title.
3. Rewrite the status line to a milestone token, after checking the premise against the tree.
   `NOT-STARTED` if the work is still real, `BUILT` if it has already been done elsewhere,
   `SUPERSEDED` if another block took it, `REMOVED` if it is no longer wanted. Keep the filed date and
   say what was checked, so a reader can tell a verified `NOT-STARTED` from an assumed one.
4. Write `## Index row`.
5. Fix every `**Proposed.**` bullet that named the old path, in whatever block carries it.
6. Do **not** run `script/roadmap --write`. The generated index is the one file every lane touches,
   so the integrator regenerates it once at the end. This is the whole collision surface of the pass
   and removing it is why four lanes can run at all.

## Slices, as they land

- 327 to 353, on `milestone/433-slice-1`, 2026-09-19. Twenty-five `NOT-STARTED`, one `PARTIAL`
  (343, ten of its twelve comments and its index line fixed since filing) and one `BUILT` (344,
  answered by calef on 2026-08-18, sixteen days before the proposal asking for it was filed). Eight
  of the twenty-seven had decayed in some part; two had decayed outright. Twenty-six
  `**Proposed.**` bullets became `**Milestone N.**`, and three citations outside `design/roadmap/`
  were repointed: `design/decisions/0144-fastpath-footprint-ceiling.md`, `notes/follow-on-work.md`
  and `kernel/src/bench.rs`.
- 354 to 380, `milestone/433-slice-2`, 2026-09-19. All 27 promoted, every premise read against
  the tree before its status was written. Twenty are `NOT-STARTED` and verified, four `SUPERSEDED`:
  354 by milestone 250 (an unviable mutant is a hole that reads as a pass) on the day it was
  filed, and 361 by milestone 188 (the IPC fastpath)'s re-measurement.
  Milestone 303 (a server and no disk it can find) answered 362 and 364, with the newer x86_64
  fixture block. Two are `BUILT` (375 by the radon bench session
  the same evening it was proposed, 380 by the §144 (the fastpath footprint gate) amendment the same day). One is `PARTIAL` (378,
  whose parser and reporting arrived as milestones 161 and 317 and whose xenon reading has not been
  taken). Seven of 27 had decayed, six of them answered by work that landed within a day of the
  filing. Three more had premises that were narrowed rather than closed and say so in their status
  lines. Twenty-three follow-on bullets rewritten (22 `**Proposed.**` and one
  `**Recorded.**`), and the moved path updated in thirteen files outside `design/roadmap/`:
  `notes/`, `kernel/`, `script/` and one audit report.
- Slice 3, 381 to 407, done 2026-09-19 on `milestone/433-slice-3`. 27 promoted: 26
  `NOT-STARTED` and one `SUPERSEDED` (399, by 405, which turns the six role constants it renames
  into programs). 6 of 27 had a premise decayed enough to change what the block claims, and 11
  more needed a factual correction in prose. The sharpest is 406, whose title is now false as
  written: `script/boot-check` landed the same day it was filed and does boot the default riscv64
  kernel on every pull request, leaving only the tour past the self-test verdict unasserted. Two
  defects found that predate the pass. Milestone 290 (three programs wearing one name) carried two
  adjacent `**Proposed.**` bullets
  with the prose on the wrong one, and milestone 265's `**Proposed.**` bullet for the unswept
  truncations named a proposal that holds a different subject. The gate accepted that because the
  check is path-shaped.
- 408 to 432, `milestone/433-slice-4`, done. 25 promoted. Three of the 25 had a decayed
  premise, one in eight, which is a lower rate than the five-of-five sample this block was minted
  on and is what the newest quarter of the pile was expected to show. 430 was already `BUILT`,
  closed by commit `c1a177c` on the same day it was filed. 431 is `SUPERSEDED`, filed on a
  premise milestone 319 had falsified the day before and restating work milestone 423 already
  carries correctly. 415 is `PARTIAL`, because its item 1 landed as `ba99c83` hours after it was
  written, and its closing section argued from a toolchain-bump decision calef had already ruled on
  as milestone 302. The other 22 are `NOT-STARTED` and each says what was checked. Three counts
  moved without changing any work (408's nine copies are not the same nine, 410 is five where the
  title says six, 416's "roughly 35" occurrences are 80). 418's central premise is false for a
  reason no sweep was needed to find: `VTD_ADDR_MASK` is a `const` literal and `cargo mutants`
  rewrites functions.

## What the pass measured, which overturns this block's own argument

**The decay is same-day, not slow**, and that is the finding worth keeping. This block was written
saying premises rot between filing and promotion, so promotion is where to catch it. Four slices
measured it and the shape is different:

| slice | promoted | disposition changed | carried something false |
|---|---|---|---|
| 1 (oldest quarter) | 27 | 2 | 8 |
| 2 | 27 | 7 | 12 |
| 3 | 27 | 6 | 17 |
| 4 (newest quarter) | 25 | 3 | 9 |
| **total** | **106** | **18 (17%)** | **46 (43%)** |

Slice 2 put the mechanism in one sentence and slice 3's numbers agreed with it independently.
Six of slice 2's seven and four of slice 3's six were answered within a day of filing, several by
the lane that wrote the proposal and then finished the work that same evening. So the pile's
cost is not that proposals rot slowly while nobody promotes them. **It is that a proposal is filed
and answered inside a day and the file is never told.**

That changes the remedy, and the change is worth stating because it makes an earlier ruling
weaker. Draining at every merge, which this block assumed as the steady state, would have caught
almost none of these: the answer usually arrived before the next merge. What would catch them is the
lane that does the work closing the proposal it just answered, in the same commit. That is a habit
at the thing rather than a sweep over the pile, and it is the same rung the `BUGS` convention
already occupies.

And the honest note on my own sample. This block was minted partly on five proposals read by
hand, all five of which had decayed. The measured disposition-changing rate is 17%, so that
sample overestimated it by a factor of five. The wider measure is the one that holds up: 43% of
the pile carried something false, which is what a lane would have worked from.

One defect class the gate cannot see, found by two lanes independently. The `**Proposed.**`
check is path-shaped: it verifies the named file exists and never that the file holds the work the
bullet describes. Milestone 290 carried a bullet whose prose belonged to the bullet above it, and
milestone 265 carried one naming a proposal about a different subject. Both passed every build for
weeks. Both surfaced only because an edit broke the path check for an unrelated reason.

A second blind spot in the same family. The gate checks `**Proposed.**` bullets and nothing
else, so 33 prose citations of a proposal path across 38 files were invisible to it and were
swept by hand at integration. They all resolved, because promotion kept the slug in the filename.
About thirty more did not, and those are older rot this pass surfaced rather than caused: proposals
deleted by earlier promotions, this morning's cluster drain among them. Their slugs have no numbered
file because the work was folded into a cluster block. Mapping those is not mechanical and is not
done here.

## BUGS

- A promoted block can still be a graveyard, one directory up. Numbering does not prioritize;
  calef's 2026-09-03 wording separates the two on purpose (*"anybody should be able to add to the
  roadmap. That's different than prioritizing that roadmap"*). What promotion buys is visibility to
  `--ready`, not attention.
- `Proposed.**` survives this pass and should probably not.** A follow-on bullet is a permanent
  record and the file it names is now, by this block's own rule, ephemeral: it exists only between a
  lane writing it and the next integrator numbering it. Whether the disposition word should be
  retired is a vocabulary question and therefore calef's; this block does not answer it.
- Nothing here stops the pile refilling, and a gate that tried would be routed around by not
  writing proposals, which the README already argues is worse than the pile. The steady state this
  block assumes is that an integrator drains it at every merge, which is a habit rather than a
  mechanism, and is rung four.
- The numbers are minted before the premises are checked. A proposal that turns out to have been
  answered still consumes a number and lands as a `BUILT` or `SUPERSEDED` block. That is the cost of
  assigning all 106 in one place, and the alternative (assign as each is verified) reintroduces the
  collision this whole directory exists to avoid.

## Follow-on

- **Recorded.** *The `**Proposed.**` check is path-shaped and cannot tell whether the file it names
  holds the work the bullet describes.* Two bullets in this pass were wrong in exactly that way
  (milestones 290 and 265) and both passed every build for weeks. Recorded in
  the disposition table beside the check it limits, now in `notes/roadmap.md`, and in this block's
  section above. A gate that could tell would have to read prose, which is the thing milestone 247
  already refused to try.
- **Recorded.** *About thirty citations of proposals deleted by earlier promotions still dangle*,
  because those slugs have no numbered file: the work was folded into a cluster block rather than
  promoted one to one. Recorded beside the promotion rule, now in `notes/roadmap.md`, which is
  now "keep the slug" precisely so this cannot recur. Mapping the older ones wants a reading of each
  cluster block and is not mechanical.
- **Decision.** *Whether `**Proposed.**` should survive as a disposition word at all.* A follow-on
  bullet is a permanent record and the file it names is, by this block's rule, ephemeral: it exists
  only between a lane writing it and the next integrator numbering it. That is a vocabulary
  question, so it is an architect's, and it is written up in
  `design/decisions/0140-follow-on-disposition-vocabulary.md`'s own terms rather than minted here.
- **Recorded.** *Nothing stops the pile refilling*, and this block's `BUGS` says the assumed steady
  state (an integrator drains it at every merge) is rung four. The measurement above weakens that
  further: the answer usually arrives before the next merge, so the habit that would work is the
  lane closing the proposal it just answered. Recorded in `notes/roadmap.md`.
- **Milestone 326.** The two mutation-testing proposals in the pile (354 and 418) were both checked
  against the live triage of milestone 326 (nobody assigned to turn a mutation score upward) rather
  than assumed either way. Neither is covered by it:
  354 is about a mutant that never built, and 418 is about a constant `cargo mutants` cannot mutate.

## Index row

The `design/roadmap/proposals/` directory was a number-assignment queue, ratified 2026-09-03 on the
argument that the collision is in the number rather than the authority. It reached 106 files and
sixteen days, with 66 of them startable today and invisible to `script/roadmap --ready`, which is
roughly half the tree's available work hidden behind a second command. calef ruled on 2026-09-19
that they are all promoted and then closed where closing is right, because a numbered block marked
`REMOVED` is a record and a deleted proposal is not. This block assigns 327 to 432 once, since a
number cannot be minted twice, and records what promotion actually costs. It is 106 index rows that do not
exist yet, 92 follow-on bullets across 71 blocks that name a path about to move, and a premise check
on every one. Five read in two days had all decayed.
