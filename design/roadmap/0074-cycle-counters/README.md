# Cycle counters: the appendices to milestone 74

[Milestone 74 (cycle counters: SBI PMU on RISC-V, `PMCCNTR_EL0` on aarch64)](../0074-cycle-counters.md)
carries where the milestone stands, why cycles matter, the parity answer for each half, the radon
measurement, the `BUGS`, argon's first-evening procedure and the follow-on. This directory carries
the build detail and the corrections its prose budget has no room for.

- [`corrections.md`](corrections.md): the two misquotations of `notes/benchmarks.md`, the old parity
  heading, and the wrong x86_64 mechanism, each with why it was wrong.
- [`riscv64-half.md`](riscv64-half.md): what the riscv64 lane built and left out, the emulator
  finding, and the working behind radon's measurement with the rows it converts.
- [`aarch64-half.md`](aarch64-half.md): what the aarch64 lane built, the EL2 defect it fixed, and
  which path each QEMU configuration took.

*Name: provisional, minted 2026-10-12 (UTC) by maintainer/prose-budget-backlog, which split the
parent under §212 (a prose budget). The directory follows §212's siting default, `design/roadmap/X/`
beside `design/roadmap/X.md`. calef names directories.*

- `corrections` Name: provisional, minted 2026-10-12 (UTC) by maintainer/prose-budget-backlog, for
  the parent's retracted quotations, heading and mechanism. calef names files.
- `riscv64-half` Name: provisional, minted 2026-10-12 (UTC) by maintainer/prose-budget-backlog, after
  the parent's "What the riscv64 half built, and what it deliberately did not". calef names files.
- `aarch64-half` Name: provisional, minted 2026-10-12 (UTC) by maintainer/prose-budget-backlog, after
  the parent's "What the aarch64 half built (2026-09-19)". calef names files.
