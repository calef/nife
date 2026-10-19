//! **The kernel's Rust entry point, and the steps all three boots share.**
//!
//! `kernel_main` is the function `_start` calls on every architecture. On `x86_64` it hands off
//! to that architecture's own boot (`x86_64_boot.rs`) before anything is brought up. aarch64 and
//! riscv64 both go through [`bring_up_machine`] and everything after it here: the test suite, the
//! measurement boots, the ladder, each port's milestone tour and the hand-over. The boot pointer it
//! stashes ([`DTB`]) is here because all three boots reach it. Milestone 890 (riscv64 and `x86_64`
//! join the shared boot path) tracks retiring the per-architecture boot files (§284 (the kernel
//! crate root splits into child modules, one per boot)).
//!
//! Moved out of the crate root by milestone 876 (the kernel crate root is split along its seams),
//! with the bodies unchanged. `kernel_main` had been split by boot phase first, calef's ruling of
//! 2026-10-09 (UTC) in design/roadmap/proposals/a-ratchet-on-function-length.md.
//!
//! Name: provisional (milestone 876's lane, 2026-10-12 UTC).

use super::*;

/// The physical address of the Device Tree Blob, as handed to us in `x0`.
///
/// Stashed here so the tests can assert that the boot protocol actually delivered one,
/// which is the whole point of shipping a flat arm64 Image instead of an ELF.
/// See notes/boot-protocol.md.
pub static DTB: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// **The boot device tree, parsed**, or the parse error if this machine did not hand us one.
///
/// **This exists so the "is that pointer real" argument is made once.** Five places used to read
/// [`DTB`] (or take the same value as an argument) and hand it to
/// `device_tree_blob::DeviceTreeBlob::from_ptr` under a hand-written `// SAFETY:` comment, each
/// rewording the same two facts: the pointer is the one firmware put in `x0`/`a1` and
/// [`kernel_main`] stashed here before anything else ran, and it is physical, so it is named
/// through the direct map. That is one fact about this module's own static, and this module is the
/// only place it can be checked. Milestone 139 round 8, and DECISIONS §94's rule about a body
/// copied verbatim into every caller.
///
/// Returns `Err` rather than panicking, because two of the callers legitimately continue without a
/// tree: the early RISC-V console keeps its defaults, and `x86_64` stores a PVH `hvm_start_info`
/// pointer in [`DTB`] rather than an FDT, so the magic check inside `from_ptr` is what tells those
/// callers apart from a real failure. Callers that cannot continue keep their own `expect`.
pub fn device_tree() -> Result<device_tree_blob::DeviceTreeBlob<'static>, device_tree_blob::Error> {
    let phys = DTB.load(core::sync::atomic::Ordering::Relaxed) as u64;
    // SAFETY: `kernel_main` stores the boot pointer here as its first statement, before any of
    // this function's callers can run, and firmware's blob stays where it is for the life of the
    // kernel, which is what `'static` claims. The value is physical (the boot protocol speaks in
    // physical addresses and we are running virtual), so the direct map names it. `from_ptr`
    // re-checks the magic before trusting anything else in the blob, which is what makes a wrong
    // pointer survivable rather than fatal.
    unsafe {
        device_tree_blob::DeviceTreeBlob::from_ptr(arch::mmu::phys_to_virt(phys) as *const u8)
    }
}

