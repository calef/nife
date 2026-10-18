//! **Where things are**: the kernel image's base, the direct map and its two translations,
//! the thread-stack area, and the fixed physical addresses of the devices every PC has.
//!
//! Cut out of `mmu.rs` unchanged by lane `maintainer/last-three-over-2000`, which left the
//! parent an index (§282 (when a file is split, the parent becomes an index)) and under §266 (a
//! Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

/// The base of the kernel **image**'s virtual addresses.
///
/// **Fixed at 0xffffffff80000000 by the target's code model**, not chosen for taste: see the module
/// header and link-x86_64.ld. It decomposes as PML4[511], PDPT[510], PD[0], which is why `boot.s`
/// can alias the first gigabyte into the high half with a single PDPT entry.
///
/// This is deliberately **not** where physical memory is mapped; see [`DIRECT_MAP_BASE`].
pub(crate) const KERNEL_VA_BASE: u64 = 0xffff_ffff_8000_0000;

/// **The base of the direct map: where physical address `pa` is readable as `pa + this`.**
///
/// `0xffff888000000000`, which is Linux's `page_offset_base`, taken rather than invented because a
/// reader who has met one `x86_64` kernel has met this number. What the value has to satisfy is
/// short: canonically high (bit 47 set and sign-extended, so the same `Ia32e::is_in_half` test that
/// admits the kernel image admits this), clear of PML4[511] where the image lives, clear of the low
/// half that ring 3 will get, and with room above it for all of physical memory. This has 64 TiB of
/// room before it would reach anything else, which is 2^46 and not a limit worth thinking about.
///
/// **`boot.s` installs the PML4 entry for this before any Rust runs**, so the arithmetic below is
/// valid from the first instruction rather than from [`init`](super::init); the module header says
/// why that matters more than it looks like it should.
///
/// Name: provisional (milestone 161 (the `x86_64` kernel port)): calef names the constants, and
/// this one was minted by a lane. The tree's analogous name is [`KERNEL_VA_BASE`], which is why
/// this is `_BASE` too.
pub const DIRECT_MAP_BASE: u64 = 0xffff_8880_0000_0000;

/// The top-level (PML4) index the direct map occupies, **duplicated in `boot.s`** because a 32-bit
/// assembler cannot compute it from the constant above. This is the gate that keeps the two in
/// agreement: change [`DIRECT_MAP_BASE`] without changing `boot.s` and the kernel does not build.
/// **Where kernel thread stacks live, virtually** (`thread.rs`'s `STACK_AREA`).
///
/// `0xffff_c900_0000_0000` is **Linux's `VMALLOC_START`**, taken rather than invented for the same
/// reason [`DIRECT_MAP_BASE`] is Linux's `page_offset_base`: a reader who has met one `x86_64` kernel
/// has met this number, and the alternative is a constant nobody can check against anything.
///
/// **The portable expression the other two architectures use does not survive here**, and the way
/// it fails is silent. `thread.rs` computed this as `KERNEL_VA_BASE | 0x10_0000_0000` ("64 GiB above
/// the direct map"), which works where the kernel base is a *half* base with room above it.
/// [`KERNEL_VA_BASE`] here is `0xffff_ffff_8000_0000`, which already has every bit of
/// `0x10_0000_0000` set, so the OR was the identity: every kernel thread stack would have been
/// mapped at the kernel image's own base, over `.text`. Caught by reading rather than by booting,
/// because the mapper's overwrite refusal turns it into `kmem: no memory to wire one` far from the
/// cause.
///
/// PML4[402], which is nothing else's: the image is PML4[511] and the direct map PML4[273].
///
/// Name: provisional (milestone 161, roadmap item 4).
pub const THREAD_STACK_AREA: u64 = 0xffff_c900_0000_0000;

const DIRECT_MAP_PML4_INDEX: u64 = 273;

const _: () = {
    assert!(
        DIRECT_MAP_BASE >> 47 == 0x1_ffff,
        "the direct map base must be canonically high, or every address in it faults"
    );
    assert!(
        (DIRECT_MAP_BASE >> 39) & 0x1ff == DIRECT_MAP_PML4_INDEX,
        "boot.s writes PML4[273]: change both or neither"
    );
    assert!(
        (KERNEL_VA_BASE >> 39) & 0x1ff != DIRECT_MAP_PML4_INDEX,
        "the image and the direct map must not share a PML4 entry"
    );
};

