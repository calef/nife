//! **An anti-aliased monospace font, and the arithmetic that turns a character into pixels**:
//! DejaVu Sans Mono (§104) at a 14x26 cell, its four faces (§166) as coverage tables, and a pure
//! function from `(character, x, y, fg, bg)` to a color.
//!
//! The rasteriser is not here. It ran once, on the host, in `tools/glyph_atlas_generator`, and its
//! output is the four files under `coverage/`, one byte of coverage a pixel. So drawing a glyph is a
//! lookup and a blend, exactly as pure as `bitmap_font`'s bit test, and the three parties that draw,
//! predict and grade the terminal's picture keep computing it independently without a font engine
//! in any of them. That generator's test fails if the tables are not what it makes from
//! `vendor/dejavu`, byte for byte.
//!
//! # Examples
//!
//! The baseline sits [`ASCENT`] rows down, so an `L`'s foot is the row just above it, and its stem
//! is on the left:
//!
//! ```
//! use glyph_atlas::{ASCENT, CELL_W, REGULAR};
//!
//! let ink = |x: u32, y: u32| REGULAR.coverage('L', x, y) > 128;
//! assert!((0..CELL_W).filter(|&x| ink(x, ASCENT - 1)).count() >= 6, "the foot runs along the baseline");
//! assert!(!ink(CELL_W / 2 + 3, ASCENT - 8), "and above it the right half is empty");
//! assert!(ink(4, ASCENT - 8), "while the stem is on the left");
//! assert!((0..CELL_W).all(|x| !ink(x, ASCENT)), "nothing of an L is below the baseline");
//! ```
//!
//! # BUGS
//!
//! - The repertoire is printable ASCII. Everything else draws the font's own missing-glyph box, as
//!   `bitmap_font` does. A bigger repertoire is a bigger table, up to the point (all of Unicode)
//!   where it is a different design.
//! - A glyph is clipped to its cell. The oblique faces lean up to 1.2 pixels past either side
//!   (`X`, `W`), which a terminal that overdraws its neighbors would show and this one cuts off.
//! - The blend is in sRGB, not linear light, so light text on a dark ground looks thinner than the
//!   font intends. Increment 4 of milestone 142 replaces [`blend`].
//!
//! Name: provisional, for the crate and every item in it. calef names things.

#![no_std]

/// A cell's width in pixels: Menlo Regular 11 at 2x, whose outlines are DejaVu's. 14 is the
/// 13.24-pixel advance rounded up, so every glyph has its own column of pixels and the grid stays
/// `x / CELL_W`.
pub const CELL_W: u32 = 14;
/// A cell's height in pixels: ascender to descender, 25.6 pixels at [`PIXELS_PER_EM`], rounded up.
pub const CELL_H: u32 = 26;
/// The size the outlines are drawn at, in pixels per em.
pub const PIXELS_PER_EM: u32 = 22;
/// Rows from the cell's top to the baseline. The font's ascender is 20.4 pixels; rounding down puts
/// the baseline on a pixel boundary, so a flat-bottomed letter ends on a whole row instead of half
/// a row of gray, and leaves 6 rows for the deepest descender (`_`, 5.2).
pub const ASCENT: u32 = 20;
/// The underline's row: the font's underline sits 1.4 pixels under the baseline and is one pixel
/// thick (`post` table, -130 and 90 units of 2048).
pub const UNDERLINE_ROW: u32 = ASCENT + 1;
/// The strikeout's row: the font's strikeout is centered 5.1 pixels above the baseline (`OS/2`
/// table, 530 and 102 units of 2048).
pub const STRIKETHROUGH_ROW: u32 = ASCENT - 6;

/// The first character in the table, the space.
pub const FIRST: u32 = 0x20;
/// The last character in the table, the tilde.
pub const LAST: u32 = 0x7e;
/// Glyphs in a face: `FIRST..=LAST`, then the missing-glyph box (the font's `.notdef`).
pub const GLYPHS: usize = (LAST - FIRST + 1) as usize + 1;
/// One face's table in bytes.
pub const FACE_BYTES: usize = GLYPHS * (CELL_W * CELL_H) as usize;

