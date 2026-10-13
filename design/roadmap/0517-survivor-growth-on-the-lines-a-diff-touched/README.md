# Survivor growth on the lines a diff touched: the appendices to milestone 517

[Milestone 517 (what fraction of survivor growth arrives on lines a pull request
touched)](../0517-survivor-growth-on-the-lines-a-diff-touched.md) carries the answer, the method,
the verdict on the kernel, the proposed green condition, what was not attempted, its Follow-on, its
`BUGS` and its Index row. This directory carries the working behind three of its checks, which its
prose budget has no room for.

- [`bounds.md`](bounds.md): how the share moves with the cut date, and the old-line check with the
  renames that read as regressions.
- [`replays.md`](replays.md): the six `--in-diff` replays that tested the blame proxy.
- [`kernel-pricing.md`](kernel-pricing.md): what putting `kernel/**` and `components/**` in the
  corpus would cost, and why.

*Name: provisional. The directory follows §212 (a prose budget)'s siting default,
`design/roadmap/X/` beside `design/roadmap/X.md`. calef names directories.*

- `bounds` Name: provisional. It holds the parent's "The bound on 81.6%, stated as a range" and "The
  other 18.4%". calef names files.
- `replays` Name: provisional. After the parent's "Does the proxy tell the truth: six replays, 197
  of 197". calef names files.
- `kernel-pricing` Name: provisional. After the parent's "What it would take to put `kernel/**` and
  `components/**` in the corpus". calef names files.
