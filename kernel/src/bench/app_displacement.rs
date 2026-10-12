//! **Milestone 134 (the register of measures), tier A, E4: what an IPC-heavy kernel costs an
//! application's working set.**
//!
//! Cut out of `bench.rs` unchanged by lane `maintainer/last-three-over-2000`, which left the
//! parent an index (§282 (when a file is split, the parent becomes an index)) and under §266 (a
//! Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

use super::*;

/// Working-set sizes for [`app_displacement()`] (E4), in KiB: below, at, and above a typical L1d
/// (32 KB) and into L2 territory, so the sweep can show whether displacement tracks a specific
/// cache level rather than growing uniformly.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
const APPDISP_WORKINGSET_KIB: &[usize] = &[4, 16, 32, 64, 128];
/// Bytes per working-set word; the workload reads and writes `u64`s.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
const APPDISP_WORD_BYTES: usize = 8;
/// The largest working set above, in words: the static scratch buffer's size.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
const APPDISP_MAX_WORDS: usize = 128 * 1024 / APPDISP_WORD_BYTES;
/// Roughly this many word-touches per measurement, regardless of working-set size, so every point
/// in the sweep takes about the same wall time: `passes = TARGET / words`.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
const APPDISP_TARGET_TOUCHES: u64 = 8_000_000;
/// Background IPC pairs during the "with traffic" batch: 16 threads, a realistic concurrent load
/// (comfortably inside [`SCALE_PAIRS`]'s own sweep, so its curve says what this load costs the
/// kernel's own IPC path; this bench asks what the same load costs an unrelated application).
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
const APPDISP_IPC_PAIRS: usize = 8;
/// Background IPC pairs for the second, higher-load condition: [`SCALE_MAX_PAIRS`] itself, 96
/// threads, the same pair count E1's own sweep tops out at and "3x past the predicted knee" by
/// that sweep's own doc comment. The register of milestone 134 (the register of measures) named
/// this the missing half of E4, in a BUGS entry closed on 2026-08-23 that now lives in
/// `notes/register-of-measures/cache-experiments.md`: the 8-pair condition sits inside E1's flat region, so
/// a null result there is expected from E1's own curve rather than independent evidence, and a
/// load nearer or past the knee is the stronger version of the experiment.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
const APPDISP_IPC_PAIRS_HIGH: usize = SCALE_MAX_PAIRS;

/// Interior-mutable scratch the workload reads and writes. One `Racy<T>` per benchmark file:
/// `sched.rs`'s corruption canary (`memory_corruption_canary_gate::arm`) defines the same idiom for the same reason,
/// a scratch region one thread at a time uses, serialized by the caller rather than by a lock that
/// would then be part of the very cost this benchmark measures the absence of.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
struct Racy<T>(core::cell::UnsafeCell<T>);
// SAFETY: access is serialized by `appdisp_batch`: the workload thread is the only one ever handed
// a reference into this buffer, and one batch's workload always finishes (and its reference drops)
// before the next batch's does (see `appdisp_batch`'s reap-to-baseline wait, which runs after
// `appdisp_workload` returns).
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
unsafe impl<T> Sync for Racy<T> {}

#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
static APPDISP_BUFFER: Racy<[u64; APPDISP_MAX_WORDS]> =
    Racy(core::cell::UnsafeCell::new([0; APPDISP_MAX_WORDS]));

#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
static APPDISP_STOP: AtomicBool = AtomicBool::new(false);

/// One background IPC pair for the "with traffic" batch: a server that echoes until the sentinel,
/// and a client that sends-then-receives until [`APPDISP_STOP`], then releases the server. Same
/// sentinel idiom as [`ipc_rtt`], parameterized on a stop flag instead of an iteration count
/// because this pair must outlive an unknown number of the workload's passes.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
fn appdisp_background_pair(rq: sched::RendezvousId, rp: sched::RendezvousId) {
    sched::spawn(move || {
        loop {
            let m = sched::ipc_receive(rq);
            if m[0] == u64::MAX {
                break;
            }
            sched::ipc_send(rp, [m[0], 0, 0]);
        }
    })
    .expect("bench: no appdisp background server");
    sched::spawn(move || {
        while !APPDISP_STOP.load(Ordering::Relaxed) {
            sched::ipc_send(rq, [1, 0, 0]);
            sched::ipc_receive(rp);
        }
        sched::ipc_send(rq, [u64::MAX, 0, 0]);
    })
    .expect("bench: no appdisp background client");
}

/// **The application**: read-modify-write `words` entries of [`APPDISP_BUFFER`], `passes` times,
/// self-timed. Yields every so often (about 64 times over the whole run, regardless of `passes`)
/// so a cooperative scheduler interleaves it with any background IPC pairs: without a voluntary
/// yield, a compute-only thread that never traps could run to completion before anything else
/// gets to, which would make "concurrent" IPC traffic not actually concurrent. The same yields run
/// in the solo condition too, so the two conditions differ only in whether anything else is ready
/// to run, not in the workload's own control flow.
#[inline(never)]
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
fn appdisp_workload(words: usize, passes: u64) -> u64 {
    // SAFETY: the caller (`appdisp_batch`) guarantees this thread is the only one with a reference
    // into the buffer for the duration of this call; see `Racy`'s doc.
    let whole: &mut [u64; APPDISP_MAX_WORDS] = unsafe { &mut *APPDISP_BUFFER.0.get() };
    let buf = &mut whole[..words];
    let yield_every = (passes / 64).max(1);
    let mut sum: u64 = 0;
    let t0 = crate::arch::timer::now();
    for p in 0..passes {
        for (i, word) in buf.iter_mut().enumerate() {
            sum = sum.wrapping_add(*word).wrapping_add(i as u64);
            *word = sum;
        }
        if p % yield_every == 0 {
            sched::yield_now();
        }
    }
    let ticks = crate::arch::timer::now() - t0;
    core::hint::black_box(sum);
    ticks
}

