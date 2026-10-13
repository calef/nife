# The function-length series

*(Appendix of [the register](../register-of-measures.md). Stem provisional, minted by milestone 888
(a ratchet on function length) on 2026-10-11 (UTC), when calef asked for a chart that drives the
long functions out, modeled on the file-size chart.)*

One row per ISO week, read from that week's commit by `script/metrics` and restated like every
other series. Three columns in `notes/project-metrics/function-length.csv`:

| column | what it is |
|---|---|
| `code_lines_share_over_100_pct` | code lines inside functions over 100 code lines, as a share of all in-scope code lines |
| `functions_over_100` | functions over 100 code lines |
| `excess_over_100` | those functions' code lines past 100, summed |

The chart plots the share with a trailing 10-week mean, calef's ruling for the file-size chart
applied here: one split moves the share in a week, so the mean says whether the splits are winning.
The count and the excess are on the page's generated line.

## The measure is clippy's

A function's length is the count clippy's `too_many_lines` reports: the lines of its body, braces
excluded, that carry code once comments and blank lines are skipped. `helpers/function_length.py`
reproduces it from source text, because this script reads past commits as blobs and may not build.
Measured at a threshold of 0 on 2026-10-13 (UTC) over the lint ratchet's eight clippy passes, the
two agreed on 11,209 of the 11,217 functions clippy reported; the 8 others are under BUGS.
`script/lint`'s lint ratchet compares them on every function clippy reports over 100, and fails on
any difference.

The numerator counts each line once, so a function nested in a long one adds nothing. The
denominator is every in-scope code line, inside a function or not, `helpers/rust_source.py`'s
code-line measure. A closure is not a function here: its lines belong to the function around it.

## The scope

Every tracked `.rs` file outside `vendor/`, test code and `#[cfg(test)]` items included (§283 (a
function is held to 100 code lines by a lint ratchet), calef, 2026-10-13 UTC).
`helpers/function_length.py`'s `in_scope` is the one scope function; the lint ratchet's gate reads
the same one, so the chart and the gate count the same files. The text measure also reaches code no
clippy pass compiles, such as the kernel's build-mode features, the UEFI loader and the exercisers.

## The backfill

`--backfill-measure function-length` restated every week from 2026W29 on 2026-10-13 (UTC), from
that week's own commit, in about 100 seconds. No clippy ran, because the text measure needs none,
so the fallback calef's request named (starting the series from today if clippy at each past
commit proved impractical) was not needed. Today's scope and today's count were applied to every
week, so an old week is a restatement, as every row here is.

## BUGS

- The function finder is a lexer, not a parser. A `fn` written in a `macro_rules!` body is skipped
  where clippy reports it: 8 functions on 2026-10-13 (UTC), all under 10 code lines. The lint
  ratchet's comparison fails if one ever grows past 100.
- About 1,160 functions in 127 files are compiled by none of the eight clippy passes: `patches/`,
  `c_library/`, `uefi_loader/`, the std exercisers, `jig/`, `cryptography_provider/`, kernel build
  modes, `notes/` and `bench/`. They build only for nife's std targets or in a separate step, so the
  text measure alone holds them, with no clippy cross-check.
- The comparison runs one way. A function clippy compiles and the text measure counts over 100
  while clippy counts it under would not be caught. At threshold 0 no such function existed.
- The register's weekly-series table has no row routing to this appendix. The register sat at
  exactly its 3,000-word cap (§212 (a prose budget)) on 2026-10-11 (UTC), so the
  row waits for its next split, and the directory's README links here instead.
- A function's name is its row key, so two functions of one name in one file take ` #2` in source
  order, and reordering them swaps their rows.
