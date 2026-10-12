//! `init_tests`: userspace init and the least-authority demo: init delegates an
//! interrupt, brings up the console, builds a driver and a running child from an ELF, and a process
//! spends a memory region without the kernel allocating.
//!
//! Name: provisional, minted 2026-10-12 (UTC) by the lane for milestone 873 (the catch-all
//! userspace test file is split by topic).

use core::sync::atomic::Ordering;

use super::test_programs::reap_bare;
use super::*;
use crate::arch::exceptions::USER_FAULTS;
use crate::sched;

/// The `least_authority_demo` program's ELF bytes (milestone 19f.2), a distinct binary in the archive, not a
/// role of the `hello` binary. `_start(x0, x1, x2)` reads its input in `x1` and needs no
/// role selector.
fn least_authority_demo_image() -> &'static [u8] {
    program("least_authority_demo").expect("no least_authority_demo program in the initrd archive")
}

/// **The shell's `run` mechanism: spawn a process, get its answer.** Milestone 10's core.
///
/// A `least_authority_demo` process is started at EL0 with an argument, computes `n*n`, reports the result on
/// an rendezvous it was handed, and exits. The whole lifecycle a shell drives when you type
/// `run n`, minus the interactive loop, which is exercised by the piped demo instead.
#[test_case]
fn a_spawned_least_authority_demo_computes_and_reports() {
    let region = crate::memory_region::create(1).expect("no region for the demo's report");
    let result = sched::create_rendezvous_from(region).expect("no rendezvous from region");
    let faults = USER_FAULTS.load(Ordering::Relaxed);

    let tid = sched::spawn(move || {
        run(
            least_authority_demo_image(), // its own binary now (19f.2), not a role of hello
            Spawn {
                arg0: 0, // no role selector; the input is in x1
                arg1: 9, // the least_authority_demo computes 9*9
                arg2: 0,
                grants: &[crate::cap::rendezvous_cap(
                    result,
                    crate::cap::Rights::WRITE,
                )],
                maps: &[],
            },
        )
    })
    .expect("spawn failed");

    let answer = sched::ipc_receive(result)[0];
    assert_eq!(
        answer, 81,
        "the spawned least_authority_demo computed the wrong answer"
    );
    assert_eq!(
        USER_FAULTS.load(Ordering::Relaxed),
        faults,
        "the least_authority_demo faulted instead of computing cleanly",
    );
    assert!(
        wait_for(|| !sched::is_thread_present(tid)),
        "the least_authority_demo never exited",
    );
    sched::reclaim_region(region).expect("the demo's report region would not reclaim");
}

/// **The kernel stops allocating.** Milestone 11's whole point, as one number.
///
/// We carve an untyped region, then a process maps page after page out of it until the region
/// is exhausted. The assertion that matters: the kernel's used-frame count **does not change
/// while the process allocates**, because every page comes from the untyped, not the kernel
/// allocator. A process cannot make the kernel allocate, so it cannot exhaust kernel memory;
/// it runs out of its own budget and stops, cleanly, with the kernel untouched.
#[test_case]
fn a_process_spends_memory_region_and_the_kernel_never_allocates() {
    let used = || crate::memory::stats().expect("no allocator").used;

    const PAGES: u64 = 24;
    let (region, report, demo) =
        memory_region_service::start(memory_region_service::depleter_image(), PAGES)
            .expect("could not create the untyped region");

    // The process sends a "ready" signal once it is fully loaded (its ELF and stack are
    // kernel-allocated, like any process). We measure the frame count THERE, so the window we
    // check contains only what it does next: map pages out of its untyped.
    sched::ipc_receive(report); // ready
    let baseline = used();
    let faults = USER_FAULTS.load(Ordering::Relaxed);

    let mapped = sched::ipc_receive(report)[0]; // the count, after it exhausted the untyped

    assert_eq!(
        used(),
        baseline,
        "the kernel allocated {} frames while a process mapped {mapped} pages: untyped is not \
         backing the process's memory",
        used() as i64 - baseline as i64,
    );
    assert!(mapped > 0, "the process mapped nothing");
    assert_eq!(
        USER_FAULTS.load(Ordering::Relaxed),
        faults,
        "the process faulted instead of exhausting its budget cleanly",
    );

    // And the untyped is genuinely spent: the process mapped until it ran dry.
    let (watermark, total) = crate::memory_region::usage(region).expect("region vanished");
    assert_eq!(total, PAGES);
    assert!(
        watermark >= mapped,
        "the process mapped {mapped} pages but the untyped only advanced {watermark}",
    );
    assert!(
        total - watermark < 4,
        "the untyped had {} pages left unspent; the process gave up early",
        total - watermark,
    );

    // Every measurement above is taken. The demo spins to hold the free-frame count still while
    // it is read, which is the reason it does not exit on its own, and also the reason it has to
    // be ended here: past this point it is a spinning thread with nothing left to hold.
    assert!(reap_bare(demo), "the untyped demo outlived its kill");
}

