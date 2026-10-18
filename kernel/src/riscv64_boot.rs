//! **The riscv64 boot**: one function per phase, ending at the hand-over.
//!
//! What this port's boot does after the shared bring-up: the ladder, the measurement and test
//! boots, a tour of the capability core, and the hand-over, with the timing helpers only its entropy
//! step uses. See notes/riscv-port.md. Moved out of the crate root by milestone 876 (the kernel
//! crate root is split along its seams), bodies unchanged. Since milestone 890 (riscv64 and `x86_64`
//! join the shared boot path) the bring-up is `bring_up_machine`'s, in `boot.rs`, and all that is
//! left here before it is the banner.
//!
//! **Temporary.** calef ruled so on 2026-10-12 (UTC), in §284 (the kernel crate root splits into
//! child modules, one per boot): this file is the port's route until it joins the shared path in
//! `boot.rs`, and the parity work shrinks it toward the CPU bring-up that belongs under `arch/`. It
//! is not an architecture layer. The work is
//! design/roadmap/0890-riscv64-and-x86-64-join-the-shared-boot-path.md.
//!
//! Name: provisional (milestone 876's lane, 2026-10-12 UTC).

use super::*;

/// riscv64's opening lines: the banner, where the kernel is running, and the screen.
///
/// **All that is left of this boot's first phase** (milestone 890 (riscv64 and `x86_64` join the
/// shared boot path)). The rest of it was the bring-up, which `bring_up_machine` now does for this
/// port: traps, the timer's rate, memory, paging, the PLIC's contexts, the scheduler, interrupts and
/// the other harts. Its four demonstration lines went with it (`traps`, `timer`, `memory`,
/// `paging`), because `self_test::run` makes each of those claims on all three architectures as its
/// `exceptions`, `timer`, `frames` and `mapping` checks. Milestone 268 (every architecture boots the
/// same way) put them there, and retired the tour's own copies of two of them the same way. They are not read by `board_console`, which reads
/// the ladder's markers.
#[cfg(target_arch = "riscv64")]
pub(super) fn riscv64_banner(
    boot_info_pointer: usize,
    screen: &Result<(machine_discovery::framebuffer::Framebuffer, u32, u32), screen::Why>,
) {
    // A live code address proves we jumped to the high-half alias in boot.s and are executing
    // there, not merely reaching the UART through the identity map (both are mapped by the boot
    // table). If this reads 0xffffffc0_8020_xxxx, Sv39 is on and the kernel is in the high half.
    let pc = kernel_main as *const () as usize;
    println!();
    println!("{}RISC-V (rv64, S-mode, Sv39)", boot_ladder::BANNER);
    println!("  hart 0 booted: high-half kernel, .bss, and the NS16550 console are up.");
    println!("  running at  : {pc:#018x}  (high half: Sv39 paging is on)");
    println!("  device tree : {boot_info_pointer:#018x}");
    screen::print_summary(screen);
}

