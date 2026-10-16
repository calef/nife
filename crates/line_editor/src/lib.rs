//! **The line discipline engine: a sans-IO editor for a serial terminal.** Byte in, echo bytes
//! out, completed lines out; no IO, IPC, UART or endpoint. The `line_editor` component feeds it one
//! byte at a time and forwards what it emits, so the editing rules are host-tested against a small
//! terminal model and only the wiring needs QEMU. The echo it emits and the sequences it reads are
//! the terminal contract's wire half, and `proto` is the IPC framing (notes/terminal-contract.md).
//!
//! - Insertion and deletion at a cursor: minimal echo when typing at the end, a redraw otherwise.
//! - Cursor movement as control characters (^B ^F ^A ^E) and ANSI sequences (CSI D/C/H/F, SS3, and
//!   the vt220 `~` forms).
//! - Kill and yank: ^K (to end), ^U (to start), ^W (word back), ^Y (insert the kill buffer).
//! - History: the last [`HIST`] non-empty lines on up and down, with the in-progress line stashed.
//! - CR or LF ends a line, and an LF straight after a CR is swallowed, so interactive (CR) and
//!   piped (LF, CRLF) input both work without the caller knowing which it has.
//! - ^C discards the line ([`Event::Interrupt`]); ^D is end of input ([`Event::Eof`]) on an empty
//!   line and a delete otherwise, Unix's double duty; ^L repaints.
//!
//! What it deliberately does not do:
//!
//! - **No terminal size tracking**: the redraw math assumes one row, so a wrapped long line
//!   redraws wrongly past the margin. Probing the terminal would block a line discipline on a
//!   terminal that may never answer, such as a piped boot script; [`LINE_MAX`] keeps it rare.
//! - **No tab completion**: that needs the command namespace. Tab is reported as [`Event::Tab`] and
//!   the terminal ignores it; the shell, running this engine itself in raw mode, answers it with
//!   [`LineDisc::pending`] and [`LineDisc::insert_text`] (§227 (how Tab reaches the shell)).
//! - **No output-side state**: newline translation is the free function [`expand_output`],
//!   because output shares no state with input editing (notes/thread-control-block.md on the tty layer).
//!
//! Name: ratified 2026-07-30 (calef) for the word and 2026-08-01 for the spelling, replacing
//! `linedisc` and then `lineedit`. Refused `linedisc` (a Unix term of art from the model this OS
//! rejects), `line_discipline` (overclaims: that covers echo, canonical mode, signals and flow
//! control) and `line_edit` (a verb phrase where its sibling `video_terminal` is a noun).

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
