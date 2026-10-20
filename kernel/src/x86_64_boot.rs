//! **The `x86_64` boot**: one function per phase, ending at the hand-over.
//!
//! A self-contained tour that diverges before aarch64's shared bring-up, because the arch layer
//! beneath that path is not built for this port. See notes/x86-port.md. Moved out of the crate
//! root by milestone 876 (the kernel crate root is split along its seams), bodies unchanged.
//!
//! **Temporary.** calef ruled so on 2026-10-12 (UTC), in §284 (the kernel crate root splits into
//! child modules, one per boot): this file is the port's route until it joins the shared path in
//! `boot.rs`, and the parity work shrinks it toward the CPU bring-up that belongs under `arch/`. It
//! is not an architecture layer. The work is
//! design/roadmap/0890-riscv64-and-x86-64-join-the-shared-boot-path.md.
//!
//! Name: provisional (milestone 876's lane, 2026-10-12 UTC).

use super::*;

/// `x86_64` phase 1: the screen, the banner, the traps, and what the loader and ACPI say.
#[cfg(target_arch = "x86_64")]
fn x86_64_first_light(
    boot_info_pointer: usize,
) -> (machine_discovery::x86_64::BootInfo, arch::machine::Acpi) {
    // A live code address proves the far jump in boot.s landed in the high half and that we are
    // executing there rather than merely reaching the UART through the identity map (the boot
    // table maps both). If this reads 0xffffffff801xxxxx, long mode is on and the kernel is in
    // the high half.
    let pc = kernel_main as *const () as usize;

    // **The screen, before the first word** (milestone 243). On a commodity machine there is no
    // serial port, so unless this runs here the entire tour below is written to a device that
    // is not there. It is deliberately the first thing the x86 arm does: everything after it is
    // visible on a monitor, and everything before it is not, so the earliest possible line is
    // the one worth buying.
    //
    // It needs nothing brought up first. The screen's address arrives in the boot handoff and
    // the boot page tables `boot.s` installed already cover the low 4 GiB, which is where a
    // framebuffer aperture is; `arch::mmu::map_everything` later carries the mapping across to
    // the fine map (`memory::record_framebuffer`, called inside).
    let screen = arch::machine::attach_screen(boot_info_pointer);

    println!();
    println!(
        "{}x86_64 (long mode, ring 0, 4-level paging)",
        boot_ladder::BANNER,
    );
    println!("  cpu 0 booted: high-half kernel, .bss, and the 16550 console are up.");
    println!("  running at  : {pc:#018x}  (high half: the long-mode jump landed)");
    println!("  boot info   : {boot_info_pointer:#018x}  (PVH hvm_start_info, not a device tree)");
    // Said here rather than where it happens, because the line has to be printed by a console
    // that already exists in order to be on the screen it is describing. A machine with no
    // framebuffer says so and is not otherwise different, which is xenon: it has a serial port,
    // which is why milestone 87 chose it.
    match screen {
        Some((found, cols, rows)) => println!(
            "  screen      : {}x{} {} at {:#x}, {cols}x{rows} cells (boot cmdline)",
            found.width,
            found.height,
            found.order.token(),
            found.base,
        ),
        None => println!("  screen      : none in the boot cmdline; this console is the UART"),
    }

    // What machine is this. CPUID needs nothing to be brought up first, which makes x86 the
    // only one of the three where this can run before anything else.
    arch::isa::init(boot_info_pointer);
    arch::isa::print_summary();

    // The GDT, the TSS and the IDT. Until this line a fault is a triple fault and a silent
    // machine reset; after it, a fault prints. That is the whole value of the step, and it is
    // why it is the second thing the tour does rather than the tenth.
    arch::init();
    let caught = arch::exceptions::self_test();
    println!("  traps       : idt installed; a breakpoint was caught and stepped over ({caught})");

    // What the loader said: the PVH memory map and the ACPI root pointer. The x86 stand-in for
    // the device tree, and the only thing that reads the map so far; `memory::init` is a
    // device-tree parser, so the frame allocator cannot come up here until there is a discovery
    // seam between the two. See notes/x86-port/acpi-and-pci.md.
    let Some(info) = arch::machine::boot_info(boot_info_pointer) else {
        println!(
            "  memory      : no PVH boot info at {boot_info_pointer:#x}; nothing else can be found"
        );
        println!("nife x86_64: early boot cannot continue, halting.");
        arch::halt(arch::HaltReason::before_scheduler());
    };
    arch::machine::print_memory_map(&info);

    // ACPI: what x86 has instead of a device tree. The RSDP is scanned for rather than taken
    // from the handoff, because QEMU's PVH loader leaves that field zero (measured); the
    // checksum inside the parser is what separates a real hit from a coincidence.
    let acpi = arch::machine::read_acpi(info.rsdp);
    arch::machine::print_acpi_summary(&acpi);

    // Which CPUs does this machine have (milestone 161's SMP item)? Here, right beside the rest
    // of the ACPI walk, mirroring where the other two architectures read their own roster
    // (`smp::read_cpu_list`, right after the device tree that describes it is parsed). Seats
    // every core the MADT lists, boot core included, at the slot its own local APIC id names.
    smp::seat_cpus_from_acpi(&acpi.cpus[..acpi.cpu_count]);

    // COM1's interrupt line (milestone 176 (the x86_64 discovery seam's wide half: COM1's IRQ
    // and a CMOS RTC)), filling the same static `memory::uart_irq()` the
    // other two architectures fill from their device tree. **It is the legacy number, 4, and
    // not the GSI it resolves to**, because an intid on this architecture is a legacy IRQ:
    // `arch::irq::enable` resolves it through the MADT's overrides (`isa_irqs`, recorded above)
    // when it arms the line. Recording the GSI here was harmless while nothing armed it, since
    // the two agree on every machine without an override; since milestone 505 (an x86_64 input
    // driver that never lets the core idle) the input driver waits on it, and a machine that
    // overrides IRQ 4 would have had its GSI resolved a second time as though it were legacy.
    memory::record_uart_irq(user::UART_RX_INTID);

    // VT-d's register window, recorded now (before `arch::mmu::init()` a few lines down)
    // rather than where it is actually brought up. `mmu::map_everything` reads
    // `memory::vtd_region()` to decide what to map device-typed, and it has to know before it
    // runs; `arch::iommu::init` itself is called later, once the fine map it needs exists.
    //
    // **It moved above the PCI block on 2026-09-04** (milestone 256) and the order is now
    // load-bearing rather than incidental: `arch::mmu::memory_mapped_io_window` takes the windows this
    // kernel already knows about out of the hole it picks a BAR window from, and VT-d's
    // register file is one of them. Recorded afterwards, it would be a window the choice below
    // could not see.
    for d in acpi.dmar.units() {
        memory::record_vtd_region(d.register_base, d.register_size);
    }
    // AMD-Vi's, for the same reason and in the same place (lane `amd-vi`). Its IVHD carries no
    // size, so the window is the 16 KiB every register this kernel reads lives in.
    for d in acpi.ivrs.units() {
        memory::record_amd_vi_region(d.register_base, arch::amd_vi::REGISTER_SIZE);
    }
    (info, acpi)
}

