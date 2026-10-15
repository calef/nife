//! **The filesystem benchmarks**: a read through the confined stack, throughput, directory
//! walks, and a search.
//!
//! Cut out of `bench.rs` unchanged by lane `maintainer/last-three-over-2000`, which left the
//! parent an index (§282 (when a file is split, the parent becomes an index)) and under §266 (a
//! Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

use super::*;

/// **The userspace file-server tax** (DECISIONS §32, the flagship userspace-reuse story). This is the
/// number a microkernel skeptic asks for about a filesystem in userspace: a client opens a file
/// through a granted *directory capability* and reads a block, over the real confined stack, a block
/// server driving the RedoxFS disk by DMA and an FS server (the vendored RedoxFS engine, `no_std`, on
/// its own heap) mounting it over blk IPC. `kernel/src/user/fs_service.rs` wires all three; the
/// client (`fixtures/src/fs_test_client/bench.rs`, `ROLE_BENCH`) times a warm read loop and reports `[ticks, iters]`.
///
/// **Why it is `--real`-only and never gates, unlike the primitives.** The FS server's mount is
/// device-driven: hundreds of block reads gated on the disk's completion interrupt, plus the engine's
/// own logic. Under `-icount shift=0` that path is not deterministic (interrupt timing is not part of
/// the instruction clock), so an icount baseline for it would enshrine exactly the non-determinism the
/// 2026-07-28 lesson warns against. So it runs only on the `--real --smp` boot (HVF, where the whole
/// stack is proven by the `redoxfs_server` test), self-skipping everywhere else via the same
/// `online_count() > 1` gate as the throughput bench, so `bench/baseline-aarch64.txt` never sees it.
///
/// **What the number means: whole-path cost, dominated by the device.** ~204 us/read (HVF,
/// `--release --smp`). A read is **not** served warm from a cache: it goes to the block server, which
/// does a DMA transfer and waits on the disk's completion interrupt, roughly 200 us per block under
/// HVF. That swamps the FS server's own IPC-contract tax, which `relay_rtt` puts at a few hundred
/// *nanoseconds*. So this is the honest whole-path cost of a userspace file read, not an isolated
/// server tax, which is exactly the case milestone 21's rule names: when device latency swamps the
/// isolation, measure the whole path and say so rather than report a fictional isolated number.
///
/// The isolated file-server cost was attempted and abandoned, and this comment used to describe that
/// abandoned design (claiming a warm cache and a contract-cost measurement), which contradicted
/// notes/benchmarks.md for long enough to mislead a reader. Corrected here: a few hundred ns of
/// serving layer sitting on a ~200 us block read with its own run-to-run spread puts the delta inside
/// the device noise. The per-hop tax lives in `relay_rtt`, where it is measurable and gated.
///
/// **Since milestone 606 (a directory walk costs what it does on Linux), the read after the first
/// is served from the FS server's memory**, like a Linux buffered read from the page cache, so this
/// row measures the confined IPC path plus a copy (about 260 ns under HVF, release, 2026-09-26)
/// and no longer the device. The history above is kept because it is why the memo exists. The
/// device path is still what `fs_seq_read` and its siblings measure: their file is 1 MiB, over
/// the memo's 256 KiB limit.
///
/// **What it is not:** a filesystem throughput number. There is no MB/s figure and no comparison
/// against ext4 or APFS, which is DECISIONS §34's unmet condition 2 and milestone 38.
pub(super) fn fs_read() {
    // Same gate as the throughput bench: meaningful only on the `--real --smp` boot, which is where
    // the RedoxFS disk is attached and the whole stack is proven. Single hart (icount, default
    // `--real`) skips, so this never reaches the deterministic baseline.
    if crate::smp::online_count() <= 1 {
        return;
    }
    // The three binaries the service needs. The block server is `block_driver` on every
    // architecture since milestone 291. Absent any of them, or the RedoxFS disk, skip.
    let (Some(blk_image), Some(redoxfs_server), Some(fs_test_client)) = (
        crate::trust::require_program("block_driver"),
        crate::trust::require_program("redoxfs_server"),
        crate::trust::require_program("fs_test_client"),
    ) else {
        return;
    };
    // Spawn the block server, the FS server, and the client in its ROLE_BENCH (timed) role.
    let Some((readiness, report)) =
        crate::user::fs_service::start(blk_image, redoxfs_server, fs_test_client, 1)
    else {
        return; // no RedoxFS disk on this run
    };
    // Sequence on readiness, exactly as the test does: the block server brings the device up, then
    // the FS server mounts the image, then the client's timed loop reports. The bench boot is the
    // only caller, so it always gets the readiness endpoints (nothing wired the service first).
    if let Some((blk_ready, ready)) = readiness {
        // `blk_ready` is `None` when the block service was already running and its sentinel has
        // been taken; a bench boot always wires it, so this arm is the one that runs here.
        if let Some(blk_ready) = blk_ready {
            let _ = sched::ipc_receive(blk_ready);
        }
        let _ = sched::ipc_receive(ready);
    }
    let [ticks, iters, ..] = sched::ipc_receive(report);
    println!("bench: fs_read {ticks} {iters}");
}

