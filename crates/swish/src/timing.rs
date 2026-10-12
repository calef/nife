//! **`time`: what the shell may say about how long something took** (milestone 86 (`time`: the
//! shell times a command)): [`write_duration`], [`write_timing`], and [`Untimed`] for the three
//! reasons there is no duration to print.
//!
//! Moved out of the crate root by milestone 884 (the shell logic crate is split along its seams).
//!
//! Name: provisional (milestone 884, 2026-10-12 UTC), for calef's naming pass.

use super::*;

/// **The three reasons a line was not timed**, printed instead of a duration.
///
/// Two of them are `date`'s, worded for the prefix position, because they are the same two facts
/// about the same page: this process holds no clock capability, or the machine has published no time
/// it believes. The third is the one the prefix position adds, and it is a fact about the line.
///
/// **A `time` that cannot measure does not run the command**, which is the decision worth naming
/// here rather than in a note. `time wc report.txt` is a request to measure something; running it
/// unmeasured and saying nothing would be DECISIONS §42's silent degradation with a stopwatch on it,
/// and running it while printing a complaint would leave a person guessing which half happened. So
/// the refusal is at the prompt, before anything is spawned, the way every other refusal in this
/// shell is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Untimed {
    /// `time` with nothing after it. There is no command to run, so there is nothing to time.
    ///
    /// The only variant, and it is a usage error rather than a capability one. `NoClock` and
    /// `UnknownClock` lived here until §72: they refused to measure on a machine whose wall clock
    /// was absent or unbelieved, which turned out to gate nothing, because a duration is two reads
    /// of an ambient counter. A `time` that cannot be refused has no other refusals to name.
    NothingToTime,
}

/// Write why a line was not timed. The two clock sentences are `date`'s with the program name
/// changed, deliberately: they name the same two causes and call for the same two fixes, so a person
/// who has read one has read the other.
pub fn write_untimed(r: Untimed, out: &mut dyn FnMut(&[u8])) {
    out(match r {
        Untimed::NothingToTime => b"  time: name a command to time: time <command>\n".as_slice(),
    });
}

/// **An elapsed duration, in the largest unit that keeps it readable**, with three digits after the
/// point and nothing after that.
///
/// Three units and no more: seconds, milliseconds, microseconds. A spawn on this system is
/// milliseconds and a builtin is microseconds, so those are the two a person meets; seconds is there
/// because a long job exists and `1234567.891 ms` is not a number anybody reads.
///
/// **Three digits is a rendering choice, not a resolution claim.** The counter ticks at 62.5 MHz on
/// the aarch64 board (16 ns) and 10 MHz on the RISC-V one (100 ns), so the last digit of a
/// microsecond reading is real on one board and rounded on the other. What it must never do is print
/// more precision than the arithmetic has, which is why there is no nanosecond unit here at all.
///
/// ```
/// use swish::write_duration;
///
/// let shown = |nanos| {
///     let mut s = Vec::new();
///     write_duration(nanos, &mut |b| s.extend_from_slice(b));
///     String::from_utf8(s).unwrap()
/// };
///
/// assert_eq!(shown(0), "0.000 us");
/// assert_eq!(shown(1_234), "1.234 us");
/// assert_eq!(shown(4_213_000), "4.213 ms");
/// assert_eq!(shown(90_500_000_000), "90.500 s");
/// ```
pub fn write_duration(nanos: u64, out: &mut dyn FnMut(&[u8])) {
    const MICRO: u64 = 1_000;
    const MILLI: u64 = 1_000_000;
    const SEC: u64 = 1_000_000_000;
    let (whole, thousandths, unit) = if nanos >= SEC {
        (nanos / SEC, (nanos % SEC) / MILLI, b" s".as_slice())
    } else if nanos >= MILLI {
        (nanos / MILLI, (nanos % MILLI) / MICRO, b" ms".as_slice())
    } else {
        (nanos / MICRO, nanos % MICRO, b" us".as_slice())
    };
    write_num(whole, out);
    out(b".");
    // Written digit by digit rather than through [`write_num`], because the fraction is **zero
    // padded** and that function writes no leading zeros: `4.13 ms` and `4.013 ms` are different
    // numbers and only one of them is what happened. `thousandths` is below 1000 by construction
    // (every branch above divides its remainder by the next unit down), so three digits is exact.
    out(&[
        b'0' + (thousandths / 100 % 10) as u8,
        b'0' + (thousandths / 10 % 10) as u8,
        b'0' + (thousandths % 10) as u8,
    ]);
    out(unit);
}

/// **The line `time` prints when it measured something.**
///
/// `real` is Unix's word for it and it is the honest one: this is wall-clock time between the shell
/// deciding to run the line and the line being over. It is **not** CPU time, and there is no `user`
/// or `sys` row to go with it, because nothing in this kernel is asked how much processor a thread
/// consumed. That is a scheduler fact and it is unqueried today; if it arrives it is another row
/// here, not another command.
///
/// The number is a difference of two reads of the monotonic counter (§72), so there is nothing to
/// disclaim: the old signature took a `stepped` flag because a wall clock corrected mid-command
/// made the answer a difference of readings on two different clocks, and a stopwatch that is
/// sometimes wrong and never says so is worse than no stopwatch. A monotonic counter cannot be
/// stepped, so that whole failure mode, and the flag that reported it, are gone.
pub fn write_timing(nanos: u64, out: &mut dyn FnMut(&[u8])) {
    out(b"  time: real ");
    write_duration(nanos, out);
    out(b"\n");
}

#[cfg(test)]
mod tests;