/// `x86_64` phase 2: the PCIe configuration window, its BAR window, and the topology.
#[cfg(target_arch = "x86_64")]
fn x86_64_pci(acpi: &arch::machine::Acpi) {
    // Turn the MCFG's ECAM window on and record it where kernel/src/pci.rs already knows to
    // look: `memory::pci_regions()`, the same static a device-tree machine fills from its
    // `pci-host-ecam-generic` node. No MCFG, no PCI at all, the same treatment the other two
    // architectures give a tree with no such node. The BAR window has no ACPI or AML source
    // and is derived from the machine instead (`arch::mmu::memory_mapped_io_window`, milestone 256).
    match acpi.ecam {
        // The MCFG's base is the configuration space of its FIRST bus, and `pci.rs` addresses a
        // function as `base + (bus << 20 | ...)` with an absolute bus number. Those agree only
        // when the first bus is zero, which every machine seen so far reports and no machine is
        // required to. Refused rather than adjusted: subtracting `lo << 20` names a base below
        // the window `mmu::map_everything` maps, so the arithmetic that looks like the fix
        // produces config reads into whatever is underneath it.
        Some((_, lo, _)) if lo != 0 => {
            println!(
                "  pci         : skipped, the MCFG's first bus is {lo} and this port assumes 0"
            );
        }
        Some((base, lo, hi)) => {
            let buses = hi as u32 - lo as u32 + 1;
            let decode = arch::machine::enable_pcie_ecam(base, buses);
            let ecam = (base, buses as u64 * 0x10_0000);
            // **Where BARs may go, asked of the machine** (milestone 256). A failure here is a
            // panic and not a fallback: the constant this replaced was checked against one
            // emulated machine and was RAM on the first real one, so a kernel that carried on
            // with it would be placing device registers over memory the allocator owns. The
            // message is the whole diagnosis, because the machine that produces it has no
            // serial cable and a photograph of the screen is the entire transcript.
            let bar = match arch::mmu::memory_mapped_io_window(ecam) {
                Ok(window) => window,
                Err(why) => panic!("no PCI BAR window on this machine: {why}"),
            };
            memory::record_pci_regions(ecam, bar);
            println!(
                "  pci         : ecam at {base:#x} (buses {lo}..={hi}), decode {}",
                match decode {
                    arch::machine::EcamDecode::AlreadyOn => "was already on",
                    arch::machine::EcamDecode::Programmed => "programmed here",
                    arch::machine::EcamDecode::Unencodable =>
                        "LEFT AS FOUND: this bus count has no PCIEXBAR length field",
                }
            );
            println!(
                "                bar window {:#x}..{:#x}, from the firmware map{}",
                bar.0,
                bar.0 + bar.1,
                match arch::machine::top_of_low_dram() {
                    Some(t) => {
                        let _ = t;
                        " and TOLUD, agreeing"
                    }
                    None => " alone (this host bridge reports no TOLUD)",
                }
            );
            // **The topology, read from the bridges, and it must happen here** (milestone 320).
            // The boot tables still cover the low 4 GiB indiscriminately at this point, so
            // every bus the MCFG describes is readable without a single mapping; `mmu::init`
            // a hundred lines down then maps exactly the buses this found. Run after it, this
            // would read its way off the end of the window it is trying to size.
            //
            // It is in the x86 arm rather than beside `memory::init` for the same reason the
            // BAR census below is: both `virt` boards describe a flat root complex in their
            // device tree, QEMU puts nothing behind a bridge on either, and a walk there would
            // print bus 0 and stop. See `pci::survey`.
            pci::survey();
            // The census is here, in the x86 arm, because this is the only architecture whose
            // BARs may already be placed by something else when the kernel arrives. Both
            // `virt` boards boot with `-bios default` and every BAR is zero, so there would be
            // nothing to report. See `pci::bar_census` for why the second number is the one to
            // watch on real firmware.
            let (functions, outside) = pci::bar_census();
            println!(
                "                {functions} function(s) on the bus, \
                 {outside} with a BAR this kernel can neither use nor adopt",
            );
        }
        // **No MCFG means no PCI, and deliberately no fallback to the legacy 0xcf8/0xcfc
        // configuration mechanism**, which this kernel could reach (`enable_pcie_ecam` uses it)
        // and which would enumerate bus 0 without any ACPI at all. Milestone 215 refused the
        // same shape one level down and the reason carries: the ports see only the first 256
        // bytes of a function's configuration space, so a machine that fell back would
        // enumerate a DIFFERENT set of capabilities from one that did not, with every extended
        // capability simply absent, and a driver that then failed would fail somewhere else
        // entirely. A machine that describes no MCFG is a machine this port does not run on
        // yet, and saying so here costs less than discovering it downstream.
        None => {
            println!("  pci         : skipped, no MCFG: no PCI at all (no legacy fallback)");
        }
    }
}

