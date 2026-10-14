//! **What machine is this?** One record per architecture, populated once at boot, printed at boot.
//!
//! Milestone 60 (design/roadmap/0060-isa-discovery.md). An emulator is whatever we ask for; a board
//! is not, so the kernel reads what the machine is rather than what the target triple implies.
//!
//! # The tiers
//!
//! 1. **The firmware's claim.** A device tree property. Cheap, structured, and *hearsay*.
//! 2. **A targeted measurement.** Write a value, read back what stuck: what the silicon does rather
//!    than what somebody wrote about it.
//! 3. **Trap-and-detect.** Execute the instruction and catch the fault. Last resort; not used.
//!
//! **aarch64 has a tier 0: the CPU describes itself.** `MIDR_EL1` and the `ID_AA64*` registers are
//! architected, mandatory, and read from the part in front of you. RISC-V removed that tier: `misa`
//! is coarse, may read as zero, and says nothing about extensions ratified after 2015, so the
//! answer is a device-tree property. That is why [`riscv64::Isa`] is a parser and [`aarch64::Isa`]
//! a decoder, and why the RISC-V side keeps its tier-2 probe: **the tree is a claim and the probe
//! is a measurement, and when they disagree the machine wins.**
//!

//! # The trap this crate is written to avoid
//!
//! `if isa.has_x()` sprouting across the kernel turns a fact into a hundred branches. So the record
//! is `Copy`, has public fields, and offers exactly two verbs, [`riscv64::Isa::missing_requirements`]
//! and [`aarch64::Isa::missing_requirements`], which answer "can this machine run us at all".
//! Everything else is for the boot print.
//!
//! [`cpu_list`] (milestone 100 (read the machine's PSCI and its CPU list)) sits beside the arch
//! records because every architecture answers "which cores" the same way: `/cpus` holds a `cpu@`
//! node per core with its hardware id in `reg`.
//! `aarch64::Psci` is arch-specific and lives in the aarch64 module. **There is no trait and no
//! per-chip abstraction** until a second real board says what the abstraction is.
//!
//! # BUGS
//!
//! - **Discovery does not make the kernel portable, it makes it honest.** Today the kernel does one
//!   thing with a missing requirement: it says so and stops.
//! - **The device tree can lie.** Tier 2 exists for that, and on RISC-V it covers exactly one fact
//!   (the ASID width).
//!
//! Name: ratified 2026-08-23 (calef, a kernel-dependency crate naming review). Renamed from `isa`,
//! which collided with the ISA bus and described only part of the scope. Linux ARM's `machine_desc`
//! and FreeBSD's `hw.machine` use "machine" for the same device-tree-plus-probe scope.

#![no_std]
// milestone 68's ratchet is workspace-wide (§107); this crate opts out until its 54-item
// worklist (notes/doc-coverage.md) is burned down.
#![allow(missing_docs)]

// ACPI (milestone 161), and it sits here rather than in the x86_64 module for the reason
// `cpu_list` does: ACPI is not an x86 standard. Every aarch64 server that is not a device-tree
// board describes itself with the same tables, and milestone 20's own text expects the machine
// after the VisionFive 2 to be a UEFI/ACPI one. x86 is its first consumer, not its owner.
pub mod aarch64;
pub mod acpi;
pub mod cpu_list;
// Where the screen is (milestone 243), and it belongs to neither architecture for the same reason
// `cpu_list` does not: the loader asks the firmware where the linear framebuffer is and tells the
// kernel in one command-line token, and milestone 157's U-Boot handoff has the identical sentence
// to say on aarch64. x86_64 is its first consumer, not its owner.
pub mod framebuffer;
// Which Arm interrupt controller (milestone 227): aarch64's question, but a device-tree binding
// read plus a pure cross-check of raw ID-register words, so it is host logic like `plic` beside it.
pub mod gic;
pub mod interrupt_id;
pub mod plic;
pub mod riscv64;
// The third architecture's boot handoff (milestone 161). Not an ISA record like the two beside it:
// x86 answers "what CPU is this" from CPUID and needs no parser for that, but it has no device
// tree, so where RAM is and where the ACPI tables are arrive in the boot protocol's own structure.
// Same reason the RISC-V half is a parser: what the firmware says is a format somebody has to read,
// and a format read only inside a booting kernel is read by nothing that runs in milliseconds.
pub mod x86_64;