/// One face: [`GLYPHS`] cells of coverage, top row first, 0 for background and 255 for ink.
///
/// **A slice, not `[u8; FACE_BYTES]`, on purpose, and that is a deliberate step down a rung.** The
/// array type would make a table of the wrong size a compile error, but the generator reads its
/// geometry from this crate, so a change to [`CELL_W`] would then stop the one tool that can make
/// the new tables from building. The size is held instead by the generator's regenerate gate and by
/// this crate's `the_tables_are_the_size_the_geometry_says`.
pub struct Face(&'static [u8]);

/// The regular face, the one the terminal draws with.
pub static REGULAR: Face = Face(include_bytes!("../coverage/regular.coverage"));
/// The bold face. Drawn nowhere yet: until milestone 142 increment 5 lands, bold is bright.
pub static BOLD: Face = Face(include_bytes!("../coverage/bold.coverage"));
/// The italic face, DejaVu's Oblique (§166). Drawn nowhere yet, for [`BOLD`]'s reason.
pub static OBLIQUE: Face = Face(include_bytes!("../coverage/oblique.coverage"));
/// The bold italic face. Drawn nowhere yet, for [`BOLD`]'s reason.
pub static BOLD_OBLIQUE: Face = Face(include_bytes!("../coverage/bold_oblique.coverage"));

impl Face {
    /// **How much of the pixel at `(x, y)` in `ch`'s cell is ink**, 0 to 255. Out-of-cell
    /// coordinates are 0 rather than a panic: the callers are pixel loops.
    pub fn coverage(&self, ch: char, x: u32, y: u32) -> u8 {
        if x >= CELL_W || y >= CELL_H {
            return 0;
        }
        let cp = ch as u32;
        let glyph = if (FIRST..=LAST).contains(&cp) {
            (cp - FIRST) as usize
        } else {
            GLYPHS - 1
        };
        self.0[glyph * (CELL_W * CELL_H) as usize + (y * CELL_W + x) as usize]
    }

    /// **The color `ch`'s cell shows at `(x, y)`**: `fg` over `bg` by the glyph's coverage there.
    /// The terminal paints with this, the kernel test predicts with it and the host grades with it.
    pub fn cell_pixel(&self, ch: char, x: u32, y: u32, fg: u32, bg: u32) -> u32 {
        blend(fg, bg, self.coverage(ch, x, y))
    }
}

/// **`fg` over `bg` at `coverage` of 255**, each `0x00RRGGBB` channel mixed on its own and rounded
/// to nearest, so 0 is exactly `bg` and 255 exactly `fg`. In sRGB, see the crate's BUGS.
///
/// ```
/// use glyph_atlas::blend;
///
/// assert_eq!(blend(0x00ff_ffff, 0x0000_0000, 0), 0x0000_0000);
/// assert_eq!(blend(0x00ff_ffff, 0x0000_0000, 255), 0x00ff_ffff);
/// assert_eq!(blend(0x00ff_0000, 0x0000_00ff, 128), 0x0080_007f);
/// ```
pub const fn blend(fg: u32, bg: u32, coverage: u8) -> u32 {
    // Most of a screen is background or solid ink, and the kernel test runs this for every pixel
    // of the scanout; the two ends are also the two values the arithmetic below must return exactly.
    match coverage {
        0 => return bg,
        u8::MAX => return fg,
        _ => {}
    }
    let a = coverage as u32;
    let mut out = 0;
    let mut shift = 0;
    while shift < 24 {
        let (f, b) = ((fg >> shift) & 0xff, (bg >> shift) & 0xff);
        out |= ((f * a + b * (255 - a) + 127) / 255) << shift;
        shift += 8;
    }
    out
}

#[cfg(test)]
mod tests;