/// `x86_64` phase 3: the local APIC and its timer, then the IO APIC and a device interrupt.
#[cfg(target_arch = "x86_64")]
fn x86_64_interrupts(boot_info_pointer: usize, acpi: &arch::machine::Acpi) {
    // The local APIC, and then a real hardware interrupt. Until this point the only trap the
    // kernel has taken is one it raised itself with `int3`; a periodic timer proves the other
    // half, that an interrupt the CPU did not ask for arrives, is dispatched by vector, and is
    // acknowledged so that a second one can follow.
    if let Some(apic) = acpi.local_apic {
        // SAFETY: the address came from the machine's own ACPI MADT, and the identity map the
        // boot tables installed still covers it.
        unsafe { arch::irq::init_local_apic(apic) };
        println!(
            "  apic        : local apic {apic:#x} up, id {}, version {:#x}, 8259s masked",
            arch::irq::local_apic_id(),
            arch::irq::local_apic_version(),
        );

        arch::timer::init_frequency(boot_info_pointer);
        let calibration = arch::timer::calibration();
        println!(
            // The worst window is printed beside the chosen one on purpose. The chosen rate is
            // the *smallest* of several timed windows, because a window's error is one-sided,
            // so the gap between them is this boot's own evidence of how much the host was
            // descheduling the vCPU mid-calibration. A quiet host prints two numbers a
            // megahertz apart; the boot that measured 4330 MHz would have printed the gap that
            // said so. See arch::x86_64::timer's CALIBRATION_WINDOWS.
            "  clocks      : tsc {} MHz ({}, best of {} windows, worst {} MHz), apic timer {} MHz (measured against the PIT)",
            arch::timer::frequency() / 1_000_000,
            arch::timer::frequency_source(),
            calibration.windows().len(),
            calibration.worst() / 1_000_000,
            arch::timer::apic_timer_frequency() / 1_000_000,
        );

        // The TSC-vs-RTC measurement instrument, off unless the `tsc_probe` feature is on.
        // It halts when it is done, so it never reaches the rest of the tour, the same shape
        // `bench` and `icount` take. See notes/tsc-under-tcg.md.
        #[cfg(feature = "tsc_probe")]
        {
            arch::tsc_probe::probe();
            arch::halt(arch::HaltReason::measurement_boot());
        }

        arch::timer::init();
        arch::interrupts::enable();
        let start = arch::timer::now();
        while arch::timer::now().wrapping_sub(start) < arch::timer::frequency() / 5 {
            arch::wait_for_interrupt();
        }
        arch::interrupts::disable();
        // The tick count is the delivery evidence; `ROUTED_IRQS` used to be printed beside
        // it and no longer counts a timer tick, because nothing routes one to a driver.
        println!(
            "  timer       : {} ticks in ~0.2s at {} Hz ({} spurious)",
            arch::timer::ticks(),
            arch::timer::TICK_HZ,
            arch::exceptions::SPURIOUS_IRQS.load(core::sync::atomic::Ordering::Relaxed),
        );
    } else {
        println!("  apic        : skipped, the MADT did not say where the local apic is");
    }

    // The IO APIC, and with it a real *device* interrupt. The local APIC timer above proves the
    // CPU takes an interrupt it did not ask for; this proves a line outside the CPU reaches it,
    // which is a different claim and needs a different device.
    //
    // The PIT is that device, and it is the right one twice over: `timer.rs` already drives it
    // for calibration, and its legacy IRQ 0 is the line every PC rewires, arriving as global
    // system interrupt 2. Arming redirection entry 0 for "the timer" would arm the 8259 cascade
    // and produce no interrupts and no error, so routing this line is simultaneously the
    // easiest device to reach and the strongest test of the override table.
    if let (Some((io_id, io_addr, gsi_base)), true) =
        (acpi.io_apic, arch::irq::is_local_apic_ready())
    {
        // SAFETY: the address and global-interrupt base came from the machine's own ACPI MADT,
        // and the boot map covers the whole low 4 GiB this device sits in.
        unsafe { arch::irq::init_io_apic(io_addr as u64, gsi_base) };
        arch::irq::record_isa_routing(&acpi.isa_irqs);
        let chip_id = arch::irq::io_apic_id();
        println!(
            "  io apic     : id {chip_id} at {io_addr:#x} up, version {:#x}, {} redirection entries, gsi base {gsi_base}",
            arch::irq::io_apic_version(),
            arch::irq::io_apic_entries(),
        );
        if chip_id != io_id {
            // Worth a line rather than an average: the MADT and the chip disagreeing about
            // which IO APIC this is means one of the two is describing a different machine.
            println!(
                "                the MADT calls it id {io_id}; the register above is the chip's own answer"
            );
        }

        // The local APIC timer is masked for the window below so the count is the PIT's alone.
        // Both would be delivered correctly; one number that means one thing is worth more.
        arch::irq::mask_timer();

        let pit = arch::irq::isa_routing(arch::timer::PIT_IRQ);
        let hz = arch::timer::start_pit_ticking(arch::timer::TICK_HZ);
        arch::irq::enable(arch::timer::PIT_IRQ);
        // Read after `enable` rather than before, which is what makes the `expect` honest
        // rather than hopeful: `enable` has just routed this same GSI and panics with a named
        // reason if the IO APIC does not own it, so an index exists by the time this runs. It
        // also makes the transcript's field the vector that was programmed instead of one
        // computed alongside it.
        let vector = arch::irq::gsi_vector(pit.gsi)
            .expect("enable routed this gsi a line ago, so the io apic owns it");

        arch::exceptions::enable_external();
        arch::interrupts::enable();
        let start = arch::timer::now();
        while arch::timer::now().wrapping_sub(start) < arch::timer::frequency() / 5 {
            arch::wait_for_interrupt();
        }
        arch::interrupts::disable();
        arch::irq::mask_gsi(pit.gsi);

        println!(
            "  device irq  : pit irq {} -> gsi {} on vector {vector:#x}: {} interrupts in ~0.2s at {hz} Hz",
            arch::timer::PIT_IRQ,
            pit.gsi,
            arch::exceptions::DEVICE_IRQS.load(core::sync::atomic::Ordering::Relaxed),
        );
    } else {
        println!("  io apic     : skipped, the MADT did not describe one");
    }
}

