//! **Multi-hart aggregate throughput** (DECISIONS §28 (SMP placement)): independent pipelines
//! across every core, measured by wall clock.
//!
//! Cut out of `bench.rs` unchanged by lane `maintainer/last-three-over-2000`, which left the
//! parent an index (§282 (when a file is split, the parent becomes an index)) and under §266 (a
//! Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

use super::*;

// --- Multi-hart aggregate throughput (DECISIONS §28, the SMP placement win) ---
//
// Every primitive above is hart-pinned by design: the icount instrument boots `-smp 1` (the
// 2026-07-28 attribution finding, notes/benchmarks.md), so those numbers are per-core path length
// and cannot show §28's placement work at all. This one is different, and its methodology has to be
// different, so it is set apart here on purpose.
//
// It runs N independent ping-pong pipelines and measures the WALL-CLOCK time to complete them. A
// single pipeline is a synchronous rendezvous: only one of its two threads is runnable at a time,
// so it keeps ~one core busy and §28's local-wake rule keeps the pair co-located and warm. N
// independent pipelines are N such streams, which §28's power-of-two placement scatters across the
// cores at spawn. So the aggregate throughput of N pipelines should approach `online_count()` times
// a single pipeline's throughput: the whole machine filled, which is the property §28 exists to
// deliver and the one no hart-pinned primitive can see.
//
// **Why it is NOT on the icount baseline, and never gates.** Two reasons, both structural:
//   1. It is meaningful only with more than one hart. The icount instrument pins `-smp 1`, and so
//      does the default `--real` run (per-core magnitudes; see xtask's `bench`), so it runs ONLY on
//      the `--real --smp` boot (HVF, 4 harts), which the harness already forbids from
//      `--check`/`--save`. Everywhere else `online_count()` is 1 and it is skipped outright, so it
//      never touches `bench/baseline-aarch64.txt`.
//   2. Under `-icount` all vCPUs share one virtual clock (again the 2026-07-28 finding), so a
//      wall-clock throughput number is not even defined there; and TCG serialises vCPUs onto one
//      host thread, so there is no real parallelism to measure. Only HVF gives each core its own
//      counter and real concurrent execution.
//
// So this is a statistical `--real` measurement read by a human with loose bounds, exactly like the
// other HVF magnitudes, not a deterministic tick baseline. It reports two lines, `smp_pipe_solo`
// (one pipeline) and `smp_pipe_all` (N pipelines); the scaling factor is the solo ns/iter divided
// by the all ns/iter, and it should land near `online_count()`.
//
// **The solo baseline needs the main thread to block** (a real `RECEIVE`, as `ipc_rtt` does), not
// busy-yield: a runnable main makes the scheduler scatter the solo pair, every rendezvous becomes a
// cross-core wake, and the derived scaling goes superlinear, which is not physical. Each batch is
// also run a few times and the **minimum** ticks kept: on a shared desktop under HVF the host
// preempts guest vCPUs, and the min is the least-contended sample, the closest thing to the true
// rate. Loose bounds, read by a human.

/// Independent ping-pong pipelines. 16 on a 4-hart boot is four per core, enough that placement and
/// stealing fill every core and the aggregate is not starved by too few streams.
const TP_PIPES: usize = 16;
/// Round trips per pipeline in a batch. Large enough that spawn and first-rendezvous costs (paid
/// before the clock starts, behind the GO barrier) are noise against the timed steady state.
pub(super) const TP_RTT: u64 = 2000;
/// Batches per measurement; the minimum ticks is kept (least host contention under HVF).
pub(super) const TP_REPEAT: usize = 4;

static TP_GO: AtomicBool = AtomicBool::new(false);

