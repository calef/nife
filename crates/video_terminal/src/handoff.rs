//! **A `Vt` as a §209 (state handoff is an opaque blob over a granted frame, and it is optional)
//! blob**: everything a replacement `display_terminal` needs to carry on where
//! its incumbent stopped (milestone 23 (a capability-routed component OS with live replacement);
//! calef's ruling of 2026-10-19 (UTC), "everything").
//!
//! The live grid, the scrollback in order, the cursor, the rendition and the parser's state, so a
//! sequence cut in half by a swap (`ESC [ 3` before it, `1 m` after) still means red. What is not
//! carried is the damage record: a replacement repaints its whole surface from the absorbed state,
//! which is what makes the screen after a swap a witness of the state rather than of the pixels
//! the incumbent left behind.
//!
//! The layout is fixed little-endian words, never a memory image of `Vt`, so a build with a
//! different field order or a different `Cell` padding reads it the same way. A cell is three words:
//! the character with the rendition flags in its top byte (a `char` needs 21 bits), then the
//! foreground and the background. A color word's top byte is its kind (`0` indexed, `1` 24-bit) and
//! the low three bytes its value. Only the cells in use are written: `cols * rows` of the live grid,
//! then the scrollback's rows oldest first, so a blob is as long as the screen has history.
//!
//! Name: provisional (the lane for milestone 23, 2026-10-19 UTC), for this module and every item
//! in it. calef names things.

use crate::cell::{Attr, Cell, Colour};
use crate::geometry::{MAX_CELLS, SCROLLBACK_CELLS};
use crate::vt::MAX_PARAMS;

/// "VTRM", the blob's first word.
///
/// Name: provisional, minted 2026-10-19 (UTC) by the lane for milestone 23.
pub const MAGIC: u32 = 0x4d52_5456;
/// The one layout this build writes and reads.
///
/// Name: provisional, minted 2026-10-19 (UTC) by the lane for milestone 23.
pub const LAYOUT: u32 = 1;

/// Words before the first cell: magic, layout, the twelve fields of the cursor and the
/// scrollback ring, the rendition's three, and the parser's [`MAX_PARAMS`] parameters.
pub(crate) const HEADER_WORDS: usize = 2 + 12 + 3 + MAX_PARAMS;
pub(crate) const CELL_WORDS: usize = 3;

/// **The longest blob a `Vt` writes**: a full live grid and a full scrollback ring. 538,116 bytes,
/// so [`HANDOFF_PAGES`] is 132.
///
/// Name: provisional, minted 2026-10-19 (UTC) by the lane for milestone 23.
pub const HANDOFF_BYTES: usize = 4 * (HEADER_WORDS + CELL_WORDS * (MAX_CELLS + SCROLLBACK_CELLS));
/// [`HANDOFF_BYTES`] in page frames: what a supervisor retypes for a `display_terminal`'s handoff.
///
/// Name: provisional, minted 2026-10-19 (UTC) by the lane for milestone 23.
pub const HANDOFF_PAGES: u64 = HANDOFF_BYTES.div_ceil(address_space_map::PAGE as usize) as u64;

/// Why a blob was not absorbed. The `Vt` is untouched by every one of them: the whole blob is
/// checked before a cell is written.
///
/// Name: provisional, minted 2026-10-19 (UTC) by the lane for milestone 23.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The first word is not [`MAGIC`]: no blob was written here.
    NoBlob,
    /// A blob of a layout this build does not read.
    Layout(u32),
    /// The blob is for a grid of another shape than the one this terminal negotiated. Its cells
    /// would land on the wrong rows, so it is refused rather than reflowed.
    Geometry,
    /// A field out of range, a cell that is not a character, or a blob cut short.
    Malformed,
}

pub(crate) const FLAG_WRAP_PENDING: u32 = 1 << 0;
pub(crate) const FLAG_CURSOR_VISIBLE: u32 = 1 << 1;
pub(crate) const FLAG_IGNORE: u32 = 1 << 2;
/// Every rendition flag `Attr` defines; a cell carrying another bit was not written by a `Vt`.
pub(crate) const KNOWN_ATTR_FLAGS: u32 = 0x3f;

pub(crate) const fn color_word(c: Colour) -> u32 {
    match c {
        Colour::Indexed(i) => i as u32,
        Colour::Rgb(r, g, b) => 1 << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32,
    }
}

pub(crate) const fn word_color(w: u32) -> Option<Colour> {
    match w >> 24 {
        0 if w <= 0xff => Some(Colour::Indexed(w as u8)),
        1 => Some(Colour::Rgb((w >> 16) as u8, (w >> 8) as u8, w as u8)),
        _ => None,
    }
}

pub(crate) fn cell_words(c: Cell) -> [u32; CELL_WORDS] {
    [
        c.ch as u32 | (c.attr.flags as u32) << 24,
        color_word(c.attr.fg),
        color_word(c.attr.bg),
    ]
}

pub(crate) fn words_cell(w: [u32; CELL_WORDS]) -> Option<Cell> {
    let flags = w[0] >> 24;
    if flags & !KNOWN_ATTR_FLAGS != 0 {
        return None;
    }
    Some(Cell {
        ch: char::from_u32(w[0] & 0x00ff_ffff)?,
        attr: Attr {
            fg: word_color(w[1])?,
            bg: word_color(w[2])?,
            flags: flags as u8,
        },
    })
}

pub(crate) fn word_at(blob: &[u8], i: usize) -> Option<u32> {
    let b = blob.get(4 * i..4 * i + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

pub(crate) fn put_word(out: &mut [u8], i: usize, w: u32) {
    out[4 * i..4 * i + 4].copy_from_slice(&w.to_le_bytes());
}

#[cfg(test)]
mod tests;
