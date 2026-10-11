---
status: NOT-STARTED
raised: 2026-10-11
milestone_dependencies: 844
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 885. The shell's test roles become scripts the shipping shell runs

*(Minted 2026-10-11 (UTC) by lane swish-witnesses-become-scripts, filed at calef's ruling the same
day; number provisional until the merge queue lands it. Title and slug are drafts.)*

calef ruled on 2026-10-11 (UTC), on pull request #1914 (milestone 844's cut), "Proceed with both".
The second of the two was this, in his words: "It seems like we shouldn't need the test roles in
swish. Couldn't we design a better implementation that the witnesses are individual programs and we
run them all with scripts rather than a specific build?"

Classification: new work. The lane claims with `--new-work`, because what it leads with is a user
feature the shell does not have: running a script without a keyboard. Deleting the roles is what
the feature pays for, not the purpose.

Reuse: the shell's own `dispatch_line`, which is the thing under test and is reused unchanged.
`system_tests/src/user/pipeline_service.rs` already wires a scripted shell and reads its transcript
(`start_with`, `transcript`, `answer`, `counts`); every witness moves onto that wiring.
`xtask/src/swish_check.rs` is the shape for the assertions: a table of lines and the substrings each
must answer. `fs_service::start_granted_dir` and `start_granted_two_dirs` keep serving the trees.
The prior art for the mode is POSIX `sh`, which reads commands from a file operand or from a
standard input that is not a terminal and stops at its end. New code is needed only for the mode
itself, the few hundred lines that read a script instead of a keyboard, because nothing that exists
can run swish's own dispatch except swish.

## What is there today

`components/src/swish.rs` (5,920 lines on main at `9532eeb53`) carries six roles that no user can
reach. `_start(role, arg, clock)` dispatches on `role`:

| role | name | launched by | wiring | how it reports |
|---|---|---|---|---|
| 1 | `ROLE_NAVIGATE` | `shell_navigation_tests` (2 tests) | a subtree at slot 0, a report rendezvous at 1, no terminal, no budget | a bitmap, `fixture::navscape` |
| 2 | `ROLE_GLOB` | `glob_grant_tests` (1 test) | the same, over `globset` | a bitmap, `fixture::globscape` |
| 3 | `ROLE_PIPELINE` | `pipeline_tests` (5) | terminal, spawn, result, budget | a transcript ending `== pipelines done` |
| 4 | `ROLE_REDIRECT` | `redirection_tests` (6), `language_tests` (6) | role 3's plus a directory | a transcript ending `== redirections done` |
| 5 | `ROLE_TIMING` | `time_tests` (2) | role 3's plus a clock page or none | a transcript ending `== timings done` |
| 6 | `ROLE_TWO_TREES` | `multi_dir_namespace_tests` (1) | two trees at slots 0 and 1, a report at 2, no terminal | a bitmap, `fixture::twotrees` |

Role 0, and any number not in the table, is the interactive shell. `crates/system_initializer`
starts it as `(0, fs_rights, clock_slot)`. The witness code is about 1,090 lines, measured by
pull request #1914's proposed decision (the shell program becomes a directory of modules), which
moves it into a temporary `witness.rs`.

Milestone 844 (the shell program is split along its seams) said the witnesses "cannot move to a
fixture binary" because they need the shell's own `Nav` and spawn path. That is right, and it is
why this milestone moves them into scripts rather than into a second program: a script is run by
the shipping binary, through the shipping dispatch.

## What gets built

1. A non-interactive mode in swish. It reads lines from a script until the end, runs each through
   `dispatch_line`, prints what the prompt would print, draws no prompt and edits no line. A user
   gets it as `swish build.sw` or `swish < build.sw`, or in whatever form the decision below rules.
2. Each witness becomes a script file in the tree (location and extension provisional, for example
   `system_tests/scripts/navigate.sw`), included in the test build and handed to the shell by the
   kernel harness.
3. Each test judges the transcript, the way `pipeline_tests` already does, plus whatever the harness
   can read from the tree afterwards through `fs_service`.
