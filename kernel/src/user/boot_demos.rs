//! **The per-architecture demonstrations**: the `x86_64` userspace demo and the two riscv64 demos,
//! each gated on its own `target_arch`.
//!
//! Cut out of `user.rs` unchanged by milestone 872 (the kernel's user-mode file is split along its
//! seams), which left the parent an index.
//!
//! Name: provisional (milestone 872's lane, 2026-10-11 UTC; §281 (the kernel's user-mode file
//! splits into child modules of `user`)).

use super::*;

/// Where the x86 demo's children put their code and stack: the address-space map's image base and
/// top stack page, where the supervision fixtures put theirs on every architecture, so a reader who
/// has seen one recognises them.
#[cfg(target_arch = "x86_64")]
const X86_DEMO_CODE_VA: u64 = address_space_map::IMAGE_BASE;
#[cfg(target_arch = "x86_64")]
const X86_DEMO_STACK_VA: u64 = address_space_map::STACK_TOP_PAGE;
/// The word the reporting child SENDs, and the address the faulting one loads from. Both are
/// distinctive so that a zero anywhere in the report is visibly a failure rather than a plausible
/// value.
#[cfg(target_arch = "x86_64")]
const X86_DEMO_WORD: u32 = 0x0161_0004;
#[cfg(target_arch = "x86_64")]
const X86_DEMO_BAD_ADDR: u32 = 0x00A5_0000;

/// What the x86 userspace demo found. Every field is something a **program in ring 3** or the
/// **kernel's own supervision path** produced, rather than something the tour assumed.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy)]
pub struct X86UserspaceReport {
    /// The word the reporting child sent, as it arrived on the endpoint. Proves the child reached
    /// ring 3, made a `syscall` that reached the portable dispatcher, and was answered.
    pub reported: u64,
    /// The thread id the kernel stamped on the faulting child's death message.
    pub faulted_tid: u64,
    /// The pc the faulting child died at, from the message. `X86_DEMO_CODE_VA + 5` if the fault
    /// landed on the instruction it was supposed to.
    pub fault_pc: u64,
    /// The address it faulted on, from the message.
    pub fault_addr: u64,
    /// **What the first round of two children cost the frame allocator**, net of their regions
    /// being destroyed.
    pub first_round_frames: isize,
    /// **What an identical second round cost.** This is the number that means something, and the
    /// reason the demo runs twice: a first round pays first-use carves that are not leaks (the
    /// kernel's object budget, the endpoint region, a thread stack the recycler has not seen yet),
    /// and a system that has reached a steady state charges the second round **zero**. It is the
    /// same distinction `thread.rs`'s stack-VA reuse test draws, and the same evidence.
    pub second_round_frames: isize,
}

