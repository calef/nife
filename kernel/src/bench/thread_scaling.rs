//! **Milestone 134 (the register of measures), tier A, E1: IPC as more threads cycle through it**,
//! and the single-hart check E1 and E4 both run behind.
//!
//! Cut out of `bench.rs` unchanged by lane `maintainer/last-three-over-2000`, which left the
//! parent an index (§282 (when a file is split, the parent becomes an index)) and under §266 (a
//! Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

use super::*;

// --- Milestone 134, tier A: E1 (thread scaling), E4 (application displacement) ---
//
// design/roadmap/0134-the-measurements-that-decide.md. Both decide DECISIONS §96 (process kernel
// or event kernel) from a different angle: E1 asks whether the KERNEL's own IPC path gets cache-
// cold as more threads cycle through it; E4 asks what an IPC-heavy kernel costs an unrelated
// APPLICATION's working set, which is Liedtke's actual claim and the one no kernel-side number can
// see. Both need a real cache (no TCG models one) and one hart (so everything contends for the
// SAME core's cache, the effect under test); see `real_single_hart_or_skip`.
//
// **Both architectures since 2026-09-04, and the second one is the point.** These were aarch64-only
// because this tree has no riscv64 *accelerator* with a real cache, which is true and was never the
// same statement as "no riscv64 machine with a real cache": radon is a StarFive VisionFive 2 whose
// four U74s have 32 KB L1i and 32 KB L1d each, and that is the machine E1's prediction was built
// against and the machine E3's own design says a negative result needs. What was missing was a way
// to RUN them there, which is a board card (`script/board-image --bench`) plus a single-hart boot
// (the `single_hart` feature). Under QEMU nothing changes: the riscv64 runs are TCG, so both still
// self-skip, by the same one-value counter-frequency test the aarch64 half already used. See
// notes/footprint-perturbation.md for the procedure and design/roadmap/0134-the-measurements-that-decide.md.

/// QEMU's `virt` machine fixes `CNTFRQ_EL0` at 62.5 MHz under TCG, with or without `-icount`
/// (measured on this tree's pinned QEMU: `cargo xtask bench` and `cargo xtask bench --check` both
/// read it back from the printed `ns/iter` column); a real core reports its own frequency (24 MHz
/// measured on the Apple Silicon dev machine under HVF). Nothing else distinguishes the two
/// accelerators inside the guest: `NIFE_ACCEL` is a host-side environment variable for the QEMU
/// launch, never passed into the boot (see xtask's `bench` and `helpers/qemu-runner-aarch64.sh`).
#[cfg(target_arch = "aarch64")]
const TCG_VIRT_CNTFRQ_HZ: u64 = 62_500_000;

/// The riscv64 twin, and the reason this experiment can reach a small-cache board at all.
///
/// RISC-V has no `CNTFRQ_EL0`; the counter rate is a device-tree property
/// (`/cpus/timebase-frequency`, read by `arch::riscv64::timer::init_frequency`). QEMU `virt`
/// states **10 MHz**; the JH7110 on radon states **4 MHz** (notes/visionfive2.md, "Timebase is
/// 4 MHz"). So the same one-value test that separates HVF from TCG on aarch64 separates the board
/// from TCG here, and it fails safe in the direction that matters: an unknown riscv64 machine
/// reads as real, and the only value that self-skips is the emulator's own.
///
/// **What it does not distinguish**, and this is the honest limitation: it says "not QEMU `virt`",
/// not "has a 32 KB L1". A second riscv64 machine with a different timebase and a large cache
/// would run this and be believed. The operator's runbook (notes/footprint-perturbation.md) is what records
/// which machine a number came from; nothing in the kernel can.
#[cfg(target_arch = "riscv64")]
const TCG_VIRT_CNTFRQ_HZ: u64 = 10_000_000;

