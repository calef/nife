//! **The atlas generator**: DejaVu Sans Mono's outlines in, `crates/glyph_atlas`'s coverage tables
//! out (milestone 142 (a text display good enough that people use it instead of a GUI) increment
//! 3).
//!
//! The rasteriser runs here, on the host, once, and its output is checked in. That split is what
//! keeps the three-party proof of on-screen text (the terminal draws, the kernel predicts, the host
//! grades) a set of table lookups rather than three copies of a font engine. This tool is not on
//! the verification path; the table is, and `tests/regenerate.rs` holds the table to exactly what
//! this tool makes from the vendored fonts, byte for byte.
//!
//! Three choices are §166 (the rasteriser dependency, and whether the glyph atlas ships one face or
//! four)'s and the determinism appendix's, not this file's, and are not to be re-litigated here:
//! `ttf-parser` plus `ab_glyph_rasterizer` 0.1.10; grayscale coverage, never subpixel; and coverage
//! quantized to `u8` at the boundary ([`quantize`]), never stored as `f32`.
//!
//! Name: provisional, for the crate and every item in it. calef names things.

use ab_glyph_rasterizer::{Point, Rasterizer, point};
use glyph_atlas::{ASCENT, CELL_H, CELL_W, FIRST, GLYPHS, LAST, PIXELS_PER_EM};
use ttf_parser::{Face, GlyphId, OutlineBuilder};

/// Each face: the vendored font it is drawn from, and the table it becomes. Paths are relative to
/// the repository root. `crates/glyph_atlas/src/lib.rs` names the same four tables in its
/// `include_bytes!`, and [`check`] fails on a table that is missing.
///
/// Name: provisional.
pub const FACES: [(&str, &str); 4] = [
    (
        "vendor/dejavu/DejaVuSansMono.ttf",
        "crates/glyph_atlas/coverage/regular.coverage",
    ),
    (
        "vendor/dejavu/DejaVuSansMono-Bold.ttf",
        "crates/glyph_atlas/coverage/bold.coverage",
    ),
    (
        "vendor/dejavu/DejaVuSansMono-Oblique.ttf",
        "crates/glyph_atlas/coverage/oblique.coverage",
    ),
    (
        "vendor/dejavu/DejaVuSansMono-BoldOblique.ttf",
        "crates/glyph_atlas/coverage/bold_oblique.coverage",
    ),
];

/// One face's table: [`GLYPHS`] cells of `CELL_W * CELL_H` coverage bytes, top row first, in the
/// order `glyph_atlas` reads them (`FIRST..=LAST`, then the missing-glyph box, the font's
/// `.notdef`).
///
/// # Panics
///
/// On a font that does not parse, lacks a glyph in the repertoire, or is not monospaced at the
/// advance the cell was sized for. Each is a wrong input, and a table drawn from one would be wrong
/// in a way nothing downstream could see.
///
/// Name: provisional.
pub fn table(font: &[u8]) -> Vec<u8> {
    let face = Face::parse(font, 0).expect("the vendored font parses");
    let scale = PIXELS_PER_EM as f32 / face.units_per_em() as f32;
    let mut out = Vec::with_capacity(GLYPHS * (CELL_W * CELL_H) as usize);
    let glyphs = (FIRST..=LAST)
        .map(|cp| {
            let ch = char::from_u32(cp).expect("the repertoire is scalar values");
            face.glyph_index(ch)
                .unwrap_or_else(|| panic!("the font has no glyph for {ch:?}"))
        })
        .chain([GlyphId(0)]);
    for id in glyphs {
        let advance = face
            .glyph_hor_advance(id)
            .expect("every glyph has an advance");
        // Centered in the cell: the advance is 13.24 pixels at 22 per em, so each side gets the
        // same fraction of the 0.76 left over rather than all of it landing on the right.
        let left = (CELL_W as f32 - advance as f32 * scale) / 2.0;
        let mut pen = Pen {
            raster: Rasterizer::new(CELL_W as usize, CELL_H as usize),
            scale,
            left,
            start: point(0.0, 0.0),
            last: point(0.0, 0.0),
        };
        // A glyph with no outline (the space) is all background, which is what an untouched
        // rasteriser yields.
        face.outline_glyph(id, &mut pen);
        let mut cell = vec![0u8; (CELL_W * CELL_H) as usize];
        pen.raster.for_each_pixel_2d(|x, y, a| {
            cell[(y * CELL_W + x) as usize] = quantize(a);
        });
        out.extend_from_slice(&cell);
    }
    out
}

