//! **This hart's identity and its per-CPU block**: the trap stash `sscratch` points at, the boot
//! hart's id, and the `tp`-based per-CPU pointer.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// **The per-hart trap stash**, the thing `sscratch` points at in both U- and S-mode. RISC-V's `tp`
/// is a general register that U-mode owns, so a trap from U-mode arrives with the user's `tp` and the
/// kernel must recover its own per-CPU pointer from a *hart-private* source. That source is this
/// struct: `sscratch` holds `&TRAP_STASH[hart]`, and `trap.s` reads the kernel `tp` from `percpu` and
/// the kernel stack from `kernel_sp`. One global `KERNEL_TP` could not do this once there is more than
/// one hart (every hart would reload hart 0's pointer); an array indexed by hart, reached through the
/// per-hart `sscratch`, is what makes the trap path SMP-correct.
///
/// `#[repr(C)]` and the field order are load-bearing: `trap.s` accesses these by fixed byte offset
/// (0, 8, 16, 24), checked below. Each hart touches only its own entry, and only during its own trap,
/// so the `AtomicUsize`s are for interior mutability through the shared static, not cross-core
/// synchronization (the asm reads/writes them plainly).
#[repr(C)]
pub(super) struct TrapStash {
    /// The current thread's kernel-stack top; where a U-mode trap lands. Set on every return to
    /// U-mode (trap.s `trap_return`), so it always names the thread about to run in U-mode.
    kernel_sp: AtomicUsize,
    /// This hart's `PerCpu` pointer: the kernel `tp` the trap entry restores.
    percpu: AtomicUsize,
    /// Two scratch words the trap entry uses to free registers before it has a stack.
    scratch0: AtomicUsize,
    scratch1: AtomicUsize,
}

impl TrapStash {
    const fn new() -> Self {
        Self {
            kernel_sp: AtomicUsize::new(0),
            percpu: AtomicUsize::new(0),
            scratch0: AtomicUsize::new(0),
            scratch1: AtomicUsize::new(0),
        }
    }
}

// trap.s hardcodes these offsets; keep them honest.
const _: () = {
    assert!(core::mem::offset_of!(TrapStash, kernel_sp) == 0);
    assert!(core::mem::offset_of!(TrapStash, percpu) == 8);
    assert!(core::mem::offset_of!(TrapStash, scratch0) == 16);
    assert!(core::mem::offset_of!(TrapStash, scratch1) == 24);
};

/// One trap stash per hart, indexed like `cpu::PERCPU`. A static so it exists before any allocator,
/// which the very first trap (and every secondary's bring-up) needs.
pub(super) static TRAP_STASH: [TrapStash; crate::cpu::MAX_CPUS] =
    [const { TrapStash::new() }; crate::cpu::MAX_CPUS];

/// The physical hart id OpenSBI handed the kernel on (`a0` at `_start`), stashed by boot.s. The
/// logical cpu id equals the physical hart id on RISC-V (so the PLIC context `2*hart+1`, the timer,
/// and IPIs all line up), and this is which hart is the boot cpu. QEMU's boot hart is usually 0 but
/// the spec does not require it, so the SMP bring-up reads this rather than assuming.
#[unsafe(no_mangle)]
pub(super) static BOOT_HARTID: AtomicUsize = AtomicUsize::new(0);

/// The physical hart id the kernel booted on (see [`BOOT_HARTID`]).
pub fn boot_hartid() -> usize {
    BOOT_HARTID.load(Ordering::Relaxed)
}

/// The logical id of the hart the kernel boots on. On RISC-V the logical cpu id equals the physical
/// hart id, and QEMU's boot hart is not guaranteed to be 0, so this reads the id OpenSBI handed us at
/// `_start` (stashed by boot.s). The SMP bring-up starts every *other* hart. The aarch64 twin is
/// always 0.
pub fn boot_cpu_id() -> usize {
    boot_hartid()
}

/// Set this hart's per-CPU pointer. RISC-V's `tp` (thread pointer) is the analog of aarch64's
/// `TPIDR_EL1`, but a general register, so this also arms the per-hart trap path: it records the
/// pointer in this hart's [`TrapStash`] and points `sscratch` at that stash, so `trap.s` can recover
/// the kernel `tp` after a U-mode round trip. See `crate::percpu` and trap.s.
pub fn set_percpu(ptr: usize) {
    // Set `tp` first so `cpu::id()` (which reads `tp`) resolves this hart's index into TRAP_STASH.
    // SAFETY: `ptr` is this hart's `PerCpu`, which is what `tp` must name.
    unsafe { instructions::write_tp(ptr) };
    let stash = &TRAP_STASH[crate::cpu::id()];
    stash.percpu.store(ptr, Ordering::Relaxed);
    let stash_ptr = stash as *const TrapStash as usize;
    // SAFETY: `sscratch` now names this hart's stash; trap.s reads it as `&TrapStash` on every trap.
    unsafe { instructions::write_sscratch(stash_ptr) };
}

/// Read this hart's per-CPU pointer (the value last handed to [`set_percpu`]).
pub fn percpu() -> usize {
    instructions::read_tp()
}

/// **Test-only: does `tp` name the hart we are physically running on?** RISC-V keeps the kernel
/// per-CPU pointer in `tp`, so `cpu::id()` is only right if `tp` is. The independent ground truth is
/// `sscratch`, which points at this hart's [`TrapStash`], is set once per hart by [`set_percpu`], and
/// never migrates. A mismatch means a preempted kernel thread resumed on a different hart still
/// carrying the stale `tp` from its origin hart, the SMP migration bug `trap.s`'s S-mode `tp`
/// handling fixes (DECISIONS §28). The two reads run under masked interrupts so a preemption between
/// them cannot split them across harts and manufacture a false mismatch. The aarch64 twin is a
/// constant `true`: there the per-CPU pointer is `TPIDR_EL1`, a system register the trap frame never
/// carries, so it cannot go stale on migration.
#[cfg(any(test, feature = "system_tests"))]
pub fn percpu_matches_hart() -> bool {
    let was_enabled = crate::arch::interrupts::disable();
    // `sscratch` holds `&TRAP_STASH[hart]` in S-mode (trap.s keeps it so).
    let sscratch = instructions::read_sscratch();
    // `cpu::id()` reads `tp`; taken here, under the same mask, so both name one instant on one hart.
    let hart_from_tp = crate::cpu::id();
    crate::arch::interrupts::restore(was_enabled);
    let base = TRAP_STASH.as_ptr() as usize;
    let hart_from_sscratch = (sscratch - base) / core::mem::size_of::<TrapStash>();
    hart_from_sscratch == hart_from_tp
}
