//! **An anti-aliased monospace font, and the arithmetic that turns a character into pixels**:
//! DejaVu Sans Mono (§104 (the rich-text font is DejaVu Sans Mono, and the palette is Solarized))
//! at a 14x26 cell, its four faces (§166 (the rasteriser dependency, and whether the glyph atlas
//! ships one face or four)) as coverage tables, and a pure function from `(character, x, y, fg,
//! bg)` to a color.
//!
//! The rasteriser is not here. It ran once, on the host, in `tools/glyph_atlas_generator`, and its
//! output is the four files under `coverage/`, one byte of coverage a pixel. So drawing a glyph is a
//! lookup and a blend in linear light, exactly as pure as `bitmap_font`'s bit test, and the three parties that draw,
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
//!
//! Name: provisional, for the crate and every item in it. calef names things.

#![no_std]

mod linear;
pub use linear::SRGB_TO_LINEAR;

/// A cell's width in pixels: Menlo Regular 11 at 2x, whose outlines are DejaVu's. 14 is the
/// 13.24-pixel advance rounded up, so every glyph has its own column of pixels and the grid stays
/// `x / CELL_W`.
///
/// Name: provisional.
pub const CELL_W: u32 = 14;
/// A cell's height in pixels: ascender to descender, 25.6 pixels at [`PIXELS_PER_EM`], rounded up.
///
/// Name: provisional.
pub const CELL_H: u32 = 26;
/// The size the outlines are drawn at, in pixels per em.
///
/// Name: provisional.
pub const PIXELS_PER_EM: u32 = 22;
/// Rows from the cell's top to the baseline. The font's ascender is 20.4 pixels; rounding down puts
/// the baseline on a pixel boundary, so a flat-bottomed letter ends on a whole row instead of half
/// a row of gray, and leaves 6 rows for the deepest descender (`_`, 5.2).
///
/// Name: provisional.
pub const ASCENT: u32 = 20;
/// The underline's row: the font's underline sits 1.4 pixels under the baseline and is one pixel
/// thick (`post` table, -130 and 90 units of 2048).
///
/// Name: provisional.
pub const UNDERLINE_ROW: u32 = ASCENT + 1;
/// The strikeout's row: the font's strikeout is centered 5.1 pixels above the baseline (`OS/2`
/// table, 530 and 102 units of 2048).
///
/// Name: provisional.
pub const STRIKETHROUGH_ROW: u32 = ASCENT - 6;

/// The first character in the table, the space.
///
/// Name: provisional.
pub const FIRST: u32 = 0x20;
/// The last character in the table, the tilde.
///
/// Name: provisional.
pub const LAST: u32 = 0x7e;
/// Glyphs in a face: `FIRST..=LAST`, then the missing-glyph box (the font's `.notdef`).
///
/// Name: provisional.
pub const GLYPHS: usize = (LAST - FIRST + 1) as usize + 1;
/// One face's table in bytes.
///
/// Name: provisional.
pub const FACE_BYTES: usize = GLYPHS * (CELL_W * CELL_H) as usize;

/// One face: [`GLYPHS`] cells of coverage, top row first, 0 for background and 255 for ink.
///
/// **A slice, not `[u8; FACE_BYTES]`, on purpose, and that is a deliberate step down a rung.** The
/// array type would make a table of the wrong size a compile error, but the generator reads its
/// geometry from this crate, so a change to [`CELL_W`] would then stop the one tool that can make
/// the new tables from building. The size is held instead by the generator's regenerate gate and by
/// this crate's `the_tables_are_the_size_the_geometry_says`.
///
/// Name: provisional.
pub struct Face(&'static [u8]);

/// The regular face, the one the terminal draws with.
///
/// Name: provisional.
pub static REGULAR: Face = Face(include_bytes!("../coverage/regular.coverage"));
/// The bold face, for SGR 1.
///
/// Name: provisional.
pub static BOLD: Face = Face(include_bytes!("../coverage/bold.coverage"));
/// The italic face, DejaVu's Oblique (§166 (the rasteriser dependency, and whether the glyph atlas
/// ships one face or four)), for SGR 3.
///
/// Name: provisional.
pub static OBLIQUE: Face = Face(include_bytes!("../coverage/oblique.coverage"));
/// The bold italic face, for SGR 1 and 3 together.
///
/// Name: provisional.
pub static BOLD_OBLIQUE: Face = Face(include_bytes!("../coverage/bold_oblique.coverage"));

impl Face {
    /// **How much of the pixel at `(x, y)` in `ch`'s cell is ink**, 0 to 255. Out-of-cell
    /// coordinates are 0 rather than a panic: the callers are pixel loops.
    ///
    /// Name: provisional.
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
    ///
    /// Name: provisional.
    pub fn cell_pixel(&self, ch: char, x: u32, y: u32, fg: u32, bg: u32) -> u32 {
        blend(fg, bg, self.coverage(ch, x, y))
    }
}

/// **`fg` over `bg` at `coverage` of 255, in linear light**: each `0x00RRGGBB` channel is taken
/// to linear light through [`SRGB_TO_LINEAR`], mixed there, and brought back to the sRGB code
/// whose linear value is nearest. 0 is exactly `bg` and 255 exactly `fg`.
///
/// Mixing the sRGB codes directly makes light text on a dark ground look bolder than its outline
/// and dark on light thinner, because sRGB's codes are not proportional to light; Solarized Dark
/// is the first case. Half coverage of white on black is half the light, sRGB 188, not code 128.
/// Integer only: the table is a `const`, and the way back is a search of the same table, so no
/// second table can disagree with the first.
///
/// ```
/// use glyph_atlas::blend;
///
/// assert_eq!(blend(0x00ff_ffff, 0x0000_0000, 0), 0x0000_0000);
/// assert_eq!(blend(0x00ff_ffff, 0x0000_0000, 255), 0x00ff_ffff);
/// assert_eq!(blend(0x00ff_ffff, 0x0000_0000, 128), 0x00bc_bcbc);
/// assert_eq!(blend(0x00ff_0000, 0x0000_00ff, 128), 0x00bc_00bb);
/// ```
///
/// Name: provisional.
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
        let f = SRGB_TO_LINEAR[((fg >> shift) & 0xff) as usize] as u32;
        let b = SRGB_TO_LINEAR[((bg >> shift) & 0xff) as usize] as u32;
        out |= (to_srgb((f * a + b * (255 - a) + 127) / 255) as u32) << shift;
        shift += 8;
    }
    out
}

/// The sRGB code whose linear value in [`SRGB_TO_LINEAR`] is nearest `linear`, the lower on a tie.
/// A binary search: the table rises strictly, which `the_linear_table_rises_and_round_trips`
/// holds it to.
const fn to_srgb(linear: u32) -> u8 {
    // The first code whose linear value is at least `linear`.
    let (mut lo, mut hi) = (0usize, 256usize);
    while lo < hi {
        let mid = (lo + hi) / 2;
        if (SRGB_TO_LINEAR[mid] as u32) < linear {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo == 256 {
        return u8::MAX;
    }
    if lo > 0 && linear - SRGB_TO_LINEAR[lo - 1] as u32 <= SRGB_TO_LINEAR[lo] as u32 - linear {
        return (lo - 1) as u8;
    }
    lo as u8
}

#[cfg(test)]
mod tests;