/// Shared precondition for E1 and E4: a real core (so there is a cache to model at all) and a
/// single hart (so every thread contends for the SAME core's cache, which is the effect under
/// test; spreading pairs across cores would dilute it, the opposite reason `smp_throughput`
/// requires more than one). Prints why and returns `false` when either does not hold, the same
/// self-skip shape every `--real`-only bench in this file already uses.
///
/// **The single-hart half is why a board card needs `single_hart`** (kernel/Cargo.toml). radon
/// seats four U74s and `smp::bring_up_secondaries` starts all of them, so a plain `board,bench`
/// card satisfies the "real core" half and fails this one, and both E1 and E4 would print a skip
/// line on the machine they were designed for. `single_hart` parks the secondaries at bring-up;
/// the cost is that `smp_throughput` self-skips on the same card, which is why the runbook takes
/// two cards rather than one. See notes/footprint-perturbation.md.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
pub(super) fn real_single_hart_or_skip(name: &str) -> bool {
    let cpus = crate::smp::online_count();
    if cpus != 1 {
        println!(
            "bench: {name} skipped (needs a single hart to isolate one core's cache; this boot has {cpus})"
        );
        return false;
    }
    let freq = crate::arch::timer::frequency();
    if freq == TCG_VIRT_CNTFRQ_HZ {
        println!(
            "bench: {name} skipped (QEMU virt detected via counter frequency {freq} Hz; TCG \
             models no cache, see design/roadmap/0134-the-measurements-that-decide.md)"
        );
        return false;
    }
    true
}

/// Pair counts for [`ipc_thread_scaling`] (E1): 1 to `SCALE_MAX_PAIRS` pairs, doubling.
///
/// **The roadmap's own words say "N from 2 to 128"; this sweeps up to 2*`SCALE_MAX_PAIRS` = 96
/// threads, not 128.** When this was written `sched::MAX_THREADS` was 128 for the WHOLE system
/// (DECISIONS §96), so 128 *pairs* (256 threads) could not exist at all, and even 64 pairs (128
/// threads) would have left no headroom for the bench boot's own thread. Read "N" as the
/// roadmap's own prediction text does, "distinct threads cycling through IPC" rather than pairs
/// ("somewhere between 16 and 32 threads"): 96 threads is 3x past the predicted knee.
///
/// **`MAX_THREADS` is 256 as of 2026-08-27 and this sweep did not move with it, deliberately.**
/// The cap stopped being the binding reason, but the sweep is a published measurement: changing
/// where it stops would make every future run incomparable with every recorded one, for a point
/// well past the knee this experiment exists to find. The number to change if the knee ever moves
/// is this one, and it should be changed for that reason rather than because more room appeared.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
pub(super) const SCALE_MAX_PAIRS: usize = 48;
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
pub(super) const SCALE_PAIRS: &[usize] = &[1, 2, 4, 8, 16, 32, SCALE_MAX_PAIRS];

/// **E1: IPC round-trip latency against thread count.** Kernel stacks are `STACK_SLOT_SPAN`
/// (28 KiB) apart, so each IPC that switches to a *different* thread touches a different stack; if
/// enough distinct threads cycle through the kernel, those stacks stop fitting in L1d and the
/// round trip goes cache-cold. The prediction: a knee somewhere in the low tens of threads,
/// against the smallest L1d this project targets (the `SiFive` U74's 32 KB).
///
/// Reuses [`tp_batch`]/[`tp_best`] (`smp_throughput`'s own machinery) at each pair count in
/// [`SCALE_PAIRS`]: N independent client/server pairs, released together behind a barrier, timed
/// wall-clock to completion, minimum of [`TP_REPEAT`] batches kept. That already IS "N pairs in
/// rotation": pinned to one hart (`real_single_hart_or_skip`), the scheduler round-robins the
/// ready pairs, so returning to a given pair's stack means other stacks were touched in between,
/// more of them as N grows. A flat line across the sweep says the process-kernel penalty is
/// invisible on this machine's cache; a knee reproduces Warton's effect on this kernel and gives
/// its magnitude.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
pub(super) fn ipc_thread_scaling() {
    if !real_single_hart_or_skip("ipc_thread_scaling") {
        return;
    }

    let mut req = [0u64; SCALE_MAX_PAIRS];
    let mut reply = [0u64; SCALE_MAX_PAIRS];
    for i in 0..SCALE_MAX_PAIRS {
        req[i] = sched::create_rendezvous();
        reply[i] = sched::create_rendezvous();
    }
    let done = sched::create_rendezvous();

    for &pairs in SCALE_PAIRS {
        let ticks = tp_best(&req[..pairs], &reply[..pairs], done, pairs);
        let threads = pairs * 2;
        println!(
            "bench: ipc_scale_{threads} {ticks} {}",
            pairs as u64 * TP_RTT
        );
    }
}
