# Milestone 117 appendix: run 3, its handoffs, and the lane that took them

*An appendix to [milestone 117 (the stranger test)](../0117-newcomer-onboarding.md), moved out of
that page on 2026-10-12 (UTC) under [§212 (a prose
budget)](../../decisions/0212-a-prose-budget-for-every-document.md). The main page is complete
without it. Run 3's full record is [notes/stranger-test/run-3.md](../../../notes/stranger-test/run-3.md);
this page is what the block said about it at the time.*

## Run 3, 2026-08-18

Run 3 closed the isolation question, which was the only thing making run 2 unscorable. Run 2's
strangers were subagents of a maintainer session whose working directory was the repository, so
`AGENTS.md` arrived in their context at turn zero and five of the eight rubric rows went unscored.
Run 3's stranger was a separate process, started with `--safe-mode` from a directory whose *child*
was the clone, since project instructions load from ancestors and never from descendants. It was
verified twice rather than assumed, and all eight rows plus B1 are scored: five answered, one
partly, one absent, one induced from instances rather than read.

It also found the harness leaking harder than the tree does, which is the finding to carry forward
rather than bury. The run's log files sat in the stranger's own working directory under the names
`stranger3-stream.jsonl` and `stranger3-stderr.log`, so its first `ls -la` told it that it was
stranger 3. It disclosed that first when asked, and said the knowledge is why it added a program
rather than only describing one. Three runs, three different isolation failures, every one of them
in the harness rather than in the repository. Run 4's fix is one line: the logs go in a sibling
directory.

What run 3 found, and none of it was fixed in its own lane, because a run that stops to fix
things stops measuring and its findings stop being traceable:

- `notes/adding-a-program.md` is stale again, two days after run 2 corrected it. Milestone 130
  (the copy that outlived its reason) deleted the `mkinitrd()` shape its step 4 describes. And
  `manifest()` is a seventh `grant_plan` edit the list of six omits. Recorded in that page's own
  `BUGS`. The page has now been rewritten by two successive strangers and gone stale between
  them, which is the argument that this is not a documentation problem. One fact lives in five
  hand-maintained places, two of the seven edits are compiler-forced, and the two that fail
  *silently* are both in the unenforced five.
- `script/test` is intermittently red on a contended host. The measured rate is 2 in 13 aarch64
  legs plus two reds of the sibling assertion, at a load average of 45 to 63 caused by other lanes
  gating on the same machine. The tree already knows this. What run 3 adds is an independent
  reproduction, and the observation that the sibling's eight-retry loop also exhausts. It also
  found that the panic message contradicts the note that explains it, so a developer meeting the
  failure is told it is the kernel's bug. Recorded in notes/load-sensitive-assertions.md's `BUGS`.
- Nothing tells anyone the machine is shared with other lanes, which is the single most
  load-bearing fact about any timing result and cost the stranger an hour.
- A stranger doing ordinary work reaches no file under `design/decisions/`, and reaches neither
  `notes/net.md` nor `notes/capabilities.md`. Two of those carry rubric answers, which is why M2 is
  absent and M1 only partial.
- The rubric itself has aged. M8 asks for three provenance states and §89 (`provisional` becomes
  the fourth provenance state) made it four; M1 quotes a phrase this tree has never written.
  Amended in the note rather than defended.

Why the block stayed `PARTIAL` after run 3. The instrument was proven and the two predicted
documents existed, but the milestone's own sentence is *"fix what the run finds, then run it
again"*, and run 3's findings were recorded rather than fixed.

## What run 3 handed off, 2026-08-18

None of this was done in run 3's lane, on purpose. Each is specified precisely enough to act on, and
each is recorded next to the feature as well as here, so a reader who never opens this block still
meets it.

1. Print the host load average beside a timing-assertion failure. An afternoon, and it is the
   cheapest item here by a wide margin. notes/load-sensitive-assertions.md has already established
   that host load is the discriminator for this whole family, and has already measured the rate
   against a load average; the harness is in a position to sample `uptime` and does not. Today a
   contended host produces a message that blames the kernel, and the reader has to think of
   `uptime` unprompted, which cost run 3 an hour. *Done 2026-08-18, and it took an afternoon.*
2. Correct `notes/adding-a-program.md` step 4, and count `manifest()` as the seventh
   `grant_plan` edit. Five minutes, and it belongs to whoever lands the next program, per that
   page's own convention. *Done 2026-08-18, and the five minutes was the wrong estimate for the
   right reason: walking the page rather than reading it found three more defects.*