/// **Milestone 19d.2b: init delegates an interrupt to a driver it builds.** The last
/// delegatable device authority after endpoints and device MMIO: an interrupt capability.
/// init holds one for a test SGI (the kernel routed it), builds a child, hands it the Irq
/// cap, and starts it. The child blocks in the interrupt's WAIT; the test raises the SGI; the
/// interrupt is delivered as a message through the delegated capability, the child wakes and
/// reports. A hang would mean the interrupt never reached the init-built child, so a passing
/// test is the proof. Completes the "init delegates every authority kind" story the
/// interrupt-driven drivers (input, virtio) rest on.
///
/// **aarch64-only, because RISC-V has no second interrupt to raise.** It has no
/// software-generated interrupt a test can assert on itself at all (the SBI IPI arrives down the
/// software-interrupt arm and never reaches `irq_route`), so the only line it can raise by hand
/// is the console UART's own, which `spawn_hello` is already routing for the input driver init
/// builds. A twin would have to share that one source between init's UART capability and the
/// test's delegated one, and would then prove delivery through whichever route was bound last
/// rather than through the delegated capability, which is the entire claim. The *property*
/// (an interrupt arriving as a message through a delegated Irq cap) is proved on RISC-V by
/// `riscv_virtio_tests::a_userspace_driver_reads_a_file_from_a_virtio_disk`, which asserts
/// `ROUTED_IRQS` rises while a userspace driver waits on its own Irq cap, and by
/// `sched::tests::an_interrupt_becomes_a_message`. See notes/interrupts.md.
#[cfg(target_arch = "aarch64")]
#[test_case]
fn userspace_init_delegates_an_interrupt_to_a_child() {
    const IRQ_WORD: u64 = 0x1590;
    const INIT_IRQ_ROLE: u64 = 25;

    let report_region = crate::memory_region::create(1).expect("no region for init's report");
    let report =
        crate::sched::create_rendezvous_from(report_region).expect("no rendezvous from region");
    let init = spawn_hello(initrd().expect("no initrd"), INIT_IRQ_ROLE, report);

    // Raise the test interrupt. The rendezvous counts it if the child is not waiting yet (it is
    // still being built), and the child's WAIT drains that pending signal, so there is no race.
    crate::arch::irq::send_sgi(INIT_TEST_SGI, crate::cpu::id());

    let word = crate::sched::ipc_receive(report)[0];
    assert_eq!(
        word, IRQ_WORD,
        "the interrupt never reached the init-built child through the delegated Irq cap",
    );
    init.release_or_fail("an init test's building budget");
    crate::sched::reclaim_region(report_region).expect("init's report region would not reclaim");
}

/// **Milestone 19d.2b: userspace init brings up the real console server.** Past 19d.2a's
/// ID-read probe: init builds the *actual* print server as a child, wires it a request/reply
/// channel and a shared page and the UART, then plays the client, asking it to print a line.
/// The server prints to the real UART (visible in the QEMU log) and acks the length; init
/// reports that length. Receiving the exact message length proves a userspace-built console
/// works end to end: a driver init constructed, on a channel init created, driving hardware
/// init delegated. This is the shape the real boot path (19d.2c) uses.
#[test_case]
fn userspace_init_brings_up_the_console_server() {
    if crate::user::machine_has_no_device_page_for_the_console() {
        crate::testing::skip!(crate::user::NO_UART_PAGE);
    }
    // The message length the init_console role prints and the server acks. Kept in sync with
    // fixtures/src/hello.rs init_console (the b"..." there); a mismatch fails loudly, not silently.
    const MSG_LEN: u64 = 66;
    const INIT_CONSOLE_ROLE: u64 = 24;

    let report_region = crate::memory_region::create(1).expect("no region for init's report");
    let report =
        crate::sched::create_rendezvous_from(report_region).expect("no rendezvous from region");
    let init = spawn_hello(initrd().expect("no initrd"), INIT_CONSOLE_ROLE, report);

    let acked = crate::sched::ipc_receive(report)[0];
    assert_eq!(
        acked, MSG_LEN,
        "the init-built console server did not print-and-ack: {acked} bytes, expected {MSG_LEN}",
    );
    init.release_or_fail("an init test's building budget");
    crate::sched::reclaim_region(report_region).expect("init's report region would not reclaim");
}

