//! **The line discipline engine: a sans-IO editor for a serial terminal** (milestone 28).
//!
//! This crate is the pure logic of the layer Unix calls the tty line discipline: byte in,
//! echo bytes out, completed lines out. It performs no IO and knows nothing about IPC, UARTs,
//! or endpoints; the `line_editor` userspace component feeds it one byte at a time and forwards
//! whatever it emits. That split is DECISIONS §7 applied: the editing rules are host-tested in
//! milliseconds (with a small terminal model, see the tests), and only the wiring needs QEMU.
//!
//! # Examples
//!
//! Typing a line, editing it, and getting it back. The [`Sink`] is where echo goes; here it is a
//! `Vec`, and in the running system it is an endpoint.
//!
//! ```
//! use line_editor::{Event, LineDisc, Sink};
//!
//! struct Echo(Vec<u8>);
//! impl Sink for Echo {
//!     fn put(&mut self, bytes: &[u8]) {
//!         self.0.extend_from_slice(bytes);
//!     }
//! }
//!
//! let mut out = Echo(Vec::new());
//! let mut ld = LineDisc::new();
//! ld.start_line(b"$ ", &mut out);
//!
//! // Type "13", then fix it: ^B moves the cursor left one, onto the 3, and typing 2 **inserts**
//! // before it rather than overwriting it.
//! for b in b"13" {
//!     assert_eq!(ld.feed(*b, &mut out), Event::None);
//! }
//! ld.feed(0x02, &mut out); // ^B
//! ld.feed(b'2', &mut out);
//!
//! // Carriage return ends the line. The caller reads it before feeding the next byte.
//! assert_eq!(ld.feed(b'\r', &mut out), Event::Line);
//! assert_eq!(ld.line(), b"123");
//!
//! // The prompt was echoed, and so was everything typed.
//! assert!(out.0.starts_with(b"$ "));
//! ```
//!
//! Three control characters mean things a caller has to act on rather than echo, and `^D`'s double
//! duty is the one worth demonstrating: on an empty line it ends input, and on a non-empty line it is
//! an ordinary delete. That is Unix's behaviour, and it is a decision rather than an accident.
//!
//! ```
//! # use line_editor::{Event, LineDisc, Sink};
//! # struct Echo(Vec<u8>);
//! # impl Sink for Echo {
//! #     fn put(&mut self, bytes: &[u8]) { self.0.extend_from_slice(bytes); }
//! # }
//! let mut out = Echo(Vec::new());
//! let mut ld = LineDisc::new();
//! ld.start_line(b"$ ", &mut out);
//!
//! // ^D on an empty line: end of input.
//! assert_eq!(ld.feed(0x04, &mut out), Event::Eof);
//!
//! // ^C discards whatever was in progress.
//! ld.start_line(b"$ ", &mut out);
//! for b in b"rm -rf" {
//!     ld.feed(*b, &mut out);
//! }
//! assert_eq!(ld.feed(0x03, &mut out), Event::Interrupt);
//!
//! // On a non-empty line, ^D deletes at the cursor instead. ^A goes to the start first.
//! ld.start_line(b"$ ", &mut out);
//! for b in b"xls" {
//!     ld.feed(*b, &mut out);
//! }
//! ld.feed(0x01, &mut out); // ^A: home
//! assert_eq!(ld.feed(0x04, &mut out), Event::None); // deletes the 'x', does NOT end input
//! assert_eq!(ld.feed(b'\r', &mut out), Event::Line);
//! assert_eq!(ld.line(), b"ls");
//! ```
//!
//! An LF straight after a CR is swallowed, which is what makes one engine serve both an interactive
//! terminal (CR) and piped CRLF text without the caller knowing which it has:
//!
//! ```
//! # use line_editor::{Event, LineDisc, Sink};
//! # struct Echo(Vec<u8>);
//! # impl Sink for Echo {
//! #     fn put(&mut self, bytes: &[u8]) { self.0.extend_from_slice(bytes); }
//! # }
//! let mut out = Echo(Vec::new());
//! let mut ld = LineDisc::new();
//! ld.start_line(b"", &mut out);
//!
//! ld.feed(b'a', &mut out);
//! assert_eq!(ld.feed(b'\r', &mut out), Event::Line);
//! assert_eq!(ld.line(), b"a");
//!
//! // The LF of the CRLF pair is not a second, empty line.
//! assert_eq!(ld.feed(b'\n', &mut out), Event::None);
//! ```
//!
//! # What it implements
//!
//! - Insertion and deletion at a cursor, with minimal echo for the common case (typing at the
//!   end of the line) and a full input-region redraw for everything else.
//! - Cursor movement: left/right/home/end, as control characters (^B ^F ^A ^E) and as ANSI
//!   sequences (CSI D/C/H/F, SS3 variants, and the vt220 `~` forms).
//! - Kill and yank: ^K (to end), ^U (to start), ^W (word back), ^Y (insert the kill buffer).
//! - History: an in-ring of the last [`HIST`] non-empty lines, browsed with up/down arrows;
//!   the in-progress line is stashed and restored when you browse back past the newest entry.
//! - Line endings: CR or LF ends a line; an LF immediately after a CR is swallowed, so both
//!   interactive terminals (CR) and piped text (LF, CRLF) work.
//! - ^C discards the line in progress ([`Event::Interrupt`]); ^D on an empty line is
//!   end-of-input ([`Event::Eof`]), on a non-empty line it deletes at the cursor, exactly the
//!   Unix double duty.
//! - ^L clears the screen and repaints the prompt and the line.
//!
//! # What it deliberately does not do
//!
//! - **No terminal size tracking.** The redraw math assumes the line fits one terminal row.
//!   `noline` probes the terminal with a cursor-position query at startup; a line discipline is
//!   always on and cannot block on a terminal that may not answer (a piped boot script never
//!   would). A wrapped long line therefore redraws incorrectly past the margin. Recorded in
//!   notes/terminal-contract.md as an honest limit; [`LINE_MAX`] keeps it rare.
//! - **No tab completion here.** Completion needs the command namespace, which is the
//!   application's knowledge, not the terminal's. The engine reports Tab as [`Event::Tab`] and
//!   leaves the line alone; the terminal component ignores it. A program that runs this engine
//!   in its own process over raw mode (the shell, DECISIONS §227 (how Tab reaches the shell)
//!   option D) answers it with [`LineDisc::pending`] and [`LineDisc::insert_text`].
//! - **No output-side state.** Application output passes through the component untouched except
//!   for newline translation, which is [`expand_output`], a free function on purpose: output
//!   processing shares no state with input editing (Unix tangles these; see notes/tcb.md's
//!   mistake-catalog reading of the tty layer).
//!
//! The echo this engine emits and the sequences it understands are the terminal contract's
//! wire half; the IPC framing is `proto`. Both documented in notes/terminal-contract.md.
//!
//! Name: ratified 2026-07-30 (calef, DECISIONS §39, landed by milestone 46) for the word and again
//! 2026-08-01 (milestone 63) for the spelling, replacing `linedisc` and then `lineedit`. Refused
//! `linedisc` (the correct Unix term of art, which the person who built this system did not
//! recognise, and which imports vocabulary from exactly the model this OS rejects),
//! `line_discipline` (overclaims: that term covers echo, canonical mode, signals and flow control,
//! and this crate is narrower) and `line_edit` (a verb phrase where its sibling `video_terminal` is
//! a noun).

