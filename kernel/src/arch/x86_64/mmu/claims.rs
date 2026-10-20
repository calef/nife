//! **Every physical range the direct map covers, as one enumeration**: RAM, the firmware's
//! reservations and the device windows, each named for the panic that reports a collision.
//!
//! Cut out of `mmu.rs` unchanged by lane `maintainer/last-three-over-2000`, which left the
//! parent an index (§282 (when a file is split, the parent becomes an index)) and under §266 (a
//! Rust source file stays under 2,000 lines). `super::ap_boot`, `super::discovery` and `super::machine`
//! name the `x86_64` items the index imports, as in `mmu.rs`.
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

use super::*;

/// The first megabyte: real-mode legacy that is nonetheless memory, and the only part of the address
/// space this kernel reads by absolute physical address (the BIOS Data Area's EBDA pointer at
/// `0x40e`, the BIOS area the RSDP is scanned out of). The frame allocator clips it rather than
/// managing it, so it is in no RAM region and would otherwise be unmapped.
pub(super) const LOW_MEGABYTE: u64 = 0x10_0000;

/// **One physical range the direct map covers, and what to call it when it goes wrong.**
///
/// Every direct-map range [`map_everything`] builds comes out of [`direct_map_claims`] as one of
/// these, and the failure path walks the *same* enumeration to say what a refused mapping collided
/// with. That is the point of the type: a diagnosis derived from a second, hand-written list of
/// "what we probably map" can drift from what is mapped, and the first time anyone reads it will be
/// on a bench with no debugger.
///
/// Name: provisional. calef names the types, and this one was minted by a lane.
pub(super) struct Claim {
    /// What a reader of [`map_everything`] would call this range, short enough to fit a panic line
    /// on a screen console.
    pub(super) what: &'static str,
    /// The physical range, half-open. Empty (`hi <= lo`) is legal and maps nothing.
    pub(super) lo: u64,
    pub(super) hi: u64,
    pub(super) flags: Flags,
    /// **Map page by page, skipping what is already mapped, instead of refusing.**
    ///
    /// True only for the loader's reserved entries, which may abut or overlap each other and the
    /// RAM regions in the map the firmware wrote; nobody promised that list is disjoint. Everywhere
    /// else a second mapping of a frame is a real defect and [`MapError::AlreadyMapped`] is the
    /// mechanism that says so, so this is deliberately not a convenience anything else may reach
    /// for.
    pub(super) guarded: bool,
}

