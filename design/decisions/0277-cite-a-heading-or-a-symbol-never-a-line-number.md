---
status: DECIDED
raised: 2026-10-11
decided: 2026-10-11
ratified_by: calef
---

# 277. Cite a heading or a symbol, never a line number

Raised and decided 2026-10-11 (UTC) in a maintainer session
(https://claude.ai/code/session_01XTmzMUrHRuzNPGgiKJjB3q), written by lane
`maintainer/no-line-links-into-docs`. *(Section number provisional until the merge queue lands it.
The helper's name below is provisional too.)*

## The ruling, as made

calef, 2026-10-11 (UTC):

> Can we establish a standard that we don't link to line numbers but instead section headers in
> documents?

Asked the same day whether citations into source code fold into the same standard, he answered:

> Yes

## The rule

- Cite a Markdown document by path and heading: `notes/net.md#the-inbound-half`, or the path with
  the heading quoted, `notes/net.md`'s "The inbound half". A passage with no heading of its own
  under the title is cited by the path and a short quote of the passage.
- Cite code by path and symbol: a function, type, constant or test, such as `kernel/src/sched.rs`
  `threads_round_robin`, or `sched::threads_round_robin`.
- Never by line number: not a path followed by `:` and a line or a range, and not a URL fragment
  `#L` and a line.

It binds every tracked file a person writes: notes, design records, briefs, code comments and
scripts.

## Why

A line number moves with every edit above it, and nothing tells the citation. A heading moves with
the text under it, and a symbol moves with its definition. The same day's §212 (a prose budget)
splits moved sixty notes into appendices, and every `notes/X.md` line citation into them went stale
at once. Pull requests #1938, #1940 and #1941 repointed them by hand to headings, verifying each
target at the commit that wrote it. A stale line citation is also invisible to every gate, because it
still resolves, to the wrong line. `script/roadmap` names that failure as the one no check of its
own can see.

## What is not a citation

Each is decided structurally by the gate rather than marked, because each is quoted output with a
shape a reader can see:

- `path:LINE:COLUMN`, the shape of a compiler diagnostic, a panic message or a grep hit. A BUGS
  entry quoting a panic from a CI log is quoting, not pointing.
- Anything in a Markdown code block, fenced or indented. That is where the tree pastes output and
  shows `EXAMPLES`.
- A GitHub permalink pinned to a commit (`blob/<sha>/...#L12`). It names an immutable file, so it
  cannot drift.
- Logs, tables and other data files (`.log`, `.tsv`, `.csv`, `.patch`, `.json`), and `vendor/`,
  `patches/` and `target/`.

Anything else that must keep a line number carries `line-citation: exception. Reason: ...` on its
line or the line above, so the exception says it is one where a reader meets it. Two do today: the
block as raised in milestone 101 (the L4 calibration), kept wrong on purpose, and a comment in
`script/citations` that quotes its own diagnostic.

## The gate

`helpers/line_citations.py`, run by `script/lint` and by its `--no-cargo` pre-push mode, with a
selftest of fixtures first.

- Citations into Markdown are held at zero. On 2026-10-11 (UTC), at `41137c18a`, the tree held 38
  by the gate's rules, in 22 files. This change repointed 35 to a heading after reading each target
  at the commit that wrote the citation, marked two, and the last was output in an indented code
  block.
- Citations into code are held by a ratchet, the shape of the `cap` gate. The same measurement found
  835 in 161 files, too many to repoint in one change. The tree total may not exceed the ceiling in
  the helper, 835, and a branch may not add any net over the files it changed against its merge base.
  A sweep lowers the ceiling with `--bank`, and nothing raises it.

## BUGS

- Prose that names a line in words, such as "line 88 of the scheduler note", is not read.
- A test fixture or a parser's sample that holds a line citation as data, in a helper's selftest, is
  counted in the code ratchet. Three helpers hold such strings today. They are on the ceiling, not
  marked, and the sweep marks or moves them.
- An indented code block is recognized by a heuristic: four spaces after a blank line, and not after
  a list item. A list continuation indented four spaces is read as prose, which errs toward
  counting.
- `path:LINE: text` is the shape of grep and lint output too, but it is counted, because a writer can
  put a colon after a citation. Such output outside a code block needs the marker.
