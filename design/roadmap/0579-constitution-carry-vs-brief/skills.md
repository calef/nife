# Milestone 579's refusal of skills, for the record

An appendix to [`design/roadmap/0579-constitution-carry-vs-brief.md`](../0579-constitution-carry-vs-brief.md),
milestone 579 (which of the constitution must be carried, and which is a brief). It holds the three
reasons, as filed on 2026-09-23, for answering no when calef asked whether the roles should become
skills. It was moved out of the block on 2026-10-12 (UTC) under §212 (a prose budget). The block is
complete without it, and says what the tree did afterwards.

calef asked whether the roles should become Claude Code skills, loaded on demand from
`.claude/skills/`. The answer is no, for three reasons that were given in conversation and belong in
the tree rather than only in a chat log.

## `AGENTS.md` is deliberately the cross-tool convention

[`design/tenets/project-history.md`](../../tenets/project-history.md) records that `CLAUDE.md` is a
symlink to it for exactly this reason: the file addresses any competent agent, not one vendor's CLI.
`.claude/skills/` is Claude Code's own format, invisible to any other tool that might read this
tree.

## Principle 3 fails outright

The test this file sets for itself is "could a competent stranger, with only this repository, get to
a passing build and a correct mental model without opening a chat window." A skill that loads only
inside one CLI, on a trigger phrase that stranger cannot see in advance, is invisible to them by
construction.

## On-demand loading is the wrong direction for anything that must fire unprompted

That is most of the carry column. A skill is consulted when an agent (or the harness) decides the
moment calls for it. The whole problem this proposal is solving for the `brief` column is "consulted
at a known task." That is a solved problem already, in `briefs/`, in plain Markdown, read by any
tool that can read a file. Nothing about the `uncertain` or `carry` rows would be improved by
putting them behind a trigger phrase. Several of them (the landmines) are precisely the rows that
must fire when nobody thought to ask.

`briefs/` already does what a skill would be asked to do here, without the vendor lock-in and
without weakening principle 3, and the block's five candidates are ready to populate it further.
