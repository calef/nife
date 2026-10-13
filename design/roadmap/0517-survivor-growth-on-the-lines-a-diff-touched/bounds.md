# How far the 81.6% can be trusted: the cut date and the old lines

An appendix to [milestone 517 (what fraction of survivor growth arrives on lines a pull request
touched)](../0517-survivor-growth-on-the-lines-a-diff-touched.md). The main page is complete without
it. It holds the two checks on the fraction's edges. One is how the share moves with the cut date.
The other is whether the survivors on lines older than the window were there when it opened.

## The bound on 81.6%, stated as a range

It is 68% to 85% across the plausible boundaries, and 81.6% at the right one. The baseline run
is dated 2026-08-03 with no hour recorded, so the cut is a day rather than an instant. The window's
first two days are unusually dense:

| cut | in-window share |
|---|---|
| 2026-08-01 | 84.8% |
| 2026-08-03 (the baseline's own date) | 81.6% |
| 2026-08-05 | 68.1% |

The 13-point drop between the third row and the second is one day. 104 survivors are blamed to
commits of 2026-08-04, the day after the baseline, and those are unambiguously in the window. The
row is in the table to show how much of the answer rides on two days of a six-week window. It is
not there because 68.1% is a candidate answer.

## The other 18.4%: pre-existing, with exactly one exception

The 142 survivors on old lines are the population an inflow gate is structurally blind to. Whether
that matters depends entirely on what they are. Old survivors are the corpus rate's to own, and no
inflow gate was ever meant to catch them. Regressions are code that was once tested and quietly
stopped being.


That is answerable without a census. Run the mutation over the tree as it stood at the window's
start, and ask of each of the 142 whether it was already a survivor then. The tree at
`c87c1575`, the 2026-08-03 merge of milestone 85 (mutation testing over the host crates), is that
tree. Every crate holding an old-line survivor was run against it: 4,730 mutants over 27 packages,
2 hours 9 minutes on this laptop, 152 survivors.

| the 142 survivors on old lines | count |
|---|---|
| already a survivor on 2026-08-03 | 141 |
| caught on 2026-08-03, surviving at the census | 1 |

The one regression is `compositor`'s `replace * with + in Rect::area`. On 2026-08-03 that mutant
is in `caught.txt`, with the three others the same function generates. At the census it is a
survivor, on a line nobody in the window touched. It is exactly the case a diff-scoped inflow gate
cannot see. It is real, and out of 771 survivors it is one.

Three near-misses are worth recording because they are the trap in this comparison. All three
read as regressions until the rename is undone: `page_frames`' two (`FrameAllocator` became
`PageFrameAllocator`) and `inter_process_communication`'s one (`Endpoint` became `Rendezvous`). The
mutation text is identical and the type name is not. So a comparison keyed on the printed mutant
name scores a pre-existing survivor as fresh damage. This is the same failure the [replay table](replays.md) hit
on `isa` becoming `machine_discovery`, in a tree that renames deliberately and often.

