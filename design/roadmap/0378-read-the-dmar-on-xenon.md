---
status: BUILT
raised: 2026-09-04
built: 2026-10-18
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 378. Read the DMAR, because a firmware checkbox is not an IOMMU

A boot on a VT-d machine prints, on each unit's `iommu` line, both claims about interrupt
remapping: the unit's own (`ECAP.IR`) and the platform's (the `INTR_REMAP` flag, bit 0 of the
Flags field in the DMA Remapping Reporting table (DMAR)). A boot log shows at a glance whether
firmware and hardware agree.

```
  iommu           : VT-d drhd at 0x00000000fed90000, root table default-deny, translating, interrupt remapping offered (unused), dmar INTR_REMAP set
```

## What is built

- **The table is parsed at boot** (milestone 161): `crates/machine_discovery`'s `parse_dmar` checks
  the fixed part, and `DmarUnits::parse` decodes every DRHD, RMRR and PCI scope. Milestone 594
  brings up every unit it names.
- **The unit's claim**, `ECAP.IR`, is read from each unit in `kernel/src/arch/x86_64/iommu.rs`
  (milestone 317). xenon's tour (`bench/xenon-2026-09-17/tour-display-225100.log`, line 119)
  printed `interrupt remapping offered (unused)` from its first unit, so
  `notes/confinement-claims.md`'s fifth claim is answered there as offered by the unit and unused by
  this kernel.
- **The platform's claim** is kept in the parsed table as
  `DmarUnits::platform_interrupt_remapping` (from `Dmar::claims_interrupt_remapping`) and printed
  beside the unit's through `InterruptRemappingClaims`, whose wording a host test holds
  (`the_dmar_intr_remap_flag_is_kept_and_printed_beside_the_units`, flag set and clear). Under
  QEMU both claims follow `intel-iommu`'s `intremap` property, which `NIFE_INTREMAP` sets
  (`helpers/qemu-runner-x86_64.sh`).
- **No gate reads the `iommu` line.** `script/boot-check` asserts the machine description reaches
  its end, not what each line says, so the wording is held by the host test alone.

## What is being proposed

Parse the DMAR (DMA Remapping Reporting table) at boot and print what it says: the remapping units,
their register base addresses, and the `INTR_REMAP` flag in the table's own flags field. Also print
each unit's extended capability register, whose `IR` bit is the one that says the hardware can remap
interrupts at all.

## Why

Two in-tree records have been waiting on a fact that turns out not to live where everyone assumed.

`notes/x86-uefi-boot.md` says the third of its open questions is xenon's alone: *"whether the
OptiPlex's firmware leaves interrupt remapping off. Nothing under QEMU can answer that, because the
answer is a setting in somebody else's firmware."* `notes/confinement-claims.md` carries the same
thing as a fifth claim: a confined component's MSI-X write is a memory write, so DMA remapping does
not cover it. VFIO refuses to hand a device to an untrusted userspace driver on a machine
without interrupt remapping for exactly that reason. `design/decisions/0086-el0-nvme-driver.md` names
it as the axis its options had not priced.

**The premise turned out to be half wrong, which is why this is a proposal rather than a bench
task.** Seventy photographs of the 7050's entire setup UI contain no interrupt-remapping control,
and the whole Virtualization Support menu is three pages: Virtualization, VT for Direct I/O,
Trusted Execution. So it is not a setting in somebody else's firmware in the sense that phrase
implies; there is no switch to photograph. What there is instead is a table the firmware
publishes, which any kernel can read, and which this kernel is already three lines from touching.

That relocates the work from the most expensive place in this project to the cheapest. AGENTS.md
notes that a null modem is where discovery costs most; a DMAR parser is host-testable logic in a
crate, exercised under QEMU, and the bench trip is reduced to reading one line off a boot tour.

## What it would settle

- `notes/confinement-claims.md`'s fifth claim stops being latent-with-no-way-to-check and
  becomes a measured yes or no on the one machine that has the hardware.
- DECISIONS §86's option set gets the number it was priced without: whether a userspace NVMe
  driver on xenon could be given interrupts at all, or would have to poll the way the current one
  does.
- `design/roadmap/0195-uefi-boot-finish.md`'s remaining open question closes, or is shown to
  need something else.

## What it does not do

It does not enable interrupt remapping, program a remapping unit, or change any driver. It reads
and reports. Turning the unit on is a separate and much larger piece of work, and it should not be
started until somebody knows whether the hardware here supports it.

## The honest cost

The DMAR's structure is a short table with a variable-length list of remapping-structure entries,
and this tree has parsed several ACPI tables already (`notes/x86-port/acpi-and-pci.md` records the XSDT, MADT
and MCFG work). The expensive part is not the parse, it is deciding what the kernel should *do*
when it finds a DMAR it did not expect, and the answer for this proposal is nothing: print it.

## Follow-on

- **Done.** The DMAR's `INTR_REMAP` flag is kept in `DmarUnits` and printed beside each unit's
  `ECAP.IR` on the `iommu` line, by this milestone's own pull request.
- **Done.** Carrying more than one DRHD, by milestone 594 (every VT-d unit translates its own
  devices), which xenon verified at its own bench boot.
- **Recorded.** This block reads and reports and never enables interrupt remapping, programs a
  remapping unit or changes a driver. `kernel/src/arch/x86_64/iommu.rs`'s header says `ECAP.IR` is
  read to report and never to act on, and `crates/machine_discovery/src/acpi.rs`'s `BUGS` says the
  same of `INTR_REMAP`. Turning a unit on is a much larger piece of work.
- **Recorded.** xenon's own pair is unread until a xenon boot of a kernel carrying this change,
  which prints both on its `iommu` lines; `notes/xenon-firmware.md` says so beside the unit's half.

## Index row

Two in-tree records were waiting on a fact nobody could check: whether xenon's firmware leaves
interrupt remapping off. There is no switch to photograph: seventy photographs of the OptiPlex
7050's setup UI contain no interrupt-remapping control. What there is instead is a table the
firmware publishes and any kernel can read, which moves the work from the bench to a parser. The
DMAR is parsed at boot (milestone 161), every unit is brought up (milestone 594), each unit's
`ECAP.IR` is read (milestone 317), and the `iommu` line prints it beside the DMAR's own
`INTR_REMAP` flag, so a boot shows whether the platform's claim and the hardware's agree.