/// The kernel's view of physical address `pa`: its address in the direct map.
///
/// **Not for the kernel image's own frames.** The direct map skips them on purpose (a second,
/// writable alias of `.text` would undo W^X; `claims::direct_map_claims` step 1), so the result for
/// an image frame is unmapped and a read through it faults.
pub(crate) const fn phys_to_virt(pa: u64) -> u64 {
    pa + DIRECT_MAP_BASE
}

/// The physical address behind kernel virtual address `va`.
///
/// **Two branches, because this architecture has two kernel bases** (see the module header). An
/// address at or above [`KERNEL_VA_BASE`] is part of the kernel image, linked there by
/// link-x86_64.ld; anything else in the high half is a direct-map address. Both callers exist:
/// `memory::image_start` hands this a linker symbol, and `sched`/`kmem` hand it pointers that came
/// out of [`phys_to_virt`]. Linux's `__pa()` makes the same distinction for the same reason.
///
/// It is the exact inverse of [`phys_to_virt`] over the direct map, now for **all** of physical
/// memory rather than the low 2 GiB the single-base arithmetic could reach.
pub(crate) const fn virt_to_phys(va: u64) -> u64 {
    if va >= KERNEL_VA_BASE {
        va - KERNEL_VA_BASE
    } else {
        va - DIRECT_MAP_BASE
    }
}

/// COM1's I/O port on every x86 machine since the PC/AT, and on QEMU's `q35`. Not an address in the
/// memory space at all; see `arch/x86_64/port.rs` for what that means and why the console reaches it
/// through a different accessor than the other two architectures' UARTs.
pub(crate) const COM1_PORT: usize = 0x3f8;

/// The local APIC's default physical base. Relocatable through `IA32_APIC_BASE`, and nothing
/// relocates it; the constant is where the machine puts it at reset.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub(crate) const LOCAL_APIC_PHYS: u64 = 0xfee0_0000;

/// The IO APIC's default physical base on a PC-compatible machine, used only when the ACPI MADT
/// did not say. `irq::io_apic_phys()` is what the machine actually reported, and q35 reports
/// exactly this.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub(crate) const IO_APIC_PHYS: u64 = 0xfec0_0000;

/// **Where q35 puts the PCIe ECAM window**, and QEMU's q35 default. `map_everything` no longer
/// maps this constant directly: the real window is read from `memory::pci_regions()`, which
/// `main.rs` fills from ACPI's MCFG the same way `memory::init` fills it from a
/// `pci-host-ecam-generic` node on the other two architectures. This stays as the value
/// `machine::print_acpi_summary` prints the MCFG's own answer against, which is the second
/// witness aarch64 and riscv64 also kept (their old device-tree hardcodes, held equal to the
/// discovered value by `pci.rs`'s own test).
///
/// **The asymmetry is worth knowing before reading a boot log.** Those two architectures only ever
/// boot QEMU `virt`, so there the second witness is an *equality invariant* and a difference would
/// be a finding. Here it is not, and not only on real hardware. Firmware places this window, so the
/// two values agree on QEMU q35 **under `SeaBIOS`** and nowhere else that has been measured: OVMF
/// relocates it to `0xe000_0000`, which is what both of `script/test --arch x86_64`'s real-firmware
/// boots print, and xenon's own MCFG says `0xf000_0000` over 128 buses. That is why
/// `print_acpi_summary` names q35 rather than this constant when it prints the comparison
/// (milestone 314); the earlier wording read as a defect report everywhere except the one firmware
/// it was written against, and was eventually reported as one.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub(crate) const PCI_ECAM_PHYS: u64 = 0xb000_0000;

/// How many PCI buses of the ECAM window the kernel maps **before it has asked the machine**. One
/// bus is 1 MiB of configuration space, and the MCFG on the first real machine this port met
/// describes a hundred and twenty-eight of them.
///
/// **The floor, not the answer, since milestone 320.** `pci::ecam_buses()` is what the kernel maps
/// and reads: this value until `pci::survey` has run, and the topology the survey found afterwards.
/// The survey runs from `kernel_main` while the boot tables still cover the low 4 GiB
/// indiscriminately, which is the only window in the boot where every bus the MCFG describes is
/// readable; `map_everything` runs after it and maps what it found.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub(crate) const PCI_ECAM_BUSES: u16 = 1;

/// A page table at physical address `pa`, reached through the direct map.
pub(crate) fn phys_to_ptr(pa: u64) -> *mut paging::PageTable {
    phys_to_virt(pa) as *mut paging::PageTable
}
