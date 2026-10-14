//! **How many CPU cycles one tick of this machine's clock is**, and what that ratio means on
//! each architecture.
//!
//! Cut out of `bench.rs` unchanged by lane `maintainer/last-three-over-2000`, which left the
//! parent an index (§282 (when a file is split, the parent becomes an index)) and under §266 (a
//! Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

use super::*;

/// **What one tick of this machine's clock costs in CPU cycles** (milestone 74's riscv64 and aarch64
/// halves, milestone 309's `x86_64` half).
///
/// Board bench rows are in ticks of `crate::arch::timer::now()`, a fixed-rate reference counter;
/// the literature is in **cycles**, and one measured ratio converts every row at once.
///
/// **A probe, not a benchmark row**: a rate that moves with frequency scaling. `bench-probe:` lines
/// are echoed and never enter the baseline; see `map_new`.
///
/// # The numerator and the denominator are different on each architecture, and the line says so
///
/// A rate printed under one name on three machines will be compared across them, so a
/// `cycles_per_tick_means` line beside the number names both halves of the ratio.
///
/// - **`riscv64`.** Core cycles (SBI PMU `CPU_CYCLES`) over ticks of the `time` CSR, a timebase the
///   device tree states: 10 MHz on QEMU `virt`, 4 MHz on radon's JH7110.
/// - **`aarch64`.** `PMCCNTR_EL0` over ticks of `CNTVCT_EL0`, the generic timer at `CNTFRQ_EL0`
///   (62.5 MHz on QEMU `virt`, 19.2 MHz on argon's TX1). **Which exception levels the numerator
///   counts is `PMCCFILTR_EL0`'s**: EL0 and EL1, not EL2, on every board, by calef's ruling A1 of
///   2026-10-07 (UTC) in design/roadmap/0353-the-aarch64-half-of-74.md. That is user plus kernel,
///   the quantity the other two architectures count, and the meaning line names it.
/// - **`x86_64`.** Unhalted core cycles (`IA32_PERF_FIXED_CTR1`) over TSC ticks. **The TSC is
///   constant-rate and core cycles are not**, so this ratio moves with frequency scaling, which is
///   information: `arch::timer::now()` here *is* `rdtsc`, so the TSC alone would print `1.00`.
///   See `arch::x86_64::pmu`'s header.
///
/// # What this measures under emulation, which is nothing
///
/// QEMU-TCG's `riscv64` `cycle` CSR is an instruction count, and its `x86_64` build models no
/// performance monitoring unless asked, so the ratio under emulation is a fact about the emulator,
/// and the line says so. notes/riscv-cycle-counters.md and
/// design/roadmap/0309-x86-64-core-cycles.md are the procedures on a real core.
///
/// # Why the window is a timed spin
///
/// Both counters are read across the same span, so the ratio does not depend on how long the spin
/// is or on the loop being compiled a particular way. A fixed iteration count would make the ratio
/// depend on the code inside the loop, which is exactly the thing that differs between an emulator
/// and a core.
pub(super) fn cycles_per_tick() {
    // The core at each end, because on aarch64 each core's counter is its own number (`C` zeroes it
    // at that core's init; `arch::aarch64::pmu`'s BUGS), so a window that migrated would difference
    // two unrelated counters. Cheap and harmless on the other two, where it has never fired.
    let cpu = crate::cpu::id();
    let Some(c0) = crate::arch::pmu::cycles() else {
        // The outcome says *why*, and on a board that is the whole diagnostic: see
        // `arch::riscv64::pmu::CycleCounter` and `arch::x86_64::pmu::CycleCounter`. **Never a
        // number here**, and in particular never a fallback to the reference clock: on x86_64 that
        // clock is the TSC, and a probe that degraded to it would print an exact 1.00 that reads
        // as a measurement.
        println!(
            "bench-probe: cycles_per_tick unavailable ({:?})",
            crate::arch::pmu::outcome()
        );
        return;
    };

    let t0 = crate::arch::timer::now();
    while crate::arch::timer::now() - t0 < cycle_probe_window_ticks() {
        core::hint::spin_loop();
    }
    let t1 = crate::arch::timer::now();
    let c1 = crate::arch::pmu::cycles().expect("the counter did not vanish mid-window");
    if crate::cpu::id() != cpu {
        println!(
            "bench-probe: cycles_per_tick unavailable (the window moved cores; per-core counters)"
        );
        return;
    }

    let ticks = t1 - t0;
    let cycles = cycle_probe_delta(c0, c1);
    // Two decimal places by integer arithmetic: this is a `no_std` kernel and the ratio is under
    // one on any emulator and in the hundreds on real silicon, so the fraction carries real
    // information at both ends.
    let hundredths = cycles.saturating_mul(100).checked_div(ticks).unwrap_or(0);
    println!(
        "bench-probe: cycles_per_tick {}.{:02} ({cycles} cycles over {ticks} ticks at cntfrq {})",
        hundredths / 100,
        hundredths % 100,
        crate::arch::timer::frequency(),
    );
    println!("bench-probe: cycles_per_tick_means {CYCLE_PROBE_MEANING}");
}