3. Stop `timer.rs`'s panic message asserting a false dichotomy. Fifteen minutes, both ISAs. Do
   not widen the bound; the tree has rejected that twice and is right. The sentence just should not
   claim something the code cannot support. *Done by milestone 62 (tests that assert on time),
   which deleted the assertion carrying the sentence on both ISAs rather than rewording it, the
   bound untouched as asked.*
4. Adding a program should not need five hand-maintained lists. A milestone, and it is the one
   run 3's stranger nominated as the highest-value thing a newcomer could offer. A `Prog` variant
   could carry its archive name and its manifest as data, and both initrd tables could be generated
   from it. This is a design fork rather than a lane. It is why a page two strangers have now
   rewritten went stale between them, and by the ladder's own reading it is a rung-one answer to a
   problem currently answered at rung four. *Became milestone 150 (adding a
   program should not need eight hand-maintained lists) on 2026-08-22.*
5. Run 4, with the harness's logs in a sibling directory, and with `CONTRIBUTING.md` and the
   reading order in the tree it is handed. Neither has been seen by a stranger: run 3's clone
   predates both. *Done 2026-08-18. The sibling directory worked; the tree leaked
   instead, and both documents earned their place.*

## The handoffs lane, 2026-08-18

Status did not move: still `PARTIAL`. This lane took the three cheap items run 3 recorded and
did not fix, and none of them is the milestone's own sentence ("fix what the run finds, then run it
again"). Handoffs 4 and 5 above were untouched: run 4 was still owed, and so was the design fork.

- Print the host load average beside a timing-assertion failure: done. `HostLoad` in
  `xtask/src/main.rs` samples `uptime` every five seconds for the length of an emulated leg. When
  the leg goes red it reports min/mean/peak, the core count and the oversubscription factor, on
  both the TCG and the `--hvf` legs. Recorded in notes/load-sensitive-assertions.md, under the
  diagnostic section it belongs to, with its own `BUGS`.
- Correct `notes/adding-a-program.md` step 4, and count `manifest()` as the seventh `grant_plan`
  edit: done, and the page was wrong in three further places nobody had found. `cargo xtask build`
  claimed to pack both archives and packs one. The `SWISH_CHECK_SCRIPT` example it gives does not
  compile against that array's type. And the page said which edits exist without ever saying which
  ones the machine catches. The page was verified by walking it with a scratch program, added and
  removed, rather than by reading it.
- Stop `timer.rs`'s panic message asserting a false dichotomy: already done by milestone 62, in
  the branch that was live while this lane ran. 62 deletes `the_handler_keeps_up_when_no_lock_is_held`
  on both ISAs, which is where that sentence lived, and replaces the sibling test's exhausted-budget
  panic with an `UNMEASURED` report. Nothing was re-fixed here.

One thing measured here that changes what a reader should believe. Run 3 recorded that a missing
`from_name()` arm fails silently. It does, but only on a condition nobody had stated: `PROG_COUNT` is
the keystone, and `prog_id_round_trips`' own doc comment claimed the opposite of what it does. A
variant added with its three compiler-forced arms and none of `from_id`, `from_name` or `PROG_COUNT`
compiles and passes every host test. The doc comment is corrected in place and step 6 now carries
the measured table.

That sharpens handoff 4 rather than answering it. The mechanism it asks for would make all three of
those unnecessary. Rust offers no way to count an enum's variants without a derive macro this tree
has not taken, so the gap cannot be closed with a gate.

## How this block's status went stale, twice

The block said "Run 2 is the remaining half" and was falsified thirty-nine minutes later, when
pull request #219 merged without touching this file, and it stayed wrong for a day. That was the
second time this one milestone's status had gone stale in exactly the same way. It is the
strongest single argument in the tree for §76 (what catches a milestone status that is
wrong in both places)'s defect class being structural rather than careless.
Found 2026-08-17 by the status-accuracy sweep. The
`IN-PROGRESS` token was additionally false because no branch existed. The three lanes had been
`milestone/117-stranger-test`, `fix/stranger-run-findings` and `claude/milestone-117-7ik9e6`, all
merged and all deleted.

The first time: the column said `NOT-STARTED` until 2026-08-16, with run 1 already recorded in
three other files. That is §76's failure again, and it is worth naming rather than quietly
correcting. The gate compares the index row against this file's own status line, both of which said
the same wrong thing, so agreement is not accuracy. Nothing in the tree can see that a milestone has
moved except a person who moves it.
