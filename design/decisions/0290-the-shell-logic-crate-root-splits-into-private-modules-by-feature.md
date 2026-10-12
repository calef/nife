---
status: PROPOSED
raised: 2026-10-12
---

# 290. The shell logic crate root splits into private modules by feature

Raised 2026-10-12 (UTC) as the cut that
[milestone 884 (the shell logic crate is split along its seams)](../roadmap/0884-the-shell-logic-crate-is-split-along-its-seams.md)
says is calef's call before a line moves. Written by lane
`milestone/884-the-shell-logic-crate-is-split-along-its-seams`. *(Section number provisional
until the merge queue lands it.)*

## What is asked

Nothing. The block named three questions, and rulings made since it was written answer each one:

| The block's question | Answered by | The answer |
|---|---|---|
| 1. Private modules with re-exports, or public modules | §272 (the grant-plan crate root splits into private modules), fork 1: *"Private modules"*; §279 (the system-initializer crate root splits into private modules) did the same | Private modules. `lib.rs` re-exports every public item by name, so no `swish::` path changes. |
| 2. Where the shared test helpers live | §272's cut, ruled with it: a crate-level `#[cfg(test)]` `test_support` module, imported by name | `test_support.rs` holds `shown`, `spec_of`, `listing` and `endowment`. Helpers one topic uses stay in that topic's tests. |
| 3. Every module name | calef's naming pass; he does not rule names on a split | Every name below is provisional and carries a `//! Name:` line. |

One more ruling changes the block's cut. §282 (when a file is split, the parent becomes an index)
was decided after the block was written. The block kept `Say`, `Status`, `Route`, `route`,
`write_num` and `write_config_values` in `lib.rs`, about 520 lines. Under §282 each moves to a
child named for it, and `lib.rs` keeps only docs, `mod` lines, `use` lines and re-exports. Where
those items land is layout, not a fork.

If calef wants any answer different, the move is mechanical to redo: each module is a list of item
names in the lane's script, and the program and every test reach the items through unchanged
`swish::` paths.

## What the file is

`crates/swish/src/lib.rs` is 3,850 lines at `17b23264e`, the block's figure. Lines 1 to 2,013 are
code and 2,014 to 3,850 are `mod tests`, 83 tests under 17 topic banners. The block's table of
what the file holds still describes it.

## The cut

Private modules, children of the crate root. Each takes `use super::*`, as `bare`, `complete` and
`sequence` already reach the root. Tests move to a `tests.rs` child beside the module they test,
as calef ruled for 842 and 845, split by what each test calls rather than by the banner it sat
under. Lines are measured after `cargo fmt`.

| file | holds | lines | tests |
|---|---|---|---|
| `lib.rs` | crate docs (unchanged), `mod` lines, imports, named re-exports | 134 | |
| `say.rs` | `Say`, `HEAP_MAX_BYTES`, `write_say` | 106 | 50 |
| `status.rs` | `Status`, `STATUS_WORD` | 105 | 71 |
| `routing.rs` | `Route`, `route` | 77 | 45 |
| `words.rs` | `is_pattern`, `expansion`, `designation`, `write_set`, `echo`, quoting (`Piece`, `pieces`) | 259 | 345 |
| `batching.rs` | `Sweep`, `write_sweep`, `write_batch` | 132 | 17 |
| `number.rs` | `write_num` | 25 | 14 |
| `apropos.rs` | `write_found`, `write_apropos` and their layout constants | 89 | 98 |
| `timing.rs` | `Untimed`, `write_untimed`, `write_duration`, `write_timing` | 114 | 76 |
| `sentences.rs` | `pwd`, `help`, refusals, the fixed sentences, `write_vouch`, the usage lines, `write_outcome` | 285 | 151 |
| `endowment.rs` | `write_holdings`, `write_census`, the bind row, `write_config_values` | 162 | 183 |
| `preview.rs` | `Vouched`, `caps <path>`, `write_caps`, `write_preview` and the row writer they share | 672 | 745 |
| `test_support.rs` | the four shared helpers | 53 | |

No file is over 1,500. `write_preview_rows` serves `caps <path>` and the preview, so the two share
`preview`, as the block said. `Vouched` goes with them because `write_image_caps` is its only
reader.

Visibility widens in one place. `write_config_values`, private today, is read by `endowment` and
`preview`, so it becomes `pub(super)` and `lib.rs` imports it privately for the second. The four
test helpers become `pub(crate)` inside a `#[cfg(test)]` module.

## What moving the code costs

- The cap ratchet. `lib.rs` naming `write_caps` and `write_image_caps` in its re-exports counts as
  two new identifiers. The worklist (`design/naming/capability-worklist.md`) already lists both,
  with `write_capabilities` as the target. #1913 met the same gate with `CapKind`, and the answer
  there was to apply calef's 2026-10-06 (UTC) ruling early rather than change the gate. This does
  the same, in its own commit ahead of the move: `write_capabilities` and
  `write_image_capabilities`. That touches two call sites in `components/src/swish/capabilities.rs`,
  the one change outside the crate beyond the block's comment.
- The comment-block ratchet. `write_sweep`'s 45-line doc moves to `batching.rs` whole, and §267's
  2026-10-11 (UTC) amendment carries its row there. Nothing is cut.
- `the_heap_cap_is_named_in_kib` moves to `say/tests.rs`, and its `cap` allowance moves with it.
- `design/file-length-baseline.tsv` and `design/parent-index-baseline.tsv` drop their rows.
- Intra-doc links. `status.rs` takes no glob, since it uses nothing from the root, so its three
  links to other modules name `crate::` paths.

## What it does not decide

No public path, sentence, wire format or behavior changes. No function body changes apart from the
rename.
