//! **Starting the secondary CPUs**: INIT-SIPI-SIPI, the busy-wait it needs, and the line every
//! boot prints about it.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// Start a secondary CPU via INIT-SIPI-SIPI (milestone 161's SMP item). `target_cpu` is a local
/// APIC id (the arch contract's "hardware id", `smp::bring_up_secondaries` reads it out of the
/// roster `smp::seat_cpus_from_acpi` built), `entry` is the trampoline's own physical page
/// (`ap_boot::trampoline_phys()`, handed back through `secondary_boot_entry`, see below), and
/// `context` is the stack top the trampoline hands to `secondary_main`.
///
/// **Blocks until the core it started is fully online, or has been given up on**, which is not
/// `smp::bring_up_secondaries`'s usual contract (aarch64 and RISC-V return as soon as the firmware
/// call is accepted) but is required here: there is exactly one trampoline scratch page, shared by
/// every `STARTUP` IPI this kernel ever sends, and `bring_up_secondaries` starts one core per call
/// with nothing else serializing them. Waiting for `online_count()` to move is what makes the next
/// call's `ap_boot::prepare` safe to overwrite that page.
///
/// # BUGS
/// **`entry` must equal `ap_boot::trampoline_phys()`.** The arch contract passes whatever
/// `smp::bring_up_secondaries` computed for `secondary_boot`'s address, which on this architecture
/// *is* that trampoline page (see `secondary_boot_entry`'s own doc), so this holds by construction
/// today; it is not re-derived here because there is nowhere else it could sensibly come from.
pub fn cpu_start(target_cpu: u64, entry: u64, context: u64) -> i64 {
    if !irq::is_local_apic_ready() {
        return -1;
    }
    debug_assert_eq!(
        entry,
        ap_boot::trampoline_phys(),
        "cpu_start's entry is not the AP trampoline's own page"
    );
    // SAFETY: this function does not return until the core it is about to start has either come
    // fully online or been given up on (see the wait loop below), which is what keeps two calls
    // from ever overwriting the shared trampoline page while an earlier one is still in use.
    unsafe { ap_boot::prepare(context) };

    let dest = target_cpu as u8;
    // **Read once, here, before the INIT, and compared against by every wait below.** This is the
    // whole fix for `ap_boot`'s BUGS #1. The wait loop used to re-read the count after the
    // STARTUP IPIs and wait for it to move from *that* value, so a core fast enough to check in
    // during the 200 us settle delays had already moved it: the loop then waited ten seconds for a
    // second increment nobody would make, returned -1, and the core was online but uncounted.
    let before = crate::smp::online_count();

    // The universal INIT-SIPI-SIPI startup algorithm (Intel MP spec, appendix B.4): INIT, a settle
    // delay, then a STARTUP IPI.
    irq::send_init(dest);
    busy_wait_us(10_000);
    let vector = (entry >> 12) as u8;
    irq::send_startup(dest, vector);
    busy_wait_us(200);

    // **The second STARTUP IPI is conditional, not unconditional.** The MP spec calls for two, "the
    // second is a no-op on a core that already started", meaning a core that has already left the
    // wait-for-SIPI state is defined to ignore a further one (QEMU's `apic_sipi` does exactly
    // that). Sending the second only when the first evidently has not worked yet costs nothing when
    // the first succeeds. It was added as a hypothesis for `ap_boot`'s BUGS #1 and was not the fix;
    // it is kept because it is no less correct than the unconditional form.
    if crate::smp::online_count() == before {
        irq::send_startup(dest, vector);
        busy_wait_us(200);
    }

    // Bounded: a core the firmware silently declines to start must not hang bring-up.
    //
    // **Ten seconds, not one.** The delays above are the real thing's timing, sub-millisecond on
    // real hardware; the budget below is not that, it is how long CPU 0 waits for the SIPI'd core to
    // run its own trampoline, adopt the fine map, and reach `secondary_main`'s online mark, all of
    // it QEMU TCG instructions on whatever host thread the emulator's own scheduler gets around to
    // next. Measured too short once already, at one second: the target core's own vCPU thread had
    // simply not been scheduled by the host yet, and `cpu_start` gave up and reported "did not
    // start" for a core that came up perfectly well a moment later, arriving too late to be counted
    // and permanently invisible to the roster. (That diagnosis predates `ap_boot`'s BUGS #1 being
    // root-caused, and "a core that came up perfectly well" but was not counted is exactly #1's
    // signature, so it may have been #1 rather than the budget; the ten seconds are kept because a
    // loaded host really can starve a vCPU thread that long.) This is exactly the host-contention shape
    // `smp::tests::wait_for`'s own sixty-second budget exists for, one level earlier: bring-up
    // itself can be starved the same way a test's own wait can.
    //
    // **`hlt` between checks, not a tight spin.** TCG runs each vCPU as one host thread, and
    // `spin_loop`'s `pause` hint does nothing for *host* scheduling; a CPU 0 that never stops
    // consuming its host thread's time slice can only make it harder for the vCPU thread whose
    // progress this loop is waiting on to get scheduled, on a busy host (this project's own
    // recorded condition; the ceiling that bounds it is
    // notes/skills/maintainer/SKILL.md, "What bounds lane count, and the three ceilings").
    // `wait_for_interrupt` parks CPU 0 on `hlt` until its own local APIC timer (armed at `TICK_HZ`,
    // already ticking: interrupts are enabled before `bring_up_secondaries` runs) wakes it, which
    // costs at most one tick of latency per check and, unlike the spin, actually yields host CPU
    // time. Measured to turn an occasional full hang (waiting past even a sixty-second budget) into
    // a reliable, clean give-up within the stated budget when a core does not come up. The "deeper
    // reason a core sometimes does not come up" that this paragraph used to defer to was the
    // re-read `before` fixes above: the cores were coming up, and this loop was not counting them
    // (`ap_boot`'s BUGS #1).
    let budget = 10 * crate::arch::timer::frequency();
    let start = crate::arch::timer::now();
    while crate::smp::online_count() == before {
        if crate::arch::timer::now().wrapping_sub(start) >= budget {
            return -1;
        }
        wait_for_interrupt();
    }
    0
}

