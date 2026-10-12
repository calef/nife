//! **What a context switch costs**: two threads yielding, and on `x86_64` the I/O-permission
//! bitmap a switch may have to carry.
//!
//! Cut out of `bench.rs` unchanged by lane `maintainer/last-three-over-2000`, which left the
//! parent an index (§282 (when a file is split, the parent becomes an index)) and under §266 (a
//! Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

use super::*;

/// **The context switch, round trip.** Two threads yielding to each other; each of our yields
/// is one switch out and (eventually) one switch back in. Ticks/iter ~= two switches.
pub(super) fn yield_switch() {
    static DONE: AtomicBool = AtomicBool::new(false);

    sched::spawn(|| {
        while !DONE.load(Ordering::Relaxed) {
            sched::yield_now();
        }
    })
    .expect("bench: no peer thread");

    for _ in 0..WARMUP {
        sched::yield_now();
    }
    timed("yield_switch", YIELD_ITERS, || {
        for _ in 0..YIELD_ITERS {
            sched::yield_now();
        }
    });
    DONE.store(true, Ordering::Relaxed);
    sched::yield_now(); // let the peer see the flag and exit
}

/// **What writing the TSS I/O permission bitmap costs on every switch-in** (`x86_64` only;
/// DECISIONS §121's amendment, 2026-08-24, "a micro-benchmark that writes an 8 KiB bitmap into the
/// current CPU's TSS on every switch, timed against a switch that does not").
///
/// The exact same two-thread yield ping-pong as [`yield_switch`] above, plus one
/// [`crate::arch::segments::bench_write_io_bitmap`] call on every resume, in both threads (a real
/// switch-in hook fires regardless of which thread it lands on, so writing only from the timed
/// loop would undercount by half). Reading this bench's `ns/iter` against `yield_switch`'s from the
/// same boot is the whole measurement: same iteration count, same two threads, same scheduler path,
/// one call different. See the module doc on [`bench_write_io_bitmap`][crate::arch::segments::bench_write_io_bitmap]
/// for what the write is (and is not) a stand-in for.
#[cfg(target_arch = "x86_64")]
pub(super) fn tss_iomap_switch() {
    use core::sync::atomic::AtomicU8;

    static DONE: AtomicBool = AtomicBool::new(false);
    // Folds every write's last byte in, so the optimizer cannot prove the bitmap writes are dead
    // even though nothing ever reads BENCH_IOMAP back through a port instruction.
    static SINK: AtomicU8 = AtomicU8::new(0);

    fn switch_and_write(pattern: &mut u8) {
        sched::yield_now();
        *pattern = pattern.wrapping_add(1);
        SINK.fetch_xor(
            crate::arch::segments::bench_write_io_bitmap(*pattern),
            Ordering::Relaxed,
        );
    }

    sched::spawn(|| {
        let mut pattern = 0u8;
        while !DONE.load(Ordering::Relaxed) {
            switch_and_write(&mut pattern);
        }
    })
    .expect("bench: no peer thread");

    let mut pattern = 0u8;
    for _ in 0..WARMUP {
        switch_and_write(&mut pattern);
    }
    timed("tss_iomap_switch", YIELD_ITERS, || {
        for _ in 0..YIELD_ITERS {
            switch_and_write(&mut pattern);
        }
    });
    DONE.store(true, Ordering::Relaxed);
    sched::yield_now(); // let the peer see the flag and exit
}

/// **What the LAZY TSS I/O-bitmap write costs on a switch that crosses a port holder** (`x86_64`
/// only; milestone 299, the measurement DECISIONS §121's 2026-08-25 refinement asked for). This is
/// **not** a second run of [`tss_iomap_switch`], which prices the naive always-write of the whole
/// 8 KiB bitmap. This prices what milestone 299 actually ships: [`set_port_range_grant`], which checks the
/// incoming grant against what the core already holds and writes only the bits that move.
///
/// The same two-thread ping-pong as [`yield_switch`], but each thread sets its own port grant on
/// resume, and the two disagree: the peer installs `None` (no ports), the main thread installs the
/// COM1 range. So **every** switch is a holder<->non-holder transition, which is the case that takes
/// the write, and reading this against `yield_switch` from the same boot is the cost of the lazy
/// write per switch that crosses a holder. It is the worst case, not the common one: the common one
/// is [`tss_iomap_lazy_nop`] below, where nothing crosses a holder and the write never happens.
///
/// The COM1 range `(0x3F8, 8)` is the real console driver's grant, so the byte count written is the
/// production one. Nothing executes `in`/`out` during the bench, so installing the grant is harmless;
/// the loop restores `None` at the end so the boot's TSS is left denying all ports.
#[cfg(target_arch = "x86_64")]
pub(super) fn tss_iomap_lazy_switch() {
    const COM1: Option<(u16, u16)> = Some((0x3F8, 8));
    static DONE: AtomicBool = AtomicBool::new(false);

    sched::spawn(|| {
        while !DONE.load(Ordering::Relaxed) {
            sched::yield_now();
            crate::arch::segments::set_port_range_grant(None); // the non-holder side
        }
    })
    .expect("bench: no peer thread");

    for _ in 0..WARMUP {
        sched::yield_now();
        crate::arch::segments::set_port_range_grant(COM1);
    }
    timed("tss_iomap_lazy_switch", YIELD_ITERS, || {
        for _ in 0..YIELD_ITERS {
            sched::yield_now();
            crate::arch::segments::set_port_range_grant(COM1); // the holder side, every iteration a transition
        }
    });
    DONE.store(true, Ordering::Relaxed);
    sched::yield_now(); // let the peer see the flag and exit
    crate::arch::segments::set_port_range_grant(None); // leave the TSS denying all ports
}

/// **What the lazy write costs when nothing holds a port** (`x86_64` only; milestone 299), which is
/// the case nearly every switch on nearly every machine actually takes. Both threads install `None`
/// on resume, so [`set_port_range_grant`] finds the core already holds `None` and returns after one
/// comparison, writing nothing. Read against `yield_switch`, the difference is the whole cost the
/// lazy form imposes on a system where nobody holds a port: a load and a branch. That gap, not
/// `tss_iomap_switch`'s ~2,682 ns, is what x86 pays for having the mechanism present.
#[cfg(target_arch = "x86_64")]
pub(super) fn tss_iomap_lazy_nop() {
    static DONE: AtomicBool = AtomicBool::new(false);

    sched::spawn(|| {
        while !DONE.load(Ordering::Relaxed) {
            sched::yield_now();
            crate::arch::segments::set_port_range_grant(None);
        }
    })
    .expect("bench: no peer thread");

    for _ in 0..WARMUP {
        sched::yield_now();
        crate::arch::segments::set_port_range_grant(None);
    }
    timed("tss_iomap_lazy_nop", YIELD_ITERS, || {
        for _ in 0..YIELD_ITERS {
            sched::yield_now();
            crate::arch::segments::set_port_range_grant(None);
        }
    });
    DONE.store(true, Ordering::Relaxed);
    sched::yield_now();
}
