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

A VT-d machine makes two claims about interrupt remapping. Each unit makes one in `ECAP.IR`. The
platform makes the other in the `INTR_REMAP` flag, bit 0 of the Flags field in the DMA Remapping
Reporting table (DMAR). The boot prints both on each unit's `iommu` line, so a boot log shows
whether firmware and hardware agree:

```
  iommu           : VT-d drhd at 0x00000000fed90000, root table default-deny, translating, interrupt remapping offered (unused), dmar INTR_REMAP set
```

## What is built

- Milestone 161 (the x86_64 kernel port) parses the table at boot. `crates/machine_discovery`'s
  `parse_dmar` checks the fixed part. `DmarUnits::parse` decodes every DRHD, RMRR and PCI scope,
  and milestone 594 brings up every unit it names.
- Milestone 317 (the interrupt-remapping flags) reads each unit's `ECAP.IR` in
  `kernel/src/arch/x86_64/iommu.rs`. xenon printed `interrupt remapping offered (unused)` from its
  first unit (`bench/xenon-2026-09-17/tour-display-225100.log`, line 119). That answers
  `notes/confinement-claims.md`'s fifth claim there: offered by the unit, unused by this kernel.
- The platform's claim is kept in the parsed table as `DmarUnits::platform_interrupt_remapping`,
  read through `Dmar::claims_interrupt_remapping`. `InterruptRemappingClaims` prints it beside the
  unit's. The host test `the_dmar_intr_remap_flag_is_kept_and_printed_beside_the_units` holds the
  value and the wording, with the flag set and clear.
- Under QEMU both claims follow `intel-iommu`'s `intremap` property. `NIFE_INTREMAP` sets it
  (`helpers/qemu-runner-x86_64.sh`). The default prints `offered` and `set`; `off` prints `absent`
  and `clear`.
- No gate reads the `iommu` line. `script/boot-check` asserts that the machine description reaches
  its end, not what each line says. The host test alone holds the wording.

## Why

Two records waited on a fact that was not where everyone assumed. `notes/x86-uefi-boot.md` asked
whether xenon's firmware leaves interrupt remapping off. `notes/confinement-claims.md` carries the
same question as a fifth claim. A confined component's MSI-X write is a memory write, so DMA
remapping does not cover it. VFIO refuses a device to an untrusted userspace driver on a machine
without interrupt remapping for that reason. `design/decisions/0086-el0-nvme-driver.md` names it
as the axis its options had not priced.

There is no switch to photograph. Seventy photographs of the OptiPlex 7050's setup UI contain no
interrupt-remapping control. Its Virtualization Support menu has three pages: Virtualization, VT
for Direct I/O, and Trusted Execution. What there is instead is a table the firmware publishes,
which any kernel can read. That moves the work from the bench to a parser, which is host-testable
and runs under QEMU.

## What it does not do

It reads and reports. It does not enable interrupt remapping, program a remapping unit, or change
any driver. Turning a unit on is a much larger piece of work.

## Follow-on

- **Done.** The DMAR's `INTR_REMAP` flag is kept in `DmarUnits` and printed beside each unit's
  `ECAP.IR` on the `iommu` line, by this milestone's own pull request.
- **Done.** Carrying more than one DRHD, by milestone 594 (every VT-d unit translates its own
  devices), which xenon verified at its own bench boot.
- **Recorded.** Nothing here acts on either claim. `kernel/src/arch/x86_64/iommu.rs`'s header says
  `ECAP.IR` is read to report and never to act on. `crates/machine_discovery/src/acpi.rs`'s `BUGS`
  says the same of `INTR_REMAP`.
- **Recorded.** xenon's own pair is unread until a xenon boot of a kernel carrying this change.
  `notes/xenon-firmware.md` says so beside the unit's half.

## Index row

Two in-tree records waited on whether xenon's firmware leaves interrupt remapping off. Its setup UI
has no such switch. The answer is in a table the firmware publishes, so the work moved from the
bench to a parser. The DMAR is parsed at boot (milestone 161), every unit is brought up (milestone
594), and the `iommu` line prints each unit's `ECAP.IR` (milestone 317) beside the DMAR's own
`INTR_REMAP` flag. A boot shows whether the platform's claim and the hardware's agree.