#![no_std]

// The crate root is an index (§282 (when a file is split, the parent becomes an index)): the
// engine, the raw queue and the output side are private modules re-exported here by name, so
// every `line_editor::` path a program names is the one it named before milestone 878 (the line
// editor's inline modules move to their own files). The three public modules keep their docs on
// their `pub mod` lines below.

mod line_disc;
mod output;
mod raw_queue;

pub use line_disc::{Event, HIST, LINE_MAX, LineDisc, PROMPT_MAX};
pub use output::{Sink, expand_output};
pub use raw_queue::{RAW_QUEUE_MAX, RawQueue};

/// The IPC framing of the terminal contract (notes/terminal-contract.md): opcodes, flags, and
/// limits shared by the discipline server, its clients, and the kernel-side tests. This is a
/// **userspace protocol**, not kernel ABI: the kernel routes these words without reading them.
pub mod proto;

/// **What a supervisor routes to a `line_editor` it can replace** (milestone 23 (a
/// capability-routed component OS with live replacement)): the capability half of the terminal
/// contract, beside the wire half in [`proto`]. Two declarations, because the output sink is one of
/// two shapes: the console's two-endpoint protocol, older than §12 (call/reply IPC: a
/// one-shot reply capability), which needs a reply endpoint, or
/// `display_terminal`'s one `CALL`, which does not. Everything else is shared.
///
/// **Slot order is the one `line_editor` already reads**: the terminal in 0, the sink in 1, the
/// console's reply in 2 when there is one, then the control endpoint and the supervisor's
/// notification endpoint. `line_editor` learns the control slot from its second start argument and
/// finds the notification endpoint in the slot after it, so one binary serves both shapes; the
/// `const` assertions below hold the declarations to that.
///
/// Name: provisional (the lane for milestone 23, 2026-09-27), for the module, both constants,
/// `HANDOFF_VA` and the role and contract strings.
pub mod component;

/// **The line editor's state as a handoff blob** (milestone 23 (a capability-routed component OS
/// with live replacement); DECISIONS §209 (state handoff is an opaque blob over a granted frame, and
/// it is optional)). What a replacement `line_editor` needs to carry on as if nothing happened: the
/// half-typed line and its cursor, the kill buffer, the history ring and the prompt.
///
/// **What is deliberately not carried**: a half-received escape sequence (the replacement starts
/// between keys), history browsing (the line being browsed is the edit line, which is carried),
/// and the snapshot of the last completed line (it was already delivered or queued). Each is a
/// state that lasts one keystroke.
///
/// The layout is this crate's, versioned by [`handoff::LAYOUT`]; a supervisor never reads it.
///
/// Name: provisional (the lane for milestone 23, 2026-09-27).
pub mod handoff;

#[cfg(test)]
mod test_support;