/// riscv64 phase 2: a process address space, a U-mode program, and a user ELF.
#[cfg(target_arch = "riscv64")]
fn riscv64_user_programs() {
    // User address spaces (the single-satp model): build a process address space, switch satp
    // to it, and keep running. That only survives if share_kernel_half copied the kernel high
    // half into the process root (otherwise this kernel code, at a high VA, unmaps itself on the
    // csrw satp). Then map and translate a user page in the live process space.
    {
        use paging::Flags;
        let aspace = user::AddressSpace::new(1).expect("no process aspace");
        let user_va = 0x40_0000u64;
        // Uninstalled again before the space drops: `while_installed` holds both halves.
        let mapped = aspace.while_installed(|| {
            let frame = memory::alloc().expect("no user frame").addr();
            arch::mmu::map_current_user_page_frame(user_va, frame, Flags::user_data(), || {
                memory::alloc().map(|f| f.addr())
            })
            .expect("user map failed");
            arch::mmu::translate_user(user_va)
        });
        println!(
            "  user address space : process satp activated (kernel half shared), user {user_va:#x} -> {mapped:x?}",
        );
    }

    // The capstone: run a real program at U-mode. The loader builds an address space from the
    // `outlaw` ELF's segments and drops to U-mode via enter_user's sret. The program yields
    // (round-tripping U-mode -> trap -> dispatch -> yield -> sret -> U-mode) twice, then exits.
    //
    // **This line used to print a syscall count, and the count proved nothing.** It read a
    // global counter before and after four yields of its own, but the program is placed on
    // whichever core the scheduler picks and had not run yet when the second read happened:
    // every captured riscv64 boot, under QEMU and on radon, printed "made 0 syscalls". The
    // counter itself was a `fetch_add` on one cache line from every syscall on every core,
    // which is what made the cheapest syscall cost more when more cores were busy
    // (notes/job-mix/null-syscall-under-load.md, 2026-10-04). It is gone rather than
    // per-core because this was its only reader. Proving the path for real means waiting for
    // the program's exit, which is a tour change and not this one.
    //
    // This step used to run a hand-assembled RISC-V blob through a one-page raw loader. It runs
    // a compiled ELF out of the initial ramdisk now (milestone 19 (run a real workload)'s user-test
    // port), so it needs one, and says so rather than quietly proving nothing when there is none.
    {
        match user::program("outlaw") {
            Some(image) => {
                sched::spawn(move || {
                    user::run(
                        image,
                        user::Spawn {
                            arg0: user::OUTLAW_ROUND_TRIP,
                            arg1: 0,
                            arg2: 0,
                            grants: &[],
                            maps: &[],
                        },
                    )
                });
                for _ in 0..4 {
                    sched::yield_now();
                }
                println!(
                    "  userspace   : a program was started at U-mode (yield/yield/exit via ecall)",
                );
            }
            None => {
                println!("  userspace   : skipped (no 'outlaw' program in the initial ramdisk)");
            }
        }
    }

    // The tour-stage breadcrumb (first-silicon diagnostics, 2026-08-15): every dump_threads
    // header prints the last stage reached, so a bench log whose serial lines went missing
    // (boots 7 through 9, notes/visionfive2.md fifth stop) still says how far the boot
    // thread got, re-stated every dump. The table, so a stage number reads without the code:
    //
    //   3 = the outlaw step finished        7 = the UART-driver step finished
    //   4 = the user-ELF step was entered   8 = the virtio probe finished
    //   5 = the user-ELF step returned      9 = the PCIe probe finished
    //   6 = the preemption step finished   10 = the hardware-entropy step finished
    //                                      11 = the banner printed; the tour is over
    //
    // **10 is not the end, and reading it as one is the mistake this table used to invite.**
    // It said `10 = the final banner printed; halting` and stopped there, which was true until
    // milestone 159 put the hardware-entropy step after what had been the last one and moved
    // the meaning down a row. 11 is the number that means finished, and it is the one the hang
    // watcher keys on (`user.rs`, `boot_stage() >= 11`), so a board log reporting 10 is a boot
    // that got as far as the entropy step and then stopped, not a boot that completed.
    sched::note_boot_stage(3);

    // Running a real compiled ELF at U-mode, on the boots that carry one to run.
    if let Some(initial_ramdisk) = user::initial_ramdisk() {
        // Entered for either shape of initial ramdisk, which it was not before milestone 295
        // (retire `components/src/builder.rs`): the breadcrumb used to fire only on the archive
        // arm, so a bare-ELF boot that died here reported the stage before it and read as having
        // died one step earlier than it did.
        sched::note_boot_stage(4);

        // **An archive initial ramdisk has no demonstration of its own here any more** (milestone
        // 295, calef's ruling of 2026-09-14). It used to have the best one on this architecture:
        // the kernel loaded `builder` out of the archive, granted it a budget and a report
        // endpoint and nothing else, and `builder` parsed `least_authority_demo` out of the
        // same archive **in userspace**, built it as a child from its own budget and started
        // it. The kernel never touched the child's bytes. That was milestone 20's proof that
        // userspace, not the kernel, composes a process, and the `init/build` line was it
        // announcing itself.
        //
        // **Milestone 268's item 4 is why it could go.** Nothing halts by default any more:
        // this same boot now ends in `riscv_hand_over`, where the progenitor composes the
        // console server, the line discipline, the input driver and `swish` out of its own
        // budget through the same granular verbs. That is the identical claim at a larger
        // scale, on the build `script/board-image` writes to a card, so the step below it was
        // making it twice.
        //
        // **What the retirement drops is the minimality half, and it is dropped knowingly.**
        // `builder` composed a process from **exactly two** capabilities; the progenitor is
        // granted the NS16550 and the UART's interrupt line as well, because it is building a
        // system rather than demonstrating a floor. See
        // design/roadmap/0295-retire-the-builder-program.md for where that claim went.
        //
        // **And the measured-boot refusal moved rather than went.** The archive used to be
        // checked against this kernel's trust root here (`trust::require("builder", ...)` and
        // the measurement table with it); it is now checked in `boot_progenitor` at the
        // handoff, which every default boot reaches. A card with the wrong archive still halts
        // with `MEASURED BOOT REFUSED`, later in the transcript than it used to.
        //
        // **A board reader who knew the `init/build` line wants `boot_ladder::PROMPT` now.**
        // That was the marker `crates/board_console`'s `Progress::userspace_ran` matched, and
        // no kernel prints it after this milestone. The live rung is `Stage::Prompt`, which
        // cannot appear unless userspace built the whole console stack, so it says more than
        // the line it replaces. `userspace_ran` itself stays, for the captured VisionFive 2
        // transcript that carries the old line and cannot be re-taken; its doc says so.
        let is_archive = nifefs::Fs::parse(initial_ramdisk)
            .map(|fs| fs.read(user::PROGENITOR_ENTRY).is_some())
            .unwrap_or(false);
        if is_archive {
            println!(
                "  user ELF    : an archive; the progenitor composes this machine's userspace at the handoff below"
            );
        } else {
            const N: u64 = 7;
            match user::riscv_least_authority_demo(initial_ramdisk, N) {
                Ok(sq) => println!(
                    "  user ELF    : loaded a {}-byte riscv ELF, ran least_authority_demo({N}) at U-mode, it sent {sq} (expected {})",
                    initial_ramdisk.len(),
                    N * N,
                ),
                Err(e) => println!("  user ELF    : load failed: {e:?}"),
            }
        }
    } else {
        println!("  user ELF    : skipped (no -initrd passed to QEMU)");
    }
    sched::note_boot_stage(5);
}

/// riscv64 phase 3: two threads that never yield are preempted anyway.
#[cfg(target_arch = "riscv64")]
fn riscv64_preemption() {
    // Preemption: the property that separates a kernel from a runtime. Spawn two threads whose
    // entire body is a tight loop, no yield, no syscall, not even a function call. Under any
    // cooperative scheduler the first one owns the CPU forever. The S-mode timer fires every
    // 10ms, riscv_trap_dispatch records the tick and defers a schedule() to the trap tail, and
    // both threads make progress anyway. This is the RISC-V half of DECISIONS §5: an arbitrary
    // binary that refuses to yield is preempted regardless. `STOP` lets them exit cleanly so
    // nothing lingers past the demo. Interrupts have been unmasked since `interrupts_init`, in the
    // shared bring-up.
    {
        use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
        static A: AtomicU64 = AtomicU64::new(0);
        static B: AtomicU64 = AtomicU64::new(0);
        static STOP: AtomicBool = AtomicBool::new(false);

        sched::spawn(|| {
            while !STOP.load(Ordering::Relaxed) {
                A.fetch_add(1, Ordering::Relaxed);
            }
        });
        sched::spawn(|| {
            while !STOP.load(Ordering::Relaxed) {
                B.fetch_add(1, Ordering::Relaxed);
            }
        });

        let p0 = sched::preemptions();
        arch::timer::spin_for(arch::timer::frequency() / 5); // ~0.2s, doing nothing but be preemptible
        STOP.store(true, Ordering::Relaxed);
        // Let the two spinners observe STOP and exit, so they are gone before we halt.
        for _ in 0..8 {
            sched::yield_now();
        }
        // The claim only prints when the numbers back it: both spinners made progress and at
        // least one preemption happened. The scheduler smoke line above earned the same
        // honesty the hard way (an unconditional success claim over a raced count).
        let (a, b, p) = (
            A.load(Ordering::Relaxed),
            B.load(Ordering::Relaxed),
            sched::preemptions() - p0,
        );
        if a > 0 && b > 0 && p > 0 {
            println!(
                "  preemption  : two never-yield threads ran {a} and {b} iterations, {p} preemptions (a thread that refuses to yield is preempted anyway)",
            );
        } else {
            println!(
                "  preemption  : FAILED: {a} and {b} iterations, {p} preemptions (a never-yield thread did not run, or nothing was preempted)",
            );
        }
    }
    sched::note_boot_stage(6);
}