/// **Every physical range the direct map covers, in the order [`map_everything`] builds them.**
///
/// Split out from the mapping loop so that the failure path can ask the same question the mapping
/// asked. See [`Claim`].
pub(super) fn direct_map_claims(each: &mut dyn FnMut(Claim)) {
    // 1. **Every RAM region**, not just the low gigabyte the old single-base arithmetic could
    //    reach.
    //
    //    The kernel image's own frames are skipped: on this architecture the image lives at a
    //    *different* base, so a direct-map entry for those frames would not collide with the
    //    section mappings below (as it would on the other two ISAs), it would be a second,
    //    WRITABLE alias of `.text`. W^X that a second mapping undoes is not W^X. Nothing needs
    //    those frames by physical address anyway: the image is on the frame allocator's forbidden
    //    list, so they are never handed out.
    let image_lo = memory::image_start();
    let image_hi = memory::image_end();
    for (start, size) in memory::ram_regions() {
        let end = start + size;
        each(Claim {
            what: "ram",
            lo: start,
            hi: image_lo.min(end),
            flags: Flags::kernel_data(),
            guarded: false,
        });
        each(Claim {
            what: "ram above the image",
            lo: image_hi.max(start),
            hi: end,
            flags: Flags::kernel_data(),
            guarded: false,
        });
    }

    // 1b. **One deliberate hole inside the excluded image range** (milestone 161's SMP item): the
    //     AP trampoline's LMA, the gap link-x86_64.ld opens between `.rodata` and `.data` so the
    //     trampoline's *source* bytes are not `.boot_scratch` or the secondary stacks (both
    //     runtime-mutated; see that file's comment). It is inside `image_lo..image_hi`, so step 1
    //     skips it same as `.text`, and it is *between* two mapped sections rather than inside
    //     either, so neither section's own mapping reaches it either. Nobody's mapping covers it
    //     unless this does. `ap_boot::prepare` is the one reader, through the direct map.
    each(Claim {
        what: "ap trampoline",
        lo: super::ap_boot::trampoline_lma(),
        hi: super::ap_boot::trampoline_lma() + super::ap_boot::trampoline_size(),
        flags: Flags::kernel_data(),
        guarded: false,
    });

    // 5. **The device registers, device-typed (uncacheable), one window each**, and they come
    //    before the firmware reservations rather than after.
    //
    //    **The order is load-bearing and it did not used to be.** The reservation fill is guarded
    //    and these are not, so whichever runs first decides the memory type of any page both want,
    //    and only one of the two answers is correct: a cacheable mapping of a device window is a
    //    write that may sit in a cache line and never reach the device. Naming the windows first
    //    makes device typing win by construction, rather than by [`firmware_fill_ceiling`] being
    //    right about where the hole starts. Both now hold; this is the one that holds without
    //    knowing anything about the machine.
    //
    //    The local APIC's address comes from the machine's own MADT when ACPI answered, and falls
    //    back to the architectural reset default when it did not: `IA32_APIC_BASE` is relocatable,
    //    and mapping the constant while the hardware is somewhere else would be a page of nothing
    //    that reads as all-ones.
    let apic = super::discovery::local_apic_phys().unwrap_or(LOCAL_APIC_PHYS);
    each(Claim {
        what: "local apic",
        lo: apic,
        hi: apic + PAGE_SIZE,
        flags: Flags::device(),
        guarded: false,
    });

    // The IO APIC's own page, from the MADT when ACPI answered and from the architectural default
    // when it did not, for the same reason the local APIC's is: the address is the machine's to
    // state, and a machine with two of them puts the second somewhere this constant does not name.
    let io_apic = super::discovery::io_apic_phys().unwrap_or(IO_APIC_PHYS);
    each(Claim {
        what: "io apic",
        lo: io_apic,
        hi: io_apic + PAGE_SIZE,
        flags: Flags::device(),
        guarded: false,
    });

    // The PCIe windows: bus 0 of the ECAM config space ACPI's MCFG named, and the BAR window the
    // kernel assigns device registers from (derived from this machine's own memory map and host
    // bridge; see `memory_mapped_io_window`, milestone 256, which has no ACPI or AML source to read instead).
    // Same shape as the other two architectures' `memory::pci_regions()` (a device-tree
    // node there, ACPI's MCFG here, both recorded before this function runs): no window recorded,
    // no mapping, and every probe in `pci.rs` reports nobody home rather than touching MMIO that
    // was never confirmed present. The one-bus cap on the ECAM side is the same as the other two
    // architectures apply and for the same reason: everything QEMU puts on this machine is on bus
    // 0, and all 256 buses would be 64K leaves describing space that reads all-ones.
    if let Some(((ecam, ecam_size), (bar, bar_size))) = memory::pci_regions() {
        each(Claim {
            what: "pci ecam",
            lo: ecam,
            hi: ecam + crate::pci::ecam_bytes().min(ecam_size),
            flags: Flags::device(),
            guarded: false,
        });
        each(Claim {
            what: "pci bar window",
            lo: bar,
            hi: bar + PCI_BAR_MAPPED.min(bar_size),
            flags: Flags::device(),
            guarded: false,
        });
    }

    // VT-d's register files (milestone 161, roadmap item 6), one per unit since milestone 594 (every VT-d unit translates its own devices),
    // device-typed, each as large as its DRHD's `Size` byte says (one page on every unit met so
    // far), at the addresses ACPI's DMAR named (`memory::record_vtd_region`, called from `main.rs`
    // before this function runs). Same shape as the local APIC and IO APIC windows above: no DRHD,
    // no mapping, and `arch::iommu::init` is simply never called. Without this a DRHD's register
    // reads fault the instant the fine map replaces the coarse boot map that covered every
    // physical address indiscriminately; the first version of this driver found that by faulting.
    for (base, size) in memory::vtd_regions().into_iter().flatten() {
        each(Claim {
            what: "vt-d registers",
            lo: base,
            hi: base + size,
            flags: Flags::device(),
            guarded: false,
        });
    }
    // AMD-Vi's register files (lane `amd-vi`), the same claim on a machine whose IVRS named them.
    for (base, size) in memory::amd_vi_regions().into_iter().flatten() {
        each(Claim {
            what: "amd-vi registers",
            lo: base,
            hi: base + size,
            flags: Flags::device(),
            guarded: false,
        });
    }

    // **The screen** (milestone 243), when the boot handoff described one. Device-typed like every
    // other aperture here, and mapped for the same reason the VT-d window is: the kernel console
    // has been writing to it through the coarse boot map since before the tour's first line, and
    // the instant this fine map replaces that one an unmapped framebuffer would fault inside
    // `println!`, whose fault handler prints. There is no diagnostic that survives that.
    //
    // It is the largest window in this function by three orders of magnitude: 1920x1080 is 8 MiB,
    // which is 2026 leaves. That is the price of a screen and it is paid once.
    //
    // **Write-combining, not device-typed** (2026-10-04). Device-typed is strong uncacheable, one
    // bus transaction per four-byte store, and on xenon the boot console's scrolling swept visibly
    // down the screen. `Flags::write_combining()` is still device memory to every rule here (pages
    // rather than blocks, never cached), and selects the PAT entry `arch::x86_64::init` made WC on
    // every core before this map exists. The console only ever writes it (`screen_console`'s
    // `PixelSink` has no read), which is the other half: WC reads are as slow as UC ones.
    //
    // **An aperture inside a RAM region would be a different problem and is not this one.** On
    // xenon the aperture sits in the 32-bit MMIO hole, which the firmware's map does not describe;
    // it is in no RAM region and never was. See this module's BUGS for what a machine that really
    // did report both would need, and why "map it twice" is not among the options on x86.
    if let Some((base, size)) = memory::framebuffer() {
        each(Claim {
            what: "framebuffer",
            lo: base,
            hi: base + size,
            flags: Flags::write_combining(),
            guarded: false,
        });
    }

    // 2. **Memory the loader did not call usable RAM but which is still *memory***: the first
    //    megabyte (IVT, BDA, EBDA, the BIOS area the ACPI RSDP is scanned out of, and the page
    //    below 1 MiB a STARTUP IPI's vector will have to name), and the reserved entries carved out
    //    of DRAM (the ACPI tables, NVS, firmware scratch). Those are read through `phys_to_virt`
    //    and are in no RAM region, so without this they become arithmetic with no mapping the
    //    moment the boot map goes.
    //
    //    Guarded, because a reserved entry can abut or overlap something step 1 already mapped and
    //    nothing promises the firmware's list is disjoint. **`AlreadyMapped` is not treated as
    //    success anywhere**; asking first is what keeps the refusal meaning something everywhere
    //    else.
    firmware_claims(each);
}

