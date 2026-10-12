//! **The VT state engine: the grid a display terminal keeps** (milestone 29, the display ladder's
//! text).
//!
//! Bytes in, a character grid out, plus the rectangle that changed. Sans-IO, exactly as the
//! `line_editor` crate is: this crate holds no endpoint, makes no syscall, and has never heard of a
//! framebuffer. `components/src/display_terminal.rs` feeds it and paints what it says.
//!
//! # Why this shape
//!
//! DECISIONS §7: pure logic belongs in a crate that compiles for the host, so most tests run in
//! milliseconds. A VT engine is almost entirely pure logic (a parser and a two-dimensional array),
//! and the parts that are not (a shared surface, an IPC endpoint) are the parts a QEMU boot has to
//! prove anyway.
//!
//! It buys something specific here beyond fast tests. Because the engine is a value with no IO, the
//! **expected picture is computable by anyone holding the same script**: the terminal component runs
//! it to draw, the kernel-side test runs it to predict what should be in the framebuffer, and
//! `cargo xtask` runs it on the host to grade what QEMU is actually scanning out. Three independent
//! witnesses, one definition, no possibility of the three agreeing on a wrong answer.
//!
//! # Examples
//!
//! Bytes in, a grid out, and the rectangle that changed. Because the engine is a value with no IO,
//! the expected picture is computable by anyone holding the same bytes, which is what lets three
//! independent witnesses agree about what should be on the screen.
//!
//! ```
//! use video_terminal::Vt;
//!
//! let mut vt = Vt::new(8, 4);
//! vt.feed(b"hello\r\nworld");
//!
//! let mut row = [0u8; 8];
//! let n = vt.row_bytes(0, &mut row);
//! assert_eq!(&row[..n], b"hello   "); // the rest of the row is blanks, not garbage
//! vt.row_bytes(1, &mut row);
//! assert_eq!(&row[..5], b"world");
//! assert_eq!(vt.cursor(), (5, 1));
//!
//! // Damage is in cells and is taken, not read: the caller repaints exactly what changed and the
//! // record clears. That is the whole reason the engine reports a rectangle instead of "redraw".
//! let dirty = vt.take_damage().expect("something changed");
//! assert!(dirty.rect.rows >= 2, "two rows were written to");
//! assert_eq!(dirty.scrolled, 0, "nothing scrolled");
//! assert_eq!(vt.take_damage(), None); // taken once
//! ```
//!
//! **Deferred wrap** is the one subtlety in printing, and it is the reason a line that exactly fills
//! the width does not scroll the screen before anything asked it to:
//!
//! ```
//! use video_terminal::Vt;
//!
//! let mut vt = Vt::new(4, 4);
//! vt.feed(b"abcd"); // exactly fills row 0
//!
//! // The cursor stays on the last cell it wrote, with a wrap pending. It has NOT moved to row 1.
//! assert_eq!(vt.cursor(), (3, 0));
//!
//! // A CR arriving now finds the cursor on row 0, which is where a line discipline expects it. If
//! // the wrap had already happened it would be a row too low.
//! vt.feed(b"\r");
//! assert_eq!(vt.cursor(), (0, 0));
//!
//! // And when the next printable byte does arrive without an intervening CR, it wraps first.
//! let mut vt = Vt::new(4, 4);
//! vt.feed(b"abcde");
//! assert_eq!(vt.cursor(), (1, 1));
//! let mut row = [0u8; 4];
//! vt.row_bytes(1, &mut row);
//! assert_eq!(row[0], b'e');
//! ```
//!
//! The escape sequences are the ones a line discipline actually emits, plus the screen verbs any
//! program expects:
//!
//! ```
//! use video_terminal::Vt;
//!
//! let mut vt = Vt::new(8, 4);
//! vt.feed(b"one\r\ntwo\r\n");
//!
//! // CSI H homes the cursor; CSI 2J clears the screen.
//! vt.feed(b"\x1b[2J\x1b[H");
//! assert_eq!(vt.cursor(), (0, 0));
//! let mut row = [0u8; 8];
//! let n = vt.row_bytes(0, &mut row);
//! assert_eq!(&row[..n], b"        ");
//! ```
//!
//! # What it implements, and why exactly this set
//!
//! The escape sequences here are **the ones the line discipline already emits** (DECISIONS §21,
//! notes/terminal-contract.md), plus the screen verbs any program expects. That is not a guess: the
//! interoperability test in this crate runs the real `line_editor` and feeds its echo stream to this
//! parser, so the two components are checked against each other rather than against a list somebody
//! wrote down.
//!
//! - Printable bytes, with **deferred wrap** at the right margin (see [`Vt::feed`]) and **UTF-8
//!   decoding** (milestone 142 increment 2): a multi-byte sequence occupies one cell, drawn as
//!   [`bitmap_font::glyph`]'s missing-glyph box for anything past basic latin, which is this font's
//!   whole repertoire (see `bitmap_font::glyph`'s own doc).
//! - `CR`, `LF` (with scrolling into [`SCROLLBACK_ROWS`] of history, milestone 142 increment 2), `BS`,
//!   `TAB`, `BEL` (ignored: there is no bell here).
//! - `CSI A/B/C/D` cursor motion, `CSI H` / `CSI f` absolute positioning.
//! - `CSI J` erase in display, `CSI K` erase in line, both with all three modes.
//! - `CSI m` (SGR): reset; bold, dim, underline, reverse, concealed and crossed-out, each with its
//!   off switch; the eight ANSI colours and their bright forms, foreground and background; and the
//!   256-colour (`38;5;n`) and 24-bit (`38;2;r;g;b`) forms of both (milestone 142 (a text display
//!   good enough that people use it instead of a GUI), the 2026-09-26 pass). Italic and blink are
//!   parsed and dropped, since italic needs a face this font does not have and blink needs a clock
//!   this engine does not read.
//! - `ESC c` (RIS), a full reset.
//!
//! Anything else is **swallowed whole**, introducer and all, rather than printed as garbage.
//!
//! # What deliberately is NOT here
//!
//! No alternate screen, no origin mode, no scrolling regions, no tab stops other than every eight
//! columns, no mouse, and no reporting sequences at all: this engine never writes to its input,
//! which is what "sans-IO" means here and what keeps it a *value*. No reflow: `MAX_COLS`/`MAX_ROWS`
//! are fixed, and nothing here resizes a live grid. The honest limits are listed in
//! notes/glyphs.md.
//!
//! Name: ratified 2026-08-01 (calef, milestone 63), replacing `vt`. Refused `vt` (two letters that
//! read as *vector table* in a kernel), `virtual_terminal` (wrong twice: that is not what VT stood
//! for, and "virtual terminal" already names Linux's virtual consoles) and `screen_grid` (the crate
//! carries 63 escape-sequence references, so the grid is the output and interpreting the protocol
//! is the work). Deliberately not the program's name: this crate is named for the protocol it
//! implements (DEC's Video Terminals) and `display_terminal` for its role.

#![no_std]

pub mod keymap;
pub mod script;

/// What the display terminal reports to whoever spawned it.
///
/// **Status, not contract**: no client can ask for any of this, and it is here rather than in the
/// terminal contract (`line_editor::proto`) because it is about the *component*, not about the terminal
/// a program talks to. The engine above is sans-IO and this module is three constants; nothing in
/// the engine reads them.
pub mod status;

mod cell;
mod damage;
mod geometry;
mod palette;
mod vt;

#[cfg(test)]
mod test_support;

pub use cell::{Attr, Cell, Colour};
pub use damage::{CellRect, Damage};
pub use geometry::{MAX_CELLS, MAX_COLS, MAX_ROWS, SCROLLBACK_CELLS, SCROLLBACK_ROWS};
pub use palette::{DEFAULT_BG, DEFAULT_FG, PALETTE};
pub use vt::Vt;
