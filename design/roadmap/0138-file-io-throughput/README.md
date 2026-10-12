# Close the read gap: the appendices to milestone 138

[`design/roadmap/0138-file-io-throughput.md`](../0138-file-io-throughput.md) carries the status, the
measurement it set out to move, the decision to take all four steps, what each step measured, the
frontier past them, the `BUGS` and the follow-on work. This directory carries the working its prose
budget has no room for.

- [`options.md`](options.md): the three options as first written, the record-level sweep that
  priced them, and the count of five repeated block reads that ruled out replacing RedoxFS.
- [`record-and-transfer.md`](record-and-transfer.md): steps 1 and 3, the 8 KiB record and the
  64 KiB file request, with their residuals, refusals and crash re-runs.
- [`batching-and-cache.md`](batching-and-cache.md): steps 4 and 2, the 16-block blk transfer and the
  64-slot metadata cache, with their tables and the combined 70.9x.
- [`frontier.md`](frontier.md): the 13 us residual as a frontier rather than a wall, and why a data
  cache stayed out of scope.

*Name: provisional, minted 2026-10-12 (UTC) by maintainer/prose-budget-paydown, which split the
parent under §212 (a prose budget). The directory follows §212's siting default. calef names
directories.*

- `options` Name: provisional, minted 2026-10-12 (UTC) by maintainer/prose-budget-paydown, for the
  parent's "The candidate fixes" and the sections that priced them. calef names files.
- `record-and-transfer` Name: provisional, minted 2026-10-12 (UTC) by
  maintainer/prose-budget-paydown, for the parent's steps 1 and 3. calef names files.
- `batching-and-cache` Name: provisional, minted 2026-10-12 (UTC) by
  maintainer/prose-budget-paydown, for the parent's steps 4 and 2. calef names files.
- `frontier` Name: provisional, minted 2026-10-12 (UTC) by maintainer/prose-budget-paydown, for the
  parent's "The question underneath" and "What is out of scope". calef names files.