/// The kernel's Rust entry point, called from `_start` once we have a stack and a
/// zeroed `.bss`.
///
/// `extern "C"` matters: it tells Rust to follow the aarch64 calling convention
/// (AAPCS64), because assembly is about to call this and the two need to agree on
/// where arguments live. `boot_info_pointer` arrives in `x0`. See notes/registers.md.
///
/// `-> !` means this never returns, which is true: there is nowhere to return *to*.
// On `x86_64`, that port's own boot below diverges, so the rest of `kernel_main` is deliberately
// unreachable there. Scoped to it so aarch64 and riscv64 keep the lint.
// And the icount boot parks before the bench boot it implies, so `bench::run()` and everything after
// it is deliberately unreachable in that one configuration (milestone 78).
// And riscv64's `storage_bench` boot diverges in `run_measurement_boot`, leaving the hand-over after
// it unreachable but compiled, which is what keeps the hand-over's callees linted in that build.
#[cfg_attr(
    any(target_arch = "x86_64", feature = "icount", feature = "storage_bench"),
    allow(unreachable_code)
)]
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(boot_info_pointer: usize) -> ! {
    DTB.store(boot_info_pointer, core::sync::atomic::Ordering::Relaxed);

    // Per-CPU pointer FIRST, before anything takes a lock. The lock path reads this core's
    // held-rank out of its per-CPU block, so `TPIDR_EL1` must point at that block before
    // `console::init` (the first lock) runs. On one core this is pure setup with no visible
    // effect; it is the foundation SMP is built on. See cpu.rs and DECISIONS §11.
    cpu::init_this_cpu(arch::boot_cpu_id());

    // Console first, exceptions second, and the order is not arbitrary: the fault
    // handler's entire job is to print, so it is useless until the UART works. The
    // window between these two lines is the last place in the kernel where a fault
    // still kills us silently.
    console::init();

    // The console's register-block *shape* comes from the device tree, and it must be adopted
    // before the first println: on the JH7110 the DW-8250's registers are four bytes apart, so a
    // byte-strided LSR poll spins on garbage forever and the banner never appears
    // (notes/visionfive2.md; console::configure_from_dtb). On QEMU the tree restates the defaults
    // and this is a no-op in effect.
    #[cfg(target_arch = "riscv64")]
    console::configure_from_dtb();

    // **The screen, before the first word** (milestone 243), on the two architectures whose boot
    // chain never lights one. The x86_64 arm below does the same thing a hundred lines down with
    // the same comment and a different discovery: there the loader measured a UEFI aperture, here
    // the kernel asks `fw_cfg` for a `ramfb` and supplies the memory itself. Both end at
    // `console::attach_screen`.
    //
    // It is deliberately here, before the banner: everything after this line is on a monitor and
    // everything before it is not, so the earliest possible line is the one worth buying. It needs
    // nothing brought up first beyond the console lock and the device tree pointer, and it must run
    // before `arch::mmu::init`, which it does by a wide margin. See `screen::attach`.
    //
    // The line itself is printed with the rest of the tour rather than here, for the same reason
    // the x86 arm gives: a line describing the screen has to be printed by a console that already
    // has one in order to appear on it.
    #[cfg(not(target_arch = "x86_64"))]
    let screen = screen::attach();

    // **The earliest line, and every architecture now has one** (milestone 268).
    //
    // riscv64 and x86_64 have opened with `nife on <arch>` since their ports were written, as the
    // first thing after the console comes up: it is the one line that says the console works and
    // says nothing else, which is exactly the claim wanted at first light on a board. aarch64 had
    // no such line at all, so `board_console`'s `Stage::Banner` (which matches `nife on `,
    // deliberately generic so that a healthy aarch64 or x86_64 board would not read as never having
    // booted) was **unreachable on aarch64** for as long as the recogniser has existed. That is
    // milestone 268's finding 3 again, one rung lower, and it was found the same way: by looking
    // for the marker rather than by anything failing.
    //
    // Levelling up, not down: the other two keep their own opening lines, which say more than this
    // one because those architectures know more at this point (long mode is on, Sv39 is on). This
    // is what aarch64 can honestly claim here, which is that the console is up and the MMU is not.
    #[cfg(target_arch = "aarch64")]
    {
        use aarch64_cpu::registers::CurrentEL;
        use tock_registers::interfaces::Readable;
        println!();
        println!(
            "{}aarch64 (EL{}, MMU off: physical addresses until mmu::init)",
            boot_ladder::BANNER,
            CurrentEL.read(CurrentEL::EL),
        );
        screen::print_summary(&screen);
    }
    #[cfg(target_arch = "riscv64")]
    riscv64_banner(boot_info_pointer, &screen);

    // **The x86_64 boot still diverges here, before the shared bring-up below.** It brings up its
    // own traps, memory, interrupts and scheduler and ends at its own hand-over, because
    // `memory::init` reads a device tree and this machine hands over a PVH `hvm_start_info` and
    // ACPI instead (milestone 161 (the `x86_64` kernel port); notes/x86-port.md). Joining
    // `bring_up_machine` is the rest of milestone 890 (riscv64 and `x86_64` join the shared boot
    // path).
    #[cfg(target_arch = "x86_64")]
    x86_64_boot(boot_info_pointer);

    bring_up_machine(boot_info_pointer);

    #[cfg(any(test, feature = "system_tests"))]
    run_test_suite();

    // The instruction-count boot (milestone 78, `script/icount`): assert the two timing claims on
    // the deterministic clock and park. **Before the bench boot, and its feature implies that one**,
    // because the two park at the same point and therefore leave the same functions unreferenced;
    // riding on `bench`'s existing conditions is what keeps this from duplicating a dozen `cfg`s
    // across five files. The `bench::run` below is then unreachable, which is what this function's
    // `allow(unreachable_code)` names.
    #[cfg(feature = "icount")]
    icount::run();

    // The benchmark boot (milestone 21, `script/bench`): run the microbenchmarks and halt,
    // instead of the tour or the shell. Diverges, so everything below is untouched by it.
    #[cfg(feature = "bench")]
    bench::run();

    #[cfg(not(any(test, feature = "system_tests", feature = "bench")))]
    {
        print_machine_description(boot_info_pointer);

        // **Rung two of the ladder** (milestone 268): the kernel proving it works on the machine it
        // has just described, before anything is handed to userspace. The same five checks on all
        // three architectures, and the verdict line is what CI and `board_console` read. It
        // reports and does not gate: a failure prints and the boot carries on to the prompt,
        // because a machine you cannot log into is a machine you cannot fix.
        self_test::run();

        #[cfg(not(feature = "shell"))]
        milestone_tour();

        // **Milestone 19d.2c, completed at 28: userspace progenitor is the boot path.** The kernel stops
        // wiring services itself. It hands off to the progenitor, which brings up the console server, the line
        // discipline (`line_editor`, milestone 28), the input driver, and the shell out of its own budget
        // through the granular verbs. This is the line that retires the kernel as the system's
        // builder. Every interactive build on aarch64 and riscv64 reaches it: `--features shell`
        // hands off straight away and the milestone tour after running its demos. On a board the
        // prompt that follows is the signal that the boot finished (milestone 268), and a machine
        // that reaches one can be logged into and diagnosed rather than only power-cycled. The
        // legacy kernel-wired `user::shell_service` is retired as a boot path (it cannot host the
        // milestone-28 shell, which speaks the terminal contract, not the raw console protocol) and
        // is kept only as dead code for reference.
        // **The tour ends and the soak begins** (milestone 219), and it takes the place of the
        // progenitor handoff rather than of the halt below it. A soak boot that also brought up a
        // console, a line discipline and a shell would be soaking those too, and the point of this
        // workload is that what it stresses is decided rather than incidental.
        run_measurement_boot();

        #[cfg(not(any(
            feature = "soak_test",
            feature = "job_mix",
            feature = "disk_throughput",
            feature = "network_bench"
        )))]
        if let Some(image) = user::initial_ramdisk() {
            println!();
            println!("nife: handing the system to the userspace progenitor.");
            // `boot_progenitor` discovers the UART's interrupt line itself and prints it with its
            // source (10 on QEMU virt, 32 on the JH7110; notes/visionfive2.md, BUGS), so a bench
            // transcript names which source won.
            if let Err(e) = user::boot_progenitor(image) {
                println!("  handoff FAILED: {e:?}");
            }
            // The boot thread's work is done; the progenitor and the services it builds run until halt.
        } else {
            println!();
            println!("nife: no archive to hand the system to (no initial ramdisk was loaded).");
        }
    }

    // bench::run diverged above, and so does soak::run (milestone 219); this is everyone else's
    // ending. **The boot thread leaves the scheduler rather than halting on its run queue**
    // (milestone 720 (provisional)), as x86_64's has since milestone 628: a halted thread that is
    // still runnable stops the core until the next tick every time round robin reaches it. The
    // idle thread, which waits only when nothing else can run, takes the core. A boot whose
    // hand-over failed ends here too, and it ends the same way: nothing is left to run.
    #[cfg(not(any(
        feature = "bench",
        feature = "soak_test",
        feature = "job_mix",
        feature = "disk_throughput",
        feature = "network_bench"
    )))]
    sched::exit()
}

