//! One cell of the grid: a character, and the rendition that says how to paint it.
//!
//! Name: provisional (the lane of milestone 874 (the terminal engine moves its palette and its
//! tests out), 2026-10-12 UTC), named for `Cell`, the type its other items exist to fill; calef
//! names things.

use super::*;

/// **One colour a cell can name**: an entry in the 256-colour table, or a 24-bit value.
///
/// The two are the two ways a program asks for colour (milestone 142, the 2026-09-26 pass): the
/// indexed form is what SGR 30..37, 90..97 and `38;5;n` carry, and the 24-bit form is what `38;2;r;g;b`
/// carries, which is what every syntax highlighter emits and was this terminal's single largest gap.
/// Four bytes, alignment one, so the rendition that holds two of them stays small.
///
/// Name: provisional (milestone 142's lane, 2026-09-26). British spelling to match the rest of this
/// crate ([`Attr::colours`], [`PALETTE`]'s doc).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colour {
    /// An index into the 256-colour table: 0..16 are [`PALETTE`], 16..232 the 6x6x6 cube, 232..256
    /// the grey ramp. See [`Colour::resolve`].
    Indexed(u8),
    /// A 24-bit colour, red, green and blue.
    Rgb(u8, u8, u8),
}

/// One channel of the 256-colour cube at step `v` (0..6): `0, 0x5f, 0x87, 0xaf, 0xd7, 0xff`.
const fn cube_level(v: u32) -> u32 {
    if v == 0 { 0 } else { 0x37 + 40 * v }
}

impl Colour {
    /// **The `0x00RRGGBB` word this colour paints**, in the surface's own pixel format.
    ///
    /// The indexed range above sixteen is xterm's, computed rather than tabulated because it is
    /// arithmetic: the cube's six levels per channel are `0, 0x5f, 0x87, 0xaf, 0xd7, 0xff` (a step of
    /// 40 after an uneven first one), and the ramp is `0x08 + 10 * k` for 24 greys that stop short of
    /// both black and white, which the cube already has. Every emulator that implements `38;5;n`
    /// agrees on these numbers, which is why a program's 256-colour theme looks the same here.
    ///
    /// Name: provisional (milestone 142's lane, 2026-09-26).
    pub const fn resolve(self) -> u32 {
        match self {
            Colour::Rgb(r, g, b) => (r as u32) << 16 | (g as u32) << 8 | b as u32,
            Colour::Indexed(i) if i < 16 => PALETTE[i as usize],
            Colour::Indexed(i) if i < 232 => {
                let n = i as u32 - 16;
                cube_level(n / 36) << 16 | cube_level(n / 6 % 6) << 8 | cube_level(n % 6)
            }
            Colour::Indexed(i) => {
                let v = 0x08 + 10 * (i as u32 - 232);
                v << 16 | v << 8 | v
            }
        }
    }
}

/// **How a cell is painted**: two colours, and the renditions that change how they are used.
///
/// Nine bytes (two [`Colour`]s and a byte of flags), where it was one byte of palette indices
/// before milestone 142's 2026-09-26 pass: truecolour does not fit in less, and a [`Cell`] is
/// sixteen bytes either way once a `char` is beside it. The cost is measured where it lands, in
/// [`SCROLLBACK_ROWS`]'s doc.
///
/// **Bold is bright, and that is a decision rather than a shortcut.** A bold weight needs a second
/// font, and in a five-column cell a bold face is a smudge; every terminal from the DEC VT onward
/// has answered SGR 1 by brightening instead, which is why the palette has eight bright entries.
/// Bold is now a **flag** rather than a rewrite of the foreground index, so it is resolved at paint
/// time: it brightens the eight normal colours and nothing else (a 256-colour or 24-bit foreground
/// is drawn as asked, which is xterm's rule), and SGR 22 restores exactly the colour that was set,
/// including an explicitly bright one. Keeping the flag is also what lets the atlas (increment 3)
/// draw a real bold face later without the parser changing. Recorded in notes/glyphs.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attr {
    pub(super) fg: Colour,
    pub(super) bg: Colour,
    pub(super) flags: u8,
}

