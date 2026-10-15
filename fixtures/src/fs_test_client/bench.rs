//! **The FS-server read benchmark** (`ROLE_BENCH`).
//!
//! Cut out of `fs_test_client.rs` unchanged on 2026-10-12 (UTC) by lane
//! maintainer/files-over-1500, for §282 (when a file is split, the parent becomes an index). What
//! another role uses is `pub(crate)` here.
//!
//! Name: provisional (lane maintainer/files-over-1500, 2026-10-12 UTC), for the module and the
//! file.

use filesystem_fixture as fixture;
use user_mode_runtime::{exit, now, send};

use crate::channel::{REPORT, open, read};

/// Iterations for the timed read loop (`ROLE_BENCH`). Warmup pays the `OPEN`; the timed loop then
/// reads the same block over and over. Self-timed, reported home like
/// `os_primitives_benchmarker`'s primitives.
///
/// **"Warm, not disk latency" is what this comment used to claim, and it was wrong when it was
/// written.** Through milestone 138 step 1 there was no warm: `IpcDisk` was a bare `Disk` with no
/// cache around it, so re-reading one block re-read it from the device every time, and the ~204 us
/// this reported was the device round trip. kernel/src/bench.rs and notes/benchmarks.md were
/// corrected on 2026-08-04 and this copy of the claim survived; milestone 38 found it by measuring
/// a *sequential* read against it and getting the same per-block cost, which is exactly what a path
/// with no cache does.
///
/// **It is warm now, and this time the word is accurate.** Step 2 wrapped `IpcDisk` in
/// `redoxfs_server::CachedDisk`, which answers a repeated single-block read (the tree walk this loop's
/// every request repeats, since `motd` lives inline in its node and rides entirely on that walk)
/// from memory. Measured (`sh bench/cache-slots-sweep.sh`): 210,490 ns to 9,474, 22.2x. This role's
/// own report still reads correctly as "the FS-server contract's cost above a raw endpoint call",
/// only warm rather than cold now; `motd` is 69 bytes and lives inline in its node, so this reads
/// the node rather than a record.
const BENCH_ITERS: u64 = 2000;
const BENCH_WARMUP: u64 = 64;

/// **The FS-server read benchmark** (the userspace-server tax, DECISIONS §32). Open `motd` once, then
/// time a loop of reads of its first block and report `[ticks, iters]` on the report endpoint, the
/// `os_primitives_benchmarker` shape. The kernel's bench boot prints it as `fs_read`; against the bare `ipc_rtt_el0`
/// round trip, the difference is what the FS-server contract costs above a raw endpoint call. It is a
/// `--real`-only magnitude (the mount is device-driven, not deterministic under icount); see
/// notes/benchmarks.md and kernel/src/bench.rs.
pub(crate) fn bench() -> ! {
    let handle = open(fixture::MOTD_NAME);
    let block = fixture::MOTD.len();
    for _ in 0..BENCH_WARMUP {
        let _ = read(handle, 0, block);
    }
    let start = now();
    for _ in 0..BENCH_ITERS {
        let _ = read(handle, 0, block);
    }
    let ticks = now().wrapping_sub(start);
    // slot 1 is REPORT; carry ticks and the iteration count, the bench line's two fields.
    send(REPORT, ticks, BENCH_ITERS, 0);
    exit();
}
