---
status: DECIDED
raised: 2026-10-11
decided: 2026-10-12
ratified_by: calef
---

# 283. A function is held to 100 code lines by a lint ratchet

Raised by the lane of milestone 888 (a ratchet on function length) on 2026-10-11 (UTC). *(Section number
provisional until the merge queue lands it.)*

## The ruling, 2026-10-12 (UTC)

calef answered each question below "A", the recommended option, recorded on pull request #1963.

1. The unit is code lines, as clippy's `too_many_lines` counts them. "A"
2. `excessive_nesting` is left out of this milestone. "A"
3. One baseline file, `design/lint-baseline.tsv`, keyed by lint. "A"

He also ruled the sequencing "A": milestone 888 lands PARTIAL, and milestone 876 (the kernel crate
root is split along its seams) completes the `kernel_main` split.

## The scope

The ratchet holds every function with a body in every tracked `.rs` file outside `vendor/`. That
includes host tools, `xtask`, build scripts, tests, benches, fixtures and items under
`#[cfg(test)]`, as well as the Rust that runs on nife. calef ruled it on 2026-10-13 (UTC): "Yes,
widen the gate". The gate and the function-length chart read one scope function,
`helpers/function_length.py`'s `in_scope`, so they count the same files.

## What was already ruled

calef ruled two parts on 2026-10-09 (UTC). The threshold is 100 code lines, with every function
over it baselined. `kernel_main` is split by boot phase. Neither is reopened here.

## What is being decided

Three questions the build could not settle alone. The lane built on the recommended answer to
each, and each was ruled as recommended.

### 1. The unit: code lines, or physical lines

§266 (a Rust source file stays under 2,000 lines) counts a file's physical lines, comments included, because a reader loads every line. Clippy's
`too_many_lines` counts code lines: blank and comment lines are free.

| | code lines (built) | physical lines |
|---|---|---|
| what clippy reports | yes, natively | no; needs our own counter |
| deleting comments to pass | buys nothing | buys a pass, §266's named incentive |
| agrees with §266 | no | yes |
| `kernel_main` today | 977 | 2,035 |
| functions over 100 | 46 | not measured |

Prior art: clippy counts code lines, measured on this toolchain at a threshold of 0. From memory,
not re-read: Checkstyle's `MethodLength` counts code lines with `countEmpty` off, and ESLint's
`max-lines-per-function` counts physical lines unless told to skip comments and blanks.

Recommendation: code lines. The reason is the incentive, and it holds at equal cost. A
function ceiling that counts comments rewards deleting the comments this tree writes on purpose,
which §266 names as the failure its own measure risks. The cost is that the two ratchets disagree
on what a line is, and each states its unit in its failure message. If no, the measure gains a
physical-line counter and the baseline is rewritten once, before it lands.

### 2. `excessive_nesting` at a threshold of 6

Run 2026-10-11 (UTC) over the five census configurations, it finds 10 blocks in 4 files. At 4 the
census found 243 blocks in 56 files on 2026-10-09, over the host and aarch64 passes only. It
measures depth, which length does not.

The options are to register it now as a second measure in the same ratchet, or to leave it out.
Recommendation: leave it out of this milestone. Ten blocks are few enough to fix as found, and
a lint at that count is better adopted under §61 (a lint is adopted on evidence from this tree) at `-D warnings` once they are fixed. That is how
`undocumented_unsafe_blocks` went in. A ratchet earns its machinery when the count is too large
to fix first. If yes, it is one entry in `helpers/lint_ratchet.py`'s table and ten baseline rows.

### 3. One baseline for every lint ratchet, or one per proposal

This is the panic proposal's open question 5, and building the shared ratchet forced it.

Recommendation: one file, `design/lint-baseline.tsv`, keyed by lint. The panic proposal
recommends the same. One file means one parser, one merge-base comparison and one `--bank`. Rows
for different lints never share a key, so two proposals editing it conflict only when they touch
the same file's row. If no, each lint names its own file in the table, a change of a few lines.

## What was blocked

Nothing was blocked on these. The ratchet ran on the recommended answers before they were ruled.
Criterion 4 of milestone 888 was held for a different reason, §266's ceiling on
`kernel/src/lib.rs`; the sequencing ruling above sends it to milestone 876.
