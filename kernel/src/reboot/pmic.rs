//! **The PMIC reset attempt**: on a JH7110, asking the board's power controller over I2C before
//! the firmware; on every other machine, nothing.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

#[cfg(target_arch = "riscv64")]
use super::*;

#[cfg(target_arch = "riscv64")]
pub(super) fn pmic_reset_attempt(marker: &str) {
    use jh7110_clock_and_reset::{
        IcClkWords, PMIC_RESET_BIT, PMIC_RESET_REG, SYS_SYSCON_BASE, SYSCLK_APB_BUS_FUNC,
        SYSCLK_AXI_CFG0, SYSCLK_BUS_ROOT, SYSCLK_STG_AXIAHB, i2c5_ic_clk, one_based_div,
        standard_mode_100k,
    };

    let Some((sys, bus)) = crate::memory::jh7110_pmic_bus() else {
        return;
    };
    let Some((controller, _size)) = bus.controller else {
        return;
    };
    let Some(address) = bus.pmic_address else {
        return;
    };

    // The input-clock chain, from the two windows the plan's guard mapped: the CRG words at their
    // ids' offsets, the PLL2 words in the syscon at the offsets the vendor's pll.c masks name.
    // SAFETY: every call passes one of those two mapped windows and an aligned offset inside it,
    // and these are read-only status words, so the read has no side effect.
    let word = |base: usize, offset: usize| unsafe {
        core::ptr::read_volatile((base + offset) as *const u32)
    };
    let crg = crate::arch::mmu::phys_to_virt(sys.base) as usize;
    let syscon = crate::arch::mmu::phys_to_virt(SYS_SYSCON_BASE) as usize;
    let words = IcClkWords {
        bus_root: word(crg, SYSCLK_BUS_ROOT as usize * 4),
        axi_cfg0: word(crg, SYSCLK_AXI_CFG0 as usize * 4),
        stg_axiahb: word(crg, SYSCLK_STG_AXIAHB as usize * 4),
        apb_bus_func: word(crg, SYSCLK_APB_BUS_FUNC as usize * 4),
        pll2_dacpd_dsmpd_fbdiv: word(syscon, 0x2c),
        pll2_postdiv1: word(syscon, 0x30),
        pll2_prediv: word(syscon, 0x34),
    };
    let ic = i2c5_ic_clk(&words);
    let mode = standard_mode_100k(ic);
    let fbdiv = (words.pll2_dacpd_dsmpd_fbdiv >> 17) & 0xfff;
    println!(
        "{marker} AXP15060: direct route, controller {controller:#x}, PMIC {address:#04x}, IC \
         clock {ic} Hz (bus_root {}, divs {}/{}/{}, pll2 fbdiv {fbdiv} prediv {} postdiv1 {}), \
         standard mode hcnt {} lcnt {} sda_hold {}",
        (words.bus_root >> 24) & 1,
        one_based_div(words.axi_cfg0, 2),
        one_based_div(words.stg_axiahb, 2),
        one_based_div(words.apb_bus_func, 4),
        words.pll2_prediv & 0x3f,
        1u32 << ((words.pll2_postdiv1 >> 28) & 3),
        mode.hcnt,
        mode.lcnt,
        mode.sda_hold,
    );

    let i2c =
        designware_i2c::DesignWareI2c::new(crate::arch::mmu::phys_to_virt(controller) as usize);
    // The register is read first so the write sets the reset bit alone and the PMIC's other bits
    // keep whatever they hold, which is the difference between option B and the power-off bit 7
    // radon's OpenSBI sets unconditionally.
    let mut current = [0u8; 1];
    if let Err(failure) = i2c.write_read(&mode, address as u8, &[PMIC_RESET_REG], &mut current) {
        println!(
            "{marker} AXP15060: read of reg {PMIC_RESET_REG:#04x} failed: {failure:?}; the \
             firmware route is next"
        );
        return;
    }
    let value = current[0] | (1 << PMIC_RESET_BIT);
    println!(
        "{marker} AXP15060: reg {PMIC_RESET_REG:#04x} read {:#04x}, writing {value:#04x} (bit \
         {PMIC_RESET_BIT} set, every other bit as found); this line is the last if the PMIC honors it",
        current[0],
    );
    // The line above is the one the 2026-10-09 bench lost, so it goes out on the wire before the
    // byte that may cut power behind it.
    crate::console::drain();
    match i2c.write_read(&mode, address as u8, &[PMIC_RESET_REG, value], &mut []) {
        Ok(()) => {
            // The write completed and the board is still up: the PMIC acknowledged a byte it did
            // not act on, or acts slower than the controller's stop. Say so and let the firmware
            // route run rather than deciding the PMIC's timing from one round trip.
            println!(
                "{marker} AXP15060: write completed without a reset; the firmware route is next"
            );
        }
        Err(failure) => {
            println!("{marker} AXP15060: write failed: {failure:?}; the firmware route is next");
        }
    }
}

/// The aarch64 and x86_64 halves have no PMIC route: their firmware interfaces (PSCI, the FADT
/// register) are the whole story, so [`cold_reset`](super::cold_reset) goes straight to `arch::reboot`.
#[cfg(not(target_arch = "riscv64"))]
pub(super) fn pmic_reset_attempt(marker: &str) {
    let _ = marker;
}