4. The six roles, their helpers (`run_line`, `pwd_is`, `Text`, `opened`, `opened_token`, `created`,
   `mtime_of`, `open_dir`, the `touch` probes, `run_name`, `line`, `bind_line`, `listing_is`), the
   witness-only `Nav` constructors, and `witness.rs` itself are deleted.
5. The verdict bitmaps `fixture::navscape`, `globscape` and `twotrees` in `filesystem_protocol`,
   with their host test rows, go once nothing reads them. That removes public items from a crate,
   which the decision below lists.

## How each witness keeps its property

The property every witness holds is the same: what is under test is the shell's own resolver and
dispatch, not a copy. A script keeps that by construction, because the only path a script line has
is the prompt's. What changes is who judges, and for three witnesses, what they run in.

**Pipeline, redirection and timing (roles 3, 4, 5).** These are scripts already: fixed arrays of
lines through `dispatch_line`, printed with `$ ` and judged from the transcript. The arrays move
verbatim into script files, and each ends with an `echo` of its done line. The three clock states
stay three capability tables, which is what makes `time`'s refusals assertions. Nothing about the
witnessed property changes.

**Navigation (role 1).** The shell-level steps become lines: `pwd`, `cd ..`, `cd /..`, `ls`,
`cd deeper`, `mkdir`, `cd <made>` then `cd /`, `touch`, `touch -t`, `bind`, and `cd` through the
bound name. A refusal is read off its sentence and off `echo $?`, which prints 2 for a refusal
(notes/swish-language.md). Whether a name was reached becomes `wc < inner` and `wc < /inner`, which
resolve through the shell's own `redirect_target`, so the wiring gains the interactive endowment
(terminal, spawn, result, budget) beside its subtree. The two runs with different roots stay
each other's control, and the harness puts the run index into the names it hands each script.

Several steps never were the shell. `CREATE` with a body, `UNLINK` while a handle is held,
`UNLINK` of a directory, `RMDIR` of a non-empty directory, `GETMTIME`, and `MKDIR` narrowed to drop
`SETTIME` all go straight down the contract through `Nav::name_call`. The code says why: `rm` is a
program, and the witness holds no spawn channel. Those assertions move to
`fs_test_client`, which already carries `ROLE_DIR_ATTACKER` for exactly this kind of probe behind
the same caretaker, or to the harness reading the tree after the script. The mtime checks become
the harness reading `GETMTIME` after the `touch` lines. The `../motd` probe becomes a line the
shell's resolver refuses.

**Globbing (role 2), the known hard case.** The witness compares `echo gl-*.txt` with the planner's
verdict for `rm gl-*.txt`, rendered through `swish::write_set`. The planner's verdict is already
printable: `caps rm gl-*.txt` plans through `grant_plan::plan_stage` (which `plan` calls) and
prints the grant's names with the same `write_set`, in the row "(the directory holding ...)". The
crate's own comment there says `caps rm *.txt` prints exactly what `echo *.txt` prints. So the
script is `echo gl-*.txt`, `caps rm gl-*.txt`, the no-match pattern and `echo $?`, `caps rm
*/gone`, and the plain line, and the harness compares the two renderings. The lane checks two
things first. One is that `caps` resolves a bare `rm` without a spawn channel; it falls through to
the archive's manifest when `bare` finds no installed program. The other is that the row's wording
is stable enough to parse. The second phase, running the real `rm` with the set grant, is the harness's already and is
unchanged.

**Two trees (role 6).** The script is short. It moves with `pwd`, `ls`, `ls /`, `cd /second`,
`cd ..`, `cd ../..` and `cd /..`. It reads with `wc < secret`, `wc < /inner`, `wc < /secret` and
`wc < /second/secret`. It ends with `rm /second/secret`, both `bind` lines, `ls /bee`, `cd` and
`pwd`. Which tree
answered becomes which body `wc` counts, since the two fixture files differ in length. The
blocker is not the script. The shipping shell cannot hold a second tree: the witness builds one
with `Nav::rooted_twice` from a mount constant only it knows. Milestone 660 (a second
filesystem mounts in the boot shell) chose the transport: a named slot probed at `_start`, and a
mount path in a shared crate. calef said it is built "with the first real second filesystem, not
before".
This is open question 2.