/// riscv64 phase 4: a device interrupt serviced by an unprivileged userspace driver.
#[cfg(target_arch = "riscv64")]
fn riscv64_device_interrupt() {
    // Device interrupts, serviced by an unprivileged userspace driver: the last piece of the
    // interrupt story, in its real form. The kernel loads `driver` from the initial ramdisk, maps
    // the NS16550 into it device-typed, and grants it an Irq capability and a report endpoint. A
    // keystroke raises a line into the PLIC; the PLIC delivers a supervisor external interrupt;
    // riscv_trap_dispatch claims it, masks the source, and notifies the endpoint the Irq cap
    // waits on; the driver (a U-mode process owning no privilege) wakes, reads the byte through
    // its own device mapping, SENDs it, and ACKs, which re-arms the source through arch::irq. The
    // kernel is never in the data path. The `cargo run` harness is non-interactive, so pipe a
    // byte to QEMU's serial *after boot*: `( sleep 4; printf A ) | ...` (the console clears the
    // RX FIFO during init, so a byte sent at t=0 is dropped).
    if let Some((plic_phys, _)) = memory::plic_region() {
        // The UART's PLIC source, from the machine's own tree: 10 on QEMU virt, 32 on the
        // JH7110. This step used to arm a QEMU constant, and on the board that enabled an
        // unrelated source, so a real keystroke could never reach the driver; boot 13 proved
        // it on silicon (notes/visionfive2.md, BUGS). The fallback when the tree does not say
        // is that same constant (user::UART_RX_INTID), and the line below names which source
        // won, so a bench transcript is diagnosable.
        let (uart_irq, uart_irq_source) = user::uart_irq_and_source();
        // SAFETY: the PLIC is device-mapped in the direct map (mmu::map_everything); this is
        // its VA. The context is the boot hart's S context (2*hart + 1), derived rather than
        // hardcoded to 1 because OpenSBI elects the boot hart by lottery; see irq::boot_s_context.
        unsafe {
            drivers::plic::init(
                arch::mmu::phys_to_virt(plic_phys) as usize,
                arch::irq::boot_s_context(),
            );
        };

        println!("  uart irq    : source {uart_irq} ({uart_irq_source})");

        let started = user::initial_ramdisk()
            .filter(|a| {
                nifefs::Fs::parse(a)
                    .map(|fs| fs.read("serial_driver").is_some())
                    .unwrap_or(false)
            })
            .and_then(|a| user::riscv_uart_driver_demo(a, uart_irq).ok());
        match started {
            Some(report) => {
                // A receiver for the driver's reports, so the boot tour does not block on input.
                sched::spawn(move || {
                    let byte = sched::ipc_receive(report)[0] as u8;
                    println!(
                        "  device IRQ  : an unprivileged userspace driver serviced the UART via its Irq cap and sent {byte:#04x} ({:?})",
                        byte as char,
                    );
                });
                println!(
                    "  device IRQ  : userspace driver started (holds an Irq cap + a UART mapping); pipe a byte to see it"
                );
                // Give a byte already piped a turn to flow through the driver before the banner.
                for _ in 0..8 {
                    sched::yield_now();
                }
            }
            None => println!(
                "  device IRQ  : skipped (no 'driver' in the initial ramdisk; run `cargo xtask initial-ramdisk-riscv`)"
            ),
        }
    }
    sched::note_boot_stage(7);
}

/// riscv64 phase 5: virtio-mmio and PCIe block device discovery.
#[cfg(target_arch = "riscv64")]
fn riscv64_block_devices() {
    // virtio block device discovery (parity C). Probe the virtio-mmio slots the `virt` machine
    // lays out (0x1000_1000..); a block device shows up when a disk is attached (NIFE_DISK).
    // The kernel owns the transport and will hand a userspace driver the device's registers, an
    // Irq capability, and a DMA region; here we just prove the discovery works on RISC-V.
    match virtio::find_block_device() {
        Some(dev) => println!(
            "  virtio      : block device found at {:#x}, PLIC IRQ {} (kernel owns the transport)",
            dev.mmio_phys, dev.intid,
        ),
        None => {
            println!("  virtio      : no block device attached (pass NIFE_DISK to attach one)");
        }
    }
    sched::note_boot_stage(8);

    // PCIe enumeration + virtio-pci bring-up (the PCIe transport, P1/P2). The disk QEMU
    // attaches as `virtio-blk-pci` arrives over the transport real hardware uses: found by
    // walking ECAM config space, its BARs placed by the kernel (OpenSBI does no PCI setup),
    // its register blocks resolved through the virtio vendor capabilities, its INTx line
    // swizzled to a PLIC input.
    match pci::find_block_device() {
        Some(d) => println!(
            "  pcie        : virtio-blk at {:02x}:{:02x}.{}, common {:#x}, notify {:#x} (mult {}), isr {:#x}, PLIC IRQ {}",
            d.bdf.bus,
            d.bdf.dev,
            d.bdf.func,
            d.common,
            d.notify_base,
            d.notify_mult,
            d.isr,
            d.intid,
        ),
        None => {
            println!("  pcie        : no virtio-blk on the bus (pass NIFE_DISK to attach one)");
        }
    }
    sched::note_boot_stage(9);
}

