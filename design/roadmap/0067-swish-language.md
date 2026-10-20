---
status: BUILT
raised: 2026-08-02
built: 2026-08-04
---
# 67. `swish` the language: quoting, sequencing, and exit status

Raised 2026-08-02 from measuring `swish` against a minimal POSIX shell; built 2026-08-04 in three
commits (`eca4da2f` quoting, `b20bfc3b` sequencing and exit status, `9155d647` the boot gate and the
note). The record did not catch up for twelve days. Closed 2026-08-16 on the tree's own evidence
rather than on a report. `crates/grant_plan/src/word.rs`,
`crates/swish/src/sequence.rs`, `swish::Status`, `kernel::user::language_tests` and
notes/swish-language.md all exist. The four rows below are each answered by a named artifact.

(The Gate paragraph that stood here said `NONE` and explained that quoting needed no decision. It
was right, the work is done, and a BUILT milestone gates nothing, so it is gone rather than stale.)

## What the milestone said was missing, and what answers each row now

| Gap | Where it lives now |
|---|---|
| **Quoting**: `"..."`, `'...'` | `grant_plan::word`: `Cursor` (is this byte bare) and `read` (take the quotes off one token) |
| **Sequencing**: `;`, `&&`, `\|\|` | `swish::sequence::{split, Joint, Sequence}`, split **outermost**, above `line::split` |
| **Exit status**: `$?`, which `&&` needs | `swish::Status`, three values, and two cells in `components/src/swish.rs` |
| `>>` and `2>` | already built with milestone 50's later work, 2026-08-03. See notes/pipes.md |

The backslash escape named in the original block is **not** built and is not planned: every token in
this shell is a slice of the line. An escape has to remove a byte from the middle of a word, which a
slice cannot do. The quoted spelling is the one that works. That is a recorded limitation
rather than a residual, and it is in notes/swish-language.md's BUGS beside the feature.

## The two things worth reading the note for

**Quoting was an authority gap, not a convenience.** A file called `my notes.txt` could not be
named. In a shell whose thesis is that naming a resource is granting it, a resource you cannot name
is a resource you cannot grant. The built rule is that **quoting delimits a word and never rewrites
one**, which keeps every token a slice. Its capability consequence is a *narrowing*. `rm "*.txt"`
designates one name where `rm *.txt` designates a set. `rm "-r"` names a file
rather than widening a directory grant.

The fork this milestone was raised to settle is settled. A refusal is not an error, and `$?` says
which. `0` ran, and `1` failed (something was attempted and did not work). `2` was refused (the
shell declined at the prompt with nothing spawned, nothing opened and no authority moved). `&&` and
`||` read one bit out of it, because they ask one question and both non-zero answers are "no". The
distinction stays where a person can see it. Unix cannot draw that line at all. There `127` and a
program's own `exit(1)` are the same kind of integer.

## What proves it, on both ISAs

`kernel::user::language_tests` reads the tail of the same scripted run `redirection_tests` reads.
That is the real shell binary through the real init, with a directory narrowed by an `fs_subtree_caretaker`
to one subtree of the real RedoxFS image. It shares that witness rather than wiring a seventh
scripted shell. That is a memory finding the note keeps: the first version's extra role put
`time_tests` over the frame pool intermittently.

The assertions are pairs, so a shell that ignored quoting entirely fails both halves of each. They
are `echo "*.txt"` against `echo *.txt`, `wc "my notes.txt"` against `wc < "my notes.txt"`, and
`worker 3 && echo yes` against `worker && echo yes`. Parity is met by `script/swish-check` running
the same script on aarch64 and riscv64 rather than by a second implementation.

## BUGS

- **Scripting is still nowhere, and that was never in scope.** `if`/`while`/`for`/functions and
  reading a script file are a much larger thing. This project has no story yet for what a script
  *is* when a program namespace is an endowment. Doing quoting and sequencing first is what makes
  that question answerable rather than theoretical.
- The status is the shell's own reading of the line, not a program's. No program in this system
  reports an exit status. A spawned program answers with a value, with bytes, or through a job frame.
  A per-program status is the one §301 (exit status and output) rules: `SYS_EXIT`'s first word, forwarded in the
  `EVENT_EXIT` message the shell already receives. Milestone 172 builds it.
- `$?` is readable only in `echo`. Substituting a word anywhere else needs the machinery that the
  variables of milestone 47 (navigation and naming) need anyway.
- There is no grouping, so `a && b || c` is left to right with no precedence. `{ }` and `( )`
  should arrive with milestone 52's subshells.
- This block stood at NOT-STARTED for twelve days after the work merged. That is the §76 (what
  catches a milestone status that is wrong in) class the
  roadmap split and its gate exist to catch, and neither caught it. The gate compares the index row
  against the file's status line. Here both said NOT-STARTED, so they agreed and the build stayed
  green. Two records that agree with each other and not with the tree are invisible to a
  consistency check. The remaining full list of limitations is in notes/swish-language.md, next to
  the feature a reader meets.

Effort: small to medium, and it was. It is mostly `grant_plan` and `swish`, both host-testable, so
nearly all of it is proven in milliseconds without an emulator.

## Follow-on

- **Recorded.** `notes/swish-language.md` carries the backslash escape in `BUGS` beside the quoting
  feature. None is planned while every token is a slice of the line. The quoted spelling is the
  one that works.
- **Recorded.** `notes/swish-language.md` also says scripting is nowhere. `if`, `while`, `for`,
  functions and reading a script file were never in scope. This project has no story yet for
  what a script *is* when a program namespace is an endowment.
- **Milestone 172.** A per-program exit status. §301 carries it on `SYS_EXIT`'s first word and
  forwards it in the death message, so it needs no `spawnproto` bit. Milestone 172 (A
  capability-native subprocess primitive) builds it.
- **Milestone 47.** `$?` readable anywhere but `echo`. Substituting a word in an arbitrary position
  needs the machinery that the variables of milestone 47 (navigation and naming) need anyway.
  Building half of it here would have been
  building it twice.
- **Milestone 52.** Grouping. `a && b || c` is left to right with no precedence between `&&` and
  `||`, and there is no `{ }` or `( )` to override it. Both should arrive with subshells.
- **Recorded.** `design/decisions/0076-roadmap-status-versus-tree.md` states the weakness that let
  this block sit at NOT-STARTED for twelve days after the work merged. The gate cannot see a status
  that is wrong in both places. Two records agreeing with each other and not with the tree
  look consistent to a consistency check.

## Index row

quoting was an authority gap rather than a convenience, and it closed. A name with a space can be
granted, and a quoted pattern narrows to one name where a bare one designates a set. `;`, `&&` and
`||` split outermost. The fork the block was raised to settle is settled: a refusal is not an error,
and `$?` says which (`0` ran, `1` failed, `2` refused). Built 2026-08-04; the record
caught up 2026-08-16
