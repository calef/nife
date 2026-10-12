# Milestone 161 appendix: the status history, the scope it carried and the machine it waited for

An appendix to [milestone 161 (the x86_64 kernel port)](../0161-x86-64-kernel-port.md), moved out
of that page on 2026-10-12 (UTC) under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md).
The main page is complete without it. Long sentences were split to meet
[§213 (writing standards)](../../decisions/0213-writing-standards.md); no fact changed.

## The status history

The block kept this because the rest of it cites it.

PARTIAL. Minted 2026-08-23, splitting real work out of the stale text of
[milestone 20 (a portable HAL, proven on a second architecture)](../0020-portable-hal.md). Early
boot was built and gated the same day.

By 2026-08-24 the kernel booted on QEMU's `q35` and reached Rust in the high half of a 4-level
address space. It printed over a 16550, installed a GDT/TSS and an IDT, and caught a breakpoint and
stepped over it. It took a calibrated timer interrupt and brought up the frame allocator. It
replaced the boot map with fine-grained W^X page tables of its own. It routed a real device line
through the IO APIC's redirection table, brought up the scheduler and preempted, and ran a kernel
thread.

It built two processes out of untyped memory and ran them at ring 3. One invoked a capability it
was granted and exited. One was refused by the page tables, and its death was delivered to its
supervisor. Both regions went back, leaving the frame count where it found it. Then it ran a
userspace built from real ELF programs, read out of an initrd it found in PVH's module list, before
it halted.

`script/test --arch x86_64` ran 170 of the kernel's own tests and skipped 67, every skip naming its
missing fixture. Those numbers are stale: the `x86-gates` lane counted 226 and 72 on 2026-09-21,
and the suite grows, so they were not re-derived. `notes/x86-port.md` was the working record.

The scope note (milestone 20's "enough of each ISA to boot, confine a ring-3/U process, and run the
test suite") was met on 2026-08-24, on QEMU. What kept the milestone PARTIAL rather than BUILT was
item 5, SMP. The mechanism that starts a second core was built. Nothing downstream of "a second core
exists" had been shown safe, and two unresolved bugs were why (`arch::x86_64::ap_boot`'s own
`BUGS`). Item 6, VT-d, was BUILT on 2026-08-25.

Item 0's wide half was split into its own milestone,
[milestone 176 (the x86_64 discovery seam's wide half)](../0176-x86-64-discovery-seam-wide-half.md), PARTIAL,
minted 2026-08-25. It was split the same way item 4's `fs_server`/`aes` gap had gone to
[milestone 164 (x86_64 userspace can't build `aes`)](../0164-x86-64-fs-server-aes.md). Two of the four
device windows this item had named as tree-only were already built: the interrupt controller by item
2, and PCI by the ACPI/MCFG work of [milestone 165 (x86_64 PCI enumeration)](../0165-x86-64-pci-acpi-mcfg.md).
So 176 tracks only what is actually left, the CMOS RTC, without holding this milestone's status
hostage to it. Item 4's own hand-off, no user program compiled for `x86_64-unknown-none`, was closed
the same day (step 13).

[DECISIONS §19 (architectural parity is a tenet)](../../decisions/0019-architectural-parity.md)
declared the target set (aarch64, riscv64, x86_64). It recorded honestly that *"x86_64 is a
declared target that does not exist yet."* Milestone 20's own "Deliverable, in two parts" already
named "bring up a second ISA, then a third: RISC-V first, x86_64 second." But 20 is marked BUILT for
the HAL split plus RISC-V alone; its title says "proven on a second architecture," singular. Checked
directly at minting, `kernel/src/arch/` held only `aarch64/` and `riscv64/`, and no `x86_64/`
existed anywhere in the tree. The x86_64 half of 20's deliverable was never tracked as open work.
This milestone is that tracking.

While it was open, nothing gated it. §19 had already settled that x86_64 is a target.
[Milestone 87 (the x86_64 bare-metal machine)](../0087-x86-machine.md)'s own text is explicit
that this can start now, under QEMU TCG, the way RISC-V's port did. It is not gated on the physical
machine.

## What it carried forward from milestone 20's original scope

Why x86_64 second is unchanged from 20's own reasoning. It is the hard proof that the HAL
abstraction is real rather than an accident of two similar RISC ISAs. RISC-V is structurally close
to aarch64 (device tree, weak memory, a similar MMU shape). x86_64 is a genuinely different model:
CISC, strong TSO memory ordering, GDT/TSS, ACPI and PCI instead of a device tree, port I/O, the
`syscall` + `swapgs` trampoline, and INIT-SIPI-SIPI SMP bring-up instead of PSCI/SBI. If the
`arch/` split survives x86, it is real. Also the reach: x86_64 is what most machines are.

§19's own list of what x86_64 will stress:

- a different boot world (UEFI/ACPI, not device tree; no OpenSBI/PSCI analog);
- the APIC instead of GIC/PLIC;
- a third page-table format behind the `paging` seam;
- TSO memory ordering.

TSO is where rule #4's weak-first discipline pays out in the direction it was bet on. Code proven
correct on a weak machine (ARM, RISC-V) is correct on TSO, and nothing about x86-first development
could have said the reverse. The PCIe transport of
[§18 (the PCIe transport)](../../decisions/0018-pcie-transport.md) is already x86's native bus. The
ECAM bridge both `virt` boards use is the same `pci-host-ecam-generic` shape x86 presents through
ACPI.

## Sequencing against the physical machine

The port started under QEMU and did not wait for the real machine. Milestone 87 tracks the physical
OptiPlex 7050's bring-up: a real 16550 COM port, VT-d, an `igb`/`e1000e`-family NIC QEMU can stand
in for, four real cores, and remote power cycling. It is the eventual parity-proof hardware. The
VisionFive 2 played the same role for RISC-V in
[milestone 16 (real hardware and IOMMU-backed driver isolation)](../0016-real-hardware-iommu.md).
QEMU's `q35` emulates the same 16550 UART the real hardware carries. So the boot and serial driver
spans both from the start, the NS16550/PL011 pattern both existing ISAs already follow.

As of 2026-08-23, the Dell C4PDJ serial module and the dev-side RS-232 chain had arrived and were
installed. Milestone 87's remaining blocker was recorded on 2026-08-18 as *"the machine is here and
the serial module is not"*. That blocker closed on the hardware side. What remained for 87 was the
bring-up itself: boot code, the UART driver, and printing a byte over serial on real silicon. That
is downstream of this milestone's boot and console work reaching a point worth trying on the box.
Milestone 87 has the machine's own status.