// **The clock the `hw entropy` step is measured with**
// (design/roadmap/0306-time-the-hw-entropy-step.md, milestone 159's own follow-on).
//
// Read here, on the line after the `pcie` print, because the gap a bench session has been
// timing by eye is exactly `pcie` to `hw entropy`: those two lines are adjacent in the
// transcript, so the wall time between them is what a stopwatch at a serial console
// measures, and a person watching one resolves it to about a second. `design/fatal-risks/README.md`
// risk 6's third part is *at real speed*, and a bytes-per-second figure worth quoting needs
// the machine to time itself.
//
// **The span deliberately includes the `hw clock` step**, which prints between the two, so
// that the number printed below is the same quantity the stopwatch was measuring rather
// than a subset of it that happens to be tidier. The bring-up and draw costs are timed
// separately underneath it, and those are the two the rate question actually wants: a
// reseed plus a generation is a once-per-boot cost, and the round trips are the rate.
//
// `arch::timer::now()` is the mechanism the boot already uses (the preemption step above spins
// on it), so nothing new is introduced here; on riscv64 it is the `time` CSR, whose rate
// came out of this machine's device tree at `init_frequency`.
#[cfg(target_arch = "riscv64")]
fn riscv64_entropy() {
    let entropy_step_start = arch::timer::now();

    // **A real, non-virtio device, driven by a confined userspace process** (milestone 159,
    // design/roadmap/0159-jh7110-trng-driver.md; fatal risk 6 in design/fatal-risks/README.md). The
    // JH7110's TRNG is a register block on the `SoC`'s own fabric: no transport to negotiate,
    // no queue, no DMA. The kernel's whole part is the two lines below (ask the device tree
    // whether the device exists, then hand a userspace program one page of its registers and
    // two endpoints) and then playing a client over the same request endpoint any other client
    // would hold. The kernel never reads a `RAND` register.
    //
    // **The skip is the honest answer on every machine CI boots.** QEMU's riscv64 `virt` board
    // has no TRNG node under either spelling, so `jh7110_trng_device` returns `None` there and
    // this prints the skip rather than a claim. Only a `StarFive` VisionFive 2 (radon, the one
    // this project benches on) can print the other line, and the line only prints when bytes
    // actually arrived.
    //
    // **Both spellings are named in the skip because milestone 239 (radon's device tree does
    // not describe the TRNG, so a working driver never runs) was mis-diagnosed off the old
    // wording.** On 2026-09-03 this line said "describes no starfive,jh7110-trng" on a board
    // whose firmware tree does describe the device, under the vendor U-Boot's own
    // `starfive,trng`; the line was true and the conclusion drawn from it was not. A skip that
    // names everything it looked for cannot be read that way twice.
    // **The clock step announces itself even when there is nothing to do** (milestone 220).
    // On every machine this repository's CI boots there is no clock controller and no TRNG,
    // and a step that printed nothing there would leave a reader of the transcript to go and
    // check the source for whether it ran. The `Some` case is printed below, beside the
    // bring-up report, because the report is what makes the address worth reading.
    if user::entropy_service::jh7110_crg_window().is_none() {
        println!(
            "  hw clock    : skipped (this machine's tree names no JH7110 clock controller and no JH7110 TRNG, so no window was mapped and nothing was stored; QEMU virt has neither)"
        );
    }
    match user::entropy_service::jh7110_trng_device() {
        // **The skip, and a reference measurement beside it when this machine can give one.**
        //
        // The skip itself is the honest answer on every machine but radon and it has not
        // changed. What is new is the number after it, and the reason it is worth printing is
        // that the TRNG figure this step exists to produce is **not interpretable on its own**.
        // A draw through this service is one `entropy_contract` exchange per 8 bytes
        // (`MAX_BYTES`), so 32 bytes is four round trips through a userspace process, and a
        // measured microsecond count over 64 bytes mixes the device's cost with the IPC's
        // without saying in what proportion. The proposal names that as the thing to state
        // before the number leaves the machine: *does it count the IPC, the poll loop, the
        // process spawn?*
        //
        // The virtio-rng backend answers it, because it is **the same path with a different
        // device at the end**: the same `entropy_contract`, the same `Wiring::fill`, the same
        // eight round trips, the same confined userspace process holding the same two
        // rendezvous capabilities. Whatever it costs here is what the JH7110's number would
        // cost with a free device, so the difference between the two is the driver's.
        //
        // **It is a reference, not a measurement of hardware**, and the line says so in those
        // words: QEMU backs virtio-rng from the host's `/dev/urandom` and its cost is the
        // emulator's, not silicon's. `notes/benchmarks.md`'s standard is to say where a
        // comparison is not apples-to-apples, and this is where.
        //
        // Only when this machine actually has such a device, which on the default riscv64
        // boot it does not: `NIFE_RNG` is what attaches one (DECISIONS §120's QEMU-only
        // stopgap), and without it `ensure` returns `None` after the bus scan and the line
        // reads exactly as it did before plus its own microsecond count.
        None => riscv64_entropy_reference(entropy_step_start),
        Some(device) => riscv64_jh7110_entropy(device, entropy_step_start),
    }
    sched::note_boot_stage(10);
}

/// riscv64 phase 6, on a machine with no TRNG: the virtio-rng reference draw, when there is one.
#[cfg(target_arch = "riscv64")]
fn riscv64_entropy_reference(entropy_step_start: u64) {
    let reference = user::program("entropy").and_then(|image| {
        let w = user::entropy_service::ensure(image, user::entropy_service::Bus::Mmio)?;
        let report = w.wait_for_ready().unwrap_or([0; 5]);
        let ready_at = arch::timer::now();
        let (mut a, mut b) = ([0u8; 32], [0u8; 32]);
        let (na, nb) = (w.fill(&mut a), w.fill(&mut b));
        let drawn_at = arch::timer::now();
        Some((report[0], na + nb, ready_at, drawn_at))
    });
    match reference {
        Some((verdict, bytes, ready_at, drawn_at))
            if verdict == entropy_contract::READY && bytes > 0 =>
        {
            println!(
                "  hw entropy  : skipped (this machine's tree names no TRNG: neither starfive,jh7110-trng nor the vendor U-Boot's starfive,trng; QEMU virt has neither); since the pcie line {} us. Reference, NOT a TRNG and NOT hardware: this emulator's virtio-rng over the same entropy_contract path, bring-up {} us, {} bytes in {} us ({} bytes/s over {} round trips)",
                micros_between(entropy_step_start, arch::timer::now()),
                micros_between(entropy_step_start, ready_at),
                bytes,
                micros_between(ready_at, drawn_at),
                bytes_per_second(bytes as u64, drawn_at.wrapping_sub(ready_at)),
                bytes as u64 / entropy_contract::MAX_BYTES,
            );
        }
        // A device was there and the service did not come up, or came up dry. Said
        // rather than swallowed, because a reference that silently degrades to the
        // plain skip would make a machine with a broken virtio-rng look like one with
        // no virtio-rng at all.
        Some((verdict, bytes, _, _)) => println!(
            "  hw entropy  : skipped (this machine's tree names no TRNG: neither starfive,jh7110-trng nor the vendor U-Boot's starfive,trng; QEMU virt has neither); since the pcie line {} us. The virtio-rng reference did not run: report {:#x}, {} bytes drawn",
            micros_between(entropy_step_start, arch::timer::now()),
            verdict,
            bytes,
        ),
        None => println!(
            "  hw entropy  : skipped (this machine's tree names no TRNG: neither starfive,jh7110-trng nor the vendor U-Boot's starfive,trng; QEMU virt has neither); since the pcie line {} us, which is the device-tree query and nothing else (no virtio-rng on this machine either, so there is no reference draw to time; pass NIFE_RNG to attach one)",
            micros_between(entropy_step_start, arch::timer::now()),
        ),
    }
}

