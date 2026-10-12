# Milestone 117 appendix: run 4 and what it handed off

*An appendix to [milestone 117 (the stranger test)](../0117-newcomer-onboarding.md), moved out of
that page on 2026-10-12 (UTC) under [§212 (a prose
budget)](../../decisions/0212-a-prose-budget-for-every-document.md). The main page is complete
without it. Run 4's full record is [notes/stranger-test/run-4.md](../../../notes/stranger-test/run-4.md);
this page is what the block said about it at the time.*

## What run 4 measured, 2026-08-18

Run 4 scored eight of eight on the mental model and passed B1 for the first time, which is the
two provisional documents working. `CONTRIBUTING.md` is directly responsible for two rows. The
stranger quoted its definition of a `BUGS` section instead of inducing one from instances as run 3
had to. And it ran the whole of `script/gates` unprompted on that file's own vocabulary, rather
than stopping at `script/test`. The reading order moved `AGENTS.md` from twelfth to seventh and
`notes/capabilities.md` from never-opened to eighth. The failure it leaves is that
`CONTRIBUTING.md` is item 2 of 8 and was read sixteenth of twenty-two, arriving after the gates it
describes had already been run.

The build was green on arrival with no change to the tree: `script/test` exit 0 first try, 1312
passed and 0 failed on both ISAs, then `script/fmt --check` and `script/lint` also exit 0. The
machine was loaded by four other lanes at a load average between about 3 and 17.5 on 8 cores. That
is far below run 3's 45 to 63, and no timing assertion fired, so the load-average diagnostic that
run 3's handoff bought is still unexercised.

## The leak moved into the tree

The isolation fix worked and the leak moved into the tree, which is the finding to carry forward.
Run 3's log files told its stranger it was stranger 3; run 4's logs went to a sibling directory
and its first `ls -la` returned only the clone. It knew anyway, within half an hour, from
`notes/adding-a-program.md`'s `BUGS` table. Those rows name run 2's `doubler` and run 3's
`triangle`, because each of those lanes recorded its walk exactly as this project's conventions
require. Four runs, four isolation failures, and this is the first one that is a consequence rather
than a mistake. The instrument's own good practice generates the leak, it compounds one row per
run, and every obvious remedy is worse than the leak. The stranger's disclosure is unusually
specific about the cost: it *"performed"*, chose its deliverable partly for how it would read, and
adopted the tree's prose style as mimicry. It did not invent a breakage, skip a gate, or open the
withheld note.

## What run 4 found

None of it was fixed in its own lane, per the same rule runs 3 and 4 both followed:

- `script/lint`'s naming worklist under-counts by exactly the `provisional` names, and then names
  the command that prints the other number: `82 still want calef` against `UNRATIFIED (86 of 162)`,
  and a census line that sums to 158 of 162. It bites precisely the state a newcomer is told to use.
  Recorded in design/naming.md's `BUGS`.
- The two archives boot different binaries under the name `init`, `hello` on aarch64 and
  `builder` on riscv64, stated in a comment 200 lines from the table that needs it. A placement
  finding rather than an absence, and run 3's closing diagnosis reproduced by a different reader.
- `cargo xtask build` and `script/lint` are both blind to a missing riscv initrd row, so the two
  commands run most often cannot catch the mistake this page warns about hardest. And nothing
  counts programs: 1312 tests before adding one and 1312 after.
- Removal is the same eight places and has no page. A half-removed program is a `PROG_COUNT` too
  large and an init slot no variant claims.
- `notes/adding-a-program.md` was right about everything, including a line number. First clean
  walk of four, against its own `BUGS` prediction and against the stranger's own written prediction.

The one criticism no previous run made, and it is aimed at the habit rather than at a document:
*"the project's habitual response to a structural problem is another document, which its own ladder
names as the worst available move... The documentation is doing work that a data structure should be
doing, and it is doing it beautifully, which is exactly what stops anyone from fixing it."* Its
three instances are all things this tree has written about at length and not changed: `AGENTS.md`'s
misleading name, this page's fourth rewrite, and the eight-place program problem whose one-place fix
is handoff 4 below.

## What blocked the milestone was never the worklist

Every run produced a fixable list, and three of the four had their lists fixed. The status did not
move, because the milestone's own sentence is *fix what the run finds, then run it again* and
"again" had no mechanism. Four lanes each rebuilt the harness by hand, and each got the isolation
wrong somewhere the others had not. That is the tell CLAUDE.md names for being on too low a rung: a
fact that exists only in an operator's memory.

## What run 4 handed off, 2026-08-18

Status did not move: still `PARTIAL`, and run 4 was the clearest evidence yet that the worklist was
not what held it. The status had been `PARTIAL` throughout because the milestone's gate was `NONE`.
The recurrence mechanism is the milestone, and it is the one thing four runs had declined to build.

1. Make the test recur without being remembered. This is the blocker and it is now four runs
   old. The harness is a clone, one deletion amended into the tip, one `claude --safe-mode`
   invocation from the clone's parent, and a debrief. Run 4 also added a `pkill` shim, so a
   stranger following `README.md`'s own quit instruction cannot kill another lane's emulator. That
   is a script somebody could write in an afternoon and nobody has, four times. Until it exists,
   117 cannot move. *Built the same day as `script/stranger-test`; see
   [harness-and-cadence.md](harness-and-cadence.md).*
2. Decide which number `script/names`' worklist line should print. A gate that says `82 still
   want calef (script/names --unratified)` beside a command that says `86` is under-reporting the
   `provisional` names, which are the ones whose own author has said they are wrong. Fifteen minutes
   of code and one decision about what the worklist is for. Recorded in design/naming.md's `BUGS`.
   *Done 2026-09-19; see [run-6.md](run-6.md).*
3. Say at the riscv initrd table that it boots a different `init`. A comment, on the row that
   needs it rather than 200 lines away. Five minutes, and the fact is already written, which is what
   makes it a placement bug rather than a documentation one. *Done; the main page's `## Follow-on`
   says where.*
4. Handoff 4 from run 3 is unchanged and is now nominated by two successive strangers: adding a
   program should not need eight hand-maintained lists. Run 4 added the removal direction to the
   case, and made the sharper version of the argument. `cargo xtask build` and `script/lint` are
   both blind to the riscv omission, and nothing in the suite counts programs. So no gate can close
   the gap that prose is currently holding shut.
5. Run 5 should be told it is being measured, rather than hidden from it. The tree's own record
   now leaks that fact within half an hour and cannot stop; pretending otherwise buys nothing and
   costs the disclosure. What still must be withheld is the answer key, which has held four times.
