# The manual: where it goes next, and its prior art

Moved verbatim from [`notes/documentation.md`](../documentation.md) on 2026-10-13 (UTC), to bring
that note under the cap of §212 (a prose budget) by the graph's whole-file count. The note keeps a
summary of each.

## Where this goes next

The guest-side `apropos`, `mdr <page>` at the prompt and length ranking, which used to head this
list, are all built. What is left, in the order it pays off:

1. **Color and the pager, which are the same decision's other two thirds.** DECISIONS §106 took
   the narrowing for a tail stage's *primary* output; it did not extend the same bit to "tell this
   stage it ends at a real screen" (color, the honest replacement for `isatty`) or to granting one
   line of *input* without granting the keyboard (the pager). Both still want the wiring bit this
   entry originally scoped for all three; §106 built the narrowest slice that unblocks `mdr <page>`.
   This is a spawn-protocol decision, the same shape as §106 itself, so it is not this lane's call
   to make unilaterally.
2. The store as something a package installs, rather than a table in `xtask`. `DOC_BUNDLES` is
   the shape milestone 40 asked for minus a package manager, and milestone 39 is where the manifest,
   the hash and the version it should hang off already live. Blocked on milestone 39 itself, whose
   own Gate is `DECISION, MILESTONE 23` with no decision taken.

Phase 3 of the roadmap (a graphical viewer as a compositor client) is untouched and still wants
milestone 33's rungs, which are themselves blocked (notes/frames.md's `Frame`-sizing fork, milestone
29's status).

## Prior art

`man` plus `apropos` plus `mandb` for the split between format, index and pager, which is this
architecture minus the troff. Dash and Zeal *docsets* for the bundle shape. `cargo doc`'s HTML as
the road not taken: it would need a browser engine, which is a mountain with no thesis behind it.
