//! **The bench boot**: every benchmark, in the fixed order a baseline is read in, then a halt.
//!
//! Cut out of `bench.rs` unchanged by lane `maintainer/last-three-over-2000`, which left the
//! parent an index (§282 (when a file is split, the parent becomes an index)) and under §266 (a
//! Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

use super::*;

/// Run every benchmark and halt. Never returns, never semihosts (see the module doc).
pub fn run() -> ! {
    println!();
    println!("bench: cntfrq {}", crate::arch::timer::frequency());
    // Which E3 image this is (milestone 134's layout control): nothing else on a card says, and E3
    // interleaves up to eight images through one card or one TFTP slot. A probe, so it is echoed
    // and never becomes a row. See kernel/src/fastpath_pad.rs for why the numbers are read as data.
    #[cfg(feature = "fastpath_pad")]
    {
        let [units, shift] = *core::hint::black_box(&crate::fastpath_pad::BUILD);
        println!("bench-probe: fastpath_pad units {units} shift {shift}");
    }

    // Milestone 134's per-IPC stack depth, in the build it is otherwise impossible to measure: the
    // release kernel radon boots. First, so nothing earlier in the suite is on any stack it reads,
    // and never in a timing build (the feature is off in every one; see kernel/Cargo.toml). The
    // rows after it still run, and their times are not results with this on.
    #[cfg(feature = "ipc_stack_depth")]
    {
        let bad =
            crate::ipc_stack_depth::kernel_thread_shapes() + crate::ipc_stack_depth::el0_shapes();
        println!("ipc-stack-depth: done ({bad} series were not measurements)");
    }

    yield_switch();
    #[cfg(target_arch = "x86_64")]
    tss_iomap_switch();
    #[cfg(target_arch = "x86_64")]
    tss_iomap_lazy_switch();
    #[cfg(target_arch = "x86_64")]
    tss_iomap_lazy_nop();
    ipc_rtt();
    relay_rtt();
    call_reply();
    broker_rtt();
    spawn_reap();
    map_new();
    #[cfg(target_arch = "riscv64")]
    rfence_self();
    // Milestone 74's riscv64 and aarch64 halves and milestone 309's x86_64 half.
    cycles_per_tick();
    coremark_compute();
    null_syscall_el0();
    ctx_switch_el0();
    ipc_rtt_el0();
    #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
    ipc_thread_scaling();
    #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
    app_displacement();
    sink_throughput();
    map_el0();
    spawn_el0();
    smp_throughput();
    fs_read();
    fs_throughput();
    fs_walk();
    fs_walk_bound();
    rg_search();

    println!("bench: done");
    // Parked, not exited: the host side saw the marker and tears QEMU down. `wfi`, so a
    // forgotten bench QEMU costs nothing while it waits to be killed (CLAUDE.md's rule).
    crate::arch::halt(crate::arch::HaltReason::measurement_boot());
}
