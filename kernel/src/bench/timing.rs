//! **The iteration counts and the timed window** every kernel-thread benchmark shares.
//!
//! Cut out of `bench.rs` unchanged by lane `maintainer/last-three-over-2000`, which left the
//! parent an index (§282 (when a file is split, the parent becomes an index)) and under §266 (a
//! Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

use super::*;

/// Iterations per benchmark. Fixed and part of the output, so a baseline is self-describing.
pub(super) const YIELD_ITERS: u64 = 2000;
pub(super) const IPC_ITERS: u64 = 1000;
pub(super) const RELAY_ITERS: u64 = 1000;
pub(super) const CALL_ITERS: u64 = 1000;
pub(super) const SPAWN_ITERS: u64 = 64;
pub(super) const MAP_ITERS: u64 = 64;
pub(super) const COREMARK_ITERS: u64 = 256;

/// Untimed shakeout before each measured loop: thread startup, first rendezvous, cold paths.
pub(super) const WARMUP: u64 = 32;

pub(super) fn timed(name: &str, iters: u64, f: impl FnOnce()) {
    // The monotonic counter, through the arch abstraction: CNTVCT_EL0 on aarch64, `rdtime` on
    // RISC-V. `frequency()` (printed by `run` as `cntfrq`) turns ticks into seconds.
    let t0 = crate::arch::timer::now();
    f();
    let t1 = crate::arch::timer::now();
    println!("bench: {name} {} {iters}", t1 - t0);
}