/// Run `pipes` independent ping-pong pipelines over the pre-created endpoint pairs and return the
/// wall-clock ticks to complete them all. The clock spans only steady state: threads are spawned
/// first, then released together by `TP_GO` after `now()` is read, so N-pipeline batches are not
/// charged N times the spawn cost that a 1-pipeline batch pays only once. `done` collects one signal
/// per pipeline so the main thread can **block** (not busy-yield) through the timed window.
pub(super) fn tp_batch(
    req: &[sched::RendezvousId],
    reply: &[sched::RendezvousId],
    done: sched::RendezvousId,
    pipes: usize,
) -> u64 {
    TP_GO.store(false, Ordering::SeqCst);
    let base = sched::thread_count();

    for i in 0..pipes {
        let (rq, rp) = (req[i], reply[i]);
        // The server half: exactly TP_RTT receive-then-send, then it returns and is reaped. It blocks
        // in `ipc_receive` until its client sends, so it effectively starts when the client does.
        sched::spawn(move || {
            for _ in 0..TP_RTT {
                let _ = sched::ipc_receive(rq);
                sched::ipc_send(rp, [1, 0, 0]);
            }
        })
        .expect("bench: throughput server spawn failed");

        // The client half: wait at the barrier, then TP_RTT send-then-receive, then signal done. Yield
        // (not spin) at the barrier so a waiting client does not burn its core before the clock.
        sched::spawn(move || {
            while !TP_GO.load(Ordering::Acquire) {
                sched::yield_now();
            }
            for _ in 0..TP_RTT {
                sched::ipc_send(rq, [1, 0, 0]);
                let _ = sched::ipc_receive(rp);
            }
            sched::ipc_send(done, [1, 0, 0]);
        })
        .expect("bench: throughput client spawn failed");
    }

    // Start the clock, release every client in one step, then BLOCK receiving one done per pipeline.
    // Blocking (not yield-spinning) is what keeps the solo pair co-located; see the methodology note.
    let t0 = crate::arch::timer::now();
    TP_GO.store(true, Ordering::Release);
    for _ in 0..pipes {
        let _ = sched::ipc_receive(done);
    }
    let ticks = crate::arch::timer::now() - t0;

    // Drain: let the servers finish their last iterations and every thread reap before the next
    // batch, so a batch's timing is never charged the previous batch's teardown.
    while sched::thread_count() > base {
        sched::yield_now();
    }
    ticks
}

/// Minimum ticks over `TP_REPEAT` batches of `pipes` pipelines: the least host-contended sample.
pub(super) fn tp_best(
    req: &[sched::RendezvousId],
    reply: &[sched::RendezvousId],
    done: sched::RendezvousId,
    pipes: usize,
) -> u64 {
    let mut best = u64::MAX;
    for _ in 0..TP_REPEAT {
        best = best.min(tp_batch(req, reply, done, pipes));
    }
    best
}

/// Inner iterations per compute worker. Sized so a single worker runs a few hundred microseconds
/// under HVF, long enough that the 41 ns counter grain and host jitter are noise against the batch.
const TC_WORK: u64 = 300_000;

static TC_SINK: AtomicU64 = AtomicU64::new(0);

/// A non-elidable integer grind: an LCG mixed with an xorshift, folded into a returned value the
/// caller sinks so the optimizer cannot delete the loop. Pure compute, no syscalls, so a running
/// worker touches no other core: this is the workload that isolates §28 **placement** from IPC's
/// cross-core wakes, which is exactly why the pipeline result and this one differ under HVF.
#[inline(never)]
fn busy(iters: u64) -> u64 {
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    for i in 0..iters {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407 ^ i);
        x ^= x >> 29;
    }
    x
}

/// Run `workers` independent CPU-bound workers behind the GO barrier and return the wall-clock ticks
/// to finish them all. Each worker does the same fixed grind, so N workers is N times the work of
/// one; §28 placement should spread them so the aggregate finishes in about `ceil(N/cores)` worker-
/// times, i.e. aggregate throughput approaches `cores` times a single worker's. No rendezvous during
/// the grind, so no cross-core wakes: the clean placement measurement. Main blocks on `done`.
fn tc_batch(done: sched::RendezvousId, workers: usize) -> u64 {
    TP_GO.store(false, Ordering::SeqCst);
    let base = sched::thread_count();
    for _ in 0..workers {
        sched::spawn(move || {
            while !TP_GO.load(Ordering::Acquire) {
                sched::yield_now();
            }
            let v = busy(TC_WORK);
            TC_SINK.fetch_add(v, Ordering::Relaxed);
            sched::ipc_send(done, [1, 0, 0]);
        })
        .expect("bench: compute worker spawn failed");
    }
    let t0 = crate::arch::timer::now();
    TP_GO.store(true, Ordering::Release);
    for _ in 0..workers {
        let _ = sched::ipc_receive(done);
    }
    let ticks = crate::arch::timer::now() - t0;
    while sched::thread_count() > base {
        sched::yield_now();
    }
    ticks
}

