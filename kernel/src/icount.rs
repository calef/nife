//! **The instruction-count instrument**: a boot mode that asserts timing claims in a unit a busy
//! host cannot move (notes/instruction-clock.md). From inside a guest, a slow handler and a
//! descheduled emulator are the same observation (notes/load-sensitive-assertions.md). Under
//! `-icount shift=0,sleep=off`, QEMU's virtual clock advances exactly one nanosecond per retired
//! guest instruction and nothing else, so a claim in instructions cannot be falsified by load.
//!
//! 1. **The timer fired at the deadline the kernel armed.** On riscv64 SBI's `set_timer` is
//!    write-only, so nothing else proves firmware was armed with `DEADLINE`.
//! 2. **The handler takes fewer than N instructions**, deadline to next arm: the quantity
//!    `MISSED_TICKS` only approximates.
//! 3. **Zero missed ticks**: in virtual time a miss cannot be the emulator.
//! 4. **The deadline advances by exactly one interval per delivered tick.** Claims 1 to 3 compare
//!    arrival against the deadline that fired, so a kernel that re-anchors its whole grid from
//!    `now` passes them byte-identically while the delivered rate falls; this law catches it, with
//!    no retry loop because claim 3 already made the window miss-free.
//!
//! It is opt-in, and not for speed (compute runs at about the same rate): every vCPU shares one
//! virtual clock, so an idle hart in `wfi` jumps it and multi-hart timing becomes fiction (hence
//! `-smp 1`), and a clock-bound wait costs instructions, about 3 s of wall clock per 0.64 s of
//! virtual time. So it is its own boot, feature and command, like `script/bench`, and the
//! `tick_trace` hooks are `#[cfg(feature = "icount")]`, absent from test and shipping builds. The
//! feature implies `bench`, because both boots park at the same point (kernel/Cargo.toml).
//!
//! Every quantity is in instructions: one counter tick is `1e9 / frequency()` nanoseconds, so
//! aarch64's 62.5 MHz counter reads in steps of 16 and riscv64's 10 MHz `rdtime` in steps of 100.
//! That resolution is printed, since it decides what a change can show. The boot prints
//! `icount: done` and parks in `wfi`; `xtask icount` owns and ends the QEMU child.
//!
//! *Provisional names throughout: the `icount` feature, this module, `tick_trace`,
//! `script/icount`.*

use crate::println;

/// Timer ticks to sample before deciding. Sixty-four is enough for a max to mean something and
/// cheap in wall clock: the guest spins through 64 tick periods of virtual time, which is 0.64 s of
/// virtual time and therefore ~6.4x10^8 guest instructions, the dominant cost of this boot.
const SAMPLE_TICKS: u64 = 64;

/// Iterations of the calibration loop. Each iteration is exactly two instructions on both ISAs, so
/// this is a 2,000,000-instruction window: long enough that the fixed cost around it is noise, short
/// enough (2 ms of virtual time) to sit well inside one 10 ms tick period most of the time.
const CALIBRATION_ITERS: u64 = 1_000_000;

/// **Per-tick timing, recorded from the timer handler itself.**
///
/// Three relaxed stores and a few relaxed read-modify-writes per tick, in trap context, which is
/// DECISIONS §9's rule for handlers (record and defer; never work, never print). Compiled in only
/// under the `icount` feature, so the test and shipping handlers are untouched by it: a handler that
/// carried this on every path would be measuring a handler that does not ship.
///
/// One set of counters rather than one per CPU, deliberately: this boot runs `-smp 1` because a
/// shared virtual clock makes multi-hart timing fictional (notes/benchmarks.md), and [`record`]
/// ignores any hart but the boot one so a stray secondary cannot silently pollute a mean.
pub mod tick_trace {
    use core::sync::atomic::{AtomicU64, Ordering};

    static COUNT: AtomicU64 = AtomicU64::new(0);
    static ARRIVAL_SUM: AtomicU64 = AtomicU64::new(0);
    static ARRIVAL_MAX: AtomicU64 = AtomicU64::new(0);
    static ARRIVAL_MIN: AtomicU64 = AtomicU64::new(u64::MAX);
    static HANDLER_SUM: AtomicU64 = AtomicU64::new(0);
    static HANDLER_MAX: AtomicU64 = AtomicU64::new(0);
    static EARLY: AtomicU64 = AtomicU64::new(0);