/// **Build one hand-assembled child out of `region` and start it.** The x86 boot tour's own
/// `build_child_in`, kept beside the demo rather than shared with `supervision_tests` because that
/// module is `#[cfg(test)]` and this runs on an ordinary boot.
///
/// `slot0` is the capability the program's own slot 0 will hold, if any; `fault_ep` goes in the
/// reserved fault slot, so `START` records it as this child's supervision endpoint.
#[cfg(target_arch = "x86_64")]
fn x86_build_child(
    region: u64,
    program: &[u32],
    slot0: Option<crate::cap::Cap>,
    fault_ep: Option<crate::sched::RendezvousId>,
) -> Result<u64, &'static str> {
    let aspace = user_address_space_create(region).ok_or("no aspace for the child")?;

    let code_phys = crate::memory_region::retype_page(region).ok_or("no code frame")?;
    // SAFETY: a fresh frame this region owns, reachable through the direct map; the program is
    // written there and then mapped executable. The kernel cannot address `X86_DEMO_CODE_VA`
    // itself, which is why the frame is written through its physical name instead.
    unsafe {
        let dst = mmu::phys_to_virt(code_phys) as *mut u32;
        for (i, &word) in program.iter().enumerate() {
            dst.add(i).write(word);
        }
    }
    // A no-op on this architecture (the instruction cache is architecturally coherent), and called
    // anyway because the seam is what the other two need and skipping it here would make this code
    // wrong to copy.
    sync_icache(
        mmu::phys_to_virt(code_phys),
        core::mem::size_of_val(program),
    );
    user_address_space_map(
        aspace,
        X86_DEMO_CODE_VA,
        code_phys,
        Flags::user_code(),
        crate::revoke::PageMapSource::NoCapability,
    )
    .map_err(|_| "could not map the child's code")?;

    let stack_phys = crate::memory_region::retype_page(region).ok_or("no stack frame")?;
    user_address_space_map(
        aspace,
        X86_DEMO_STACK_VA,
        stack_phys,
        Flags::user_data(),
        crate::revoke::PageMapSource::NoCapability,
    )
    .map_err(|_| "could not map the child's stack")?;

    let tid = crate::sched::create_thread_control_block(region).ok_or("no tcb")?;
    if let Some(first) = slot0 {
        let slot = crate::sched::thread_control_block_insert_cap(tid, first, None)
            .map_err(|_| "no room for the child's slot 0")?;
        if slot != 0 {
            return Err("the child's capability did not land in slot 0, which its code assumes");
        }
    }
    if let Some(ep) = fault_ep {
        // The spawn-slot convention: a supervision endpoint goes in the reserved fault slot, and
        // the kernel consumes it at START so the child cannot forge fault messages on it.
        let capability = crate::cap::rendezvous_cap(ep, crate::cap::Rights::READ);
        crate::sched::thread_control_block_insert_cap(
            tid,
            capability,
            Some(abi::fault::FAULT_EP_SLOT),
        )
        .map_err(|_| "no room for the fault endpoint")?;
    }
    crate::sched::configure_thread_control_block(
        tid,
        X86_DEMO_CODE_VA,
        X86_DEMO_STACK_VA + FRAME_SIZE,
        aspace,
    )
    .map_err(|_| "could not configure the child")?;
    crate::sched::start_thread_control_block(tid, [0; 3])
        .map_err(|_| "could not start the child")?;
    Ok(tid)
}

/// **Prove there is a userspace on `x86_64`**, which is the claim roadmap item 4 exists to make and
/// is a strictly larger one than item 3's ring-3 probe.
///
/// Two children, because the two halves of "a process" fail differently and a single program that
/// did both could hide one behind the other:
///
///   - **One reports and exits.** Its whole world (address space, code page, stack page, TCB) is
///     carved from one untyped region, it is dispatched to ring 3 by the *scheduler* rather than by
///     a hand-written entry path, it invokes a capability it was granted, and the word it SENDs
///     arrives here. That is the loader-shaped path minus the ELF: every kernel object a process
///     needs, built from a budget, in the order a real spawn builds them.
///   - **One faults.** It loads from an address nothing maps, the page tables refuse it, and the
///     trap path turns that into a supervision message naming the thread, the pc and the address.
///     Until this item the same arm recorded the fault and then panicked, because there was no
///     thread to kill.
///
/// And then both regions are destroyed and the frame count is compared, because a userspace that
/// leaks its processes is not one.
///
/// Name: provisional (milestone 161, roadmap item 4).
#[cfg(target_arch = "x86_64")]
pub fn x86_userspace_demo() -> Result<X86UserspaceReport, &'static str> {
    let before = crate::memory::free_page_frames();
    let round = x86_userspace_round()?;
    let after_first = crate::memory::free_page_frames();
    // The same two children again, from scratch. See `X86UserspaceReport::second_round_frames`.
    x86_userspace_round()?;
    let after_second = crate::memory::free_page_frames();

    Ok(X86UserspaceReport {
        first_round_frames: before as isize - after_first as isize,
        second_round_frames: after_first as isize - after_second as isize,
        ..round
    })
}

