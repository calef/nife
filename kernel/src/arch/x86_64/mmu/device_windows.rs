//! **The device windows**: the virtio-mmio bus q35 does not have, and the PCI BAR window
//! and interrupt base the arch contract names.
//!
//! Cut out of `mmu.rs` unchanged by lane `maintainer/last-three-over-2000`, which left the
//! parent an index (§282 (when a file is split, the parent becomes an index)) and under §266 (a
//! Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

// ---------------------------------------------------------------------------------------------
// The device windows. Both `virt` machines describe theirs in a device tree; q35 describes its PCIe
// window in ACPI's MCFG and has no virtio-mmio bus at all.
// ---------------------------------------------------------------------------------------------

/// **There is no virtio-mmio transport on q35.** Both `virt` machines put eight virtio devices on a
/// memory-mapped bus the kernel finds by walking fixed slots; a PC has never had such a thing, and
/// every virtio device on q35 is a PCI function. So this base is zero and [`VIRTIO_SLOTS`] is zero,
/// which is what makes `virtio::find_block_device` find nothing rather than probe an address that
/// answers with bus noise. The PCIe transport (DECISIONS §18) is the one that carries here, which is
/// the arrangement that transport was built for.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const VIRTIO_MMIO_BASE: u64 = 0;
/// Bytes between consecutive virtio-mmio transports. Meaningless with no bus; see
/// [`VIRTIO_MMIO_BASE`].
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const VIRTIO_SLOT_STRIDE: u64 = 0x1000;
/// **Zero virtio-mmio slots**, which is the fact, not a placeholder. See [`VIRTIO_MMIO_BASE`].
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const VIRTIO_SLOTS: u64 = 0;
/// How many slots the probe walks: none, see [`VIRTIO_SLOTS`]. A function because riscv64's answer
/// depends on the device tree.
pub const fn virtio_slots() -> u64 {
    VIRTIO_SLOTS
}
/// The interrupt the first virtio-mmio slot would raise. Unreachable with no slots.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const VIRTIO_IRQ_BASE: u32 = 0;

/// Bytes of PCI BAR space the kernel maps for device registers, matching what the other two
/// architectures reserve. 2 MiB covers every BAR QEMU hands out on this machine, and since
/// milestone 256 it is the size of the window rather than a slice of a larger one: **where** that
/// window goes is [`memory_mapped_io_window()`](super::memory_mapped_io_window())'s answer and is
/// the machine's, not a constant's.
///
/// It stays small on purpose. It only ever has to hold the BARs this kernel **places**, and a BAR
/// the machine placed itself is adopted where it stands (`pci::place_bars`) rather than moved into
/// here, so on real firmware almost nothing is drawn from it. Every byte of it is mapped with
/// 4 KiB leaves at boot, which is this module's first recorded BUG, so a window sized for the
/// whole MMIO hole would be page tables for hundreds of megabytes nothing decodes.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const PCI_BAR_MAPPED: u64 = 0x20_0000;

/// The first interrupt a PCI function raises.
///
/// **Zero, and nothing on this architecture reads it any more** (milestone 215, `x86_64` PCI
/// interrupt routing). On the other two machines this is a fixed base the device tree states and
/// legacy INTx lines land at base + pin. On q35 a PCI function's legacy interrupt goes through the
/// PIRQ router to an IO APIC input the ACPI `_PRT` names, `_PRT` is AML, and this tree has no
/// interpreter for it; zero was the marker for that rather than a value to trust.
///
/// What replaced it is not a better constant, it is **not a constant at all**: a function's
/// interrupt is bound by `arch::x86_64::irq::alloc_msi_vector`, which reserves a vector and hands
/// `kernel/src/pci.rs` the address and data to write into the function's own MSI-X table. The
/// device is told where to deliver, so there is no board-specific routing to encode here. The
/// constant survives because the arch contract's other two implementations are real and the
/// portable code names it; `kernel/src/pci.rs` reaches it only on a machine whose
/// `alloc_msi_vector` answers `None`, which this one never does.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub const PCI_IRQ_BASE: u32 = 0;
