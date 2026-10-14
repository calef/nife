//! **Starting the secondary harts and interrupting them**: SBI HSM, the line every boot prints
//! about it, and the reschedule IPI.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// Start a secondary hart, via the SBI HSM
/// (Hart State Management) extension's `sbi_hart_start(hartid, start_addr, opaque)`. The firmware
/// starts the target hart at `entry` (a physical address) in S-mode with paging off, `a0` = its hart
/// id and `a1` = `context`. Returns the SBI error (0 = success; a hart QEMU did not create, when
/// `-smp` is smaller than `MAX_CPUS`, returns a nonzero error rather than hanging). See boot.s
/// `secondary_boot`.
pub fn cpu_start(target_hart: u64, entry: u64, context: u64) -> i64 {
    const SBI_HSM_EID: usize = 0x0048_534D; // "HSM"
    const SBI_HART_START_FID: usize = 0;
    let args = [
        target_hart as usize,
        entry as usize,
        context as usize,
        0,
        0,
        0,
    ];
    // SAFETY: HSM hart_start with (hartid, start_addr, opaque). It starts another hart at `entry`,
    // which this function's callers (the SMP bring-up) point at boot.s `secondary_boot`; it touches
    // none of this hart's memory.
    unsafe { sbi::call(SBI_HSM_EID, SBI_HART_START_FID, args) }.error as i64
}

/// Can this machine start a secondary hart at all? Asked once by `smp::bring_up_secondaries`.
///
/// **Always true here, and the asymmetry with aarch64 is the point.** RISC-V has exactly one
/// mechanism: SBI HSM `hart_start`, an `ecall` to the firmware that is already under us by
/// construction (this kernel is entered in S-mode, so something is running in M-mode). There is no
/// conduit to choose and no function id to look up, which is why milestone 100 is an aarch64
/// milestone with a RISC-V half rather than the other way round. Whether the firmware *implements*
/// HSM is a separate question and one `isa::init` already answers: `SBI_REQUIRED` includes it, so a
/// firmware without it stops the boot before this is reached.
pub fn can_start_secondaries() -> bool {
    true
}

/// Print how this machine starts a hart. One line, on every boot, beside the SMP count; the aarch64
/// twin has something to say here because it read `/psci`, and this one says what is fixed.
pub fn print_bring_up_mechanism() {
    crate::println!("  smp: sbi hsm hart_start (the only mechanism RISC-V defines)");
}

/// Send a reschedule inter-processor interrupt to `target_hart` via the SBI IPI extension. The
/// firmware sets the target hart's `sip.SSIP`, so it takes a supervisor software interrupt
/// (`scause` = 1) and drains its inbox. The RISC-V analog of an aarch64 reschedule SGI; used by
/// `arch::irq::send_reschedule` (and so by `sched::place_on` when it hands a thread to another hart).
pub fn sbi_send_ipi(target_hart: usize) {
    const SBI_IPI_EID: usize = 0x0073_5049; // "sPI"
    const SBI_SEND_IPI_FID: usize = 0;
    let mask = 1usize << target_hart; // hart_mask, relative to base 0
    // SAFETY: send_ipi with (hart bitmap, mask base 0). It sets another hart's `sip.SSIP` and
    // touches no memory of ours. The result is ignored, as it always was.
    unsafe { sbi::call(SBI_IPI_EID, SBI_SEND_IPI_FID, [mask, 0, 0, 0, 0, 0]) };
}
