//! **A serial console for a board this repository cannot see**: open the port, log
//! every byte, recognise how far the boot got, and stop on a deadline. See notes/board-console.md.
//!
//! - [`port`] finds the adapter and puts it in raw mode at 115200 8N1.
//! - [`watch`] reads until the policy says stop, teeing every byte to a log file.
//! - [`progress`] decides, from the text alone, how far the boot got.
//! - [`screen`] turns a screendump of nife's framebuffer console back into text, so [`progress`]
//!   judges a monitor exactly as it judges a cable (six target machines have no serial port).
//! - [`board`] declares each board's firmware prologue, the one board-specific thing here.
//! - [`lottery`] reports what the thread-placement lottery drew in each boot of a many-boot log.
//! - [`exposure`] reads a soak capture into `notes/multicore-defect-curve.md`'s exposure row.
//! - [`stop`] is the only part that writes: one byte that ends a self-rebooting soak, sent only
//!   after the board announces an armed reboot loop, and logged in hex.
//!
//! **It writes only what a named mode sends, and logs every byte it sends** (calef, 2026-09-19).
//! The hazard is an open keyboard beside U-Boot's autoboot countdown, and an echoed line could
//! spell a marker, since the recogniser matches substrings; keep the write surface at named
//! commands. It does not touch power.
//!
//! It is excluded from Miri (`script/undefined-behavior-check`) for cost: its lib tests took 55
//! minutes under the interpreter, and it has no `unsafe` for Miri to check. Some [`port`] tests
//! need the host filesystem and some [`watch`] tests are wall-clock driven, so they fail there.
//!
//! # BUGS
//!
//! notes/board-console.md's BUGS lists them, in full in `notes/board-console/limitations.md`.
//!
//! Name: provisional, and ruled: calef ruled **`serial_console`** on 2026-09-13, pairing it with
//! `screen_console`. The block stays `provisional` until the rename is performed.

pub mod board;
pub mod exposure;
pub mod lottery;
pub mod port;
pub mod progress;
pub mod screen;
pub mod stop;
pub mod watch;

/// **Fixtures captured before milestone 297 renamed the console markers**, made readable by the
/// recogniser that reads the current ones.
///
/// `script/soak` became `script/soak-test` on 2026-09-14 (calef's ruling), and the markers moved
/// with it: `soak:` to `soak-test:`, `soak-census:` to `soak-test-census:`. Two files in
/// `tests/fixtures/captured/` are QEMU runs taken before that, and **they are left exactly as the
/// machine printed them**, because a capture is a record and this tree has already carried one
/// rewritten transcript for twelve days (`design/naming/rename-what-moves.md`, "a quotation never
/// moves").
///
/// So the transposition happens here, at test time, on a copy, and it is deliberately the narrowest
/// edit that makes the file parse: the marker prefix and nothing else. Every number, every field
/// and every other line is the machine's own.
///
/// **Why the recogniser itself does not simply match both spellings.** It did for `init/build`
/// (milestone 295) and had to: that marker survives only in a VisionFive 2 capture off real
/// silicon, which cannot be re-taken, so the live parser is the only thing that can ever read it.
/// These two are QEMU runs. What makes them unrepeatable is time rather than hardware, which is a
/// weaker claim, and it is a claim about *these files* rather than about anything a board will
/// print tomorrow. A permanent second spelling in the parser would be paying forever for a fact
/// that belongs to two files, and `captured/qemu-2026-09-14-riscv64-soak-test.log` is a real run in
/// the current vocabulary, so nothing here rests on hand-written text either.
///
/// Name: provisional (milestone 297 (`soak` becomes `soak-test`)): calef names public items, and
/// this one is `pub(crate)` and test-only, so it is the cheap end of that rule rather than an
/// exception to it.
#[cfg(test)]
pub(crate) fn respell_pre_297_markers(capture: &str) -> String {
    capture
        .replace("soak-census:", "soak-test-census:")
        .replace("soak: ", "soak-test: ")
}