/// The loader-reserved half of [`direct_map_claims`]: the low megabyte, every reserved entry below
/// [`firmware_fill_ceiling`], and the initial ramdisk.
///
/// Physical page 0 is deliberately left out, so that `phys_to_virt(0)` faults instead of quietly
/// naming the interrupt vector table.
///
/// Claims only the low megabyte when the boot structure cannot be re-read. That is the right
/// failure: this kernel got its RAM regions from the same structure, so a machine that reaches here
/// without one had no ACPI tables to lose access to either.
///
/// Name: provisional (milestone 161, renamed from `map_firmware_regions` when it stopped mapping
/// and started describing).
fn firmware_claims(each: &mut dyn FnMut(Claim)) {
    each(Claim {
        what: "low megabyte",
        lo: PAGE_SIZE,
        hi: LOW_MEGABYTE,
        flags: Flags::kernel_data(),
        guarded: true,
    });

    let Some(info) = super::machine::boot_info(crate::DTB.load(Ordering::Relaxed)) else {
        return;
    };
    let ceiling = firmware_fill_ceiling(|span| {
        for i in 0..info.memmap_entries as usize {
            let Some(e) = super::machine::memory_map_entry(&info, i) else {
                break;
            };
            span(
                e.addr & !(PAGE_SIZE - 1),
                e.end().next_multiple_of(PAGE_SIZE),
            );
        }
    });

    for i in 0..info.memmap_entries as usize {
        let Some(e) = super::machine::memory_map_entry(&info, i) else {
            break;
        };
        if e.is_usable_ram() {
            continue; // step 1 already did these, and did them minus the kernel image
        }
        let start = (e.addr & !(PAGE_SIZE - 1)).max(LOW_MEGABYTE);
        let end = e.end().next_multiple_of(PAGE_SIZE).min(ceiling);
        each(Claim {
            what: "firmware reservation",
            lo: start,
            hi: end,
            flags: Flags::kernel_data(),
            guarded: true,
        });
    }

    // **The initial ramdisk, explicitly, regardless of how the memmap classified the bytes it
    // occupies.** Found 2026-08-25 (decisions §86 (whether an NVMe driver can leave the kernel)'s
    // VT-d/NVMe data point): attaching an NVMe controller grows the ACPI tables enough that the
    // memmap's reserved entry immediately above the top of RAM widens to swallow the initial
    // ramdisk's last few hundred bytes (PVH's loader places it at a fixed offset below the top of
    // guest memory, sized for a smaller device set than this boot's tables need). The ceiling above
    // exists to keep this loop out of the MMIO hole, so widening it is not the fix. Claiming the
    // initial ramdisk's own recorded bounds directly sidesteps the question of which reserved entry
    // is real backing memory and which is not: `bring_up_memory` already claimed this exact range
    // as `forbidden` before this runs, so nothing else can be relying on it staying unmapped, and a
    // guarded claim is idempotent over anything already covered.
    if let Some((istart, isize)) = memory::initial_ramdisk_region() {
        each(Claim {
            what: "initial ramdisk",
            lo: istart & !(PAGE_SIZE - 1),
            hi: (istart + isize).next_multiple_of(PAGE_SIZE),
            flags: Flags::kernel_data(),
            guarded: true,
        });
    }
}