/// `x86_64` phase 4: the frame allocator, the fine page tables, and the IOMMU.
#[cfg(target_arch = "x86_64")]
fn x86_64_memory(info: &machine_discovery::x86_64::BootInfo, acpi: &arch::machine::Acpi) {
    // The frame allocator, from the PVH memory map. This is the seam: `memory::init` is a
    // device-tree front end and there is no tree here, so the x86 side assembles the same two
    // slices (RAM, and what is already spoken for) and hands them to the half that does not
    // care where they came from.
    stack::init();
    // Paint the boot stack's unused region for the high-water instrument (milestone 84), before
    // anything else can push a frame into it. Both other boots do this here, in the same breath
    // as `stack::init`; without it the instrument reads whatever the trampoline left and reports
    // 100% of a 64 KiB stack in use, which is what the first x86 test run said.
    #[cfg(any(test, feature = "system_tests"))]
    stack::paint_boot_stack();
    let regions = arch::machine::bring_up_memory(info);
    let first = memory::alloc().expect("no frame from the allocator");
    println!(
        "  frames      : allocator up over {regions} ram region(s) (first frame {:#x})",
        first.addr(),
    );
    memory::print_summary();

    // Replace the boot map (4 GiB identity, everything present/writable/executable) with
    // fine-grained W^X tables and switch `CR3` to them. We keep running, and keep printing,
    // across the switch, which is what proves the fine map covers this code and this stack.
    // The identity map is gone afterwards, and with it the last alias of physical memory in the
    // half ring 3 will get. See notes/x86-port/interrupts-and-the-fine-map.md.
    arch::mmu::init();
    arch::mmu::print_summary();
    println!(
        "  image       : text {:#x}..{:#x}, stack {:#x}..{:#x}",
        arch::mmu::text_start(),
        arch::mmu::text_end(),
        arch::mmu::stack_bottom(),
        // `arch::mmu`'s rather than this file's `stack_top`, which is `#[cfg(not(test))]`
        // because it belongs to the aarch64 tour. Same linker symbol, same answer, and this
        // one exists in a test build, which is the boot this tour now has to survive.
        arch::mmu::stack_top(),
    );

    // Milestone 162: RDSEED needs no ring 3, no capability, and nothing this port has not
    // already built, so it is provable here even though the entropy service itself cannot run
    // yet (no userspace to spawn it into). `draw_random_seed` already checked CPUID leaf 7
    // EBX.18 before ever executing the instruction.
    match arch::isa::draw_random_seed() {
        Some(v) => {
            println!("  entropy     : rdseed supported (cpuid leaf 7 ebx.18), drew {v:#018x}");
        }
        None if arch::isa::get().has_random_seed_instruction() => {
            println!("  entropy     : rdseed supported but stayed dry across every retry");
        }
        None => println!("  entropy     : rdseed not supported (cpuid leaf 7 ebx.18 clear)"),
    }

    // **Unhalted core cycles** (milestone 309), the x86_64 half of milestone 74's measurement
    // side. Here rather than beside `isa::print_summary` at the top of this tour, which is
    // where the riscv64 boot puts its own `pmu::init`: this one's did-it-count check spins
    // against the TSC, so it has to come after `timer::init_frequency` above. Beside `entropy`
    // because the two are the same shape, a CPUID-gated probe that reports what it found and
    // refuses to guess.
    //
    // Silent and harmless where there is no performance monitoring, which is every QEMU boot
    // unless its `pmu` property is on: the counter is then `None` forever and no MSR is ever
    // read. `arch::x86_64::pmu` is emphatic about why this is not `rdtsc`.
    arch::pmu::init();
    arch::pmu::print_summary();

    // The per-CPU interrupt stacks go live here, for the reason both other boots arm them right
    // after their own `mmu::init`: their guard pages are holes in the map that was just
    // installed, and before that they are covered by the coarse boot map and are not holes yet.
    interrupt_stack::init();

    // VT-d (milestone 161, roadmap item 6), if the DMAR named a DRHD. The same position the
    // SMMUv3 and the RISC-V IOMMU come up in on the other two boots: after the fine page
    // tables (a DRHD's register file is device-typed MMIO, reachable only through the map
    // `mmu::init` just installed) and before anything that could attach a device. The
    // kernel-resident NVMe driver (`kernel/src/non_volatile_memory_express.rs`, decisions §86) is the first PCI
    // device this architecture confines through it, on the same terms as the aarch64/riscv64
    // legs' SMMUv3/riscv-iommu confinement; a boot with no NVMe controller attached (no
    // NIFE_NVME on this leg) still proves the driver stands up against real hardware: root
    // table installed, translation enabled, read back from the register the hardware itself
    // reports status through.
    if !acpi.dmar.units().is_empty() {
        // **Every unit the DMAR names, each translating the devices it owns** (milestone 594 (every VT-d unit translates its own devices),
        // provisional number). `init` polls GSTS.RTPS then GSTS.TES itself on each unit and
        // panics rather than returning if either write never takes, so a unit reported up
        // below is the hardware's own status register, not an assumption that the write
        // succeeded. It prints one `vt-d` line per unit, including any it refused and why.
        arch::iommu::init(&acpi.dmar);
    } else if !acpi.ivrs.units().is_empty() {
        // **An AMD machine** (lane `amd-vi`): no DMAR, an IVRS instead. Every unit it names,
        // each enabled over an all-blocked device table; one `amd-vi` line per unit, read back
        // from the unit's own status register, as the VT-d line is.
        println!("  vt-d        : skipped, no DMAR (this machine's IOMMU is AMD-Vi)");
        arch::amd_vi::init(&acpi.ivrs);
    } else {
        // Loud, because the alternative is a boot that reads like every other one while
        // nothing stands between a device and all of memory.
        println!("  vt-d        : skipped, no DMAR");
        println!(
            "  iommu       : NONE: this machine's ACPI names neither a DMAR (VT-d) nor an IVRS \
             (AMD-Vi), so no device's DMA is confined; every driver below runs unconfined"
        );
    }
}

