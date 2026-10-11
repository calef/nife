# The native ABI: the appendices to notes/abi.md

[`notes/abi.md`](../abi.md) carries the syscall convention, the object surface, the entry contract,
how a program meets its capabilities, the fault endpoint and the one ambient read. These files carry
what its prose budget has no room for.

- [`cycle-counter-grant.md`](cycle-counter-grant.md): why the cycle counter is granted rather than
  ambient (milestone 229 (build the cycle-counter grant DECISIONS 139 decided)), per architecture, and what the build gate does not cover.

*Name: provisional, minted 2026-10-11 (UTC) by lane/next-ten-longest-notes, which split the parent
under §212 (a prose budget). The directory follows §212's siting default, `notes/X/` beside
`notes/X.md`. calef names directories.*

- `cycle-counter-grant` Name: provisional, minted 2026-10-11 (UTC) by lane/next-ten-longest-notes,
  named for the grant milestone 229 built.
