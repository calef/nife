---
status: NOT-STARTED
raised: 2026-10-12
milestone_dependencies: 525, 554, 568, 809, 909
decision_dependencies: 297, 208, 159, 244
machine_requirements: none
specific_machine: none
needs_person: no
---
# 908. The kernel is a package: `jig` installs it, the next boot runs it, and a failed boot rolls back

*(Minted 2026-10-12 (UTC) by lane `maintainer/foreign-programs-via-jig`, under §297 (software
reaches a nife system as a package `jig` installs from basalt's index, the kernel included). The
number is provisional until the merge queue lands it; the title and slug are drafts.)*

calef, 2026-10-12 (UTC): *"We want to be able to update the kernel through packages."*

Asked the same day whether aarch64 under QEMU boots `uefi_loader` through edk2 (a) or through
U-Boot's `bootefi` (b), calef answered *"a"* (2026-10-12 UTC). aarch64 takes the UEFI path x86_64
takes under OVMF.

## What already plans it

- Milestone 858 (lab machines update themselves through packages, and only a new kernel reboots
  them) plans a kernel package. Its fork 5 keeps the kernel and the base as separate packages. Its
  correction puts the kernel in a boot slot that updates with a reboot and rolls back as built. It
  is the lab's, on silicon with a person at the bench. This milestone is the same path under QEMU.
- Milestone 525 (a bad upgrade cannot brick the machine: two boot slots, tries and priority) built
  slots, tries and the chooser in `uefi_loader`, on x86_64 only. Slot state lives in GPT attribute
  bits.
- Milestone 554 (a good upgrade sticks: what marks a trial boot successful) built the
  confirmation.
- §208 (installing a package is granting it, and the installed programs are versioned) gives the
  generations. §159 (lab machines upgrade like user machines, and only a new kernel needs a reboot)
  says only the kernel reboots. §244 (the installed disk has four partitions, and a boot slot is 64
  MiB) gives the slots their space.

## What is missing

1. A kernel package. basalt builds one per architecture (milestone 909 (basalt composes the
   bootable image from its packages)).
2. Installing it writes the inactive slot. `jig install` of a kernel package hands the bytes to the
   progenitor, which writes the slot and marks it a trial with tries, as 525's chooser reads them.
3. The loader finds the active kernel. On x86_64 that is 525's chooser. On aarch64 and riscv64 there
   is no chooser, and milestone 568 (the boot file has nowhere to go on a device-tree machine)
   blocks one.
4. Each architecture's boot path. x86_64 boots `uefi_loader` from OVMF. aarch64 under QEMU boots
   it from edk2, the same UEFI path, so 525's slot chooser is reused rather than written twice.
   riscv64 boots OpenSBI, then U-Boot, which runs `uefi_loader` with `bootefi`, as 858's fork 7
   ruled for radon and argon.
5. Rollback. A new kernel that never confirms (554) spends its tries, and the chooser starts the
   previous slot.

## Done when

Under QEMU on aarch64, riscv64 and x86_64, from a booted nife at the prompt:

- `jig install` of a new kernel package from basalt's index succeeds, and the machine reboots into
  the new kernel, which confirms and stays.
- `jig install` of a kernel package that cannot boot is followed by a reboot. The tries run out and
  the machine comes back on the previous kernel, at the prompt, saying the update failed.

Reuse: 525's slots and chooser, 554's confirmation, `crates/boot_slot`, and 858's design for the
kernel package, moved to QEMU rather than redesigned.

## BUGS

- argon's own path, U-Boot's `bootefi` (858's fork 7), is not what this milestone tests on
  aarch64. The loader is UEFI code either way, so the gap is the firmware, not the chooser.

## Index row

A kernel update is a package install and a reboot, with rollback on a failed boot, on all three
architectures under QEMU before any lab machine relies on it.
