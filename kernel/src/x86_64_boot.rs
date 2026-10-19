//! **`x86_64`'s milestone tour**, and the watch its boot thread keeps after the hand-over.
//! `kernel_main` reaches the tour where aarch64 reaches `milestone_tour.rs` and riscv64 reaches
//! `riscv64_boot.rs`, and the boot around both is `boot.rs`'s on all three. See notes/x86-port.md.
//!
//! **Temporary.** calef ruled so on 2026-10-12 (UTC), in §284 (the kernel crate root splits into
//! child modules, one per boot): this file is the port's route until it joins the shared path in
//! `boot.rs`. What is left is the tour and the watch, and whether the tours merge is the open
//! question in design/roadmap/0890-riscv64-and-x86-64-join-the-shared-boot-path.md.
//!
//! Name: provisional (milestone 876's lane, 2026-10-12 UTC).

use super::*;

/// **`x86_64`'s milestone tour**: the demonstrations between the self-test and the hand-over, the
/// twin of aarch64's in `milestone_tour.rs` and riscv64's in `riscv64_boot.rs`, called from the same
/// line of `kernel_main`.
///
/// Every step is a report or a demonstration rather than bring-up: the fine map's image bounds, the
/// hardware entropy source, a kernel thread, two ring-3 processes built from untyped, and what the
/// loader handed over beside the kernel.
// The `shell`, `bench` and test builds never call it. Allowed rather than `cfg`-ed out, for
// riscv64's reason: the tour and the demonstrations beneath it still compile, and are linted, in
// every configuration.
#[cfg_attr(
    any(test, feature = "system_tests", feature = "bench", feature = "shell"),
    allow(dead_code)
)]
pub(super) fn milestone_tour() {
    x86_64_image_and_entropy();
    x86_64_kernel_task();
    x86_64_userspace();
    x86_64_handoff_report();
}

/// The kernel image's bounds under the fine map, and whether this core has a hardware entropy
/// source.
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

/// A kernel thread, which is the cheapest proof that `kmem`, `untyped` and the context switch all
/// work on this architecture: its stack came from the kernel's own budget and running it at all is
/// one `switch_to` out and one back.
fn x86_64_kernel_task() {
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

/// Two ring-3 processes built from untyped, one reporting and one faulting, and what they cost.
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
}

/// What the loader handed over beside the kernel: the initial ramdisk and the boot file.
fn x86_64_handoff_report() {
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
}

/// **Watch the progenitor, bounded, and say what it saw** (milestone 182 (`x86_64`'s own
/// interactive-boot entry point)): `x86_64`'s half of `boot.rs`'s `hand_over`, which loads,
/// measures, endows and starts the first process through the same body on all three.
///
/// The boot thread has nothing else to do once the system is handed over. A progenitor still
/// running when the bound expires is the ordinary interactive outcome (the shell is up and waiting
/// for input), and a thread that left through a ring-3 fault left a record
/// (`arch::exceptions::last_user_fault`) the kernel's own fault report above it corroborates.
///
/// **The gates that type at this port's prompt wait for the line this prints**, which lands after
/// the prompt: `script/swish-check`'s `x86_64` leg (`xtask/src/swish_check/verdict.rs`) and the
/// install, rollback and confirm boots. Changing what it prints changes what they wait for.
///
/// Name: provisional (milestone 890 (riscv64 and `x86_64` join the shared boot path)'s lane, 2026-10-19 UTC).
#[cfg_attr(
    any(test, feature = "system_tests", feature = "bench"),
    allow(dead_code)
)]
pub(super) fn watch_the_progenitor(tid: crate::thread::ThreadId, faults_before: usize) {
    use core::sync::atomic::Ordering;

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
