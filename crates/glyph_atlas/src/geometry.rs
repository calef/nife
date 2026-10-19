//! The cell's geometry and the table's shape: the numbers the generator draws at and the reader
//! reads by.
//!
//! Name: provisional.

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
