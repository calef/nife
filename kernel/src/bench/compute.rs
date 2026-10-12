//! **The compute workload** (milestone 19e (run a real workload)): CoreMark, which measures the
//! core rather than nife.
//!
//! Cut out of `bench.rs` unchanged by lane `maintainer/last-three-over-2000`, which left the
//! parent an index (§282 (when a file is split, the parent becomes an index)) and under §266 (a
//! Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

use super::*;

/// **The compute workload (milestone 19e), for the record.** Unlike the paths above, this touches
/// no OS primitive: it is pure computation (the CoreMark-derived kernel, `crates/coremark`), so its
/// cost is the *core's*, not nife's. It is here because the same crate runs as an EL0 workload
/// and later on macOS and Linux, and this line is where the nife compute number is recorded,
/// on the same two instruments as everything else. Running it in the kernel is fine: compute is
/// privilege-independent, so this number equals the EL0 workload's. `SINK` keeps it live.
pub(super) fn coremark_compute() {
    static SINK: AtomicU64 = AtomicU64::new(0);
    timed("coremark", COREMARK_ITERS, || {
        let crc = coremark::run(COREMARK_ITERS as u32);
        SINK.store(crc as u64, Ordering::Relaxed);
    });
}