/// **The `u8` boundary.** `ab_glyph_rasterizer`'s raw `f32` output has changed in a patch release,
/// and its accumulated error leaves coverage a hair above zero outside a glyph or a hair above one
/// inside it. Rounding to 256 levels here absorbs both, and nothing past this function sees a float.
///
/// Name: provisional.
pub fn quantize(a: f32) -> u8 {
    (a.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// The outline, in font units with y up, drawn into the cell, in pixels with y down and the
/// baseline [`ASCENT`] rows from the top.
struct Pen {
    raster: Rasterizer,
    scale: f32,
    left: f32,
    start: Point,
    last: Point,
}

impl Pen {
    fn at(&self, x: f32, y: f32) -> Point {
        point(x * self.scale + self.left, ASCENT as f32 - y * self.scale)
    }
}

impl OutlineBuilder for Pen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.start = self.at(x, y);
        self.last = self.start;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.at(x, y);
        self.raster.draw_line(self.last, p);
        self.last = p;
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (c, p) = (self.at(x1, y1), self.at(x, y));
        self.raster.draw_quad(self.last, c, p);
        self.last = p;
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (c1, c2, p) = (self.at(x1, y1), self.at(x2, y2), self.at(x, y));
        self.raster.draw_cubic(self.last, c1, c2, p);
        self.last = p;
    }

    // A contour the font left open is closed with a line back to its start, as every TrueType
    // renderer does; without it the coverage of the open side would leak to the cell's edge.
    fn close(&mut self) {
        if self.last != self.start {
            self.raster.draw_line(self.last, self.start);
        }
        self.last = self.start;
    }
}

/// The repository root, from this crate's own manifest directory.
///
/// Name: provisional.
pub fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every face's table as this tool makes it, beside its path.
///
/// Name: provisional.
pub fn tables() -> Vec<(&'static str, Vec<u8>)> {
    FACES
        .iter()
        .map(|(font, table_path)| {
            let bytes =
                std::fs::read(root().join(font)).unwrap_or_else(|e| panic!("reading {font}: {e}"));
            (*table_path, table(&bytes))
        })
        .collect()
}

/// Where the sRGB-to-linear table is written: Rust source, so the runtime crate reads it as a
/// `const` and a reviewer reads it as numbers.
///
/// Name: provisional.
pub const LINEAR_PATH: &str = "crates/glyph_atlas/src/linear.rs";

/// **The sRGB-to-linear table as Rust source**: each of the 256 sRGB codes as linear light scaled
/// to `u16::MAX`, by IEC 61966-2-1's curve, rounded to nearest. Computed here in `f64` so the
/// runtime crate does no floating point: `glyph_atlas::blend` only indexes the table and searches
/// it. `powf` is the platform's, so a host whose `pow` rounds differently at one of these 256
/// points fails the regenerate gate rather than shipping a different table.
///
/// Name: provisional.
pub fn linear_source() -> String {
    let mut out = String::from(
        "//! **sRGB to linear light**, one `u16` per sRGB code, written by \
         `tools/glyph_atlas_generator`\n\
         //! (`linear_source`) and checked by its regenerate gate. Do not edit; regenerate.\n\
         //!\n\
         //! Name: provisional.\n\n\
         /// Linear light for each sRGB code, `0` black and `u16::MAX` white.\n\
         ///\n\
         /// Name: provisional.\n\
         #[rustfmt::skip] // the generator's layout, so the regenerate gate compares bytes\n\
         pub const SRGB_TO_LINEAR: [u16; 256] = [\n",
    );
    for row in 0..32 {
        out.push_str("   ");
        for code in row * 8..row * 8 + 8 {
            let c = code as f64 / 255.0;
            let linear = if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            };
            out.push_str(&format!(" {},", (linear * 65535.0).round() as u16));
        }
        out.push('\n');
    }
    out.push_str("];\n");
    out
}

/// Everything the generator writes, beside its path: the four face tables and the linear table.
///
/// Name: provisional.
pub fn outputs() -> Vec<(&'static str, Vec<u8>)> {
    let mut all = tables();
    all.push((LINEAR_PATH, linear_source().into_bytes()));
    all
}

/// The first difference between the checked-in tables and what this tool makes now, described in
/// glyphs and pixels, or `None` when every table is byte-identical.
///
/// Name: provisional.
pub fn check() -> Option<String> {
    for (path, want) in outputs() {
        let got = match std::fs::read(root().join(path)) {
            Ok(b) => b,
            Err(e) => return Some(format!("{path}: {e}")),
        };
        if path == LINEAR_PATH && got != want {
            return Some(format!("{path}: differs from what the generator writes"));
        }
        if got.len() != want.len() {
            return Some(format!(
                "{path}: {} bytes checked in, the generator makes {}",
                got.len(),
                want.len()
            ));
        }
        if let Some(i) = (0..got.len()).find(|&i| got[i] != want[i]) {
            let per = (CELL_W * CELL_H) as usize;
            let (glyph, px) = (i / per, i % per);
            let cp = FIRST + glyph as u32;
            let name = if cp <= LAST {
                format!("{:?}", char::from_u32(cp).unwrap_or('?'))
            } else {
                "the missing-glyph box".into()
            };
            return Some(format!(
                "{path}: {name} differs at ({}, {}): checked in {}, generated {}",
                px as u32 % CELL_W,
                px as u32 / CELL_W,
                got[i],
                want[i]
            ));
        }
    }
    None
}