/// riscv64 phase 6, on a JH7110: wire the TRNG driver, draw twice, and time it.
#[cfg(target_arch = "riscv64")]
fn riscv64_jh7110_entropy(device: jh7110_entropy::Discovered, entropy_step_start: u64) {
    match user::program("jh7110_entropy") {
        None => println!(
            "  hw entropy  : JH7110 TRNG at {:#x}, but no 'jh7110_entropy' in the initial ramdisk (run `cargo xtask initial-ramdisk-riscv`)",
            device.reg_base,
        ),
        Some(image) => {
            // **The wiring is timed, and the console output between the segments is not**
            // (the proposal above). `ensure` maps the register window and spawns the
            // driver; the readiness wait is the driver's own bring-up (a reseed, then a
            // first generation). The `hw clock` line prints between the two, and a
            // `println!` here is a polled UART: on radon at 115200 baud that line is
            // roughly 30 ms of the kernel doing nothing but shift bits out, which is the
            // same order as the bring-up it would otherwise be added to. So the bring-up
            // figure is the sum of two measured segments rather than one span across
            // them, and the whole-gap figure beside it (`since the pcie line`) is the one
            // that still includes the console, because that is the quantity a stopwatch
            // at the serial port was measuring.
            let wire_start = arch::timer::now();
            match user::entropy_service::ensure(image, user::entropy_service::Bus::Jh7110) {
                None => println!(
                    "  hw entropy  : JH7110 TRNG at {:#x}, but the service would not wire",
                    device.reg_base,
                ),
                Some(w) => {
                    let wired_at = arch::timer::now();
                    riscv64_clock_report(w.clock);
                    // The driver's bring-up report next:
                    // `[READY, first_refill_ok, bytes_in_hand]`, or a 0xDEAD_.. word whose
                    // low byte names the step. A device that never answered says so here,
                    // and since 2026-09-04 so does one that answered with zeros:
                    // `entropy_contract::readiness` decides that word from the bytes, which
                    // is what the boot below found it was not doing.
                    let wait_start = arch::timer::now();
                    let report = w.wait_for_ready().unwrap_or([0; 5]);
                    let ready_at = arch::timer::now();
                    // Then two draws through the request endpoint, as a client. Two,
                    // because one proves only that *something* was returned: a stuck
                    // register file, a driver serving its buffer twice, and a device that
                    // never started all present as a repeat.
                    //
                    // **Each draw is four round trips, not one**, and that is the fix for
                    // the failure radon printed on 2026-09-04. `Wiring::get` is a single
                    // `entropy_contract` exchange, and that protocol carries `MAX_BYTES = 8`,
                    // so `get(32, ..)` returns 8 and can never return 32: the success line
                    // below was unreachable on any device, working or not. It went
                    // unnoticed for three days because this branch runs on exactly one
                    // machine in the world and QEMU takes the `skipped` arm.
                    //
                    // Filling all 32 bytes is also the stronger test, which is why the
                    // count stays 32 rather than dropping to 8. The driver's pool holds
                    // one 32-byte generation, so draw `a` empties it and draw `b` forces a
                    // second trip to the hardware. Two 8-byte draws would both have come
                    // out of the same buffer, and `a != b` would then prove only that the
                    // cursor advanced. Now it compares two device generations, which is
                    // what the line below claims.
                    let (mut a, mut b) = ([0u8; 32], [0u8; 32]);
                    let (na, nb) = (w.fill(&mut a), w.fill(&mut b));
                    let drawn_at = arch::timer::now();
                    // The two segments the console does not sit inside: the wiring (window
                    // mapped, driver spawned) and the readiness wait (the driver's reseed
                    // and first generation).
                    let bringup_ticks =
                        wired_at.wrapping_sub(wire_start) + ready_at.wrapping_sub(wait_start);
                    let draw_ticks = drawn_at.wrapping_sub(ready_at);
                    // The service refuses to report ready on an all-zero first bufferful
                    // now, so this is a client checking a claim rather than the only thing
                    // standing between a boot and zeros served as randomness. It stays
                    // because a tour that only repeated the service's own verdict would
                    // have caught nothing on 2026-09-04.
                    let zeros = a.iter().all(|&x| x == 0);
                    if report[0] == entropy_contract::READY
                        && na == 32
                        && nb == 32
                        && !zeros
                        && a != b
                    {
                        println!(
                            "  hw entropy  : JH7110 TRNG at {:#x} served 32+32 bytes to a client through a capability that names no device; first draw {:02x}{:02x}{:02x}{:02x}.., second differs; STAT after init {:#010x} ({}); since the pcie line {} us, of which bring-up {} us (window mapped, driver spawned, reseed, first generation; the hw clock line's own console time excluded) and {} bytes in {} us ({} bytes/s over {} round trips)",
                            device.reg_base,
                            a[0],
                            a[1],
                            a[2],
                            a[3],
                            (report[1] >> 32) as u32,
                            mode_note((report[1] >> 32) as u32),
                            micros_between(entropy_step_start, arch::timer::now()),
                            micros(bringup_ticks),
                            na + nb,
                            micros(draw_ticks),
                            bytes_per_second((na + nb) as u64, draw_ticks),
                            (na + nb) as u64 / entropy_contract::MAX_BYTES,
                        );
                    } else {
                        // `report[2]` is the driver's bring-up diagnostic and it is the
                        // number a bench session reads first. It is **always** the raw
                        // `(STAT << 32) | ISTAT` now, on success as well as on failure:
                        // it used to be the byte count when the report word said READY,
                        // and on 2026-09-04 that cost an hour, because the `0x20` printed
                        // here was read as an undocumented `ISTAT` bit 5 when it was the
                        // number 32 wearing a register's clothes. All zeros means the
                        // register window read as nothing at all (a gated clock, an
                        // undeasserted reset, or a base that is not the TRNG) rather than
                        // a device that answered wrongly. See components/src/jh7110_entropy.rs.
                        // The tree's own two words about this node come with the failure,
                        // not in a separate line, because they are what a bench session
                        // reads next: an all-zero diagnostic on a node the firmware calls
                        // `disabled` is a clock or reset the firmware had no reason to
                        // ungate (milestone 220's territory), while the same zeros on a
                        // node it calls `okay` are a different problem entirely.
                        println!(
                            "  hw entropy  : FAILED: JH7110 TRNG at {:#x} (tree says {}, status {}): report {:#x}, bring-up diagnostic {:#018x}, STAT after init {:#010x} ({}), draws {na}/{nb} bytes, first-all-zero {zeros}, draws-differ {}; since the pcie line {} us, of which bring-up {} us and draws {} us",
                            device.reg_base,
                            core::str::from_utf8(device.compatible).unwrap_or("?"),
                            if device.status_okay {
                                "okay"
                            } else {
                                "disabled"
                            },
                            report[0],
                            report[2],
                            (report[1] >> 32) as u32,
                            mode_note((report[1] >> 32) as u32),
                            a != b,
                            micros_between(entropy_step_start, arch::timer::now()),
                            micros(bringup_ticks),
                            micros(draw_ticks),
                        );
                    }
                }
            }
        }
    }
}