/// Minimum ticks over `TP_REPEAT` compute batches: the least host-contended sample.
fn tc_best(done: sched::RendezvousId, workers: usize) -> u64 {
    let mut best = u64::MAX;
    for _ in 0..TP_REPEAT {
        best = best.min(tc_batch(done, workers));
    }
    best
}

/// **Aggregate multi-hart throughput.** See the block comment above for the methodology and why this
/// is a `--real`-only, non-gating measurement. Skipped on a single hart (the icount boot), where it
/// would measure nothing §28 does.
///
/// Two workloads, on purpose. **Compute** (`smp_compute_*`) is N independent CPU-bound workers: no
/// syscalls, so no cross-core wakes, so the host keeps every busy vCPU on a real core and the
/// aggregate scales cleanly to `cores`. This is the direct picture of §28 placement filling the
/// machine. **Pipelines** (`smp_pipe_*`) is N synchronous IPC ping-pong pairs, and under HVF it does
/// NOT scale, it goes slightly backwards: a semi-idle guest vCPU is descheduled by the host, so the
/// cross-core wakes an IPC pipeline needs whenever placement/stealing splits a pair pay host
/// reschedule latency that a single co-located pair never does. That is a virtualization property,
/// not a scheduler defect (it is the same reason the icount primitive suite is pinned to one hart),
/// and recording both is the honest result: compute parallelises on this instrument, synchronous IPC
/// does not, and the reason is the host underneath.
pub(super) fn smp_throughput() {
    let cpus = crate::smp::online_count();
    if cpus <= 1 {
        return; // single hart (the icount instrument): there is no placement win to show.
    }

    // Rendezvous pairs, created once and reused across batches so a repeated run does not leak the
    // endpoint table down. One request and one reply endpoint per pipeline, plus a shared done EP.
    let mut req = [0u64; TP_PIPES];
    let mut reply = [0u64; TP_PIPES];
    for i in 0..TP_PIPES {
        req[i] = sched::create_rendezvous();
        reply[i] = sched::create_rendezvous();
    }
    let done = sched::create_rendezvous();

    // Compute: the clean placement win. Warm once, then measure solo and the full machine.
    let _ = tc_batch(done, 1);
    let compute_solo = tc_best(done, 1);
    let compute_all = tc_best(done, TP_PIPES);

    // Pipelines: the IPC workload, which does not parallelise under HVF (see the doc comment).
    let _ = tp_batch(&req, &reply, done, 1);
    let pipe_solo = tp_best(&req, &reply, done, 1);
    let pipe_all = tp_best(&req, &reply, done, TP_PIPES);

    // Each `*_all` is TP_PIPES times the work of its `*_solo`. The scaling factor is solo ns/iter
    // divided by all ns/iter: near `cores` for compute (the machine filled), near or below 1 for
    // pipelines under HVF. The `smp_cpus` line records the ceiling.
    println!("bench: smp_compute_solo {compute_solo} {TC_WORK}");
    println!(
        "bench: smp_compute_all {compute_all} {}",
        TP_PIPES as u64 * TC_WORK
    );
    println!("bench: smp_pipe_solo {pipe_solo} {TP_RTT}");
    println!(
        "bench: smp_pipe_all {pipe_all} {}",
        TP_PIPES as u64 * TP_RTT
    );
    println!("bench: smp_cpus {cpus} {cpus}");
}
