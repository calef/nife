# The HVF leg: the appendices to notes/hvf-leg.md

[`notes/hvf-leg.md`](../hvf-leg.md) carries the leg's current state on a GICv3, how to run it, what
it costs, why it is local, the semihosting and SMMU findings in brief, what the physical core found,
timer behavior, and BUGS. These files carry the evidence its prose budget has no room for.

- [`semihosting.md`](semihosting.md): the probe that showed HVF does not answer semihosting, the
  exception the guest takes, and how the host ends the run instead.
- [`five-assertions.md`](five-assertions.md): the five tests the physical core found, each failure
  and its fix, and the settle windows the leg makes weaker rather than flakier.

*Name: provisional, minted 2026-10-11 (UTC) by lane/fifth-ten-longest-notes, which split the parent
under §212 (a prose budget). The directory follows §212's siting default, `notes/X/` beside
`notes/X.md`. calef names directories.*

The stems below were minted the same day by the same lane. Each is named for its content, and calef
names files.

- `semihosting` Name: provisional, minted 2026-10-11 (UTC) by lane/fifth-ten-longest-notes.
- `five-assertions` Name: provisional, minted 2026-10-11 (UTC) by lane/fifth-ten-longest-notes,
  after the parent's own heading.
