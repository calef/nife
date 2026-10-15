---
status: IN-PROGRESS
raised: 2026-09-24
branch: milestone/652-twelve-decision-gates
promoted_from: twelve-decision-gates-cite-only-ruled-sections
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 652. Twelve `DECISION` gates cite only sections that are already ruled

*Title and slug are drafts.*
Raised by milestone 591 (a ruling should make the gate it answers
fail until someone updates it). Its lane converted 27 `DECISION` gates to `DECISION §N` and could
not convert these twelve. Every section each one cites is `DECIDED` or `AMENDED`, so the new token
would fail the day it was written. **Name provisional**: this file's stem is a lane's coinage.

Since this block was raised, §207 (the roadmap is a graph, and the block says so in fields a
script can walk) moved the gate out of prose and into the frontmatter. The three dispositions read
in today's fields. A gate nothing stops a start is `decision_dependencies: none`. A fork with a
written-up section names its number. A wait that is not a section at all stays
`unwritten`, with the paragraph saying what the wait is.

## What each block got

Read per block on 2026-10-15 by this milestone's lane. Five forks were written up as `PROPOSED`
sections, numbers 300 to 304, provisional until the merge queue lands them; each is calef's to
rule. The other seven needed no new section, for the reasons in their rows.

| Block | Outcome |
|---|---|
| 39 | Unchanged. [§151 (the goal of the repository split is independent release)](../decisions/0151-repository-goal-is-independent-release.md) ruled the goal and kept the order open, and the paragraph already says the block waits on that order. |
| 48 | `decision_dependencies: 300`. The `Tcb::SUSPEND`/`RESUME` fork [§24 (interrupting the foreground)](../decisions/0024-interrupting-the-foreground.md) deferred is written up as [§300 (the paused state)](../decisions/0300-tcb-suspend-resume-the-paused-state.md), waiting on calef. |
| 137 | Stays `unwritten`. The only open choice is priced where §99 (where Apple's metadata lands) left it and has nowhere to land until an SMB server exists again; the real wait is the retirement ruling, which no field carries. |
| 172 | `decision_dependencies: 301`. The wait-and-collect fork is [§301 (exit status and output)](../decisions/0301-exit-status-and-output.md). |
| 188 | Already carried `decision_dependencies: 95`, which the graph resolves; the prose still said the token stays `DECISION`, a sentence the frontmatter switch retired. It now says the E3 run on radon is the wait, carried by the machine and person fields. |
| 334 | `decision_dependencies: 302`. The fork is [§302 (the screen fact, and one line of input)](../decisions/0302-the-screen-fact-and-one-line-of-input.md). |
| 340 | `none`, and `needs_person: yes`. §97 (six gates run on every pull request and none of them can stop one) is decided; what remains is a checkbox flip at a GitHub settings page, which is a person and an account, not a decision. |
| 356 | `decision_dependencies: 303`. The fork is [§303 (the region)](../decisions/0303-the-retention-declaration-says-nothing-about-the-region.md). |
| 376 | `decision_dependencies: 304`. The fork is [§304 (who may turn a device off)](../decisions/0304-who-may-turn-a-device-off.md). |
| 388 | Yielded. Its own lane claimed the milestone the same day (#2211) and rewrites the block; this lane read the gate and changed nothing here. |
| 391 | Stays `unwritten` deliberately: §149 (may the kernel answer on an endpoint) refused to settle the general case until a real consumer exists, and [milestone 269 (`machine` at the prompt)](0269-machine-as-a-program.md) is that consumer. 269 is now the block's `milestone_dependencies`, which the sequencing always said and no field carried. |
| 417 | `none`. The premise moved: [milestone 106 (a wait that ends on either the interrupt or the deadline)](0106-deadline-wait.md) built `Timer::ARM` and §101 (notification objects)'s binding ends a blocked `RECEIVE_CAP` at a deadline, so the reporting usurper is buildable on surface that exists. The first-class receive stays unwritten until a second consumer asks, and the `try_receive_cap` naming is reported for calef. |

## Index row

Twelve `DECISION` gates cited only ruled sections, so none could take the checked form and all
twelve could go stale the way [milestone 198 (a package manager)](0198-package-manager.md) did.
Read on 2026-10-15. Five forks became `PROPOSED` sections that gate their blocks by number. Two
gates were answered outright: 340 to a person at a settings page, and 417 to `none`, because the
timer's built mechanism dissolved the fork. Three stay bare on purpose, because their
wait is real but is not a section. 188's fields were already right once the prose stopped
describing a token the frontmatter switch retired. And 388's own lane claimed the milestone the
same day, so this one read the gate and yielded the block.
