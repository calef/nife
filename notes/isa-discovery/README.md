# ISA discovery: the appendices to notes/isa-discovery.md

[`notes/isa-discovery.md`](../isa-discovery.md) carries what the boot reads, the tiers, which call
sites vary, what the machine corrected, the board shapes the parser survives, the CPU-list and PSCI
sequel, and the `BUGS`. This directory carries the detail its prose budget has no room for.

- [`psci-conduit.md`](psci-conduit.md): why milestone 100 (read the machine's PSCI)'s two hardcoded
  facts failed in different shapes, and the QEMU option that made the `smc` conduit parseable but
  not callable.
- [`lookup-tables.md`](lookup-tables.md): why the crate's four lookup tables are arrays rather than
  `match`es, why SBI ids are derived from their tags, and the one line no test covers.
- [`the-crate-name.md`](the-crate-name.md): how `isa` became `machine_discovery`, and the prior art
  behind the name.
- [`counter-rate-in-userspace.md`](counter-rate-in-userspace.md): the closed limitation on
  `user_mode_runtime::cntfrq`, and why the reasoning that kept it open was wrong.

*Name: provisional, minted 2026-10-11 (UTC) by lane/final-longest-notes, which split the parent
under §212 (a prose budget). The directory follows §212's siting default, `notes/X/` beside
`notes/X.md`. calef names directories.*

- `psci-conduit` Name: provisional, minted 2026-10-11 (UTC) by lane/final-longest-notes, for the
  parent's PSCI failure shapes and `smc` fixture. calef names files.
- `lookup-tables` Name: provisional, minted 2026-10-11 (UTC) by lane/final-longest-notes, after the
  parent's "Four lookup tables, and why none of them is a `match`". calef names files.
- `the-crate-name` Name: provisional, minted 2026-10-11 (UTC) by lane/final-longest-notes, after the
  parent's "Naming". calef names files.
- `counter-rate-in-userspace` Name: provisional, minted 2026-10-11 (UTC) by
  lane/final-longest-notes, for the closed `cntfrq` entry in the parent's `BUGS`. calef names files.
