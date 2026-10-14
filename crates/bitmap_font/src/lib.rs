//! **A 7x8 bitmap font, and the arithmetic that turns a character into pixels**: a constant table
//! of monochrome glyphs and a pure function from `(character, x, y)` to a colour.
//!
//! Not a scalable font, which wants an allocator, floating point, a hinting rasteriser and a file
//! to load, none of which a `no_std` component has. Here the glyphs are a `static`, drawing one is
//! a bit test, and the expected picture is a function three parties evaluate independently: the
//! terminal that draws ([`cell_pixel`]), the kernel test that checks, and the host that reads the
//! scanout back out of QEMU. That is what makes on-screen text provable rather than plausible. The
//! classic bug, a mirrored font, hides in symmetric letters; [`is_ink`]'s example prints an `F`.
//!
//! Not here: kerning, proportional widths, anti-aliasing, anything past basic latin ([`glyph`]
//! draws a box for it) and font loading. The limits are in notes/glyphs.md.
//!
//! Name: ratified 2026-08-23 (calef), renamed from `bitfont` to spell out the contraction.

#![no_std]

pub mod glyphs;

mod render;

pub use render::{cell_pixel, glyph, is_ink};

/// A glyph cell's width in pixels.
///
/// The font is a **fixed-pitch** 7x8, which is what lets a terminal grid be `x / GLYPH_W` and
/// nothing more.
///
/// Seven, not eight, and the reason is the Kaypro II's video board: it shifted out a zero, five
/// bits of character generator, and a zero, so the cell is five ink columns with a one-pixel gutter
/// on each side. Every glyph here keeps that gutter (asserted below), which is where the
/// inter-character gap comes from, so the advance is 7 and there is no tracking to add.
///
/// The narrower cell is also what makes the terminal usable: 128 / 7 is **18 columns** where 128 / 8
/// was 16. The scanout is not a whole multiple of 7, so the two rightmost pixels of a full-width
/// surface are outside the grid and stay background; see `components/src/display_terminal.rs`.
pub const GLYPH_W: u32 = 7;

/// A glyph cell's height in pixels. Eight rows, of which the last carries only the descenders
/// (`g j p q y , ;`) and the underscore, so consecutive text rows do not collide.
pub const GLYPH_H: u32 = 8;

/// **The glyph for a byte this font has no picture for**: a hollow box, the convention every
/// terminal uses for "I cannot draw this".
///
/// It exists so that [`glyph`] is *total*. A font that returned `None`, or blank, for an unmapped
/// byte would make a mojibake bug look like a spacing bug, and the whole point of drawing something
/// visible is that a wrong byte in the grid shows up as a wrong picture rather than as nothing.
pub static MISSING: [u8; 8] = [0x3e, 0x22, 0x22, 0x22, 0x22, 0x22, 0x3e, 0x00];

#[cfg(test)]
mod tests;
