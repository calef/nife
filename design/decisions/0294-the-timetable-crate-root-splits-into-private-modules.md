---
status: PROPOSED
raised: 2026-10-12
---

# 294. The timetable crate root splits into private modules

Raised 2026-10-12 (UTC) as the cut that
[milestone 877 (the timetable crate root is split along its seams)](../roadmap/0877-the-timetable-crate-root-is-split-along-its-seams.md)
says is calef's call before a line moves. Written by lane
`milestone/877-the-timetable-crate-root-is-split-along-its-seams`. *(Section number provisional
until the merge queue lands it.)*

It asks calef nothing. The block raised four questions on 2026-10-11, before §282 (when a file is
split, the parent becomes an index) was ruled. Each now has a ruling that answers it, set out
below, and the lane builds on those answers. Every module name ships provisional for calef's
naming pass.

## What the file is today

`crates/timetable/src/lib.rs` is 2,468 lines on main at `17b23264e`, unchanged since the block
measured it at `396187b0b`. It holds three layers and their tests: the document parser, the
registry (what each entry becomes, and when it fires) and the printer. The block's own table has
the line ranges.

## The cut

Private modules, children of the crate root, each with its tests in a `tests.rs` child, as §272
(the grant-plan crate root splits into private modules) laid out. `lib.rs` re-exports every public
item by name, so no path outside the crate changes. Names are provisional.

| file | holds | lines, about |
|---|---|---|
| `lib.rs` | crate docs, `mod` lines, named re-exports, `NANOS_PER_SEC`, `MAX_ENTRIES` | 120 |
| `document.rs` | `Schedule`, `Entry`, `Document`, `Error`, `parse` and its three helpers | 240 |
| `admission.rs` | `Held`, `SHIPPED_HELD`, `Unbacked`, `Admission`; `admit` and its installed forms, `schedulable`, `unbacked` | 270 |
| `registry.rs` | `Row`, the beat sentinels, `WallReading`, `Registry`; `next_after` | 460 |
| `plan_text.rs` | `write_plan`, its four writers, `Audit` | 280 |
| `test_support.rs` (`#[cfg(test)]`) | `REFERENCE`, `shown`, `with_clock`, which more than one test file uses | 40 |
| `document/tests.rs`, `admission/tests.rs`, `registry/tests.rs`, `plan_text/tests.rs` | the 40 tests, by the layer each checks | 125, 430, 330, 200 |

The two tests of the verdict word go to `registration.rs`'s own tests, beside
`registration::verdicts`, which they check. The two constants stay in `lib.rs` as the exception
§282 allows: one line each, and three of the four layers use them.

## The block's four questions, and what answers each

| block question | answered by | answer |
|---|---|---|
| 1. By feature, or tests only | §272 and §279 (the system-initializer crate root splits into private modules), and calef's rulings on #1913 and #1915 that tests sit beside the code they test, split by topic | By feature, each layer with its own tests. |
| 2. Whether the registry's 29 tests leave `lib.rs` | §282 | The registry itself leaves, so its tests go with it. The block's 1,690-line `lib.rs` is not an index. |
| 3. Private modules with re-exports, or public | §272's fork 1, calef: *"Private modules"* | Private, with named re-exports. `contract`, `recurrence` and `registration` stay public, as they are today. |
| 4. Every module name | calef's naming pass, as for #1919 and §272's fork 4 | Provisional, each marked with a `//! Name:` block. |

The block's premise for question 2 changed under it: it put the registry in `lib.rs` because the
file was then under the ceiling either way. §282 moves the registry out for regrowth, not for
length, which is this file's history: 1,619 lines on 2026-09-24 and 2,468 nine days later.

## What the move costs beyond moving code

- Visibility. Three items widen to `pub(super)`, which in a child of the crate root means the
  crate. `Registry::held`, a field, because `write_plan` reads it and now sits in a sibling; §282's
  third reason says that is where a reader should see the dependency. `admit` and
  `admit_installed_row`, because `Registry::register` and `Registry::plan_at_fire` call them. The
  test helpers in `test_support` are `pub(crate)` under `#[cfg(test)]`. Nothing else widens, and no
  public item changes. This is the block's done-when 2, as §282 amends it.
- The falsification patches. All five under `crates/timetable/falsifications/` that diff
  `lib.rs` patch `next_after`. The block thought one patched `unbacked`; its hunk header only names
  `unbacked` as context. Each is regenerated against `registry.rs` and shown to fail its harness
  again with `script/falsifications`.
- Intra-doc links in moved items resolve from their new module. Those the moved code's own imports
  do not cover get a `crate::` path. The crate docs' link to `Endowment` targets
  `grant_plan::Endowment`, since `lib.rs` no longer imports it.
- The ratchets. `design/file-length-baseline.tsv` and `design/parent-index-baseline.tsv` drop
  their rows for `lib.rs`. The crate docs stay at line 1 at 82 lines, so the comment-block row
  holds, and no moved comment block is over 40 lines.
- The Kani harnesses in `proofs.rs` name `crate::next_after` and `crate::recurrence`, which still
  resolve. `#[cfg(kani)] mod proofs;` stays in `lib.rs`.
- Citations under `design/` that name `lib.rs` for an item that moved are listed on the pull
  request for the maintainer, under §277 (cite a heading or a symbol, never a line number).

## What it does not decide

No public path, wire format or syscall changes. No function body changes, and no test body.