/// riscv64's opening lines: the banner, where the kernel is running, and the screen. aarch64's
/// are inline in [`kernel_main`]; these are a function because they say more, since this port has
/// Sv39 on and is running in the high half by the time it prints.
///
/// The bring-up's own demonstration lines (`traps`, `timer`, `memory`, `paging`) are not printed
/// here: `self_test::run` makes each of those claims on all three architectures as its
/// `exceptions`, `timer`, `frames` and `mapping` checks, and `board_console` reads the ladder.
#[cfg(target_arch = "riscv64")]
fn riscv64_banner(
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

/// The shared bring-up, which aarch64 and riscv64 reach: arch, memory, paging, IOMMU, interrupts,
/// cores.
///
/// **riscv64 runs it since milestone 890** (riscv64 and `x86_64` join the shared boot path). That
/// port's boot used to do every step here itself, in a different order: traps, then the timer and
/// interrupts on, then memory, paging and the scheduler. What it needs that aarch64 does not is
/// three calls into its own `arch/`, each `cfg`-gated beside the aarch64-only line that was already
/// here, and its `arch::irq::init`, which this function reaches through [`interrupts_init`].
fn bring_up_machine(boot_info_pointer: usize) {
    arch::init();
    stack::init();
    // Paint the boot stack's unused region for the high-water report (milestone 84), before
    // memory::init and everything after it can push frames into it.
    #[cfg(any(test, feature = "system_tests"))]
    stack::paint_boot_stack();

    // Now that faults are reportable, go find out how much RAM we actually have. A bug
    // in here is a fault, and a fault is now legible rather than fatal-and-silent.
    memory::init();

    // What machine is this (milestone 60). Three `mrs` reads, and the reason they happen HERE is
    // the line below: `mmu::init` takes `TCR_EL1.IPS` from this record and builds tables on a 4 KiB
    // granule this checks the part actually has. A machine that cannot run us says so and stops,
    // rather than turning the MMU on and faulting on the next instruction fetch with no console
    // line to explain it.
    arch::isa::init(boot_info_pointer);

    // Which cores does this machine have, and how are they started (milestone 100)? Both come out
    // of the device tree, and both have to be read HERE: `mmu::init` on the next line replaces the
    // coarse boot map, and the blob is only reachable through that one. The list used to be
    // `0..cpu::MAX_CPUS` and the PSCI conduit used to be `hvc`, compiled in; `isa::init` above took
    // the `/psci` half.
    smp::read_cpu_list();

    // Does the machine agree with its own firmware about how fast the counter runs (2026-09-21)?
    // Here for the same reason as the two reads above: the blob is only reachable through the
    // coarse boot map, which `mmu::init` on the next line replaces. `CNTFRQ_EL0` is firmware-set,
    // and a firmware that writes a wrong-but-plausible number is the one failure no later check can
    // see. Silent on every machine we test on, which states nothing to compare against; see
    // `arch::timer::check_frequency_against_device_tree` for what that costs and why it refuses
    // rather than preferring one source. riscv64 has no such function and needs none.
    #[cfg(target_arch = "aarch64")]
    arch::timer::check_frequency_against_device_tree(boot_info_pointer);

    // riscv64's two reads from the same window, for the same reason: the tree is only reachable
    // through the boot map. The counter's rate is a `/cpus` property, because RISC-V has no
    // `CNTFRQ_EL0` to read (milestone 100 (read the machine's PSCI and its CPU list)), and it must
    // be known before `pmu::init` below times itself and before `interrupts_init` arms the first
    // deadline. The PLIC's S-mode contexts are the tree's answer to a layout QEMU and the JH7110
    // disagree on, read before anything touches a context (`arch::irq::init_contexts`,
    // notes/visionfive2.md).
    #[cfg(target_arch = "riscv64")]
    {
        arch::timer::init_frequency(boot_info_pointer);
        arch::irq::init_contexts(boot_info_pointer);
    }

    // And now the sketchiest moment in the kernel. The instant SCTLR_EL1.M is set, the very
    // next instruction is fetched through the MMU. See arch/aarch64/mmu.rs.
    arch::mmu::init();

    // The per-CPU interrupt stacks go live here, and not one line earlier: their guard pages are
    // holes in the map `mmu::init` just installed, and the coarse boot map has no holes at all. A
    // trap before this point runs its handler on whatever stack it interrupted, which is what every
    // trap did before milestone 124. See kernel/src/interrupt_stack.rs.
    interrupt_stack::init();

    // riscv64 says what the machine is here rather than only in the machine description, so the
    // test, shell and bench boots report it too: it is the line whoever brings up a board reads
    // first. After `mmu::init`, because one of its numbers is measured there. Then it asks firmware
    // for a cycle counter (milestone 74 (cycle counters)), which reads the SBI extension set
    // `isa::init` filled in; aarch64 starts its counter per core inside `timer::init` instead.
    #[cfg(target_arch = "riscv64")]
    {
        arch::isa::print_summary();
        arch::pmu::init();
    }

    // The SMMUv3 (milestone 16b), if the machine was started with `iommu=smmuv3`. Its register
    // block is a device-tree node (memory::smmu_region), mapped by mmu::init just above; absent, the
    // kernel runs exactly as before. Bringing it up here installs an all-invalid stream table and
    // sets default-deny, so every PCIe stream aborts until virtio::register confines its device.
    // The CPU's own ECAM and BAR reads are not DMA, so PCI enumeration below is unaffected. See
    // kernel/src/iommu.rs, notes/iommu.md. Not compiled on x86_64, whose VT-d `init` takes the
    // DMAR's units rather than one base (milestone 594) and runs in the x86 arm above; the region
    // is never recorded there anyway.
    #[cfg(not(target_arch = "x86_64"))]
    if let Some((smmu_base, _)) = memory::smmu_region() {
        arch::iommu::init(smmu_base);
    }
    // The RISC-V IOMMU (milestone 16b (real hardware + IOMMU-backed driver isolation)), if QEMU
    // presents one (`-device riscv-iommu-pci`). It is a PCI function rather than a tree node, so
    // the kernel enumerates it and places its BAR first; absent, this is a no-op. Here for the
    // SMMUv3's reason: before any path (test, shell, tour) confines a virtio-pci device.
    #[cfg(target_arch = "riscv64")]
    pci::init_iommu();

    // The heap must come AFTER the MMU: it hands out addresses, and with paging on an
    // address is only usable if something has mapped it. From here, `Vec` works.

    // And now interrupts, which is where every lock in the kernel stops being a formality.
    //
    // The scheduler comes up FIRST, so that the very first timer tick already has somewhere to
    // send a reschedule. Adopting the boot context as thread 0 costs one allocation.
    sched::init();
    interrupts_init(boot_info_pointer);

    // Bring the other cores online. They come up idle: step 2 proves the bring-up path works
    // (PSCI, per-core stacks, the MMU replay), and leaves real multi-core scheduling to step 3.
    // Core 0 has IRQs on by now, so it keeps ticking while it waits for the others to check in.
    // See smp.rs and DECISIONS §11.
    smp::bring_up_secondaries();

    // Whether this machine has a running cycle counter (milestone 74's aarch64 half). Every core
    // started and checked its own in `timer::init`; this is the one line that reports them all,
    // here because it is the first point every core has answered. Every build prints it, test and
    // bench included, for the reason the riscv64 boot gives: whether the machine has one is a fact
    // about the machine, and the test and bench transcripts are where QEMU's answer gets read.
    arch::pmu::print_summary();
}

/// Bring up the interrupt controller and the timer, then **unmask interrupts**.
///
/// This is the line the whole locking discipline was written for. From here, a timer interrupt
/// can land between any two instructions in the kernel, and every `IrqSafeMutex` starts
/// actually masking something. See DECISIONS §9 and notes/locking.md.
fn interrupts_init(_dtb: usize) {
    use crate::arch::{interrupts, irq, timer};

    // Bring up the interrupt controller (the GIC on aarch64), reading its location from the device
    // tree. Portable code names `arch::irq`, never a specific controller, which is what lets a
    // second architecture be a new `arch/` directory rather than a diff here. On RISC-V this is the
    // PLIC, then the boot hart's software and external interrupt sources, which each secondary
    // unmasks for itself as it comes online.
    irq::init();

    timer::init();

    // The point of no return, in a much friendlier sense than the MMU's. After this, we are
    // preemptible.
    interrupts::enable();
}

#[cfg(all(feature = "system_tests", not(test)))]
unsafe extern "Rust" {
    /// The hook the system-test image defines (`system_tests/src/main.rs`). A kernel built with the
    /// `system_tests` feature calls it where a `cargo test` kernel calls `test_main`, so the same boot
    /// runs the other crate's suite.
    ///
    /// **A foot gun, and why it fails loudly rather than quietly.** Cargo unifies features across one
    /// invocation, so `cargo build --workspace` for a bare-metal target would build the kernel *binary*
    /// with this feature on. Nothing defines this symbol in that binary, so the link fails. That is the
    /// wanted outcome: a kernel that would run a test suite instead of booting never gets built by
    /// accident. `script/drift` builds `system_tests` separately for exactly this reason.
    fn system_tests_main();
}

/// Runs the suite this image carries: the kernel's own unit tests under `cargo test -p kernel`, or
/// the system tests when `system_tests/` links this library with its feature on.
#[cfg(any(test, feature = "system_tests"))]
pub(super) fn run_test_suite() {
    // **Every kernel image carries its trust root, a test image included** (milestone 609 (the
    // system tests leave the kernel crate)). `uefi_loader/build.rs` and `sealed_pair` tell which
    // archive a kernel belongs to by finding these digests in its bytes. The unit-test image never
    // enters the archive, so nothing in it calls `trust::verify`, and without this the linker drops
    // `TRUST_ROOT`: `uefi-test` then refused the kernel image as "NOT SEALED" against the archive
    // it was built with. Before the split, the system tests in the same image kept it.
    core::hint::black_box(trust::TRUST_ROOT);
    #[cfg(test)]
    test_main();
    // SAFETY: `system_tests_main` is a plain Rust function the linked system-test image defines,
    // with this exact signature; it takes nothing and returns when its suite has run.
    #[cfg(all(feature = "system_tests", not(test)))]
    unsafe {
        system_tests_main();
    }
}

/// The measurement boots, each diverging; with none built, the no-op below stands in for this and
/// the boot hands over.
///
/// **Two definitions, so the type says what the boot does.** Each `run` here is `-> !`, and a boot
/// that called them inline ended there as far as the compiler could see. Gathered into one
/// function, it can see that only if the function is `-> !` too, which is true only of a build
/// that has a measurement boot. So this one is built with a measurement feature and the no-op below
/// without, and that is what lets `kernel_main` end without its `sched::exit()` in a measurement
/// build, and `x86_64_boot` says `-> !`.
#[cfg(any(
    feature = "job_mix",
    feature = "soak_test",
    feature = "disk_throughput",
    feature = "network_bench",
    feature = "storage_bench"
))]
#[cfg_attr(
    any(test, feature = "system_tests", feature = "bench"),
    allow(dead_code)
)]
pub(super) fn run_measurement_boot() -> ! {
    // **The sweep, when this build asked for one** (milestone 168). Before the soak arm and
    // the halt for the same reason the soak sits before the halt: the tour has finished, so the
    // whole boot is still evidence, and this run ends by halting rather than by beating
    // forever. The two features are alternatives rather than a stack; `script/job-mix` builds
    // only this one.
    #[cfg(feature = "job_mix")]
    job_mix::run();
    #[cfg(feature = "soak_test")]
    soak::run();
    // **Fatal risk 6's bench boot, when this build asked for one** (milestone 261). The same
    // position and the same reason as the two above: the tour is evidence, and this replaces
    // the hand-over. It WRITES to the NVMe disk; see kernel/src/disk_throughput.rs.
    #[cfg(feature = "disk_throughput")]
    disk_throughput::run();
    // **Milestone 494 (a driver for the network card a PC actually has)'s bench boot**, in the
    // same position for the same reason; see kernel/src/network_bench.rs.
    #[cfg(feature = "network_bench")]
    network_bench::run();
    #[cfg(feature = "storage_bench")]
    storage_bench::run();
}

/// No measurement boot was built, so there is nothing to run, and the boot goes on to hand over.
#[cfg(not(any(
    feature = "job_mix",
    feature = "soak_test",
    feature = "disk_throughput",
    feature = "network_bench",
    feature = "storage_bench"
)))]
#[cfg_attr(
    any(test, feature = "system_tests", feature = "bench"),
    allow(dead_code)
)]
pub(super) fn run_measurement_boot() {}