/// One measurement: as many background IPC pairs as `req`/`reply` have slots for (zero for the
/// solo condition), running for exactly as long as [`appdisp_workload`] takes, then released.
/// Returns the workload's own ticks, which is the number E4 cares about: what the APPLICATION's
/// throughput did, not what the kernel's did.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
fn appdisp_batch(
    words: usize,
    passes: u64,
    req: &[sched::RendezvousId],
    reply: &[sched::RendezvousId],
) -> u64 {
    APPDISP_STOP.store(false, Ordering::SeqCst);
    let base = sched::thread_count();
    for i in 0..req.len() {
        appdisp_background_pair(req[i], reply[i]);
    }
    // Let the background pairs reach their first blocking RECEIVE before the clock starts, so their
    // cold-start cost lands outside the timed window, same reason every other batch here warms up.
    for _ in 0..8 {
        sched::yield_now();
    }
    let ticks = appdisp_workload(words, passes);
    APPDISP_STOP.store(true, Ordering::SeqCst);
    while sched::thread_count() > base {
        sched::yield_now();
    }
    ticks
}

/// Batches per measurement point; the minimum ticks is kept. Same methodology as [`tp_best`] and
/// the same reason ([`tp_best`]'s doc, `smp_throughput`'s methodology note), turned up from
/// [`TP_REPEAT`]'s 4 to 5: a first unrepeated run of this bench showed swings of 2 to 3x between
/// nominally identical conditions (a host-scheduling artifact of a batch running for a much longer
/// window than a ping-pong pipeline does, so it has more chances to catch a host preemption), which
/// is exactly the least-contended-sample problem `tp_best` exists to solve, just louder here.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
const APPDISP_REPEAT: usize = 5;

/// Minimum ticks over [`APPDISP_REPEAT`] batches: the least host-contended sample.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
fn appdisp_best(
    words: usize,
    passes: u64,
    req: &[sched::RendezvousId],
    reply: &[sched::RendezvousId],
) -> u64 {
    let mut best = u64::MAX;
    for _ in 0..APPDISP_REPEAT {
        best = best.min(appdisp_batch(words, passes, req, reply));
    }
    best
}

/// **E4: application working-set displacement, the Liedtke measurement proper.** Everything else
/// in this file measures the KERNEL's cost; this measures the cost the kernel imposes on an
/// APPLICATION after the syscall returns, which is what Liedtke's argument was actually about and
/// which no kernel-side benchmark can see by construction.
///
/// The "application" is [`appdisp_workload`]: a tunable working set it reads and writes
/// repeatedly, timed alone and then timed again with [`APPDISP_IPC_PAIRS`] background IPC pairs
/// running concurrently on the same core, and a third time with [`APPDISP_IPC_PAIRS_HIGH`], a load
/// near/at E1's own knee. The throughput lost between conditions, swept over
/// [`APPDISP_WORKINGSET_KIB`], is the number: how much an IPC-heavy kernel costs an unrelated
/// application's cache, not how much it costs the kernel's own IPC path.
///
/// The high-load condition is the register's own follow-up, from a BUGS entry that said a load
/// nearer E1's knee was the stronger experiment and had not been taken (closed 2026-08-23; now in
/// `notes/register-of-measures/cache-experiments.md`). The original 8-pair run (16 threads) sits
/// inside E1's flat region, where E1 found no cost on the dev Mac, so a null result there was
/// expected rather than informative. 48 pairs (96 threads) puts the background load past E1's
/// knee: on the dev Mac E1 was flat to 16 threads and rose 8-11% by 64-96, and on radon
/// (2026-09-04) the knee is at 16 threads, 1.68x. That is where this experiment can distinguish
/// "the kernel's own path got slower" from "the application's cache got evicted". The register's
/// "not a knee" is about E4's own result at this load (a small, consistent 2-9% displacement on
/// the dev Mac), not about E1.
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
pub(super) fn app_displacement() {
    if !real_single_hart_or_skip("app_displacement") {
        return;
    }

    let mut req = [0u64; APPDISP_IPC_PAIRS_HIGH];
    let mut reply = [0u64; APPDISP_IPC_PAIRS_HIGH];
    for i in 0..APPDISP_IPC_PAIRS_HIGH {
        req[i] = sched::create_rendezvous();
        reply[i] = sched::create_rendezvous();
    }

    for &kib in APPDISP_WORKINGSET_KIB {
        let words = kib * 1024 / APPDISP_WORD_BYTES;
        let passes = (APPDISP_TARGET_TOUCHES / words as u64).max(1);
        let solo = appdisp_best(words, passes, &[], &[]);
        let with_ipc = appdisp_best(
            words,
            passes,
            &req[..APPDISP_IPC_PAIRS],
            &reply[..APPDISP_IPC_PAIRS],
        );
        let with_ipc_high = appdisp_best(words, passes, &req, &reply);
        println!("bench: appdisp_{kib}k_solo {solo} {passes}");
        println!("bench: appdisp_{kib}k_ipc {with_ipc} {passes}");
        println!("bench: appdisp_{kib}k_ipc96 {with_ipc_high} {passes}");
        let lost_pct = |busy: u64| {
            if solo > 0 {
                ((busy as i64 - solo as i64) * 100) / solo as i64
            } else {
                0
            }
        };
        println!(
            "bench-probe: appdisp_{kib}k_throughput_lost_pct {}",
            lost_pct(with_ipc)
        );
        println!(
            "bench-probe: appdisp_{kib}k_highload_throughput_lost_pct {}",
            lost_pct(with_ipc_high)
        );
    }
}