/// **Filesystem throughput through the confined FS server** (milestone 38, DECISIONS §34
/// condition 2). Six phases over one file the client creates on the RedoxFS image: sequential
/// write, sequential read, random read, a **record-aligned** read (the cheapest 4 KiB read RedoxFS
/// can serve, which is what makes the record-geometry claim a measurement rather than arithmetic),
/// random write, and the cost of producing a page of payload, which the write phases pay inside
/// their window and a reader should be able to subtract. `fs_read` above answers "what does one file
/// read cost"; this answers "how fast does this architecture move a file", which is the question a
/// microkernel skeptic actually asks and the one the tree had no number for at all.
///
/// **It reports both a row and a probe line, and the split is deliberate.** The row
/// (`bench: fs_seq_read <ticks> <transfers>`) is the harness's ordinary shape, so the table's
/// ns/iter column is the per-request latency, which is the number that compares to `fs_read`. The
/// probe line carries MiB/s, which is the number that compares to another operating system, and it
/// is a probe rather than a row because the baseline gate must never see any of this: like
/// `fs_read`, every phase here is device- and interrupt-driven, so it self-skips off the
/// `--real --smp` boot and could not be deterministic if it did not.
///
/// **One transfer is 4096 bytes and cannot be more**, because a `filesystem_protocol` request carries its
/// payload through the one page the client shares with the server. So these figures are a request
/// rate in disguise, and any comparison against a system whose client may pass a 64 KiB buffer is
/// comparing two different things. notes/benchmarks.md states that next to the numbers rather than
/// under them.
pub(super) fn fs_throughput() {
    // The same gate as `fs_read`: this is meaningful only on the `--real --smp` boot, which is the
    // one that attaches the RedoxFS disk and proves the stack.
    if crate::smp::online_count() <= 1 {
        return;
    }
    let (Some(blk_image), Some(redoxfs_server), Some(fs_test_client)) = (
        crate::trust::require_program("block_driver"),
        crate::trust::require_program("redoxfs_server"),
        crate::trust::require_program("fs_test_client"),
    ) else {
        return;
    };
    // `start` reuses the service `fs_read` already wired, so this spawns one more client on the
    // same FS server and the same shared page: `readiness` comes back `None` and there is nothing
    // to wait for but the reports.
    let Some((readiness, report)) = crate::user::fs_service::start(
        blk_image,
        redoxfs_server,
        fs_test_client,
        filesystem_fixture::throughput::ROLE,
    ) else {
        return; // no RedoxFS disk on this run
    };
    if let Some((blk_ready, ready)) = readiness {
        // `blk_ready` is `None` when the block service was already running and its sentinel has
        // been taken; a bench boot always wires it, so this arm is the one that runs here.
        if let Some(blk_ready) = blk_ready {
            let _ = sched::ipc_receive(blk_ready);
        }
        let _ = sched::ipc_receive(ready);
    }
    let hz = crate::arch::timer::frequency();
    for _ in 0..filesystem_fixture::throughput::PHASES {
        let [ticks, transfers, phase, ..] = sched::ipc_receive(report);
        let Some(name) = filesystem_fixture::throughput::name(phase) else {
            println!("bench-probe: fs_throughput unknown phase {phase}");
            continue;
        };
        println!("bench: {name} {ticks} {transfers}");
        // MiB/s, in integer arithmetic on the kernel's side because the kernel is where the counter
        // frequency and the transfer size are both known. `bytes * hz / ticks` is bytes per second
        // and the shift turns that into MiB/s; the extra factor of 100 buys two decimal places,
        // without which the interesting figures here (1.52 and 2.59) both print as "2". Ticks of
        // zero would mean the counter did not advance across a whole phase, which is a broken run
        // rather than a fast one, so it reports zero instead of dividing.
        let bytes = transfers * filesystem_fixture::throughput::UNIT as u64;
        let hundredths = bytes
            .checked_mul(hz)
            .and_then(|v| v.checked_mul(100))
            .and_then(|v| v.checked_div(ticks))
            .map(|v| v >> 20)
            .unwrap_or(0);
        println!(
            "bench-probe: {name}_mib_per_s {}.{:02}",
            hundredths / 100,
            hundredths % 100
        );
    }
}