/// SGR 1.
pub(super) const BOLD: u8 = 1 << 0;
/// SGR 2, faint.
pub(super) const DIM: u8 = 1 << 1;
/// SGR 4 (and 21, drawn the same: one underline row is all an eight-row cell has room for).
pub(super) const UNDERLINE: u8 = 1 << 2;
/// SGR 7.
pub(super) const REVERSE: u8 = 1 << 3;
/// SGR 8, concealed.
pub(super) const INVISIBLE: u8 = 1 << 4;
/// SGR 9, crossed out.
pub(super) const STRIKETHROUGH: u8 = 1 << 5;

/// The glyph row an underline is drawn on: the last, which carries only descenders and the
/// underscore (`bitmap_font::GLYPH_H`'s doc), so an underline runs under the letters rather than
/// through them and joins across cells.
pub(super) const UNDERLINE_ROW: u32 = bitmap_font::GLYPH_H - 1;
/// The glyph row a strikethrough is drawn on: the hyphen's row, which is also the crossbar of `e`,
/// so a struck-out word is crossed at the middle of its lower-case letters.
pub(super) const STRIKETHROUGH_ROW: u32 = 4;

impl Attr {
    /// The rendition a reset terminal writes with.
    pub const DEFAULT: Attr = Attr::new(Colour::Indexed(DEFAULT_FG), Colour::Indexed(DEFAULT_BG));

    /// A plain rendition: these two colours, and no flag set.
    pub const fn new(fg: Colour, bg: Colour) -> Attr {
        Attr { fg, bg, flags: 0 }
    }

    /// The same rendition with `flag` turned on or off.
    pub(super) const fn with(self, flag: u8, on: bool) -> Attr {
        Attr {
            flags: if on {
                self.flags | flag
            } else {
                self.flags & !flag
            },
            ..self
        }
    }

    const fn has(self, flag: u8) -> bool {
        self.flags & flag != 0
    }

    /// The foreground as it was set, before bold, dim or reverse are applied.
    pub const fn fg(self) -> Colour {
        self.fg
    }

    /// The background as it was set, before reverse is applied.
    pub const fn bg(self) -> Colour {
        self.bg
    }

    /// Whether the reverse-video bit is set. See [`colours`](Self::colours) for what it does to
    /// the actual paint colours.
    ///
    /// Name: provisional, flagged 2026-09-24 by the boolean-predicate pass
    /// (design/naming/functions-that-answer-yes-or-no-worklist.md). It does not yet follow the Rust predicate
    /// rule calef ratified 2026-09-24; recommended `is_reverse_video`, because `is_reverse` reads
    /// as a direction.
    pub const fn reverse(self) -> bool {
        self.has(REVERSE)
    }

    /// Whether SGR 1 is in force. Name: provisional (milestone 142's lane, 2026-09-26).
    pub const fn is_bold(self) -> bool {
        self.has(BOLD)
    }

    /// Whether SGR 2 is in force. Name: provisional (milestone 142's lane, 2026-09-26).
    pub const fn is_dim(self) -> bool {
        self.has(DIM)
    }

    /// Whether SGR 4 is in force. Name: provisional (milestone 142's lane, 2026-09-26).
    pub(crate) const fn is_underlined(self) -> bool {
        self.has(UNDERLINE)
    }

    /// Whether SGR 8 is in force. Name: provisional (milestone 142's lane, 2026-09-26).
    pub const fn is_invisible(self) -> bool {
        self.has(INVISIBLE)
    }

    /// Whether SGR 9 is in force. Name: provisional (milestone 142's lane, 2026-09-26).
    pub(crate) const fn is_struck_through(self) -> bool {
        self.has(STRIKETHROUGH)
    }

