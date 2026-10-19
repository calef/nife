//! **A `Vt` as a §209 blob**: everything a replacement `display_terminal` needs to carry on where
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
//! foreground and the background. A colour word's top byte is its kind (`0` indexed, `1` 24-bit) and
//! the low three bytes its value. Only the cells in use are written: `cols * rows` of the live grid,
//! then the scrollback's rows oldest first, so a blob is as long as the screen has history.
//!
//! Name: provisional (the lane for milestone 23, 2026-10-19 UTC), for this module and every item
//! in it. calef names things.

use super::{MAX_PARAMS, State, Vt};
use crate::cell::{Attr, Cell, Colour};
use crate::geometry::{MAX_CELLS, SCROLLBACK_CELLS};
#[cfg(test)]
use crate::geometry::{MAX_ROWS, SCROLLBACK_ROWS};

/// "VTRM", the blob's first word.
pub const MAGIC: u32 = 0x4d52_5456;
/// The one layout this build writes and reads.
pub const LAYOUT: u32 = 1;

/// Words before the first cell: magic, layout, the twelve fields of the cursor and the
/// scrollback ring, the rendition's three, and the parser's [`MAX_PARAMS`] parameters.
const HEADER_WORDS: usize = 2 + 12 + 3 + MAX_PARAMS;
const CELL_WORDS: usize = 3;

/// **The longest blob a `Vt` writes**: a full live grid and a full scrollback ring. 538,116 bytes,
/// so [`HANDOFF_PAGES`] is 132.
pub const HANDOFF_BYTES: usize = 4 * (HEADER_WORDS + CELL_WORDS * (MAX_CELLS + SCROLLBACK_CELLS));
/// [`HANDOFF_BYTES`] in page frames: what a supervisor retypes for a `display_terminal`'s handoff.
pub const HANDOFF_PAGES: u64 = HANDOFF_BYTES.div_ceil(page_frames::FRAME_SIZE as usize) as u64;

/// Why a blob was not absorbed. The `Vt` is untouched by every one of them: the whole blob is
/// checked before a cell is written.
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

const FLAG_WRAP_PENDING: u32 = 1 << 0;
const FLAG_CURSOR_VISIBLE: u32 = 1 << 1;
const FLAG_IGNORE: u32 = 1 << 2;
/// Every rendition flag `Attr` defines; a cell carrying another bit was not written by a `Vt`.
const KNOWN_ATTR_FLAGS: u32 = 0x3f;

const fn state_word(s: State) -> u32 {
    match s {
        State::Ground => 0,
        State::Esc => 1,
        State::Csi => 2,
        State::Str => 3,
        State::StrEsc => 4,
    }
}

const fn word_state(w: u32) -> Option<State> {
    match w {
        0 => Some(State::Ground),
        1 => Some(State::Esc),
        2 => Some(State::Csi),
        3 => Some(State::Str),
        4 => Some(State::StrEsc),
        _ => None,
    }
}

const fn colour_word(c: Colour) -> u32 {
    match c {
        Colour::Indexed(i) => i as u32,
        Colour::Rgb(r, g, b) => 1 << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32,
    }
}

const fn word_colour(w: u32) -> Option<Colour> {
    match w >> 24 {
        0 if w <= 0xff => Some(Colour::Indexed(w as u8)),
        1 => Some(Colour::Rgb((w >> 16) as u8, (w >> 8) as u8, w as u8)),
        _ => None,
    }
}

fn cell_words(c: Cell) -> [u32; CELL_WORDS] {
    [
        c.ch as u32 | (c.attr.flags as u32) << 24,
        colour_word(c.attr.fg),
        colour_word(c.attr.bg),
    ]
}

fn words_cell(w: [u32; CELL_WORDS]) -> Option<Cell> {
    let flags = w[0] >> 24;
    if flags & !KNOWN_ATTR_FLAGS != 0 {
        return None;
    }
    Some(Cell {
        ch: char::from_u32(w[0] & 0x00ff_ffff)?,
        attr: Attr {
            fg: word_colour(w[1])?,
            bg: word_colour(w[2])?,
            flags: flags as u8,
        },
    })
}

