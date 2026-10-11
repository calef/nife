# What a timed wait costs: the appendices to notes/timed-wait.md

[`notes/timed-wait.md`](../timed-wait.md) carries the price list, each numbered section's finding,
where the three shapes differ, the blocked-thread census and BUGS. These files carry the
measurements behind sections 2 to 5, which its prose budget has no room for. Each keeps its
section's number and heading, because other files cite the sections by number.

- [`the-three-structures.md`](the-three-structures.md): section 2, the host model of no structure,
  a sorted list and a timer wheel, with its table and error bars.
- [`the-tick-path.md`](the-tick-path.md): section 3, the timer tick's instruction counts per
  function and the exact cost of the prototype deadline check.
- [`the-interrupt-stack.md`](the-interrupt-stack.md): section 4, why a deadline expiry keeps the
  interrupt-stack proof, the gate run, and the `SCHED` cost.
- [`the-cost-of-spinning.md`](the-cost-of-spinning.md): section 5, the derivation of what today's
  yield-spin costs against a deadline wait.

*Name: provisional, minted 2026-10-11 (UTC) by lane/sixth-ten-longest-notes, which split the parent
under §212 (a prose budget). The directory follows §212's siting default, `notes/X/` beside
`notes/X.md`. calef names directories.*

The stems below were minted the same day by the same lane. Each is named for its content, and calef
names files.

- `the-three-structures` Name: provisional, minted 2026-10-11 (UTC) by
  lane/sixth-ten-longest-notes.
- `the-tick-path` Name: provisional, minted 2026-10-11 (UTC) by lane/sixth-ten-longest-notes.
- `the-interrupt-stack` Name: provisional, minted 2026-10-11 (UTC) by
  lane/sixth-ten-longest-notes.
- `the-cost-of-spinning` Name: provisional, minted 2026-10-11 (UTC) by
  lane/sixth-ten-longest-notes.
