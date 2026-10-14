//! **The small primitives the portable kernel calls**: halting, waiting for an interrupt, the
//! stack pointer and `SPSel`, the device write barrier and instruction-cache sync.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// Park this core forever.
///
/// **`wfi`, not `wfe`, and the difference is not academic.**
///
/// `wfe` waits for an *event*: an `sev` from another core, or a lock release. QEMU's
/// emulation treats it as little more than a hint, so `loop { wfe() }` keeps translating
/// and executing, and a halted kernel burns **99.7% of a host CPU core**. We discovered
/// this the way you'd expect: eleven abandoned QEMU processes cooking the laptop overnight
/// at a combined 729%.
///
/// `wfi` waits for an *interrupt*, and QEMU implements it as an actual vCPU halt: the host
/// thread sleeps. An idle kernel becomes genuinely idle.
///
/// It is also the more correct instruction for what we mean. We are not waiting for an
/// event from a sibling core. We are idling until something interrupts us, of which there
/// is currently nothing, which is exactly the point.
/// Takes a [`crate::arch::HaltReason`], whose constructors are the list of who may stop a core for good;
/// a thread whose work is done leaves with `sched::exit` instead (milestone 720 (provisional)).
// A test image on this architecture never halts: its panics and its suite's end both exit through
// semihosting.
#[cfg_attr(any(test, feature = "system_tests"), allow(dead_code))]
pub fn halt(_: crate::arch::HaltReason) -> ! {
    loop {
        aarch64_cpu::asm::wfi();
    }
}

/// Wait for one interrupt, then return. **The idle thread's whole body.**
///
/// When every other thread is blocked (all waiting on I/O, say), the scheduler runs the idle
/// thread, which parks the CPU here until *something* interrupts: the timer, or the device a
/// blocked driver is waiting on. The handler may wake a thread; when `wfi` returns, the idle
/// thread yields and the scheduler picks up whatever became runnable.
///
/// `wfi`, not `wfe`, for the reason in `halt`: QEMU implements `wfi` as a real vCPU halt (the
/// host thread sleeps), so an idle kernel is genuinely idle. See notes/scheduler and CLAUDE.md.
pub fn wait_for_interrupt() {
    aarch64_cpu::asm::wfi();
}

/// This core's current stack pointer. Reading `sp` is arch-specific (rule 1), so the stack-overflow
/// canary check (stack.rs) goes through here rather than embedding an `asm!` in portable code.
pub fn current_sp() -> u64 {
    instructions::read_sp()
}

/// `SPSel`, the register that says which stack pointer the name `sp` currently means at EL1:
/// bit 0 set is `SP_EL1`, clear is `SP_EL0`. Reading a system register is arch-specific (rule 1),
/// so the trap-frame sanity test (`el1_runs_on_sp_el1`, `user/el0_boundary_tests.rs`) goes through here rather
/// than embedding an `asm!` in portable code, the same shape as [`current_sp`]. Test-only because
/// nothing but that test asks, and aarch64-only on purpose: RISC-V does not bank the stack pointer
/// by privilege level, so there is no analogous register to read (notes/riscv-parity-scope.md).
#[cfg(feature = "system_tests")]
pub fn spsel() -> u64 {
    instructions::read_spsel()
}

/// Order all prior normal-memory writes before the next device (MMIO) write.
///
/// The kernel builds a virtio descriptor ring in normal memory, then rings the device with an MMIO
/// write. The device is a **separate observer** that reads that ring by DMA, so the ring stores
/// must be globally visible before the "go" signal lands, or the device reads stale bytes. A `dsb`
/// guarantees it. On QEMU DMA is coherent and the notify is processed synchronously, so this is
/// effectively free; on real hardware it is load-bearing. Arch-specific by rule 1, so it lives here
/// rather than in the transport (kernel/src/virtio.rs).
pub fn direct_memory_access_write_barrier() {
    aarch64_cpu::asm::barrier::dsb(aarch64_cpu::asm::barrier::SY);
}

/// Make the instruction fetcher aware of code we just wrote as data, over `[va, va+len)`.
///
/// The D-cache and the I-cache are **not coherent** on aarch64. This is not a QEMU quirk, it is the
/// architecture: the assumption is that writing code is rare and paying for coherence on every store
/// is not worth it, so the loader has to say so explicitly, and every loader on every ARM machine
/// does exactly this. `dc cvau` cleans the data cache to the point of unification, `ic ivau`
/// invalidates the instruction cache, and the barriers make the two agree. Get it wrong and the CPU
/// executes whatever was in that page *before* the program landed there.
///
/// Arch-specific by rule 1 (RISC-V does this with a single `fence.i`), so it lives here rather than
/// in the ELF loader (kernel/src/user.rs). See notes/riscv-port.md, leak #3.
pub fn sync_icache(va: u64, len: usize) {
    const LINE: u64 = 64; // conservative: the real size is in CTR_EL0

    let mut p = va & !(LINE - 1);
    let end = va + len as u64;

    // SAFETY: cache maintenance on a mapped, readable range is always sound.
    unsafe {
        while p < end {
            core::arch::asm!("dc cvau, {p}", p = in(reg) p, options(nostack));
            p += LINE;
        }
        core::arch::asm!("dsb ish", options(nostack));

        let mut p = va & !(LINE - 1);
        while p < end {
            core::arch::asm!("ic ivau, {p}", p = in(reg) p, options(nostack));
            p += LINE;
        }
        core::arch::asm!("dsb ish", "isb", options(nostack));
    }
}
