# Milestone 317 appendix: riscv64, offered and declined in one word

An appendix to [milestone 317 (the interrupt-remapping flags, and where MSI confinement actually
lives)](../0317-interrupt-remapping-flags.md), moved out of that page on 2026-10-12 (UTC) under
[§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md). The main page is
complete without it. This is the riscv64 measurement in full: where the mechanism sits, the boot
reading, why no flag exists, the inversion against x86_64, and what is a reading rather than a
measurement.

## Where the mechanism sits

§86 did not survey this architecture, and it is the one where the mechanism is closest to hand.

The RISC-V IOMMU puts MSI confinement *inside the device context this driver already writes*. There
is no separate feature to enable and no separate device to drive. A context in the extended format
carries `msiptp`, `msi_addr_mask` and `msi_addr_pattern` alongside the `iosatp` that
`arch::riscv64::iommu::attach` fills in on every attach.

Measured from a boot, not from the source comment (`CAPS` printed from inside the guest, QEMU
11.1.1's `virt`, `riscv-iommu-pci` as the runner already attaches it):

```text
CAPS = 0x78c2cf4f10    MSI_FLAT (bit 22) set, Sv39 (bit 9) set, version 0x10
```

So the comment in that file was right, and now something other than a comment says so:

```text
iommu           : riscv-iommu at 0x0000000040000000, device directory default-deny, translating, MSI page table offered (mode off)
```

The extended 64-byte context is the live path, and `attach` writes those four words zero,
which is `msiptp.MODE = Off`. No MSI page table is allocated. Nothing programs an entry in one.
The capability is offered and declined, the same posture `x86_64` is in, reached by a completely
different route.

## Does riscv64 need a flag?

No, and that is a complete answer. `-device riscv-iommu-pci` as the runner already spells it
reports `MSI_FLAT`, so the capability is present with no option to add. Unlike `intel-iommu`, there
is no property to turn it off. Both sides of the `x86_64` comparison exist there because QEMU offers
`intremap=off`; here only one side exists, and no flag this lane could add would create the other.
`helpers/qemu-runner-riscv64.sh` is unchanged by this milestone.

## The inversion, which is the sharpest thing here

`arch::riscv64::iommu`'s own comment says the base 32-byte format is kept "because real silicon
without MSI support will report otherwise." That branch has run zero times, and cannot be made to
run. Milestone 143's block states the hardware fact plainly: no RISC-V SoC on the market ships
the ratified IOMMU specification, and radon (the `VisionFive` 2, JH7110) predates it and has no
IOMMU at all.

So the three architectures are not merely in different places, they are in different places *along
two axes*:

| | emulated | on silicon this project owns |
|---|---|---|
| `x86_64` | capability offered, unused | xenon (`OptiPlex` 7050) has a real VT-d unit, unmeasured |
| aarch64 | capability absent (GICv2) | argon (Jetson TX1) predates the question here too |
| riscv64 | capability offered, unused | no such silicon exists (milestone 143) |

riscv64 is confined in the case we can test and untestable in the case we cannot. That is the
inverse of `x86_64`, where the emulated answer is easy and a second witness is sitting on a shelf.
Any claim built on MSI confinement inherits that shape, and DECISIONS §19's parity gate is what
will ask about it.

## What is a reading rather than a measurement

`msiptp.MODE = Off` means, per the RISC-V IOMMU specification, that MSI address translation is not
performed and the transaction is translated as an ordinary memory access. If that is right, an
MSI-shaped write from an attached device is still bounded by its `iosatp` domain rather than
escaping it. That would make riscv64's position genuinely different from `x86_64`'s, where VT-d
intercepts the MSI address range and DMA remapping does not cover it.

Nobody here has tested that, and this tree carried a fabricated block quote for twelve days, so
it is marked rather than asserted. It is recorded in the driver's own `BUGS` section beside the
code, which is where a reader meets it.
