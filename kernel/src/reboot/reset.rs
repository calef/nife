//! **Restarting the machine**: the method the reboot object answers, the cold reset the kernel
//! itself asks for, and the route prepared before either.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// **`REBOOT`: restart the machine, or say why not** (DECISIONS §251, "The method").
///
/// Returns only when the firmware refused, with the refusal already printed; a successful reset
/// does not come back. The caller is `syscall::invoke`'s arm for `Object::Reboot`, and the
/// authority check is the capability itself: §251 asks for no rights bit beyond holding it.
pub fn restart() -> abi::Error {
    // Out of the ring first: once the reset starts, the drainer never runs again.
    crate::console::enter_reset();
    println!("{MARKER} the kernel was asked to restart the machine");
    let refused = cold_reset(MARKER);
    println!(
        "{MARKER} every reset route was refused ({refused:?}; the lines above say how); the \
         machine keeps running"
    );
    refused
}

/// **Try every reset route this machine has, board routes before firmware routes** (milestone 592,
/// 2026-10-10). The one place the order lives, so the reboot object and the rebooting soak cannot
/// disagree about it.
///
/// On a JH7110 whose tree names the PMIC's bus, the first route is the direct one: this kernel
/// programs the I2C controller's timing itself and writes the AXP15060's reset bit, which does not
/// depend on OpenSBI's driver at all. That is option B, built after the 2026-10-10 bench put the
/// firmware route in the outcome table's fourth row: bus up, PMIC read still failing ten times.
/// Whatever that attempt prints, the firmware route follows it, because on 2026-10-09 the firmware
/// route worked and one attempt's evidence is not a verdict either way.
pub fn cold_reset(marker: &str) -> abi::Error {
    prepare_reset_route(marker);
    pmic_reset_attempt(marker);
    arch::reboot(marker)
}

/// **Put back what the firmware's reset needs and U-Boot took away** (milestone 592 (radon's cold reboot dies in OpenSBI's PMIC write),
/// provisional).
///
/// On a JH7110 board, OpenSBI performs SBI SRST as an I2C write to the AXP15060 PMIC on I2C5, and
/// radon's U-Boot removes its I2C driver at `Starting kernel`, which gates the bus's clock and
/// asserts its reset. Radon's OpenSBI re-enables a clock, but it computes which one from the bus
/// node's name and U-Boot's tree names it `i2c@12050000`, so it ungates UART4's core clock
/// instead; and it never releases a reset. So the kernel ungates and releases I2C5 itself, from
/// the plan `memory::init` read out of the device tree, and prints every word it saw.
///
/// A no-op with no output on every machine that is not a JH7110 (`memory::jh7110_pmic_bus` is
/// `None` there), which is every machine CI boots. Two callers since milestone 805: the rebooting
/// soak and [`restart`], the two SBI resets nife makes on purpose. Moved here from `soak.rs` so the
/// second one could reach it without the soak's feature. The board test exit's shutdown takes the
/// same road and is recorded as a `BUGS` entry in milestone 592 rather than changed here.
///
/// Name: ratified 2026-10-06 (calef, #1783: "`prepare_reset_route`"), from
/// `prepare_the_reset_route`.
pub fn prepare_reset_route(marker: &str) {
    // Only a JH7110 has anything to prepare, and only riscv64 compiles the branch that reads it.
    #[cfg(not(target_arch = "riscv64"))]
    let _ = marker;
    #[cfg(target_arch = "riscv64")]
    if let Some((sys, bus)) = crate::memory::jh7110_pmic_bus() {
        use jh7110_clock_and_reset::Step;
        println!(
            "{marker} JH7110: bringing the PMIC's I2C bus back up first, because OpenSBI \
             resets this board with an I2C write to the AXP15060 (milestone 592). SYS CRG at \
             {:#x} ({}); plan {} ({} specifier(s) skipped{}).",
            sys.base,
            if sys.from_tree {
                "named by this machine's device tree"
            } else {
                "NOT named by this machine's tree: the constant mainline and the vendor agree on"
            },
            if bus.from_tree {
                "from the tree's own clocks and resets of the PMIC's bus"
            } else {
                "is the constant I2C5 plan (clock 143, reset 81), NOT read from this tree"
            },
            bus.skipped,
            if bus.truncated { ", TRUNCATED" } else { "" },
        );
        // SAFETY: `memory::init` recorded this window only for a machine whose tree names a
        // JH7110, and `mmu::map_everything` mapped exactly it, device-typed, in the direct map.
        // The plan's identifiers were bounded by `SYS` when it was built, and `bring_up` bounds
        // them again.
        let report = unsafe {
            crate::drivers::jh7110_clock_and_reset::bring_up(
                crate::arch::mmu::phys_to_virt(sys.base) as usize,
                &jh7110_clock_and_reset::SYS,
                bus.plan(),
            )
        };
        let clocks = bus.plan().iter().filter_map(|s| match s {
            Step::EnableClock(i) => Some(*i),
            Step::DeassertReset(_) | Step::SelectParent { .. } => None,
        });
        for (n, index) in clocks.enumerate().take(report.clocks) {
            println!(
                "{marker} JH7110: clock {index} {:#010x} -> {:#010x} ({})",
                report.clock_before[n],
                report.clock_after[n],
                if jh7110_clock_and_reset::is_clock_enabled(report.clock_after[n]) {
                    "running"
                } else {
                    "NOT running: the enable bit did not read back"
                },
            );
        }
        let reset = bus.plan().iter().rev().find_map(|s| match s {
            Step::DeassertReset(id) => Some(*id),
            Step::EnableClock(_) | Step::SelectParent { .. } => None,
        });
        if let Some(id) = reset {
            println!(
                "{marker} JH7110: reset {id} assert {:#010x} -> {:#010x}, status {:#010x} \
                 ({}, {} polls){}",
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
                    "; the bus was already up, so U-Boot's handover is NOT why the reset hangs"
                } else {
                    ""
                },
            );
        }
        if report.rejected > 0 {
            println!(
                "{marker} JH7110: {} step(s) REJECTED by the SYS domain's bounds: the plan \
                 and the domain disagree, which is a bug in this kernel, not the board",
                report.rejected
            );
        }
    }
}