/// `x86_64` phase 5: the scheduler, the other cores, the ladder, and a kernel thread.
#[cfg(target_arch = "x86_64")]
// A bench boot diverges into `bench::run()` part way through, so the kernel thread after it is
// deliberately unreachable there. And a bench, test or system-test build has no machine
// description, the one reader of `boot_info_pointer` here. `kernel_main` carried the first allow
// for the whole `x86_64` arm, and used the pointer elsewhere, until the arm was split into phases;
// this is the phase that needs both.
#[cfg_attr(feature = "bench", allow(unreachable_code))]
#[cfg_attr(
    any(test, feature = "system_tests", feature = "bench"),
    allow(unused_variables)
)]
fn x86_64_scheduler(boot_info_pointer: usize) {
    // **The scheduler** (milestone 161, roadmap item 4). Everything below this line is a
    // process rather than a program, which is the distinction item 3 stopped at.
    //
    // Interrupts are off here (the two measurement windows above turned them back off), which
    // is what `sched::init` needs: a timer tick landing mid-init would run the deferred
    // `schedule()` before the idle thread is registered and hit "nothing runnable and no idle
    // thread". The RISC-V boot masks them explicitly at this point for the same reason.
    sched::init();

    // Re-arm the local APIC timer and let it run for good. `timer::init` rewrites the LVT
    // unmasked, which is what undoes the `mask_timer` the PIT window needed so its count would
    // be the PIT's alone. From here this timer is what preempts.
    arch::timer::init();
    arch::interrupts::enable();
    println!(
        "  scheduler   : up on 1 cpu, preempting at {} Hz (idle thread registered)",
        arch::timer::TICK_HZ,
    );

    // **SMP** (milestone 161's SMP item): INIT-SIPI-SIPI through the local APIC, same call and
    // same position as the other two boots. What it does *first*, whether or not any secondary
    // starts, is mark the boot core in `ONLINE_MASK`, and that is not optional: everything that
    // broadcasts (`online_cpus`, `nth_online`, the shootdown loops) reads that mask, so a kernel
    // that never called this has an empty online set while `online_count` says one, and the two
    // disagreeing is what `the_online_set_is_the_mask_and_the_sampler_stays_inside_it` caught on
    // the first x86 test run.
    smp::bring_up_secondaries();

    // **The ladder** (milestone 268): say what this machine is, then prove the kernel works on
    // it, then hand over. The third rung is what this architecture cannot reach yet: there is
    // no entry point that hands x86_64 to a `swish` prompt until DECISIONS §149 says how a
    // shell gets a console here and milestone 182 builds it, so this boot still ends by
    // halting a few hundred lines below. Everything under that rung runs before userspace
    // exists and waits on neither, which is why it lands now.
    //
    // The `bench` exclusion is on the functions rather than here; see the riscv64 site.
    #[cfg(not(any(test, feature = "system_tests", feature = "bench")))]
    {
        print_machine_description(boot_info_pointer);
        self_test::run();
    }

    // The benchmark boot (milestone 21, `script/bench`; DECISIONS §121's amendment measuring
    // the TSS I/O-bitmap switch cost): run the microbenchmarks and halt, instead of the rest of
    // the tour. Everything `bench::run()` needs is up by this line (the scheduler, `sched::spawn`,
    // the timer tick), and it diverges, so the kernel thread proof, the userspace demo and the
    // `#[cfg(test)]` suite below are untouched by it. The same shape the other two architectures
    // already use (see `icount`/`bench` above in this function's aarch64 half).
    #[cfg(feature = "bench")]
    bench::run();

    // A kernel thread, which is the cheapest proof that `kmem`, `untyped` and the context
    // switch all work on this architecture: its stack came from the kernel's own budget and
    // running it at all is one `switch_to` out and one back.
    {
        use core::sync::atomic::AtomicU64;
        static SAW: AtomicU64 = AtomicU64::new(0);
        let captured = 0x1610_0004u64;
        match sched::spawn(move || SAW.store(captured, core::sync::atomic::Ordering::SeqCst)) {
            Some(_) => {
                // **Yield until it runs, with a bound, rather than yielding a fixed number
                // of times and assuming that was enough.** The fixed count was 8, which held
                // for a year under emulation and reported `0x0  FAILED` on xenon's second
                // boot (2026-09-17): eight yields on four real 2.7 GHz cores is a far smaller
                // window than eight under QEMU, and the thread had simply not been picked up
                // yet. The count is printed because it is the diagnostic that tells a late
                // thread apart from one that never ran: a thread that never runs still
                // exhausts the bound and still reports FAILED, so this waits out a race
                // without hiding a hang.
                let mut spins = 0;
                while SAW.load(core::sync::atomic::Ordering::SeqCst) != captured && spins < 10_000 {
                    sched::yield_now();
                    spins += 1;
                }
                let saw = SAW.load(core::sync::atomic::Ordering::SeqCst);
                println!(
                    "  kernel task : a spawned thread ran and carried its captured state ({saw:#x}) after {spins} yield(s){}",
                    if saw == captured { "" } else { "  FAILED" },
                );
            }
            None => println!("  kernel task : FAILED: could not spawn a kernel thread"),
        }
    }
}

