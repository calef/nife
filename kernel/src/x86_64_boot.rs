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

/// `x86_64` phase 1: the screen, the banner and the traps.
#[cfg(target_arch = "x86_64")]
fn x86_64_first_light(boot_info_pointer: usize) {
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
}

/// `x86_64` phases 2 to 4: `bring_up_machine`'s steps, in its order, through this machine's seams.
#[cfg(target_arch = "x86_64")]
fn x86_64_bring_up(boot_info_pointer: usize) {
    arch::init();
    stack::init();
    #[cfg(any(test, feature = "system_tests"))]
    stack::paint_boot_stack();
    memory::init();
    arch::isa::init(boot_info_pointer);
    smp::read_cpu_list();
    arch::irq::init_contexts(boot_info_pointer);
    arch::timer::init_frequency(boot_info_pointer);
    arch::mmu::init();
    interrupt_stack::init();
    arch::isa::print_summary();
    arch::pmu::init();
    arch::discovery::bring_up_iommus();
    sched::init();
    arch::irq::init();
    arch::timer::init();
    arch::interrupts::enable();
    smp::bring_up_secondaries();
    arch::pmu::print_summary();
}

/// The kernel image's bounds under the fine map, and whether this core has a hardware entropy
/// source: the two lines of the bring-up's old memory phase that are reports rather than steps.
#[cfg(target_arch = "x86_64")]
fn x86_64_image_and_entropy() {
    println!(
        "  image       : text {:#x}..{:#x}, stack {:#x}..{:#x}",
        arch::mmu::text_start(),
        arch::mmu::text_end(),
        arch::mmu::stack_bottom(),
        arch::mmu::stack_top(),
    );

    // Milestone 162 (real hardware entropy on x86_64 and aarch64): RDSEED needs no ring 3, no capability, and nothing this port has not already
    // built, so it is provable here even though the entropy service itself runs in userspace.
    // `draw_random_seed` checks CPUID leaf 7 EBX.18 before ever executing the instruction.
    match arch::isa::draw_random_seed() {
        Some(v) => {
            println!("  entropy     : rdseed supported (cpuid leaf 7 ebx.18), drew {v:#018x}");
        }
        None if arch::isa::get().has_random_seed_instruction() => {
            println!("  entropy     : rdseed supported but stayed dry across every retry");
        }
        None => println!("  entropy     : rdseed not supported (cpuid leaf 7 ebx.18 clear)"),
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

    x86_64_image_and_entropy();

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
    x86_64_first_light(boot_info_pointer);
    x86_64_bring_up(boot_info_pointer);
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