/// Busy-wait roughly `us` microseconds, against the calibrated TSC. No `wfi`/`hlt` equivalent
/// here: INIT-SIPI-SIPI's delays are a handful of instructions on real hardware, and parking this
/// core for them would need an interrupt to wake it that nothing is going to send.
pub(super) fn busy_wait_us(us: u64) {
    let ticks = crate::arch::timer::frequency() / 1_000_000 * us;
    let start = crate::arch::timer::now();
    while crate::arch::timer::now().wrapping_sub(start) < ticks {
        core::hint::spin_loop();
    }
}

/// Can this machine start a secondary CPU at all? Yes, once the local APIC is up: INIT-SIPI-SIPI is
/// sent *through* it, unlike PSCI or SBI, which need no device at all before the first call.
pub fn can_start_secondaries() -> bool {
    irq::is_local_apic_ready()
}

/// Print how this machine starts a CPU. One line, on every boot, beside the SMP count.
pub fn print_bring_up_mechanism() {
    if irq::is_local_apic_ready() {
        crate::println!(
            "  smp: init-sipi-sipi via the local apic, trampoline at {:#x}",
            ap_boot::trampoline_phys()
        );
    } else {
        crate::println!(
            "  smp: the local apic is not up, so no core can be started here (init-sipi-sipi needs it)"
        );
    }
}

/// **The physical address `smp::bring_up_secondaries` hands `cpu_start` as `entry`.**
///
/// On aarch64 and RISC-V this is `virt_to_phys(secondary_boot)`: their `secondary_boot` is an
/// ordinary high-linked label, and firmware wants its physical address. Here `secondary_boot` is
/// **already** physical: link-x86_64.ld gives it the fixed low virtual address
/// (`AP_TRAMPOLINE_PHYS`) it has to execute from, because a `STARTUP` IPI can only name a page
/// below 1 MiB, so its own address *is* that page and converting it again would be wrong (this was
/// milestone 161's first named bug: `smp::bring_up_secondaries` used to call `virt_to_phys` on it
/// unconditionally, computing `secondary_boot's address - DIRECT_MAP_BASE`, an underflow, since
/// `secondary_boot`'s address is nowhere near the direct map).
pub fn secondary_boot_entry(secondary_boot_addr: u64) -> u64 {
    secondary_boot_addr
}