/// `x86_64` phase 6: userspace, the initial ramdisk and the boot file, or the test suite.
#[cfg(target_arch = "x86_64")]
fn x86_64_userspace() {
    // **Userspace**, which is the step this whole tour has been building toward: every line
    // above it is about the kernel talking to the machine, and this one is about the kernel
    // running a program and refusing it. Two hand-assembled children, each built out of its own
    // untyped region the way a real spawn builds one; see `user::x86_userspace_demo`.
    match user::x86_userspace_demo() {
        Ok(report) => {
            println!(
                "  userspace   : a process built from untyped ran at cpl 3 and sent {:#x} on a granted cap",
                report.reported,
            );
            println!(
                "                thread {} died at pc {:#x} on addr {:#x}, delivered to its supervisor",
                report.faulted_tid, report.fault_pc, report.fault_addr,
            );
            println!(
                "                two children cost {} frames the first round and {} the second{}",
                report.first_round_frames,
                report.second_round_frames,
                if report.second_round_frames == 0 {
                    " (steady state)"
                } else {
                    "  (LEAKED)"
                },
            );
        }
        Err(why) => println!("  userspace   : FAILED: {why}"),
    }

    // **The initial ramdisk** (milestone 161 (the x86_64 kernel port), item 4's hand-off). Not a
    // demo but a report, and the two facts it carries are the ones every `cfg(initial_ramdisk)`
    // test depends on: that the PVH module list reached the kernel at all, and that what it points
    // at parses as an archive. A boot where the first is true and the second is not looks identical
    // to a boot with no initial ramdisk from any test's point of view, which is why they are
    // printed apart.
    match user::initial_ramdisk() {
        None => println!("  initial ramdisk : none (no -initrd passed to QEMU)"),
        Some(image) => match nifefs::Fs::parse(image) {
            Ok(fs) => println!(
                "  initial ramdisk : {} bytes at {:#x}, {} programs, from the PVH module list",
                image.len(),
                memory::initial_ramdisk_region().unwrap().0,
                fs.len(),
            ),
            Err(e) => println!(
                "  initial ramdisk : {} bytes at {:#x}, but it does not parse: {e:?}",
                image.len(),
                memory::initial_ramdisk_region().unwrap().0,
            ),
        },
    }

    // **The boot file** (milestone 198 (a package manager, and the trivial install that makes a
    // second customer possible)'s rung 2a), printed for the same reason the initial ramdisk is: it
    // is the one fact that says whether this system can install itself, and a boot that cannot is
    // not broken. `uefi_loader` reads its own file back off the volume it was started from and
    // hands it over as the second PVH module; QEMU's `-kernel` path passes no such thing, so
    // "none" is the normal answer on most of this tree's boots.
    match memory::boot_file_region() {
        None => println!("  boot file   : none (this boot did not come from a file)"),
        Some((at, size)) => {
            println!("  boot file   : {size} bytes at {at:#x}, from the PVH module list");
        }
    }

    // A test build runs the kernel suite right here and exits via semihosting, instead of the
    // rest of the tour. Everything the tests need is now up: the frame allocator, the fine page
    // tables, the scheduler and its idle thread, the timer and interrupts. The x86 equivalent of
    // the `#[cfg(test)] test_main()` both other boots reach.
    #[cfg(any(test, feature = "system_tests"))]
    {
        run_test_suite();
        arch::halt(arch::HaltReason::test_build());
    }
}