/// One round of the demo: build both children, collect what each produced, and give their regions
/// back. Called twice by [`x86_userspace_demo`], which is what turns its frame numbers into
/// evidence.
///
/// **Every kernel object a child needs comes out of that child's own region**, its two endpoints
/// included (`create_rendezvous_from`), so one `DESTROY` reclaims the whole of it and the frame
/// count is an exact statement rather than an approximate one. The first version drew the endpoints
/// from the kernel's shared pool and never collected the reporting child's corpse, and the tour
/// reported sixteen frames a round going missing: correct, and exactly the kind of thing a
/// steady-state number is for.
#[cfg(target_arch = "x86_64")]
fn x86_userspace_round() -> Result<X86UserspaceReport, &'static str> {
    use abi::fault::{EVENT_EXIT, EVENT_FAULT};

    // Sixteen pages is what the supervision fixtures give a child on the other two architectures:
    // an address space's root and tables, a code page, a stack page, a TCB, and here two endpoints.
    let report_region =
        crate::memory_region::create(16).ok_or("no region for the reporting child")?;
    let report_ep =
        crate::sched::create_rendezvous_from(report_region).ok_or("no reporting endpoint")?;
    let reporter_supervisor = crate::sched::create_rendezvous_from(report_region)
        .ok_or("no supervision endpoint for the reporting child")?;
    let reporter = x86_build_child(
        report_region,
        &x86_programs::report(X86_DEMO_WORD),
        Some(crate::cap::rendezvous_cap(
            report_ep,
            crate::cap::Rights::WRITE,
        )),
        Some(reporter_supervisor),
    )?;
    let reported = crate::sched::ipc_receive(report_ep)[0];

    // **Collect the corpse before reclaiming the region**, which is what a supervisor is for and
    // what the first draft of this left out: a region still holding a live TCB is refused, and the
    // refusal is silent because `destroy` has nowhere to report it.
    let exit = crate::sched::ipc_receive(reporter_supervisor);
    if exit[0] != EVENT_EXIT {
        return Err("the reporting child's clean exit did not arrive as an EXIT event");
    }
    crate::sched::reap_supervised(reporter_supervisor, reporter)
        .map_err(|_| "the reporting child's corpse refused to be reaped")?;

    // The faulting child, in a region of its own.
    let fault_region =
        crate::memory_region::create(16).ok_or("no region for the faulting child")?;
    let fault_ep = crate::sched::create_rendezvous_from(fault_region)
        .ok_or("no supervision endpoint for the faulting child")?;
    let child = x86_build_child(
        fault_region,
        &x86_programs::fault(X86_DEMO_BAD_ADDR),
        None,
        Some(fault_ep),
    )?;
    let msg = crate::sched::ipc_receive(fault_ep);
    if msg[0] != EVENT_FAULT {
        return Err("the child's death did not arrive as a FAULT event");
    }
    if msg[1] != child {
        return Err("the fault message named the wrong thread");
    }
    if msg[2] != X86_DEMO_CODE_VA + x86_programs::FAULT_PC_OFFSET {
        return Err("the faulting pc was not the load instruction");
    }
    if msg[3] != X86_DEMO_BAD_ADDR as u64 {
        return Err("the faulting address was not carried in the message");
    }
    crate::sched::reap_supervised(fault_ep, child)
        .map_err(|_| "the faulting child's corpse refused to be reaped")?;

    crate::memory_region::destroy(report_region);
    crate::memory_region::destroy(fault_region);

    Ok(X86UserspaceReport {
        reported,
        faulted_tid: msg[1],
        fault_pc: msg[2],
        fault_addr: msg[3],
        first_round_frames: 0,
        second_round_frames: 0,
    })
}