/// **What the ratio above is a ratio of**, in this machine's own vocabulary. Printed rather than
/// left to a note, because the number leaves the machine and the note does not travel with it.
#[cfg(target_arch = "riscv64")]
const CYCLE_PROBE_MEANING: &str = "core cycles (SBI PMU CPU_CYCLES) per tick of the `time` CSR, a fixed-rate timebase the device \
     tree states";

/// The aarch64 twin. **The filter clause is the one a reader must not drop**: which exception levels
/// are counted is a policy (`arch::pmu::PMCCFILTR_COUNT_EL0_AND_EL1`, calef's ruling A1, 2026-10-07
/// UTC), and a figure compared against a system that counted differently, such as seL4 on firmware
/// that left `P` set, is a different quantity. The line that leaves the machine carries that rather
/// than a note that does not. It no longer says PROVISIONAL because the value no longer is.
#[cfg(target_arch = "aarch64")]
const CYCLE_PROBE_MEANING: &str = "cycles (PMCCNTR_EL0, PMCCFILTR_EL0 0: EL0+EL1 counted, EL2 not) per tick of \
     CNTVCT_EL0, the generic timer at CNTFRQ_EL0";

/// The `x86_64` twin, and the difference a cross-architecture reader has to know: the denominator
/// here is the TSC, which is constant-rate, so the ratio moves with the core's frequency where
/// riscv64's does not.
#[cfg(target_arch = "x86_64")]
const CYCLE_PROBE_MEANING: &str = "unhalted core cycles (IA32_PERF_FIXED_CTR1) per TSC tick; the TSC is constant-rate and core \
     cycles are not, so this ratio moves with frequency scaling and turbo";

/// Ticks to spin for. At QEMU's 10 MHz `time` CSR this is 10 ms, long enough that the two counter
/// reads at each end are noise and short enough not to stretch a bench run.
///
/// **A constant here and a computed value on `x86_64`**, because RISC-V's timebase is stated by the
/// machine and x86's is measured by this kernel: 100,000 ticks is 10 ms on QEMU and 25 ms on
/// radon's 4 MHz JH7110, both fine. It stays a constant for the same reason `SCALE_MAX_PAIRS` did:
/// milestone 74's block records a reading of `100002 ticks`, and changing the window would make
/// every future radon run incomparable with the one on record.
#[cfg(target_arch = "riscv64")]
fn cycle_probe_window_ticks() -> u64 {
    100_000
}

/// Ten milliseconds of generic timer, whatever `CNTFRQ_EL0` says: 62.5 MHz on QEMU and 19.2 MHz on
/// argon, so a constant would be a different window on each.
#[cfg(target_arch = "aarch64")]
fn cycle_probe_window_ticks() -> u64 {
    crate::arch::timer::frequency() / 100
}

/// Ten milliseconds of TSC, whatever this part's TSC rate turned out to be. A constant would be
/// microseconds on one machine and a visible pause on another, because the TSC's rate is a property
/// of the part rather than an architected number (see `arch::x86_64::timer`'s header on why this
/// kernel has to measure it against the PIT at all).
#[cfg(target_arch = "x86_64")]
fn cycle_probe_window_ticks() -> u64 {
    crate::arch::timer::frequency() / 100
}

/// The difference between two counter reads. The SBI PMU counter this kernel accepts is 64 bits
/// wide (`arch::riscv64::pmu` records the width and QEMU's is 64), and `PMCCNTR_EL0` is 64 bits by
/// architecture, so a plain wrapping subtraction is the whole of it on both.
#[cfg(any(target_arch = "riscv64", target_arch = "aarch64"))]
fn cycle_probe_delta(first: u64, second: u64) -> u64 {
    second.wrapping_sub(first)
}

/// The `x86_64` twin, which cannot be a plain subtraction: the architectural fixed counters are
/// commonly 48 bits, so a wrap mid-window would otherwise produce a difference near 2^64 and print
/// a preposterous ratio rather than an error. `arch::x86_64::pmu` knows the width `CPUID` reported.
#[cfg(target_arch = "x86_64")]
fn cycle_probe_delta(first: u64, second: u64) -> u64 {
    crate::arch::pmu::elapsed_cycles(first, second)
}