    /// One tick's numbers, all in counter ticks.
    ///
    /// - `fired`: the deadline this interrupt was armed against, read before the re-arm overwrites
    ///   it. On aarch64 that is `CNTV_CVAL_EL0`, which the hardware itself consults; on riscv64 it
    ///   is the `DEADLINE` word the kernel handed to SBI, which is the whole point of claim 1.
    /// - `arrival`: the counter as the handler read it, before it did anything about the tick.
    /// - `done`: the counter after the next deadline is armed, which on riscv64 is after the SBI
    ///   `ecall` has returned.
    ///
    /// Called from `rearm` on both ISAs. Anything that is not the boot hart is dropped rather than
    /// mixed in.
    pub(crate) fn record(fired: u64, arrival: u64, done: u64) {
        if crate::cpu::id() != 0 {
            return;
        }
        // An arrival *before* its own deadline would mean the machine fired early, which is a
        // finding rather than a measurement, so it is counted separately instead of being folded
        // into a mean as a zero.
        let Some(arrival_latency) = arrival.checked_sub(fired) else {
            EARLY.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let handler = done.wrapping_sub(fired);

        COUNT.fetch_add(1, Ordering::Relaxed);
        ARRIVAL_SUM.fetch_add(arrival_latency, Ordering::Relaxed);
        ARRIVAL_MAX.fetch_max(arrival_latency, Ordering::Relaxed);
        ARRIVAL_MIN.fetch_min(arrival_latency, Ordering::Relaxed);
        HANDLER_SUM.fetch_add(handler, Ordering::Relaxed);
        HANDLER_MAX.fetch_max(handler, Ordering::Relaxed);
    }

    /// Ticks recorded since the last [`reset`].
    pub(crate) fn count() -> u64 {
        COUNT.load(Ordering::Relaxed)
    }

    /// Interrupts that arrived before the deadline they were armed against. Should be zero; a
    /// nonzero value means the emulator, not the kernel, and the boot says so rather than averaging
    /// it away.
    pub(crate) fn early() -> u64 {
        EARLY.load(Ordering::Relaxed)
    }

    /// `(count, arrival_min, arrival_mean, arrival_max, handler_mean, handler_max)`, in counter
    /// ticks. Means are integer divisions; the caller converts to instructions.
    pub(crate) fn snapshot() -> (u64, u64, u64, u64, u64, u64) {
        let n = COUNT.load(Ordering::Relaxed).max(1);
        (
            COUNT.load(Ordering::Relaxed),
            ARRIVAL_MIN.load(Ordering::Relaxed),
            ARRIVAL_SUM.load(Ordering::Relaxed) / n,
            ARRIVAL_MAX.load(Ordering::Relaxed),
            HANDLER_SUM.load(Ordering::Relaxed) / n,
            HANDLER_MAX.load(Ordering::Relaxed),
        )
    }

    /// Drop everything recorded so far. Called once the boot is ready to measure, so the ticks that
    /// landed during console bring-up and secondary startup are not in the sample.
    pub fn reset() {
        COUNT.store(0, Ordering::Relaxed);
        ARRIVAL_SUM.store(0, Ordering::Relaxed);
        ARRIVAL_MAX.store(0, Ordering::Relaxed);
        ARRIVAL_MIN.store(u64::MAX, Ordering::Relaxed);
        HANDLER_SUM.store(0, Ordering::Relaxed);
        HANDLER_MAX.store(0, Ordering::Relaxed);
        EARLY.store(0, Ordering::Relaxed);
    }
}

/// Instructions per counter tick under `-icount shift=0`: one tick is `1e9 / frequency()`
/// nanoseconds and one nanosecond is one instruction.
///
/// This is the instrument's resolution, and it is exact only because both machines' counter
/// frequencies divide a gigahertz: 62.5 MHz gives 16 and 10 MHz gives 100. A board whose counter
/// did not would make every number here a rounded one, so the division is checked rather than
/// assumed.
fn instructions_per_tick() -> u64 {
    let freq = crate::arch::timer::frequency();
    assert!(freq > 0, "icount: the counter frequency is zero");
    assert!(
        1_000_000_000u64.is_multiple_of(freq),
        "icount: the counter frequency ({freq} Hz) does not divide 1 GHz, so a counter tick is not \
         a whole number of instructions and every number this boot prints would be rounded"
    );
    1_000_000_000 / freq
}

/// **Prove we are actually on the instrument before measuring anything with it.**
///
/// The failure this prevents is the one this repository keeps writing comments about: a set
/// variable naming a missing file, an env var that silently did nothing, a device that was not
/// attached. Booted without `-icount shift=0` this kernel runs perfectly well and every number
/// below becomes a wall-clock number wearing an instruction's units, which is worse than an error
/// because it looks like a measurement.
///
/// The check is direct. The arch layer runs a loop of exactly `2 * CALIBRATION_ITERS + 1`
/// instructions and says so; under the instrument the counter must advance by exactly that many
/// nanoseconds. A tolerance of 1% covers the one timer interrupt that may land inside the window
/// (a tick handler is a four-figure number of instructions against a seven-figure window) and is
/// nowhere near wide enough to admit either of the other two ways this kernel is ever run: plain
/// TCG retires far fewer than one instruction per nanosecond of wall clock, and HVF on this host
/// retires several.
fn calibrate(per_tick: u64) {
    let before = crate::arch::timer::now();
    let expected = crate::arch::timer::calibration_loop(CALIBRATION_ITERS);
    let observed = (crate::arch::timer::now() - before) * per_tick;

    println!("icount: calibration {observed} {expected}");
    let slack = expected / 100;
    assert!(
        observed >= expected - slack && observed <= expected + slack,
        "icount: this boot is not running under -icount shift=0. A loop of exactly {expected} \
         instructions advanced virtual time by {observed} ns, and on the instrument those two are \
         the same number. Run it through `script/icount`, which passes the flag."
    );
}

/// Run the claims and park. Never returns, never semihosts (the bench boot's contract, and for the
/// same reason: `xtask` owns the child on both accelerators).
pub fn run() -> ! {
    let per_tick = instructions_per_tick();
    println!();
    println!("icount: cntfrq {}", crate::arch::timer::frequency());
    println!("icount: instructions_per_counter_tick {per_tick}");
    println!(
        "icount: tick_interval {} {}",
        crate::arch::timer::interval(),
        crate::arch::timer::interval() * per_tick
    );

    calibrate(per_tick);

    // Everything before this line was console bring-up, secondary startup and a calibration loop,
    // and the ticks that landed during it are not the ticks under test.
    tick_trace::reset();
    let missed_before = crate::arch::timer::missed_ticks();

    // A consistent (ticks, deadline) pair, for claim 4. The timer handler runs to completion
    // between two of this thread's instructions (`-smp 1`, and nothing else is running), so the
    // two values it maintains are never observed half-updated; what a naive pair of reads CAN
    // straddle is a whole tick landing in between. Re-read the count until it brackets the
    // deadline unchanged, which is the same guard the suite's twin uses.
    let snapshot = || loop {
        let t = tick_trace::count();
        let d = crate::arch::timer::deadline();
        if tick_trace::count() == t {
            break (t, d);
        }
    };

    let (ticks_before, deadline_before) = snapshot();
    while tick_trace::count() < ticks_before + SAMPLE_TICKS {
        core::hint::spin_loop();
    }
    let (ticks_after, deadline_after) = snapshot();
    let missed = crate::arch::timer::missed_ticks() - missed_before;
    let ticks_measured = ticks_after - ticks_before;
    let deadline_delta = deadline_after - deadline_before;

    let (count, arrival_min, arrival_mean, arrival_max, handler_mean, handler_max) =
        tick_trace::snapshot();
    let early = tick_trace::early();

    // Every number in instructions, which is the only unit in this boot that means anything.
    println!("icount: ticks {count}");
    println!(
        "icount: arrival_instructions min {} mean {} max {}",
        arrival_min * per_tick,
        arrival_mean * per_tick,
        arrival_max * per_tick
    );
    println!(
        "icount: handler_instructions mean {} max {}",
        handler_mean * per_tick,
        handler_max * per_tick
    );
    println!("icount: missed_ticks {missed}");
    println!("icount: early_arrivals {early}");
    println!(
        "icount: deadline_delta {} expected {}",
        deadline_delta,
        ticks_measured * crate::arch::timer::interval()
    );

    // **Claim 1: the timer fired at the deadline the kernel armed.**
    //
    // `arrival - fired` is the interrupt's delivery latency plus the trap path, and on the
    // instrument it is a property of this kernel's code alone. What it rules out is the thing no
    // wall-clock test on either ISA can: a timer armed with something other than the grid cell the
    // kernel recorded. Such an implementation re-anchors on every tick, so its arrival latency
    // grows by a handler's worth of instructions per tick and leaves this bound within a handful of
    // ticks, while `DEADLINE` and `CNTV_CVAL_EL0` keep reading back exactly as the re-arm law
    // demands.
    //
    // The bound is `ARRIVAL_BOUND` in the arch layer, because the trap path is the arch's own and
    // the two ISAs do not have the same one. Measured first, then set with margin; the run prints
    // what it saw, so the margin is visible rather than asserted about.
    assert_eq!(
        early, 0,
        "icount: {early} interrupts arrived BEFORE the deadline they were armed against, which is \
         the emulator rather than this kernel"
    );
    assert!(
        arrival_max * per_tick <= crate::arch::timer::ARRIVAL_BOUND,
        "icount: an interrupt arrived {} instructions after the deadline it was armed against, over \
         a bound of {}. On this instrument that is the kernel's own path length, not the host: \
         either the trap path grew, or the timer was armed with something other than the deadline \
         the kernel recorded.",
        arrival_max * per_tick,
        crate::arch::timer::ARRIVAL_BOUND
    );

    // **Claim 2: the handler takes fewer than N instructions.**
    //
    // Measured deadline-to-armed, so it covers interrupt delivery, the trap path, the tick
    // bookkeeping and the re-arm (on riscv64 including the SBI `ecall` round trip). That span is
    // what `MISSED_TICKS` counts indirectly: a miss is this number exceeding one tick period. The
    // missed-tick assertions on both ISAs could only ever say "it did not exceed 10 ms", and could
    // not tell a handler that took 10 ms from an emulator that was not running for 10 ms. This says
    // what it actually is.
    assert!(
        handler_max * per_tick <= crate::arch::timer::HANDLER_BOUND,
        "icount: the timer handler took {} instructions from deadline to re-arm, over a bound of \
         {}. Nothing about the host can move this number.",
        handler_max * per_tick,
        crate::arch::timer::HANDLER_BOUND
    );

    // **And the third, which closes a BUGS entry rather than adding a claim.** The miss taxonomy on
    // both ISAs calls a re-arm less than one interval late a slow handler and a whole interval or
    // more the emulator, and notes/load-sensitive-assertions.md records that the cut leaves a window
    // one tick period wide in which a host deschedule is still blamed on this kernel. Virtual time
    // has no deschedules, so here a miss has exactly one possible cause and needs no taxonomy.
    assert_eq!(
        missed, 0,
        "icount: {missed} deadlines had already passed when the handler re-armed. Under -icount \
         the host cannot cause that, so it is the handler taking longer than a whole tick period."
    );

    // **Claim 4: the deadline advanced by exactly one interval per delivered tick.**
    //
    // The re-arm law, and the reason it is asserted here rather than left to the suite is that
    // claims 1-3 cannot see a violation of it at all: a kernel that re-anchors the grid from `now`
    // arms the timer with the very word it records, so the arrival latency stays at its clean-run
    // value on every tick while the delivered rate collapses. Milestone 62 measured exactly that
    // (the table in this module's header), and until 2026-08-18 the roadmap and both notes said
    // this instrument was strictly stronger than the suite's drift test. It was not.
    //
    // No retry loop, and that is claim 3 paying for itself: a miss re-anchors the grid on purpose
    // (`rearm`'s safety valve), so the law only holds over a miss-free window, and `missed == 0` was
    // asserted a few lines above. The suite's twin has to hunt for such a window because a loaded
    // host manufactures misses; virtual time cannot.
    assert_eq!(
        deadline_delta,
        ticks_measured * crate::arch::timer::interval(),
        "icount: {ticks_measured} ticks moved the armed deadline off the grid. Over a window with \
         no missed tick the deadline advances by exactly one interval per tick; re-arming relative \
         to `now` inside the handler instead of from the deadline that fired does exactly this, and \
         nothing else in this boot can see it."
    );

    println!("icount: done");
    crate::arch::halt(crate::arch::HaltReason::measurement_boot())
}
