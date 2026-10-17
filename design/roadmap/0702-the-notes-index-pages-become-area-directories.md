---
status: BUILT
raised: 2026-09-30
built: 2026-10-17
promoted_from: the-notes-index-pages-become-area-directories
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 702. The notes index pages become area directories

*Title and slug are drafts.*
## What is in force

Ruled B. calef, 2026-10-17 (UTC), relayed by the maintainer session: *"Yes, B."*

A new collection in `notes/` gets a directory with its `README.md` beside it; `notes/coes/` is the
first, and `notes/net/` and `design/fatal-risks/` are the prior art. The flat notes and the
`notes/README/` area pages are the legacy shape. An area migrates into its own directory only when
that area is next swept anyway; nothing is moved for its own sake. The rule is written where a
person adding a note reads it, the "To add a note" paragraph of [notes/README.md](../../notes/README.md).

Two shapes live side by side until the last area moves. This block is the marker for that: the flat
shape is the legacy and the directory with its README is the direction.

## Why B

- Migrating everything at once (A) is 233 file moves, a rewrite of links in 69 files outside
  `notes/` and of the lint index check and the citation ratchet's path pairs, in one sweep. Reaching
  the same endpoint gradually costs the same and lands in pieces.
- A directory forces a single home for each note. The flat shape lets a note sit in one place while
  two area pages mention it, and that is worth keeping for the notes not yet swept.
- Extending the `README/` pages (C) keeps growing the shape calef named as wrong on 2026-09-30: a
  directory named `README/` is an index split masquerading as structure.

## Follow-on

- **Recorded.** Nothing enforces "migrate an area when it is next swept". `script/lint` accepts a new flat note that
belongs to an area already moved into a directory. No area has moved yet, so no such check has
anything to guard; add one with the first area migration.

## Prior art

`design/fatal-risks/` (consolidated 2026-09-30, calef's ruling) and `notes/net/` (split from
`notes/net.md` by a lane, kept since). Both put the README beside the collection it summarizes.

## Index row

`notes/` keeps its flat notes and `README/` area pages as legacy; a new collection gets a directory with its README beside it (ruled B, 2026-10-17).
