# The interrupt-remapping flags: the appendices to milestone 317

[Milestone 317 (the interrupt-remapping flags, and where MSI confinement actually
lives)](../0317-interrupt-remapping-flags.md) carries what the milestone is and is not, and a
summary of each architecture's position. It also carries the aarch64 measurement, what milestone 227
(a GICv3 driver) would owe, EXAMPLES, its `BUGS` and its Follow-on. This directory carries the x86_64 and riscv64 measurements in full,
which its prose budget has no room for.

- [`x86-64.md`](x86-64.md): the `ECAP` readings, the `NIFE_INTREMAP` knob and its default, the
  suite on both machines, and the `kernel-irqchip` check.
- [`riscv64.md`](riscv64.md): the `CAPS` reading, why riscv64 needs no flag, the inversion against
  x86_64, and what is a reading of the specification rather than a measurement.

*Name: provisional. The directory follows §212 (a prose budget)'s siting default,
`design/roadmap/X/` beside `design/roadmap/X.md`. calef names directories.*

- `x86-64` Name: provisional. After the parent's "x86_64: the premise was false, and that is the
  finding". calef names files.
- `riscv64` Name: provisional. After the parent's "riscv64: offered, declined in one word, and
  untestable on silicon we own". calef names files.
