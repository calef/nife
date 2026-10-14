//! **This CPU's identity and its per-CPU block**: the boot CPU's id, and the `gs`-based
//! pointer every core reads its own block through.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// **The local APIC id of the CPU the kernel booted on**, recorded once by `boot.s`'s `_start_high`
/// (just after it zeroes `.bss`, and before it calls `kernel_main`) from `CPUID` leaf 1's
/// `EBX[31:24]`, "Initial APIC ID".
///
/// **Recorded rather than recomputed, and that distinction is the whole point** (milestone 316,
/// `ap_boot.rs`'s BUGS #3). `CPUID` answers *"which core am I"*. Every caller of [`boot_cpu_id`]
/// wants *"which core booted"*, and the two are the same number only while there is one core. Once
/// a secondary is online, a caller that §28's placement has migrated onto it read its own id and
/// called it the boot core's, which made `smp::tests::every_secondary_runs_scheduled_work` wait
/// forever for a mark the real boot core never sets, and made `stack::report_high_water` scan a
/// never-painted slot and report it at 65536/65536. Reading a value stamped by code that runs
/// exactly once, on exactly the boot processor, cannot say that whoever asks later.
///
/// This is riscv64's `BOOT_HARTID` in x86 clothing. The difference is only in where the number
/// comes from: OpenSBI hands RISC-V the hart id in `a0` and it would be lost if boot.s did not
/// catch it, whereas `CPUID` remains readable forever, which is exactly what made the recomputing
/// version look correct. aarch64 needs no static at all because its boot core is architecturally 0.
///
/// The name is **provisional** (an architect names things): it mirrors `BOOT_HARTID` on the
/// architecture that already had this shape.
#[unsafe(no_mangle)]
pub(super) static BOOT_CPU_ID: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);

/// The logical id of the CPU the kernel boots on.
///
/// **Read from a boot-time record, not recomputed** (milestone 316; it was a live `CPUID` read from
/// milestone 161 until then, and `BOOT_CPU_ID`'s own doc has why that was wrong and what it broke).
/// Before 161 it was the constant 0, on the reasoning that the boot processor is selected by
/// hardware rather than by firmware choice the way RISC-V's boot hart is. That is true and beside
/// the point: the number this returns has to agree with the *roster's* seating
/// (`smp::seat_cpus_from_acpi`, which seats every core, boot core included, at the slot its own
/// local APIC id names, the same logical-id-equals-hardware-id invariant `read_cpu_list` gives the
/// other two architectures), and nothing guarantees the boot CPU's local APIC id is 0 in general,
/// only that it usually is on QEMU.
///
/// Still callable from the kernel's very first Rust statement, which is what
/// `cpu::init_this_cpu(arch::boot_cpu_id())` needs: `boot.s` stamps the record before it calls
/// `kernel_main`, so there is no window in which this reads the `AtomicUsize`'s zero default.
pub fn boot_cpu_id() -> usize {
    BOOT_CPU_ID.load(core::sync::atomic::Ordering::Relaxed)
}

/// The `gs`-relative offset of `PerCpu::x86_trap.self_ptr`, which [`percpu`] loads.
pub(super) const PERCPU_SELF_OFF: usize =
    core::mem::offset_of!(crate::cpu::PerCpu, x86_trap.self_ptr);

/// Set this CPU's per-CPU pointer, by writing the `gs` segment base, and then the block's own
/// address into the block, through that base, for [`percpu`] to read back.
pub fn set_percpu(ptr: usize) {
    // SAFETY: `IA32_GS_BASE` exists on every long-mode CPU, and this value is the per-CPU block
    // this kernel reserves the register for.
    unsafe { write_msr(IA32_GS_BASE, ptr as u64) };
    // SAFETY: `gs` now bases at `ptr`, a `PerCpu` this core owns, and the offset is that struct's
    // own `self_ptr` field, computed by `offset_of!`. Only this core reaches its block through `gs`.
    unsafe {
        asm!(
            "mov gs:[{off}], {p}",
            off = const PERCPU_SELF_OFF,
            p = in(reg) ptr,
            options(nostack, preserves_flags),
        );
    }
}

/// Read this CPU's per-CPU pointer (the value last handed to [`set_percpu`]).
///
/// **One `gs`-relative load, not `rdmsr IA32_GS_BASE`** (milestone 758 (the IPC fast paths shrink
/// back inside their band), provisional). The two answer the same question, since `gs`'s base IS
/// that MSR, but `rdmsr` is a microcoded, serialising instruction that clobbers `rax`, `rdx` and
/// `rcx`, and this runs several times per syscall: every lock, unlock and current-thread read.
/// Inlined, it was the largest single line in x86_64's IPC fast-path closures on 2026-10-04 UTC.
///
/// It reads through `gs`, so in the windows where `IA32_GS_BASE` holds the user's value (around
/// the `swapgs` at `syscall` entry and before `iretq`, `trap.s`) it would be a load from a user
/// address. Nothing ordinary runs there: interrupts are masked, and the NMI, double fault and
/// machine check enter through `isr_paranoid`, which puts the kernel's GS base in place first.
pub fn percpu() -> usize {
    let p: usize;
    // SAFETY: after `set_percpu` on this core, `gs:[PERCPU_SELF_OFF]` is this core's own
    // `PerCpu::x86_trap.self_ptr`, which `set_percpu` wrote. `readonly`: it reads memory and writes
    // none. Not `pure`, so LLVM never merges two reads across a migration point.
    unsafe {
        asm!(
            "mov {p}, gs:[{off}]",
            p = out(reg) p,
            off = const PERCPU_SELF_OFF,
            options(nostack, readonly, preserves_flags),
        );
    }
    p
}

/// **Test-only: does the per-CPU pointer name the CPU we are physically running on?**
///
/// A constant `true` here, and for aarch64's reason rather than RISC-V's. RISC-V keeps the pointer
/// in `tp`, an ordinary register a trap frame carries, so it can go stale when a preempted thread
/// resumes on a different hart. `IA32_GS_BASE` is an MSR: it is not saved or restored by any context
/// switch and does not travel with a thread, so there is nothing that could make it stale. The
/// independent ground truth (the local APIC id) is not readable yet either way.
#[cfg(any(test, feature = "system_tests"))]
pub fn percpu_matches_hart() -> bool {
    true
}