/// **Milestone 19d.2: userspace init builds a device driver and hands it the hardware.**
/// The step beyond 19d.1: not just a child, but a child that touches a *device*. init holds a
/// UART **device capability** (a new delegatable authority to map MMIO device-typed), builds
/// a driver child, and maps the UART's registers into it. The child reads the PL011's
/// PrimeCell identification registers, whose value is the fixed `0xB105F00D` every real PL011
/// returns. Receiving that constant proves the whole chain: device access is a capability the
/// kernel minted and init delegated, `MAP_INTO` mapped it device-typed (not cached normal
/// memory, which would corrupt MMIO), and a userspace-init-built driver drove real hardware.
///
/// **aarch64-only, because the assertion is a PL011 register and RISC-V `virt` has no PL011.**
/// `0xB105F00D` in the PrimeCell identification registers is what makes this test exact rather
/// than "the read did not fault"; the NS16550 on the other machine has no equivalent constant to
/// name, and swapping in a virtio magic number would be a different test wearing this one's
/// name. Device delegation to a userspace driver *is* proved on RISC-V, by
/// `riscv_virtio_tests::a_userspace_driver_reads_a_file_from_a_virtio_disk`, which is a stronger
/// version of the same claim (device MMIO, a DMA region, and an interrupt, all delegated).
#[cfg(target_arch = "aarch64")]
#[test_case]
fn userspace_init_builds_a_driver_that_reads_real_hardware() {
    const PL011_PRIMECELL_ID: u64 = 0xB105_F00D;
    const INIT_DEV_ROLE: u64 = 23;

    let report_region = crate::memory_region::create(1).expect("no region for init's report");
    let report =
        crate::sched::create_rendezvous_from(report_region).expect("no rendezvous from region");
    let init = spawn_hello(initrd().expect("no initrd"), INIT_DEV_ROLE, report);

    let id = crate::sched::ipc_receive(report)[0];
    assert_eq!(
        id, PL011_PRIMECELL_ID,
        "the init-built driver did not read the PL011's id: device delegation or the              device-typed mapping is broken",
    );
    init.release_or_fail("an init test's building budget");
    crate::sched::reclaim_region(report_region).expect("init's report region would not reclaim");
}

/// **Milestone 19d: userspace init parses a real ELF and builds a running process from it.**
/// The kernel loads exactly one program, init (a role of the same binary), and hands it the
/// initrd mapped read-only plus a building budget and a report rendezvous. init parses that
/// ELF *in userspace* (the `elf` crate, no longer in the kernel's trusted core) and loads it
/// as a child through the granular verbs: retype an address space, copy each segment into
/// retyped frames and map them, retype and endow a TCB, configure, start. The child runs code
/// the kernel never parsed and reports the agreed word. Receiving it is the whole thesis of
/// milestone 19 working end to end: a verified kernel that runs a workload it did not load.
#[test_case]
fn userspace_init_parses_an_elf_and_builds_a_running_child() {
    const CHILD_WORD: u64 = 0xC0FFEE;
    const INIT_ROLE: u64 = 20;

    let report_region = crate::memory_region::create(1).expect("no region for init's report");
    let report =
        crate::sched::create_rendezvous_from(report_region).expect("no rendezvous from region");
    let init = spawn_hello(initrd().expect("no initrd"), INIT_ROLE, report);

    let word = crate::sched::ipc_receive(report)[0];
    assert_eq!(
        word, CHILD_WORD,
        "init did not build a running child from the ELF it parsed in userspace",
    );
    init.release_or_fail("an init test's building budget");
    crate::sched::reclaim_region(report_region).expect("init's report region would not reclaim");
}

/// **Milestone 19e: init builds a `least_authority_demo`, passes it an argument, and gets the answer back.**
/// Every child before this took only its role in `x0`. A `least_authority_demo` computes on an input, so 19e
/// widened `START` to carry three initial registers. init builds a `least_authority_demo`, starts it with the
/// input in `x1`, and the `least_authority_demo` squares it and reports. Receiving `n*n` (not `n`, not garbage)
/// proves the argument crossed `START` into a fresh EL0 thread's registers intact. This is the
/// mechanism a real spawn service runs on: a workload parameterized by data, not just identity.
#[test_case]
fn init_builds_the_demo_and_passes_it_an_argument() {
    const INIT_LEAST_AUTHORITY_DEMO_ROLE: u64 = 28;
    const WORKER_INPUT: u64 = 7;

    let report_region = crate::memory_region::create(1).expect("no region for init's report");
    let report =
        crate::sched::create_rendezvous_from(report_region).expect("no rendezvous from region");
    let init = spawn_hello(
        initrd().expect("no initrd"),
        INIT_LEAST_AUTHORITY_DEMO_ROLE,
        report,
    );

    let answer = crate::sched::ipc_receive(report)[0];
    assert_eq!(
        answer,
        WORKER_INPUT * WORKER_INPUT,
        "the least_authority_demo did not receive its START argument: expected n*n back",
    );
    init.release_or_fail("an init test's building budget");
    crate::sched::reclaim_region(report_region).expect("init's report region would not reclaim");
}
