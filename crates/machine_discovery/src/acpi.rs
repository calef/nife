//! **ACPI: what x86 has instead of a device tree.**
//!
//! Milestone 161. This module sits beside the arch records rather than inside the `x86_64` one, for
//! the reason [`cpu_list`](crate::cpu_list) does: ACPI is not an x86 standard. Every aarch64 server
//! that is not a device-tree board describes itself with exactly these tables, and milestone 20's
//! own roadmap text says the machine after the VisionFive 2 "should probably be a UEFI/ACPI machine
//! rather than another Device Tree board". x86 is simply its first consumer here.
//!
//! # The shape, and how it differs from a device tree
//!
//! A device tree is one blob with one root and a tree of nodes, and `crates/device_tree_blob` walks
//! it. ACPI is a **linked structure of independent tables**, each with its own signature and
//! checksum, reached from a root pointer that is not itself a table:
//!
//! ```text
//!   RSDP  ("RSD PTR ")            found by scanning low memory, or handed over by the loader
//!    |
//!    +-- RSDT (32-bit pointers)   ACPI 1.0
//!    +-- XSDT (64-bit pointers)   ACPI 2.0+, and what to prefer when both exist
//!         |
//!         +-- APIC ("MADT")       the local APICs, the IO APICs, and the interrupt rewiring
//!         +-- MCFG                where the PCIe ECAM window is
//!         +-- DMAR                where the IOMMU is, on an Intel machine (VT-d)
//!         +-- IVRS                where the IOMMU is, on an AMD machine (AMD-Vi): [`ivrs`]
//!         +-- FACP, HPET, WAET, ...
//! ```
//!
//! **Every table is checksummed and this module checks**, which a device tree has no equivalent of.
//! That is not politeness: the RSDP is found by *scanning memory for a string*, so without the
//! checksum any sixteen bytes that happen to spell `RSD PTR ` would be believed.
//!
//! # What is here and what is deliberately not
//!
//! Here, one private module per table, re-exported by name: `sdt` (header, checksum, errors),
//! `root` (RSDP, root walk), `madt`, `mcfg`, `dmar`, `gtdt`, `spcr`, `fadt`, and [`ivrs`]. Not here,
//! and it is the big one: **AML**, the DSDT's bytecode for everything ACPI has no fixed table for,
//! including PCI interrupt routing (`_PRT`) and every power-management method. AML needs an
//! interpreter, which is a project rather than a parser, and nothing in this kernel needs it yet.
//!

//! # BUGS
//!
//! - **Nothing validates that two tables do not overlap**, or that a table's length is sane
//!   relative to where it sits in memory. The caller supplies the bytes, so the caller is where a
//!   physical-address sanity check belongs.
//! - **The MADT's `flags` bit 0 (`PCAT_COMPAT`) is reported and not acted on.** It means the machine
//!   also has 8259 PICs that must be masked before the APICs are used. Whoever brings the APIC up
//!   has to mask them; this only says whether they are there.
//! - **A DMAR's PCI device scopes and RMRRs are decoded into fixed arrays** ([`DmarUnits`],
//!   milestone 261 (the NVMe driver leaves the kernel)'s bench rehearsal, RMRRs since milestone 594 (every VT-d unit translates its own devices)) and anything past them sets
//!   [`DmarUnits::truncated`], which turns an ownership question into "unknown" rather than a
//!   guess. IOAPIC, HPET and ACPI-namespace scopes are skipped, and ATSR, RHSA, ANDD, SATC and
//!   SIDP are not decoded at all. An ANDD names an ACPI-namespace device (an I2C or serial
//!   controller with no PCI identity) that can issue DMA; this kernel drives none, so none is
//!   confined, and a machine where one matters needs ANDD and the type-5 scope decoded.
//! - **`Dmar::flags`' `INTR_REMAP` bit is reported and never acted on.** Interrupt remapping is a
//!   real VT-d feature this parser reports the platform's claim to
//!   ([`DmarUnits::platform_interrupt_remapping`]) and this kernel does not build.
//! - **An MCFG window whose `end_bus` precedes its `start_bus` is reported as written.** Nothing in
//!   the encoding forbids it and [`mcfg_entry`] does not refuse it, because the bus numbers are the
//!   firmware's claim and this module reports claims. [`McfgEntry::size`] answers zero for such a
//!   window, which is what "no bus is in this range" means; a caller that reads `start_bus` and
//!   `end_bus` itself, as the boot print does, sees the inverted pair.

mod bytes;
mod dmar;
mod fadt;
mod gtdt;
mod madt;
mod mcfg;
mod root;
mod sdt;
mod spcr;
#[cfg(test)]
mod tests;

// The IVRS, AMD's counterpart, is a module of its own beside the DMAR's: a different table with
// a different idea of how a device is named (by id, where the DMAR names it by path).
pub mod ivrs;

use bytes::{u16, u32, u64};
pub use dmar::{
    DeviceScope, Dmar, DmarEntry, DmarStructures, DmarUnits, Drhd, InterruptRemappingClaims,
    MAX_DRHDS, MAX_RMRR_SCOPES, MAX_RMRRS, MAX_SCOPE_PATH, MAX_SCOPES, Ownership, ReservedRegion,
    SCOPE_PCI_ENDPOINT, SCOPE_PCI_SUBHIERARCHY, dmar_structures, first_drhd, parse_dmar,
};
pub use fadt::{ArmBoot, ResetRegister, ResetSpace, parse_arm_boot, parse_fadt_reset};
pub use gtdt::{GTDT_ACTIVE_LOW, GTDT_EDGE_TRIGGERED, Gtdt, parse_gtdt};
pub use madt::{
    INTI_POLARITY_MASK, INTI_TRIGGER_MASK, ISA_BUS, ISA_IRQ_COUNT, IsaIrqRouting, MADT_PCAT_COMPAT,
    Madt, MadtEntries, MadtEntry, isa_irq_table, madt_entries, parse_madt,
};
pub use mcfg::{McfgEntry, mcfg_entry};
pub use root::{RSDP_SIGNATURE, Rsdp, parse_rsdp, root_entry, root_entry_count};
pub use sdt::{AcpiError, SDT_HEADER_LEN, SdtHeader, is_checksum_ok, parse_sdt_header};
pub use spcr::{SPCR_PL011, SPCR_SBSA_UART, Spcr, parse_spcr};
