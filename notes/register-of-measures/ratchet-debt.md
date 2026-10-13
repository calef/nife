# Three ratchets' debt

*(Appendix of [the register](../register-of-measures.md). Stem provisional, minted on 2026-10-13
(UTC) by lane/ratchet-metrics, as are the files `writing-standards.csv`, `comment-blocks.csv` and
`parent-index.csv`, their columns and their charts. calef asked for these charts on 2026-10-13 so
the three baselines' debt can be driven down.)*

Three gates in `script/lint` hold a debt to a baseline that only falls. They enforce §213
(writing standards), §267 (a comment states the constraint as it is now) and §282 (when a file is
split, the parent becomes an index). A baseline says what may not grow. These series say how much
is left, one row per ISO week in `notes/project-metrics/`, read by `script/metrics` from that
week's commit.

| file | column | what it is |
|---|---|---|
| `writing-standards.csv` | `writing_over_median` | documents whose median sentence is over 20 words |
| | `writing_over_longest` | documents whose longest sentence is over 40 words |
| | `writing_over_bold` | documents with more than 4 bold spans per 1,000 words |
| `comment-blocks.csv` | `comment_block_files_over_cap` | Rust files holding a comment block over 40 lines |
| | `comment_blocks_over_cap` | the blocks themselves, which is what a baseline row counts |
| | `comment_block_excess_lines` | each such block's lines past 40, summed |
| `parent-index.csv` | `parents_over_index` | split parents holding code an index may not hold |
| | `parent_index_lines` | the code lines of those items, summed |
| | `parent_index_marked_lines` | the part of that sum an `// Index exception` mark excuses |

## One measure, the gate's

Each series calls the gate's own functions on the week's text, so the chart and the gate cannot
disagree about a file:

- §213: `prose_ratchet.measured` measures each document and `prose_ratchet.over` judges it, over
  `prose_ratchet.documents`, the scope `prose_budget` already shares. The 200-word floor for the
  median and the bold density is the gate's. A document over two limits counts in both columns, so
  the chart groups the three bars rather than stacking them.
- §267: `comment_block_ratchet.blocks_over` over its `in_scope` files, every `.rs` outside
  `vendor/`.
- §282: `parent_index.judge` over the same `in_scope` rule. A small item the handful allowance lets
  stand is not debt, because §282 allows it.

## Counted from the tree, exemptions included

The series never read a baseline's rows. A baseline holds ceilings, and a ceiling can sit above
what the tree measures until someone banks the shrink; the tree is the debt. A `writing-standards`
exception marker and an `// Index exception` mark excuse an item at the gate and not here, for the
prose budget's reason: a chart that subtracted what the gate excuses would hide the debt it exists
to show. `parent_index_marked_lines` keeps the excused part visible on its own.

## Every week, from 2026W29

`--backfill-measure` restated each series from 2026W29, each week from its own commit. The helpers
are today's, applied to old text, so a week from before a gate existed reads as the debt that gate
would have found then. That is every row's rule here, and it is why no week is blank.

## BUGS

- `writing_over_bold` reads zero from 2026W41, where §213's bold backlog was cleared, and the gate
  refuses any bold ceiling. It stays a column so a regression would show.
- Today's measure applied to an old week can read a construct the old tree used differently. The
  roadmap's field tokens are one: `measured` reads both forms, so they do not inflate the median.
- None of these sees quality. A shorter sentence can be worse, and a comment block cut to 40 lines
  can lose a fact; §267's remedy is to move history to the commit message, not to delete it.
