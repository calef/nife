---
status: PROPOSED
raised: 2026-10-15
milestone_dependencies: 860
decision_dependencies: 267
machine_requirements: none
specific_machine: none
needs_person: no
---
# Kernel comments in the Linux style

calef asked on 2026-10-15 (UTC) what applying Linux's commenting convention to this tree would look
like and what it would cost. He asked for this proposal after the answer. It amends §267 (a comment
states the constraint as it is now) and the "Comments" section of `AGENTS.md`, so nothing here is in
force until he rules on it.

## Linux's convention

`Documentation/process/coding-style.rst` and `scripts/checkpatch.pl` hold it:

1. A comment says what a function does and why, at its head. A comment inside a function body is
   discouraged; the function should be small and its working obvious.
2. A public function gets a kernel-doc header: a one-line summary, then `@arg:`, `Context:` (what
   locks and what context) and `Return:`. The published documentation is extracted from these.
3. Design lives in `Documentation/`, not in source.
4. History, credit and bug references go in commit trailers (`Fixes:`, `Link:`), never in comments.
5. An invariant is code where it can be: `lockdep_assert_held()`, `might_sleep()`, `BUILD_BUG_ON()`.
6. Two comments are demanded rather than discouraged: one on every memory barrier, naming what it
   pairs with.

This tree already has 4 (§267's second rule and its 2026-10-09 amendment), 5 where a lane has done
it, and 6 in its `// SAFETY:` discipline. Rule 2's place is taken by rustdoc. What it does not have
is rules 1 and 3, and the density they produce.

## The gap, measured

`notes/comment-cost-2026-10-07.md` measured comment lines per code line with one lexer across trees:
this kernel 0.92, Linux's `kernel/` 0.33, seL4 0.15, Redox's kernel 0.09 and Rust's `core` 1.22.

A cruder count of `kernel/` on 2026-10-15 (whole-line comments only) splits the mass by kind:

| kind | lines |
|---|---|
| code | 43,054 |
| doc comments (`///`, `//!`) | 27,802 |
| plain comments (`//`) | 9,339 |
| plain comments indented 8 or more, so inside a body | at least 2,885 |
| `// SAFETY:` lines | 674 |

The bulk is item documentation, not narrative inside bodies. Of the lines in blocks over 10 lines,
89% are doc comments, and over 20 lines it is 92%. A rule about function bodies alone would move at
most about 9,000 lines and leave the kernel near 0.65. The long doc block is the real target, and it
is Linux's rule 3: a design essay on an item belongs in the note for its subsystem.

| a block cap of | blocks over it | lines over it |
|---|---|---|
| 40 (in force) | 0 | 0 |
| 20 | 287 | 2,586 |
| 10 | 942 | 8,392 |

## The rule proposed, for `kernel/` only

1. An item's doc comment is in the kernel-doc shape. It gives a summary sentence, then the
   arguments, the context it may run in and what it holds, and what it returns or refuses, with a
   link to its note. A design argument goes in the note.
2. A comment inside a function body states a constraint the code cannot show: an ordering, a
   `SAFETY` argument, a hardware quirk. Narrative, worked examples and "how this works" move to the
   item's doc comment or the note.
3. An invariant a comment states as "the caller must..." becomes an assertion or a type where one
   fits, as `NameCopy` did for milestone 825 (a hostile client races the file server's name window).

Everything else stays: §267's rules, `SAFETY` comments, `missing_docs`, and the tree outside
`kernel/`. The newcomer test in `AGENTS.md` is met by the note a doc comment links to, which is
where Linux meets it too.

## The mechanism

- The block cap in `kernel/` steps from 40 to 20 and then to 10, each step a `--bank` of
  `helpers/comment_block_ratchet.py` with the over-cap blocks listed and held. The rest of the tree
  stays at 40.
- A second count in the same helper: plain comment lines inside function bodies in `kernel/`, held
  by a ceiling that only falls, like `helpers/line_citations.py`. `// SAFETY:` lines are not
  counted.
- Both are charted in `notes/project-metrics.md` with the existing chart code.

Reuse: `helpers/comment_block_ratchet.py`'s block definition, baseline and `--bank`, taken; the
lexer in `notes/comment-cost-2026-10-07/rustlex.py`, taken for finding function bodies;
`script/metrics`' ratchet chart, taken. What is written is the in-body count and a per-directory cap.

## Cost

- Lanes: about 11,000 lines are judged, 8,392 over a 10-line cap plus the in-body narrative. Each
  is cut, moved to a note, or turned into code. At 500 to 1,000 lines a reviewable pull request
  (assumed, not measured), that is 11 to 22 paydown pull requests, at most two in the queue.
- CI: little. `helpers/prose_only.py` drops whole-line comments before judging a Rust file, so a
  pull request that changes only comments and notes skips the jobs a prose change skips.
- The return: comments are 57% of the bytes in this tree's Rust, and every agent that opens a kernel
  file reads all of them. The sweep moves the kernel from about 0.86 comment lines per code line to
  about 0.6 by the crude count, and the essays it moves are read only by an agent that follows the
  link.

The end state is not Linux's 0.33, and this proposal does not aim at it. Reaching it would mean
cutting the item documentation the newcomer test rests on, not moving it.

## Done means

- §267 is amended in calef's words, and `AGENTS.md`'s "Comments" section says the same.
- The `kernel/` cap is 10, every block over it is gone or listed, and the in-body count has a
  ceiling, both in `script/lint` with selftests.
- Both counts are on `notes/project-metrics.md`.

## What is not decided

Whether the rest of the tree follows once the kernel has been done. `crates/` is the next largest,
and its comments are read by the same agents.

*Name: provisional, maintainer/kernel-comments-linux-style, 2026-10-15 (UTC).*