## What `_start` keeps

`_start(role, arg, clock)` keeps `arg` (the directory rights the shell is told) and `clock` (the
clock slot), because both are told for reasons the code records. `role` is no longer read. The
system initializer passes 0 there and can keep doing so; changing its three-word spawn convention
is outside this milestone. Every wiring now grants a budget, so the branch that skips `heap_init`
for roles 1 and 2 goes. Script mode is chosen by what the shell holds, the way `HOLDS_CONFIG` is
probed, never by a number in `x0`, so no role dispatch comes back under another name.

## What has to be decided first

The lane writes these up as a `status: PROPOSED` decision before building, and its pull request
carries `needs-architect`.

1. Where a script comes from. (a) The input slot a program's manifest declares, so `swish
   build.sw`, `swish < build.sw` and the harness's in-memory stream are one path. Recommended:
   a script is then a file grant like any other, and the user feature and the test are the same
   code. (b) A `-c` argument, which caps a script at an argument page. (c) The shell opening a
   path through its own directory, which fails in the navigation and glob subtrees, whose fixture
   trees hold no script.
2. Two trees: build 660's shell half here (probe the named slot, call `rooted_twice`), with only the
   harness filling the slot until a real second filesystem exists, or leave role 6 until 660 lands.
   Building it here goes against calef's "not before" for the shell half, so it is his call. If he
   says wait, this milestone is done when five roles are gone, and role 6 is deleted by 660.
3. Exit status. No program reports one (notes/swish-language.md), and `SYS_EXIT` carries none, so
   a status word would be a syscall surface change (rule 3). Recommended: none. The harness reads
   `echo $?` lines and whether the shell exited or faulted.
4. Removing `navscape`, `globscape` and `twotrees` from `filesystem_protocol`'s public `fixture`.
5. Every new name: the mode, the script extension, the scripts' directory.

## Risks the lane measures

- Memory. Every scripted shell is a live process the kernel never reclaims, and one more once put
  `time_tests` over the frame pool (`pipeline_service.rs`, the `TRANSCRIPT` comment). Three
  witnesses gaining a 128-page budget and a terminal costs frames. Measure the pool before and
  after; the remedy is reclaiming a script's shell once it exits, not a bigger pool.
- Stack. Roles 1 and 2 run on 4 and 7 measured pages. The script wirings take the interactive
  `SHELL_EXTRA_STACK` of 11, which must stay level with the boot's.
- A transcript is prose. A reworded sentence fails a test. That is the intent, and the failure
  names the line.

## Done when

1. `witness.rs` (or, if 844 has not split the file, every witness item in `swish.rs`) is deleted,
   and no role constant or witness helper remains in swish.
2. `_start` has no dispatch beyond the interactive shell and script mode, chosen by what the shell
   holds.
3. A user can run a script at the prompt in the form question 1 rules, with `EXAMPLES` and a
   `BUGS` section in notes/swish-language.md.
4. Every assertion the six witnesses made still runs on aarch64, riscv64 and x86_64, DECISIONS §19
   (architectural parity is a tenet). Each one moved to `fs_test_client` or the harness is listed in
   the pull request beside the bit it replaced, so a reader can check none was dropped.
5. `user.shell_navigation_tests.two_shells_with_different_roots_cannot_name_each_others_files.patch`
   still fails the test, rerun on the new script, and its record says where it now fails. It is the
   only falsification patch on these tests.
6. `script/test` passes on all three architectures, and so does `script/swish-check`.
7. Every new name carries a `Name:` line marked provisional.

## Index row

The shipping shell carries six test-only roles, about 1,090 lines that only tests reach. This gives
swish a script mode as a user feature and turns each witness into a script the shipping binary runs
and the harness judges, so tests exercise the real shell rather than a role compiled into it. The
glob witness reads the planner through `caps`. The two-tree witness waits on calef's answer about
milestone 660's transport. Comes after 844.