// **The clock and reset controller's own answer, first** (milestone
// 220), because it decides how the line below should be read. On
// 2026-09-04 radon printed an all-zero TRNG register file, and the
// two candidate explanations (a block nobody had powered, or a
// driver talking to the wrong address) are told apart here and
// nowhere else. The `before` words are what separate them again:
// clocks that were already enabled mean this milestone's premise was
// wrong and the zeros have some other cause.
//
// Printed even when it says nothing happened, because "no controller
// in this tree" is itself the finding on any machine but radon, and a
// silent step is one a bench reader has to go and check the source
// for.
#[cfg(target_arch = "riscv64")]
fn riscv64_clock_report(clock: Option<jh7110_clock_and_reset::Report>) {
    match clock {
        Some(report) => {
            let found = user::entropy_service::jh7110_crg_window();
            println!(
                "  hw clock    : JH7110 STG CRG at {:#x} ({}): clocks {:#010x},{:#010x} -> {:#010x},{:#010x} ({}); reset {} assert {:#010x} -> {:#010x}, status {:#010x} ({}, {} polls){}",
                found.map_or(0, |f| f.base),
                match found {
                    Some(f) if f.from_tree => "named by this machine's device tree",
                    Some(_) =>
                        "NOT named by this machine's tree: the constant both published trees agree on",
                    None => "address unknown",
                },
                report.clock_before[0],
                report.clock_before[1],
                report.clock_after[0],
                report.clock_after[1],
                if report.has_clocks_running() {
                    "running"
                } else {
                    "NOT running: the enable bit did not read back, so nothing is behind this window"
                },
                jh7110_clock_and_reset::STGRST_SEC_AHB,
                report.reset_assert_before,
                report.reset_assert_after,
                report.reset_status_after,
                if report.released {
                    "released"
                } else {
                    "STILL HELD"
                },
                report.polls,
                if report.was_already_up() {
                    "; the firmware had already done all of this, so a gated clock is NOT why the TRNG reads zeros"
                } else {
                    ""
                },
            );
        }
        // Unreachable in practice, and printed rather than ignored for
        // that reason: this arm is inside `Some(device)`, so the tree
        // named a TRNG, and `memory::init` records a window for exactly
        // that case. If it ever fires, the guard and the caller have
        // drifted apart and a bench transcript should say so.
        None => println!(
            "  hw clock    : no JH7110 clock-and-reset window was mapped even though this tree names a TRNG, so nothing was ungated"
        ),
    }
}