fn word_at(blob: &[u8], i: usize) -> Option<u32> {
    let b = blob.get(4 * i..4 * i + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn put_word(out: &mut [u8], i: usize, w: u32) {
    out[4 * i..4 * i + 4].copy_from_slice(&w.to_le_bytes());
}

impl Vt {
    /// How many bytes [`Vt::write_handoff`] writes for this terminal as it stands.
    pub fn handoff_len(&self) -> usize {
        let cells = (self.cols * self.rows + self.sb_len * self.cols) as usize;
        4 * (HEADER_WORDS + CELL_WORDS * cells)
    }

    /// **Write this terminal's whole state into `out`**, and return how many bytes that took, or
    /// `None` when `out` is shorter than [`Vt::handoff_len`]. A blob that does not fit is not
    /// truncated, because a truncated blob that read back would be state silently lost.
    pub fn write_handoff(&self, out: &mut [u8]) -> Option<usize> {
        let len = self.handoff_len();
        if out.len() < len {
            return None;
        }
        let mut flags = 0;
        for (on, bit) in [
            (self.wrap_pending, FLAG_WRAP_PENDING),
            (self.cursor_visible, FLAG_CURSOR_VISIBLE),
            (self.ignore, FLAG_IGNORE),
        ] {
            if on {
                flags |= bit;
            }
        }
        let header = [
            MAGIC,
            LAYOUT,
            self.cols,
            self.rows,
            self.col,
            self.row,
            self.sb_len,
            self.view_offset,
            flags,
            state_word(self.state),
            self.nparams as u32,
            u32::from(self.utf8_need),
            self.utf8_code,
            0,
            colour_word(self.attr.fg),
            colour_word(self.attr.bg),
            self.attr.flags as u32,
        ];
        for (i, w) in header.iter().enumerate() {
            put_word(out, i, *w);
        }
        for (i, p) in self.params.iter().enumerate() {
            put_word(out, header.len() + i, u32::from(*p));
        }
        let mut at = HEADER_WORDS;
        let live = (self.cols * self.rows) as usize;
        for c in &self.cells[..live] {
            for w in cell_words(*c) {
                put_word(out, at, w);
                at += 1;
            }
        }
        // Oldest first, so the reader rebuilds the ring from slot zero whatever `sb_tail` was.
        for age in (0..self.sb_len).rev() {
            for col in 0..self.cols {
                for w in cell_words(self.scrollback_cell(col, age)) {
                    put_word(out, at, w);
                    at += 1;
                }
            }
        }
        Some(len)
    }

    /// **Become the terminal a blob describes.** The blob's grid must be this terminal's own
    /// shape, the one it negotiated with its screen before absorbing. The whole blob is checked
    /// first, so on any [`Refusal`] this terminal is as it was.
    ///
    /// Everything is marked as damage: the caller repaints its whole surface from what it
    /// absorbed.
    pub fn absorb_handoff(&mut self, blob: &[u8]) -> Result<(), Refusal> {
        let w = |i| word_at(blob, i).ok_or(Refusal::Malformed);
        if w(0).ok() != Some(MAGIC) {
            return Err(Refusal::NoBlob);
        }
        match w(1)? {
            LAYOUT => {}
            other => return Err(Refusal::Layout(other)),
        }
        if (w(2)?, w(3)?) != (self.cols, self.rows) {
            return Err(Refusal::Geometry);
        }
        let (col, row, sb_len, view_offset) = (w(4)?, w(5)?, w(6)?, w(7)?);
        let flags = w(8)?;
        let state = word_state(w(9)?).ok_or(Refusal::Malformed)?;
        let nparams = w(10)? as usize;
        let utf8_need = w(11)?;
        let utf8_code = w(12)?;
        let attr = Attr {
            fg: word_colour(w(14)?).ok_or(Refusal::Malformed)?,
            bg: word_colour(w(15)?).ok_or(Refusal::Malformed)?,
            flags: u8::try_from(w(16)?).map_err(|_| Refusal::Malformed)?,
        };
        let mut params = [0u16; MAX_PARAMS];
        for (i, p) in params.iter_mut().enumerate() {
            *p = u16::try_from(w(17 + i)?).map_err(|_| Refusal::Malformed)?;
        }
        if col >= self.cols
            || row >= self.rows
            || sb_len > self.scrollback_depth()
            || view_offset > sb_len
            || flags & !(FLAG_WRAP_PENDING | FLAG_CURSOR_VISIBLE | FLAG_IGNORE) != 0
            || nparams > MAX_PARAMS
            || utf8_need > 3
            || u32::from(attr.flags) & !KNOWN_ATTR_FLAGS != 0
        {
            return Err(Refusal::Malformed);
        }
        let live = (self.cols * self.rows) as usize;
        let history = (sb_len * self.cols) as usize;
        let cell = |k: usize| -> Result<Cell, Refusal> {
            let at = HEADER_WORDS + CELL_WORDS * k;
            words_cell([w(at)?, w(at + 1)?, w(at + 2)?]).ok_or(Refusal::Malformed)
        };
        // Every cell is checked before any is written, so a refusal leaves the terminal whole.
        for k in 0..live + history {
            cell(k)?;
        }
        for (k, c) in self.cells[..live].iter_mut().enumerate() {
            *c = cell(k)?;
        }
        for (k, c) in self.scrollback[..history].iter_mut().enumerate() {
            *c = cell(live + k)?;
        }
        self.sb_len = sb_len;
        self.sb_tail = sb_len % self.scrollback_depth();
        self.view_offset = view_offset;
        self.col = col;
        self.row = row;
        self.attr = attr;
        self.wrap_pending = flags & FLAG_WRAP_PENDING != 0;
        self.cursor_visible = flags & FLAG_CURSOR_VISIBLE != 0;
        self.ignore = flags & FLAG_IGNORE != 0;
        self.state = state;
        self.params = params;
        self.nparams = nparams;
        self.utf8_need = utf8_need as u8;
        self.utf8_code = utf8_code;
        self.scrolled = 0;
        self.drawn = None;
        self.damage_all();
        Ok(())
    }
}

#[cfg(test)]
mod tests;