    /// The `(foreground, background)` colours this rendition actually paints with, as
    /// `0x00RRGGBB` words. One place, so the cursor and the SGR path cannot disagree about what any
    /// flag means. In order:
    ///
    /// 1. **Bold** brightens a foreground among the eight normal colours (see [`Attr`]).
    /// 2. **Dim** moves the foreground halfway to the background, channel by channel, in sRGB. The
    ///    midpoint is xterm's choice of amount. sRGB rather than linear light on purpose: dim asks
    ///    for half the *apparent* brightness, and the linear-light midpoint of white and black is
    ///    sRGB 188, which reads as barely dimmed. Linear light is right for anti-aliased coverage
    ///    (increment 4), where the question is how much physical light a partly covered pixel emits.
    /// 3. **Reverse** swaps the two.
    /// 4. **Invisible** paints the foreground in the (post-reverse) background, so the text is
    ///    there to copy and not there to see. Last, so the block cursor (which is reverse toggled on
    ///    a cell) still shows on a concealed cell rather than vanishing with its text.
    pub const fn colours(self) -> (u32, u32) {
        let fg = match self.fg {
            Colour::Indexed(i) if i < 8 && self.has(BOLD) => Colour::Indexed(i + 8),
            other => other,
        };
        let (mut f, mut b) = (fg.resolve(), self.bg.resolve());
        if self.has(DIM) {
            f = midpoint(f, b);
        }
        if self.has(REVERSE) {
            (f, b) = (b, f);
        }
        if self.has(INVISIBLE) {
            f = b;
        }
        (f, b)
    }
}

/// The per-channel midpoint of two `0x00RRGGBB` words, rounding down.
const fn midpoint(a: u32, b: u32) -> u32 {
    channel_midpoint(a, b, 16) | channel_midpoint(a, b, 8) | channel_midpoint(a, b, 0)
}

const fn channel_midpoint(a: u32, b: u32, shift: u32) -> u32 {
    ((((a >> shift) & 0xff) + ((b >> shift) & 0xff)) / 2) << shift
}

impl Default for Attr {
    fn default() -> Attr {
        Attr::DEFAULT
    }
}

/// One cell of the grid: a character and how to paint it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// The character, decoded from UTF-8 by the engine (milestone 142 increment 2). One `char`, not
    /// one byte: a multi-byte UTF-8 sequence occupies exactly one cell, the same as it occupies one
    /// column, rather than one cell per encoded byte. `bitmap_font` still only has pictures for basic
    /// latin (`bitmap_font::glyph`'s own doc), so a non-ASCII `char` here draws the missing-glyph
    /// box; what changed is that it draws *one* box per character instead of a run of wrong pictures,
    /// one per UTF-8 continuation byte.
    pub ch: char,
    /// How to paint it.
    pub attr: Attr,
}

// **Sixteen bytes a cell, as a gate rather than a hope.** A `char` (four) beside an [`Attr`] (nine,
// alignment one) pads to sixteen; before the truecolour pass it was eight. Every `Vt` holds
// `MAX_CELLS + SCROLLBACK_CELLS` of these, so this number is what the memory cost in
// [`SCROLLBACK_ROWS`]'s doc is computed from, and a change that grew the cell again (a wider flag
// word, a third colour for SGR 58's underline colour) must fail here and be priced there.
const _: () = assert!(core::mem::size_of::<Cell>() == 16);

impl Cell {
    /// A blank cell in `attr`. Erasing writes **spaces in the current rendition**, not zeroes, which
    /// is what makes `CSI K` on a coloured background leave the background rather than a black gap.
    pub const fn blank(attr: Attr) -> Cell {
        Cell { ch: ' ', attr }
    }
}

impl Default for Cell {
    fn default() -> Cell {
        Cell::blank(Attr::DEFAULT)
    }
}

#[cfg(test)]
mod tests;