/// The riscv64 boot after `bring_up_machine`, one function per phase, ending at the hand-over
/// (calef, 2026-10-09 (UTC)).
#[cfg(target_arch = "riscv64")]
#[cfg_attr(
    any(
        feature = "icount",
        feature = "bench",
        feature = "shell",
        test,
        feature = "system_tests"
    ),
    allow(unreachable_code)
)]
// The machine description is the one reader of `boot_info_pointer` here, and a bench, test or
// system-test build has none; the bring-up that used to read it moved to `bring_up_machine`.
#[cfg_attr(
    any(test, feature = "system_tests", feature = "bench"),
    allow(unused_variables)
)]
pub(super) fn riscv64_boot(boot_info_pointer: usize) -> ! {
    // **The ladder** (milestone 268): say what this machine is, then prove the kernel works on
    // it, then hand over. Here, and not further down the tour, because every build that is not
    // a measurement build passes this line: the `shell` boot below hands to the progenitor from
    // here, and a board's boot is read from exactly these lines.
    //
    // The `bench` and `icount` boots park before userspace by design and are excluded on the
    // functions themselves rather than here (see `print_machine_description`'s own doc and
    // `kernel/Cargo.toml`'s feature comments: a board has no command line, so a measurement
    // build is compile-time or it is nothing). A `test` boot exits through semihosting a few
    // lines below and is excluded the same way.
    #[cfg(not(any(test, feature = "system_tests", feature = "bench")))]
    {
        print_machine_description(boot_info_pointer);
        self_test::run();
    }

    // The instruction-count boot (milestone 78, `script/icount`) diverges here, on the ISA whose
    // claim it was written for: SBI's `set_timer` is write-only, so this is the only place in
    // the tree that proves the firmware was armed with the deadline the kernel recorded. Before
    // the bench boot, whose feature it implies; see the aarch64 site for why.
    #[cfg(feature = "icount")]
    icount::run();

    // A bench build runs the primitive suite here and parks, instead of the tour. It needs the
    // `os_primitives_benchmarker` and `coremark` programs in the initial ramdisk (cargo xtask
    // initial-ramdisk-riscv packs them). The RISC-V equivalent of the aarch64 boot's `#[cfg(feature
    // = "bench")] bench::run()`.
    #[cfg(feature = "bench")]
    bench::run();

    // A `shell` build hands the machine to userspace progenitor here and parks, instead of the tour.
    // The kernel loads `system_initializer` from the initial ramdisk, grants it the NS16550 and the
    // UART interrupt, and it builds the console server, input driver, and shell out of its own
    // budget; the shell is interactive over the serial. This is the RISC-V equivalent of the
    // aarch64 `shell` path (named `initboot` there until milestone 296 (the `initboot` feature was
    // `shell`) found the two features identical). Needs the shell programs in the initial ramdisk
    // (cargo xtask initial-ramdisk-riscv packs them).
    #[cfg(feature = "shell")]
    {
        riscv_hand_over();
        // The boot thread's work is done, and it leaves the scheduler rather than parking on its
        // run queue; the progenitor and its children (console/input/shell) run from here on, and
        // the idle thread takes the hart when none of them can. Milestone 720 (provisional) has
        // why a parked boot thread costs a tick each time round robin reaches it; it is the
        // reason `arch::halt` takes a `HaltReason` this build cannot make.
        sched::exit();
    }

    // A test build runs the kernel suite right here and exits via semihosting, instead of the
    // demonstration tour below. Everything the tests need is now up: memory and the frame
    // allocator, the Sv39 paging, the scheduler and its idle thread, the timer, interrupts, and
    // the other harts. The RISC-V equivalent of the `#[cfg(test)] test_main()` on the aarch64 boot.
    #[cfg(any(test, feature = "system_tests"))]
    {
        // The PLIC too: the parity-C disk tests route a device interrupt, and `plic::enable`
        // through a never-initialized PLIC is a store through a zero base (a fault this test
        // path found the hard way; the shell and tour paths already did this). SEIE as well:
        // enabling a source at the PLIC delivers nothing while supervisor external interrupts
        // are masked in `sie`, and that bit is otherwise only set by the UART driver paths.
        if let Some((plic_phys, _)) = memory::plic_region() {
            // SAFETY: the PLIC is device-mapped in the direct map; the context is the boot
            // hart's S context (derived, not hardcoded: OpenSBI's hart lottery).
            unsafe {
                drivers::plic::init(
                    arch::mmu::phys_to_virt(plic_phys) as usize,
                    arch::irq::boot_s_context(),
                );
            };
            arch::exceptions::enable_external();
        }
        run_test_suite();
        arch::halt(arch::HaltReason::test_build());
    }

    // **The kernel mapping check and the context-switch check both moved into
    // `self_test::run`** (milestone 268, item 2).
    //
    // They were here, unnamed, as `kmap test` and `scheduler   : 2 of 2 ...`, and they were two
    // of the three self-tests this tree already had. Collecting them is the milestone's second
    // item: the same two checks now run on aarch64 and x86_64 as well, report through one
    // verdict line that CI reads, and are still measured the way this arm measured them (the
    // address computed from the machine's own RAM rather than a constant, the wait bounded by
    // the clock rather than by a yield count, both learned on the VisionFive 2).
    //
    // **Nothing was deleted to make three arms agree.** The claims went up into the ladder and
    // gained two architectures; that is what levelling up looks like, and it is the opposite of
    // the trap this milestone's block warns about.
    riscv64_user_programs();
    riscv64_preemption();
    riscv64_device_interrupt();
    riscv64_block_devices();
    riscv64_entropy();

    println!("{}RISC-V.", boot_ladder::TOUR);
    // 11, not 10: the hang watcher falls silent at "the tour has finished" and milestone 159
    // added a stage after what used to be the last one. See `user.rs`'s `boot_stage() >= 11`.
    sched::note_boot_stage(11);
    // **The tour ends and the soak begins** (milestone 219). After the tour's last line, so a
    // soak boot is a superset of an ordinary one and the whole boot is still evidence; before
    // the halt, because the halt is the thing it replaces.
    run_measurement_boot();
    // **Nothing halts by default** (milestone 268, item 4). The tour used to end here in
    // `arch::halt()`, and that was the right thing to do while the arch layer beneath the
    // shared path was still being built: there was nothing honest to fall through to. There is
    // now. `riscv_hand_over` is the same call the `shell` build makes a few hundred lines
    // above, so this architecture's default boot ends where aarch64's already did, at a
    // `swish` prompt, and **the prompt is the signal that the boot finished**.
    //
    // What this buys on a board is the whole reason calef decided it: a machine that reaches a
    // prompt can be logged into and diagnosed, and one that halted could only be power-cycled.
    // The VisionFive 2's card boots exactly this configuration (`script/board-image` builds
    // `--features board`, not `shell`), so it is the card that gains most.
    //
    // **The tour is untouched and still runs first.** Every demonstration above this line still
    // runs, still prints, and still ends with the line `board_console` calls `Stage::Tour`.
    // Those steps are the only thing in this tree proving the boot's middle stages on real
    // silicon (notes/visionfive2.md), so this adds a rung above them rather than removing any.
    //
    // `halt` afterwards, and it is not dead: the boot thread's own work is done and it parks in
    // a preemptible `wfi` loop so the progenitor and its children get scheduled. A boot with no
    // archive says so inside `riscv_hand_over` and parks the same way.
    #[cfg(not(any(
        feature = "soak_test",
        feature = "job_mix",
        feature = "disk_throughput",
        feature = "network_bench",
        feature = "storage_bench"
    )))]
    {
        riscv_hand_over();
        // Leaves the scheduler rather than parking in it, as the `shell` build's hand-over above
        // does (milestone 720 (provisional)).
        sched::exit();
    }
}

/// **Counter ticks as microseconds** (design/roadmap/0306-time-the-hw-entropy-step.md), for
/// the boot tour's `hw entropy` line.
///
/// Microseconds rather than milliseconds because the interesting half of the number is an IPC round
/// trip, and rather than raw ticks because a tick is a different amount of time on every machine
/// this runs on: QEMU's riscv64 `virt` counts at 10 MHz and the JH7110 at 4 MHz, so a transcript
/// quoting ticks could not be compared against another board's without the rate beside it. The rate
/// itself came out of this machine's own device tree (`arch::timer::init_frequency`), which is why
/// there is no constant here to go stale.
///
/// Truncating division, deliberately: this is an instrument, and a figure that rounded up would
/// report a nonzero duration for work that took no measurable time at all.
///
/// Provisional name (an architect's call): `micros`, with `micros_between` and `bytes_per_second`
/// beside it.
#[cfg(target_arch = "riscv64")]
#[allow(dead_code)]
fn micros(ticks: u64) -> u64 {
    // `frequency()` asserts the rate is nonzero rather than returning a poison value, and it has
    // been read from the tree since `bring_up_machine`, long before this tour, so there is no
    // pre-init case to guard here. `saturating_mul` bounds the scaling rather than the duration: a
    // gap long enough to overflow is one nobody is timing.
    ticks.saturating_mul(1_000_000) / arch::timer::frequency()
}