/// **A directory walk through a confined grant** (milestone 606 (a directory walk costs what it
/// does on Linux), provisional number). `std_exerciser` holds the tree milestone 121 (`ripgrep` on nife: enumeration as a
/// capability) priced, behind an
/// `fs_subtree_caretaker`, with the rights the confined `rg pattern src/` will hold, and walks it
/// with plain `std::fs` (`walk_pricing`): list every directory, read every file. The row is the
/// median of that program's warm walks; the probes are the rest of what it printed, the split by
/// `std::fs` call first. This is the release-kernel figure the kernel suite cannot give (its kernel
/// is a debug build), and the one `bench/host/run_linux_walk.sh` prices Linux against on the same
/// `virt,accel=hvf` machine and the same virtio disk tier.
///
/// `--real --smp` only, for `fs_read`'s reasons: it is device- and interrupt-driven, so it never
/// reaches the icount baseline. Skips when the boot has no RedoxFS disk or the archive no
/// `std_exerciser` (it is built with `-Zbuild-std` against `nife-dev`; the `--smp` leg builds it).
///
/// # BUGS
///
/// - **Warm only.** The program's first walk counts the tree and fills the FS server's memo; the
///   row is a walk after that, which is what a repeated `rg` or `find` is. A cold walk is a
///   different number, and Linux's `drop_caches` would be the matching control.
pub(super) fn fs_walk() {
    if crate::smp::online_count() <= 1 {
        return;
    }
    let (Some(blk), Some(server), Some(caretaker), Some(walker)) = (
        crate::trust::require_program("block_driver"),
        crate::trust::require_program("redoxfs_server"),
        crate::trust::require_program("fs_subtree_caretaker"),
        crate::trust::require_program("std_exerciser"),
    ) else {
        println!("bench-probe: fs_walk skipped (a program is missing from the archive)");
        return;
    };
    use filesystem_protocol::dir;
    let Some(spawned) = crate::user::fs_service::start_std_narrowed(
        blk,
        server,
        caretaker,
        walker,
        filesystem_fixture::walk::ROOT,
        dir::ENUMERATE | dir::READ | dir::DESCEND,
        None,
    ) else {
        return; // no RedoxFS disk on this run
    };
    walk_rows(spawned.report, "fs_walk");
}

