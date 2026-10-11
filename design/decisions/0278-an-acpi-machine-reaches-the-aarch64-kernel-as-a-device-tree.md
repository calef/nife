---
status: DECIDED
raised: 2026-09-23
decided: 2026-10-11
ratified_by: calef
---

# 278. An ACPI machine reaches the aarch64 kernel as a device tree the loader writes

*Section number provisional until the merge queue lands it; 274 was the highest on `main` when this
was written, and 271, 272 and 275 to 277 are claimed on open branches. Written by the maintainer on
2026-10-11 (UTC) on branch `maintainer/acpi-via-device-tree`.*

This decision was recorded after the fact. The question was raised on 2026-09-23 (UTC) as item 5
of milestone 578 (an ACPI discovery path for aarch64), which said a lane should not pick. The lane
that built the ACPI path chose option 1 below anyway, in commit `30ed6d4d2`, and PR #1153 merged
on 2026-09-24 (UTC). No section recorded the choice. calef ratified it on 2026-10-11 (UTC),
answering *"Yes"* when the maintainer put option 1 to him for ratification.

## The question

How does an ACPI machine's description reach the aarch64 kernel? The boot contract the kernel
implements is Linux's arm64 one: `x0` holds the physical address of a device tree, and `x1` to `x3`
are zero. There is no second register and no slot for a memory map. Every aarch64 server that
follows Arm's Server Base Boot Requirements presents ACPI and may present no device tree. That is
every aarch64 cloud machine. Milestone 578's
[fork](../roadmap/0578-acpi-on-aarch64.md#the-loader-half-and-exactly-what-aarch64-would-mirror)
states the three answers in full.

## The decision

Option 1. The UEFI loader reads the firmware's ACPI tables and memory map while boot services are
still alive. It writes the device tree the kernel would have been handed
(`uefi_loader/src/device_tree_from_acpi.rs`, `build`), and the kernel sees only a device tree.
When the firmware offers its own tree as well, that tree still wins, because it is the richer
description. The written tree marks itself with the root `compatible` string
`nife,machine-from-acpi`, so a reader debugging a boot can tell which kind it was handed.

## What was refused

- Option 2, a second front door in the kernel. `x0` would point at ACPI's RSDP instead, told apart
  by its `RSD PTR ` signature. Refused. The kernel's memory, CPU, console and interrupt setup would
  each grow a second discovery path, and the memory map would still need a home outside ACPI.
- Option 3, a nife-specific handover structure, as x86_64 has with `hvm_start_info`. Not put to
  calef separately; ratifying option 1 leaves it unneeded. It is the most surface of the three, and
  it is a format two programs agree on.

## Why

These are the three reasons as they were put to calef.

1. The loader and kernel contract does not change. `x0` is still a device tree, the contract
   U-Boot's `booti` also meets. The kernel stays agnostic about firmware, and the boot path for a board
   with a device tree is untouched.
2. One description parser in the aarch64 kernel, not two. Every kernel consumer already reads the
   device tree through `machine_discovery`'s decoders. Option 2 would double that trusted surface,
   and the proof work that goes with it.
3. It is built and it boots. `NIFE_ACPI=on helpers/qemu-stick.sh aarch64 target/stick` reaches the
   same shell prompt `acpi=off` does, measured on 2026-09-23 (UTC)
   ([notes/boot-stick/device-tree-handover.md](../../notes/boot-stick/device-tree-handover.md#edk2-on-aarch64-hid-the-device-tree-when-it-presented-acpi)).

Two caveats on reason 2. The ACPI table decoders in `machine_discovery::acpi` exist either way,
since the x86_64 kernel reads ACPI itself through `read_acpi` in
`kernel/src/arch/x86_64/machine.rs`. What option 1 saves is the second discovery path in the
aarch64 kernel, not the decoders. And the translation moves trust into the loader rather than
removing it. A wrong GIC address written by `build` is a wrong GIC address the kernel believes.
Its host tests decode the written tree with the kernel's own readers, which is what keeps that
honest.

Reason 3 is partly an effort argument. Would option 1 still win if both cost the same? Reasons 1
and 2 say yes: an unchanged contract and one parser are properties of the design, not of the work
already done.

## The cost

A device tree cannot say everything ACPI says. Information with no device tree counterpart must be
translated into a node the kernel reads, or carried across the handover separately, when a
milestone needs it. The `BUGS` section of `uefi_loader/src/device_tree_from_acpi.rs` is the full
list. The gaps that matter now:

- **PCI.** A host bridge's BAR windows and interrupt routing live in AML (`_CRS`, `_PRT`), which
  needs an interpreter. No PCI node is written, so a bus that is there is invisible.
- **Power management.** Much of it lives in AML too, and the translation carries only the FADT's
  PSCI flags.
- **MSI.** MADT entries 0x0D (GIC MSI frame) and 0x0F (GIC ITS) still decode to
  `MadtEntry::Other` in `machine_discovery::acpi`, and no ITS node is written.

## What it means for milestones 578 and 317

Milestone 578 (an ACPI discovery path for aarch64): item 5 is answered by this section, and item 6,
the kernel's second front door, is not built. What remains is translation work in the loader: the
MSI frame and ITS entries, a gate that boots `acpi=on`, and the Oracle Always Free boot.

Milestone 317 (the interrupt-remapping flags, and where MSI confinement actually lives) found that
aarch64's MSI confinement lives in the GICv3 ITS. Whatever milestone first drives the ITS reads it
from the device tree, as `machine_discovery::gic` reads the rest of the GIC today. On an ACPI
machine that means the loader must first write the ITS node from MADT entry 0x0F. The ITS
work therefore owes a translation as well as a driver, and should be scoped with both.

## BUGS

- The decision was taken by a lane and ratified seventeen days later. Nothing in the tree flagged
  the gap; the maintainer found it on 2026-10-11 (UTC) by grep, reviewing milestone 578 in PR #1946.
- No gate boots `acpi=on`. The proof that this decision works is a transcript run by hand.