/// **Start the interrupt-driven UART driver as an unprivileged userspace process** (milestone 20).
///
/// The device-interrupt story's real form: a driver that owns the UART's interrupt by *capability*,
/// not by privilege. The kernel loads `driver` from the archive, builds its address space, maps the
/// NS16550's registers into it device-typed (so the driver reads the byte itself; the kernel is not
/// in the data path), and grants it exactly two capabilities: an `Irq` capability for the UART
/// interrupt (slot 0) and a report endpoint (slot 1). It routes the interrupt to the endpoint the
/// `Irq` cap waits on, starts the driver, and arms the source (PLIC), the receive interrupt (UART),
/// and supervisor external interrupts (`sie.SEIE`).
///
/// Returns the report endpoint. This does **not** block: the caller spawns a receiver so the boot
/// tour continues, and the driver's `WAIT`/read/report/`ACK` loop runs whenever a byte arrives. The
/// `ACK` is the point of the whole exercise: it crosses the `arch::irq` seam (the PLIC on RISC-V, the
/// GIC on aarch64) to re-arm the source, from an unprivileged process holding only a capability.
#[cfg(target_arch = "riscv64")]
pub fn riscv_uart_driver_demo(
    archive: &'static [u8],
    uart_irq: u32,
) -> Result<crate::sched::RendezvousId, LoadError> {
    const DRIVER_UART_VA: u64 = address_space_map::pair_page(0x0070_0000); // must match components/src/serial_driver.rs UART_VA
    const UART_PHYS: u64 = 0x1000_0000; // the NS16550 on QEMU virt

    let fs = nifefs::Fs::parse(archive).expect("initial ramdisk is not a nifefs archive");
    let driver_bytes = fs
        .read("serial_driver")
        .expect("archive has no 'serial_driver' program");
    let elf = Elf::parse(driver_bytes).map_err(LoadError::NotLoadable)?;

    // The driver's address space: its segments, a stack, and the UART's registers device-typed.
    let content: u64 = elf
        .segments()
        .map(|seg| {
            let (s, e) = seg.page_range(FRAME_SIZE);
            (e - s) / FRAME_SIZE
        })
        .sum::<u64>()
        + 1
        + PROGENITOR_STACK_PAGES
        + 8;
    let mut space =
        AddressSpace::new(content).ok_or(LoadError::Unmappable(MapError::OutOfPageFrames))?;
    map_segments(&mut space, &elf)?;
    for k in 0..PROGENITOR_STACK_PAGES {
        space
            .map_new(USER_STACK_VA - k * FRAME_SIZE, Flags::user_data())
            .map_err(LoadError::Unmappable)?;
    }
    // The UART registers, device-typed and user-accessible: the driver reads RBR/LSR directly.
    space
        .map_physical(
            DRIVER_UART_VA,
            UART_PHYS,
            Flags::user_device(),
            crate::revoke::PageMapSource::NoCapability,
        )
        .map_err(LoadError::Unmappable)?;

    let aspace_name = readopt_user_address_space(space).expect("register driver address space");

    // Route the UART interrupt to an endpoint; the Irq cap's WAIT blocks on it. The report endpoint
    // is where the driver SENDs each byte, and where the caller's receiver waits.
    let irq_ep = crate::sched::create_rendezvous();
    crate::sched::bind_irq(uart_irq, irq_ep);
    let report = crate::sched::create_rendezvous();

    let thread_control_block_region = crate::memory_region::create(2).expect("no tcb region");
    let tid =
        crate::sched::create_thread_control_block(thread_control_block_region).expect("no tcb");
    // slot 0: the Irq capability (READ permits WAIT/ACK). slot 1: the report endpoint (WRITE).
    let s0 = crate::sched::thread_control_block_insert_cap(
        tid,
        crate::cap::irq_cap_rights(uart_irq, crate::cap::Rights::READ),
        None,
    )
    .expect("insert irq cap");
    assert_eq!(s0, 0, "the Irq cap must land in slot 0");
    let s1 = crate::sched::thread_control_block_insert_cap(
        tid,
        crate::cap::rendezvous_cap(report, crate::cap::Rights::WRITE),
        None,
    )
    .expect("insert report");
    assert_eq!(s1, 1, "the report endpoint must land in slot 1");
    crate::sched::configure_thread_control_block(tid, elf.entry(), USER_STACK_TOP, aspace_name)
        .expect("configure");
    crate::sched::start_thread_control_block(tid, [0, 0, 0]).expect("start");

    // Arm the whole chain, now that the driver is running and routed: the source at the PLIC, the
    // receive interrupt at the UART, and supervisor external interrupts in `sie`.
    crate::drivers::plic::enable(uart_irq, crate::arch::irq::boot_s_context());
    crate::console::rx_enable();
    crate::arch::exceptions::enable_external();

    Ok(report)
}
