//! **The small primitives the portable kernel calls**: halting, waiting for an interrupt, the
//! stack pointer, the DMA write barrier and instruction-cache sync.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// Stop this hart forever, cheaply. `wfi` parks the hart until an interrupt; with nothing left to
/// wake it, that is the rest of time at zero host CPU. The same discipline as aarch64: `wfi`, never
/// a spin. See notes/skills/qemu-hygiene/SKILL.md, "Never leave QEMU running".
/// Takes a [`crate::arch::HaltReason`], whose constructors are the list of who may stop a core for good;
/// a thread whose work is done leaves with `sched::exit` instead (milestone 720 (provisional)).
pub fn halt(_: crate::arch::HaltReason) -> ! {
    loop {
        instructions::wfi();
    }
}

/// Park until the next interrupt (the scheduler's idle primitive).
pub fn wait_for_interrupt() {
    instructions::wfi();
}

/// This core's current stack pointer, for the stack-overflow canary check (stack.rs).
pub fn current_sp() -> u64 {
    instructions::read_sp()
}

/// A DMA write memory barrier: order all prior stores before any device sees a later one. RISC-V's
/// `fence ow, ow` orders outer (device/IO) writes; the plain `fence` here is the conservative full
/// barrier, matching aarch64's `dsb sy`. Tightened when a real DMA driver lands.
pub fn direct_memory_access_write_barrier() {
    instructions::full_fence();
}

/// Make the instruction fetcher aware of code just written as data. Where aarch64 needs a
/// clean/invalidate loop over cache lines, RISC-V has one instruction: `fence.i` synchronizes this
/// hart's instruction stream with its prior stores. It has no address range (it covers everything),
/// so `va`/`len` are ignored; a multi-hart port will additionally need to fence the other harts. See
/// notes/riscv-port.md, leak #3.
pub fn sync_icache(va: u64, len: usize) {
    let _ = (va, len);
    instructions::fence_i();
    // The other harts' instruction fetch is not ordered by anything above, and RISC-V has no
    // broadcast form: a thread scheduled onto another hart can fetch stale bytes for code this
    // hart just wrote. TCG never shows this (its icache is perfectly coherent), and the U74 did,
    // on first-silicon day (2026-08-14): init's freshly built child hung on the board and nowhere
    // else. Push fence.i to every other online hart via SBI RFENCE, the mechanism the TLB
    // shootdown already uses one FID over.
    let others = crate::smp::online_harts_mask() & !(1 << crate::cpu::id());
    if others != 0 {
        sbi_remote_fence_i(others);
    }
}