/// The two-timestamp form of [`micros`]. `wrapping_sub` because the `time` CSR is a free-running
/// 64-bit counter and this is the arithmetic the rest of this file already uses on it (see the
/// JH7110 entropy step above); at 10 MHz it wraps after about 58,000 years.
#[cfg(target_arch = "riscv64")]
#[allow(dead_code)]
fn micros_between(start: u64, end: u64) -> u64 {
    micros(end.wrapping_sub(start))
}

/// **Bytes per second over a measured span**, for the same line.
///
/// Computed from ticks rather than from the microsecond figure beside it, so the rate does not
/// inherit that figure's truncation. Zero for a span too short to measure, which is the honest
/// answer: a rate derived from a zero-length interval is not a large number, it is no number.
///
/// **What it counts, which has to be stated before it is quoted.** Everything between the readiness
/// report and the last byte landing: the `entropy_contract` round trips (one per
/// `entropy_contract::MAX_BYTES`, so four per 32-byte draw), the two context switches each one costs,
/// the driver's own poll loop, and the device. It does **not** count the process spawn or the
/// bring-up, which are the once-per-boot cost reported separately. It is therefore not comparable
/// to a Linux `hwrng` throughput figure, which is a read from an already-running kernel driver with
/// no IPC in it at all; `notes/benchmarks.md`'s rule is to say so where the number is printed.
#[cfg(target_arch = "riscv64")]
#[allow(dead_code)]
fn bytes_per_second(bytes: u64, ticks: u64) -> u64 {
    if ticks == 0 {
        return 0;
    }
    bytes.saturating_mul(arch::timer::frequency()) / ticks
}

/// **Say what the TRNG's `STAT` says about its output width** (milestone 159), for the boot tour's
/// `hw entropy` line.
///
/// This is the one bit that decides whether the 32 bytes that line reports are 32 bytes of device
/// output. In 128-bit mode only `RAND0..RAND3` carry a generation's answer, so a driver that
/// assembles all eight words is serving 16 real bytes and 16 of whatever the upper registers hold.
/// The driver writes `MODE.R256` during bring-up precisely because the width a given JH7110 resets
/// to is a build-time parameter of the silicon; `STAT.R256` is the read-back that says it took, and
/// nothing has ever read it on hardware.
///
/// An all-zero `STAT` is called out separately because it is not a mode report at all: it is the
/// signature of a register window that answered with nothing, which milestone 220's clock line
/// above is what explains.
// riscv64-only because the JH7110 is; `allow(dead_code)` because the boot tour that calls it is
// itself compiled out of the shell and bench builds, the same way `image_for_virtio` is.
#[cfg(target_arch = "riscv64")]
#[allow(dead_code)]
fn mode_note(stat: u32) -> &'static str {
    if stat == 0 {
        "the whole status register read zero, so this says nothing about the mode"
    } else if stat & jh7110_entropy::STAT_R256 != 0 {
        "256-bit: all eight RAND words are the answer"
    } else {
        "128-BIT: only RAND0..3 are the answer, so 16 of every 32 bytes are not device output"
    }
}

/// **Hand the machine to the userspace progenitor**, riscv64's half of the ladder's top rung
/// (milestone 268, item 4).
///
/// The kernel loads the progenitor from the initial ramdisk, grants it the NS16550 and the UART's
/// interrupt, and it builds the console server, the line discipline, the input driver and `swish`
/// out of its own budget through the granular verbs. This is the line that retires the kernel as
/// the system's builder on this architecture, and since milestone 295 it is the **only** place this
/// architecture makes that claim: **userspace, not the kernel, composes a process.**
/// `components/src/builder.rs` used to make it in miniature one step up the tour, from exactly two
/// capabilities, and calef retired it on 2026-09-14 on the ground that this carries it at a larger
/// scale on the same boot. The difference is that this one composes the whole running system rather
/// than one child, and a person can then type at it; what it does not carry is the minimality half,
/// which is why design/roadmap/0295-retire-the-builder-program.md exists and says where that went.
///
/// **Two callers, one body** (milestone 268). It was the inside of the `#[cfg(feature = "shell")]`
/// block and nothing else; since this milestone the *default* boot ends here too, so the two paths
/// cannot drift. A `shell` build reaches it early and parks; a default build runs the whole tour
/// first and then reaches it.
///
/// Excluded from `test` and `bench` builds for `print_machine_description`'s reasons: a test boot
/// exits through semihosting before the tour and a bench boot diverges into `bench::run`, so
/// neither has a system to hand over.
///
/// Name: provisional (milestone 268 (every architecture boots the same way)). This is the x86
/// caller of the shared `user::boot_progenitor` loader (milestone 166 (one boot loader, reached two
/// inconsistent ways)): it finds the initial ramdisk, hands it over, then watches the boot thread
/// bounded and reports how it left; `riscv_hand_over` is its riscv twin. calef names what a reader
/// meets.
#[cfg(target_arch = "riscv64")]
// A `soak` or `job_mix` build replaces the handoff with its own workload and never calls this, and
// a `test` or `bench` build parks before it; all four are boots with nothing to hand over. Allowed
// rather than `cfg`-ed out, so the function still compiles in every configuration: a handoff that
// only type-checks in the configurations that use it is one that rots in the others.
#[cfg_attr(
    any(
        test,
        feature = "bench",
        feature = "soak_test",
        feature = "job_mix",
        feature = "disk_throughput",
        feature = "network_bench",
        feature = "storage_bench"
    ),
    allow(dead_code)
)]
fn riscv_hand_over() {
    if let Some((plic_phys, _)) = memory::plic_region() {
        // SAFETY: the PLIC is device-mapped in the direct map; the context is the boot hart's S
        // context (derived, not hardcoded: OpenSBI's hart lottery).
        unsafe {
            drivers::plic::init(
                arch::mmu::phys_to_virt(plic_phys) as usize,
                arch::irq::boot_s_context(),
            );
        };
    }
    let Some(initial_ramdisk) = user::initial_ramdisk() else {
        println!();
        println!(
            "nife: no archive to hand the system to (run `cargo xtask initial-ramdisk-riscv`)."
        );
        return;
    };
    println!();
    println!("nife: handing the system to the userspace progenitor.");
    // `boot_progenitor` discovers the UART's interrupt line itself and prints it with its source
    // (10 on QEMU virt, 32 on the JH7110; notes/visionfive2.md, BUGS), so a bench transcript names
    // which source won.
    if let Err(e) = user::boot_progenitor(initial_ramdisk) {
        println!("  handoff FAILED: {e:?}");
    }
}
