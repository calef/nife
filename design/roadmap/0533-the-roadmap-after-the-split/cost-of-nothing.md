# What it costs to do nothing: an appendix to milestone 533

[Milestone 533 (whether the roadmap should become issues, and what a citation means after the repository
split)](../0533-the-roadmap-after-the-split.md) keeps the options and the recommendation. This appendix
holds the proposal's account of what stays painful once the decouple-the-lanes work on #999 lands, and
its answer to calef's question of whether the current approach is still right.

The in-flight work on #999 removes three couplings, and they should not be re-argued here. From its
block: `script/fatal-risks` reads the blocks rather than the generated index; `script/audits` does
too; and `script/decisions` no longer fails on a gap, with `design/naming.md` gaining the rule
*"Both schemes are sparse: a duplicate is the defect, a gap is not."* A new `script/lint` check stops
the index coupling growing back, allowing only `script/metrics` and `script/catch-up`, which read
historical revisions where no block-derived answer exists.

What remains painful after that, which is the real case for changing anything further:

- The number is still hand-minted and still global. 443 makes a collision cheap to *resolve*
  (take the next free number, leave the hole) rather than impossible to *have*. Two sessions that
  cannot see each other still both reach for the next number, and the integrator still arbitrates at
  merge. The prior art's answer to this (let the tracker mint it) is untouched by 443.
- The citation identifier is a bare number and 443 does not look at it. This is the whole of the
  parent's "The split is the forcing function" section, and it is entirely unaffected.
- 21 scripts under `script/` read `design/roadmap/`, not 12 as first estimated: `audits`, `bench`,
  `board-netboot`, `boot-check`, `catch-up`, `ci-build`, `citations`, `fastpath-footprint`,
  `fatal-risks`, `icount`, `job-mix`, `journeys`, `lint`, `metrics`, `names`, `repeat-under-load`,
  `roadmap`, `swish-check`, `vendor-verify`, `vendor-watch`, `verify`, plus `xtask/src/main.rs` and
  two files under `helpers/`. Every one of them is a reason the record must stay readable from a
  checkout with no network. 443 changes which *file* two of them read, not that they read the tree.
- The proposals pile has no owner. 24 files, all dated the same day, promoted only by an
  integrator who happens to look. Milestone 247 argued a due-date gate would be routed around, and
  that argument is good; it leaves the pile's health depending on someone noticing, which is rung
  zero in the vocabulary AGENTS.md uses for exactly this.
- Nothing here works for a person without a clone, and §151's third-party programs are the point
  of the split.

So the honest answer to "are we sure our current approach is still the right approach" is: yes for
the record, yes for the gates, and the scheduling layer already left. What is not right is that
the identifier is a number, and that is not a problem 443 was scoped to touch.