/// The `x86_64` boot, one function per phase, ending at the hand-over (calef, 2026-10-09 (UTC)).
#[cfg(target_arch = "x86_64")]
pub(super) fn x86_64_boot(boot_info_pointer: usize) -> ! {
    let (info, acpi) = x86_64_first_light(boot_info_pointer);
    x86_64_pci(&acpi);
    x86_64_interrupts(boot_info_pointer, &acpi);
    x86_64_memory(&info, &acpi);
    x86_64_scheduler(boot_info_pointer);
    x86_64_userspace();

    // **The tour ends and the soak begins** (milestone 219), before the halting line rather
    // than after it: a boot that says it is halting and then does not would be the tool's
    // problem and the reader's.
    run_measurement_boot();
    // **Nothing halts by default** (milestone 268), on this architecture as on the other two:
    // the boot hands the machine to the progenitor, loaded from the archive and measured, and
    // the boot thread parks in a preemptible `wfi` loop so it gets scheduled. That is milestone
    // 182's entry point. What it cannot reach yet is a prompt, because a shell needs a console
    // and this one is port I/O; `x86_hand_over` says so in the transcript rather than leaving
    // a silent machine to be read as a hang.
    #[cfg(not(any(
        feature = "soak_test",
        feature = "job_mix",
        feature = "disk_throughput",
        feature = "network_bench"
    )))]
    {
        // **The install offer** (milestone 198 (a package manager, and the trivial install that
        // makes a second customer possible), rung 2a), and it has to be here rather than after
        // the handoff: it asks its question on the console, and the progenitor gives the
        // console's UART to a userspace input driver. It is a no-op on every boot that did not
        // come from a file the loader could read back, which is every boot but a UEFI one.
        user::install_service::offer();
        x86_hand_over();
        // **The boot thread leaves the scheduler rather than halting in it** (milestone 628
        // (provisional), measured 2026-10-03). It used to `arch::halt()` here, which is `hlt` in
        // a loop on a thread that is still runnable: every time round-robin reached it, the core
        // stopped until the next tick, up to 10 ms, with the shell, the console and the input
        // driver all ready behind it. x86_64's input driver polled and yielded then (no COM1
        // interrupt reached userspace until milestone 505), so the rotation reached it constantly, and the swish-check leg
        // paid about three seconds a line in ticks: 488 s for 118 lines under TCG on patagonia,
        // 126 s after this line, and the second boot's eight lines went from 23.8 s to 0.9 s.
        // `exit` marks it Finished and the idle thread, which halts only when nothing else can
        // run, takes the core. The boot thread owns no kernel stack (it runs on `boot.s`'s), so
        // reaping it frees nothing. notes/benchmarks/swish-check-x86-leg.md has the numbers.
        sched::exit();
    }
}

