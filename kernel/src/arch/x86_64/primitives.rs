//! **The small primitives the portable kernel calls**: halting, waiting for an interrupt, the
//! stack pointer, the DMA write barrier and instruction-cache sync.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// Stop this CPU forever, cheaply. `hlt` parks it until an interrupt; with interrupts masked and
/// nothing left to wake it, that is the rest of time at zero host CPU. The same discipline as the
/// other two architectures' `wfi`. See notes/skills/qemu-hygiene/SKILL.md, "Never leave QEMU running".
/// Takes a [`crate::arch::HaltReason`], whose constructors are the list of who may stop a core for good;
/// a thread whose work is done leaves with `sched::exit` instead (milestone 720 (provisional)).
pub fn halt(_: crate::arch::HaltReason) -> ! {
    loop {
        instructions::hlt();
    }
}

/// Park until the next interrupt (the scheduler's idle primitive).
pub fn wait_for_interrupt() {
    instructions::hlt();
}

/// This CPU's current stack pointer, for the stack-overflow canary check (stack.rs).
pub fn current_sp() -> u64 {
    instructions::read_rsp()
}

/// A DMA write memory barrier: order all prior stores before any device sees a later one.
///
/// **`sfence`, and the reason it is nearly free is the whole reason x86 is worth porting to.** This
/// machine is TSO: stores are already globally ordered with respect to each other, so the ordinary
/// case needs no instruction at all. `sfence` is here because non-temporal stores and
/// write-combining memory are the exceptions TSO does not cover, and a DMA buffer can legitimately
/// be either.
///
/// This is where rule #4's bet pays out in the direction it was made. Code proven correct under
/// ARM's weak model and RISC-V's RVWMO is correct here by construction; nothing about developing on
/// x86 first could have said the reverse, and the tree would have accumulated invisible
/// strong-ordering assumptions that only a real port would have found.
pub fn direct_memory_access_write_barrier() {
    instructions::sfence();
}

/// Make the instruction fetcher aware of code just written as data.
///
/// **Nothing to do, and this is the one place x86's complexity buys something.** The instruction
/// cache on x86 is architecturally coherent with the data caches: a store to an address, followed
/// by a fetch of that address, sees the store, on this core and on every other, with no
/// software action at all. aarch64 needs a clean/invalidate loop over cache lines and a broadcast;
/// RISC-V needs `fence.i` locally and an SBI RFENCE remotely (and getting that wrong is what hung
/// init on first silicon). Here the guarantee is the hardware's.
///
/// A serialising instruction is still required if the *modifying* store and the fetch are separated
/// by a jump the CPU may have already speculated past, which the callers here are not doing.
pub fn sync_icache(va: u64, len: usize) {
    let _ = (va, len);
}
