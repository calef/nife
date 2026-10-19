//! **Which exception level firmware entered at, this core's identity and its per-CPU block**,
//! read through `TPIDR_EL1`.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

unsafe extern "C" {
    /// The exception level core 0 was entered at, written by `boot.s` before anything else runs.
    /// See [`entry_el`].
    static boot_entry_el: u64;
}

/// **Which exception level did firmware enter this kernel at?** 1 or 2.
///
/// The kernel always *runs* at EL1: `boot.s` drops itself when it finds itself at EL2, which is
/// where U-Boot enters a payload on the boards this project is headed for (milestone 127, the seL4
/// machine). This is the record of what the entry actually was, and it is worth having as more
/// than trivia, because two other facts follow from it.
///
/// **The PSCI conduit follows it.** A machine that runs the kernel below an EL2 states `smc` in
/// `/psci`, because an `hvc` from EL1 would arrive at an EL2 with no vector table; a machine that
/// enters at EL1 states `hvc`. `isa`'s test asserts exactly that pairing rather than a constant.
///
/// **And so does what a stranger reads at a serial console**, which is milestone 127's entire
/// deliverable. The boot banner prints this line first.
///
/// The name is provisional (AGENTS.md: names are an architect's).
///
/// Its two callers are the banner in `main.rs` and `isa`'s conduit test, and the `bench` boot
/// compiles the banner out, so it has no caller in exactly that configuration. Same shape and same
/// exemption as `isa::print_summary` beside it. Not `any(test, feature = "bench")` like that one:
/// the test build does have a caller here.
#[cfg_attr(feature = "bench", allow(dead_code))]
pub fn entry_el() -> u64 {
    // SAFETY: an 8-byte aligned word in .bss, written once by `_boot_el1` on core 0 before any
    // Rust runs and never again. Volatile because nothing in the Rust program writes it, and a
    // compiler that noticed that would be entitled to fold the read to the zero .bss began at.
    unsafe { core::ptr::read_volatile(&raw const boot_entry_el) }
}

/// Point `TPIDR_EL1` at this core's per-CPU block.
///
/// `TPIDR_EL1` is a scratch system register the architecture reserves for software's own use;
/// the kernel keeps a per-core pointer in it and reads it back in one `mrs`. This is the
/// standard aarch64 per-CPU base (Linux uses `TPIDR_EL1` identically). The portable side of
/// this lives in `kernel/src/cpu.rs`; only the register touch belongs here (DECISIONS §4).
pub(crate) fn set_percpu(ptr: usize) {
    TPIDR_EL1.set(ptr as u64);
}

/// The logical id of the core the kernel boots on. On aarch64's `virt` the boot core is MPIDR
/// affinity 0, so it is always 0; the SMP bring-up starts every *other* core. (RISC-V's boot hart is
/// not guaranteed to be 0, so its `arch::boot_cpu_id` reads the actual hart id.)
pub(crate) fn boot_cpu_id() -> usize {
    0
}

/// Read this core's per-CPU pointer back. One instruction.
pub fn percpu() -> usize {
    TPIDR_EL1.get() as usize
}

/// **This core's `PerCpu::current`, read with IRQs masked**, so no interrupt can fall between
/// finding the block and reading the field (notes/mmu.md, BUGS).
///
/// [`percpu()`] and then a load is two steps. A preempted kernel thread that resumes on another core
/// between them still holds the first core's block in a general register, and reads the name of
/// whatever thread that core runs now. riscv64 and `x86_64` close the window with one load relative
/// to the per-CPU register; aarch64 has no addressing mode relative to a system register, so it
/// masks instead. Linux's answer is `current` in `SP_EL0`, which would mean swapping that register
/// on every trap and switch. On the syscall path IRQs are already masked and this costs the `DAIF`
/// read and a branch.
///
/// Name: provisional.
#[inline]
pub fn read_current_thread() -> u64 {
    let was_enabled = super::interrupts::disable();
    let v = crate::cpu::current()
        .current
        .load(core::sync::atomic::Ordering::Relaxed);
    super::interrupts::restore(was_enabled);
    v
}

/// **Test-only: does the per-CPU pointer name the hart we run on?** Always `true` on aarch64: the
/// pointer lives in `TPIDR_EL1`, a system register the trap frame never saves or restores, so a
/// thread migrating between cores keeps whatever `TPIDR_EL1` the destination core set at boot. The
/// RISC-V twin has to check (its per-CPU pointer is `tp`, a general register that CAN ride a stale
/// trap frame across a migration, DECISIONS §28); this exists so a portable test can call it on both.
#[cfg(any(test, feature = "system_tests"))]
pub fn percpu_matches_hart() -> bool {
    true
}