/// **The same walk through a grant the FS server enforces itself** (milestone 606, calef's
/// ruling D): no caretaker, the program holding the server's endpoint with a badge the kernel
/// bound to the subtree. The row beside `fs_walk` is the price of the caretaker's hop. It takes
/// the grant back afterwards, so a later row starts from the same server state.
pub(super) fn fs_walk_bound() {
    if crate::smp::online_count() <= 1 {
        return;
    }
    let (Some(blk), Some(server), Some(walker)) = (
        crate::trust::require_program("block_driver"),
        crate::trust::require_program("redoxfs_server"),
        crate::trust::require_program("std_exerciser"),
    ) else {
        return;
    };
    use filesystem_protocol::dir;
    let Some(spawned) = crate::user::fs_service::start_std_bound(
        blk,
        server,
        walker,
        filesystem_fixture::walk::ROOT,
        dir::ENUMERATE | dir::READ | dir::DESCEND,
    ) else {
        return;
    };
    walk_rows(spawned.report, "fs_walk_bound");
    while crate::sched::is_thread_present(spawned.thread) {
        sched::yield_now();
    }
    if !spawned.release() {
        println!("bench-probe: fs_walk_bound could not take its grant back");
    }
}

/// Warm `rg` runs [`rg_search`] times, after one untimed run that fills the FS server's memo. Odd,
/// so the median is a run.
const RG_SEARCH_RUNS: usize = 5;

/// **Unmodified `ripgrep` searching the priced tree** (milestone 121, `ripgrep` on nife), the
/// search half of the walk [`fs_walk`] prices. `rg` holds the same grant behind the same caretaker
/// and is told `fixture::walk::RG_SEARCH` as its argv. The row is ripgrep's own `--stats` figure,
/// the median of [`RG_SEARCH_RUNS`] warm runs: it starts after argument parsing and ends after the
/// last file, so this system's process creation and image loading are not in it, and neither are
/// Linux's in `bench/host/run_linux_rg.sh`, which runs the same words through the same function
/// on the same machine. The spawn-to-exit time, which does include them, is a probe beside it.
///
/// Present only when somebody ran `cargo xtask basalt-fetch`: `rg` is never part of the base image
/// (calef, 2026-10-07), so the row skips on every ordinary bench boot and in CI.
///
/// # BUGS
///
/// - **One process per run.** Each run is a fresh `rg`, so its heap, its allocator and its regex
///   compilation are cold every time; only the FS server is warm. That is what a person typing
///   `rg` twice gets, on both sides.
pub(super) fn rg_search() {
    if crate::smp::online_count() <= 1 {
        return;
    }
    let (Some(blk), Some(server), Some(caretaker), Some(rg)) = (
        crate::trust::require_program("block_driver"),
        crate::trust::require_program("redoxfs_server"),
        crate::trust::require_program("fs_subtree_caretaker"),
        crate::trust::require_program("rg"),
    ) else {
        println!("bench-probe: rg_search skipped (no rg in the archive; cargo xtask basalt-fetch)");
        return;
    };
    use filesystem_protocol::dir;
    use filesystem_fixture::walk as tree;
    let hz = crate::arch::timer::frequency();
    let mut searched = [0u64; RG_SEARCH_RUNS];
    let mut whole = [0u64; RG_SEARCH_RUNS];
    for run in 0..=RG_SEARCH_RUNS {
        let started = crate::arch::timer::now();
        let Some(spawned) = crate::user::fs_service::start_std_narrowed(
            blk,
            server,
            caretaker,
            rg,
            tree::ROOT,
            dir::ENUMERATE | dir::READ | dir::DESCEND,
            Some(tree::RG_SEARCH.as_bytes()),
        ) else {
            return; // no RedoxFS disk on this run
        };
        let mut text = [0u8; 8192];
        let len = drain_report(spawned.report, &mut text);
        while crate::sched::is_thread_present(spawned.thread) {
            sched::yield_now();
        }
        let ended = crate::arch::timer::now();
        if !spawned.release() {
            println!("bench-probe: rg_search could not take its grant back");
            return;
        }
        let text = core::str::from_utf8(&text[..len]).unwrap_or("");
        let Some(ns) = rg_seconds(text) else {
            println!("bench-probe: rg_search printed no stats ({len} bytes)");
            return;
        };
        if run == 0 {
            // The counts, once, so a reader can see the search found what the kernel test asserts.
            for line in text
                .lines()
                .filter(|l| l.contains("files searched") || l.contains("matches"))
            {
                println!("bench-probe: rg_search {line}");
            }
            continue;
        }
        searched[run - 1] = ns;
        whole[run - 1] = (ended - started) * 1_000_000_000 / hz;
    }
    searched.sort_unstable();
    whole.sort_unstable();
    let mid = RG_SEARCH_RUNS / 2;
    println!("bench: rg_search {} 1", searched[mid] * hz / 1_000_000_000);
    println!(
        "bench-probe: rg_search {} ns by rg's own clock, {} to {}",
        searched[mid],
        searched[0],
        searched[RG_SEARCH_RUNS - 1]
    );
    println!(
        "bench-probe: rg_search spawn to exit {} ns, {} to {}",
        whole[mid],
        whole[0],
        whole[RG_SEARCH_RUNS - 1]
    );
}