/// **Hand `x86_64` to the progenitor, and say how far it got** (milestone 182, inside milestone
/// 268's lane).
///
/// The same call `kernel_main`'s hand-over makes on aarch64 and riscv64, so the three architectures load, measure, endow and start
/// the first process through one body, and now reach the same place: the progenitor builds a
/// userspace console and a `swish` prompt announces the boot finished. Until milestone 299 this
/// architecture could not, because the console is port I/O (DECISIONS §121) with no page a driver
/// could map; §121's reversal gave x86 a `PortRange` capability, so the console and input drivers are
/// userspace processes holding COM1's ports, and the prompt above is transmitted by an `out` from
/// ring 3 like every other line the shell prints.
///
/// **The boot thread still watches, bounded, and reports what it saw**, because it is the boot
/// thread and has nothing else to do once the system is handed over: a progenitor still running when
/// the bound expires is the ordinary interactive outcome (the shell is up and waiting for input), and
/// a thread that left through a ring-3 fault left a record (`arch::exceptions::last_user_fault`) the
/// kernel's own fault report above it corroborates.
///
/// Name: provisional (milestone 182 (`x86_64`'s own interactive-boot entry point)).
#[cfg(target_arch = "x86_64")]
// Uncalled in the configurations that have nothing to hand over: a test or bench build parks before
// it, and a measurement build replaces it with its own workload.
#[cfg_attr(
    any(
        test,
        feature = "bench",
        feature = "soak_test",
        feature = "job_mix",
        feature = "disk_throughput",
        feature = "network_bench"
    ),
    allow(dead_code)
)]
fn x86_hand_over() {
    use core::sync::atomic::Ordering;

    let Some(initial_ramdisk) = user::initial_ramdisk() else {
        println!();
        println!("nife: no archive to hand the system to (run `cargo xtask initial-ramdisk-x86`).");
        return;
    };
    println!();
    println!("nife: handing the system to the userspace progenitor.");

    let faults_before = arch::exceptions::USER_FAULTS.load(Ordering::Acquire);
    let tid = match user::boot_progenitor(initial_ramdisk) {
        Ok(tid) => tid,
        Err(e) => {
            println!("  handoff FAILED: {e:?}");
            return;
        }
    };

    // Ten seconds of TCG is far past the progenitor's first console build, which is microseconds of
    // real time after it starts. The bound exists so the boot thread parks rather than waits.
    let deadline = arch::timer::now() + 10 * arch::timer::frequency();
    while sched::is_thread_present(tid) && arch::timer::now() < deadline {
        sched::yield_now();
    }

    let faults = arch::exceptions::USER_FAULTS
        .load(Ordering::Acquire)
        .wrapping_sub(faults_before);
    if sched::is_thread_present(tid) {
        println!(
            "nife x86_64: the progenitor is running at ring 3; {faults} of the processes it built \
             stopped on purpose."
        );
        // DECISIONS §121, reversed 2026-09-15, made the console server and the input driver
        // userspace processes holding a `PortRange` capability for COM1's ports (milestone 299), so
        // `swish`'s prompt above was transmitted by an `out` from ring 3, not by the kernel. A
        // holdover from §121's kernel-console era printed a "no prompt" line here; the prompt is now
        // the last thing above this, and `faults` above is zero because the two device drivers no
        // longer trap on first use.
        println!(
            "  prompt     : the shell above is served by userspace console and input drivers \
             holding COM1 as a port capability (milestone 299)."
        );
        return;
    }
    match arch::exceptions::last_user_fault() {
        Some((fault, addr)) if faults > 0 => println!(
            "nife x86_64: the progenitor ran at ring 3 and stopped ({fault:?} at {addr:#x}); \
             its fault report is above."
        ),
        _ => println!("nife x86_64: the progenitor left ring 3 without faulting."),
    }
}