/// `rg --stats`'s last line, `<seconds> seconds` with six decimals, in nanoseconds.
fn rg_seconds(text: &str) -> Option<u64> {
    let line = text.lines().rev().find(|l| l.ends_with(" seconds"))?;
    let (whole, frac) = line.strip_suffix(" seconds")?.split_once('.')?;
    let mut ns = whole.parse::<u64>().ok()? * 1_000_000_000;
    let mut scale = 100_000_000;
    for b in frac.bytes() {
        ns += u64::from(b.checked_sub(b'0')?) * scale;
        scale /= 10;
    }
    Some(ns)
}

/// A program's whole transcript from its stdout sink, as far as `out` holds it.
fn drain_report(report: crate::sched::RendezvousId, out: &mut [u8]) -> usize {
    let mut len = 0usize;
    loop {
        let words = sched::ipc_receive(report);
        let mut chunk = [0u8; byte_sink_protocol::INLINE_MAX];
        match byte_sink_protocol::unpack(words[0], words[1], words[2], &mut chunk) {
            byte_sink_protocol::Msg::Bytes(n) => {
                let take = n.min(out.len() - len);
                out[len..len + take].copy_from_slice(&chunk[..take]);
                len += take;
            }
            byte_sink_protocol::Msg::Eof | byte_sink_protocol::Msg::Malformed => return len,
        }
    }
}

/// Drain a priced walk's transcript from `report` and print it as the `row` and its probes.
fn walk_rows(report: crate::sched::RendezvousId, row: &str) {
    // The transcript, decoded from the program's stdout sink and kept only as far as this buffer.
    let mut text = [0u8; 2048];
    let mut len = 0usize;
    loop {
        let words = sched::ipc_receive(report);
        let mut chunk = [0u8; byte_sink_protocol::INLINE_MAX];
        match byte_sink_protocol::unpack(words[0], words[1], words[2], &mut chunk) {
            byte_sink_protocol::Msg::Bytes(n) => {
                let take = n.min(text.len() - len);
                text[len..len + take].copy_from_slice(&chunk[..take]);
                len += take;
            }
            byte_sink_protocol::Msg::Eof => break,
            byte_sink_protocol::Msg::Malformed => {
                println!(
                    "bench-probe: fs_walk malformed sink message {:#x}",
                    words[0]
                );
                return;
            }
        }
    }
    let text = core::str::from_utf8(&text[..len]).unwrap_or("");
    let hz = crate::arch::timer::frequency();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("walk whole ") {
            let ns: u64 = rest
                .split(' ')
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
            // The program timed in nanoseconds; the row is in counter ticks like every other.
            println!("bench: {row} {} 1", ns * hz / 1_000_000_000);
        }
        if let Some(rest) = line.strip_prefix("walk ") {
            println!("bench-probe: {row} {rest}");
        }
    }
}
