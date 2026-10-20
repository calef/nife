//! The atlas's own tests: the tables' shape, and that each face is the one its name says.
//!
//! Name: provisional.

use super::*;
extern crate std;
use std::vec::Vec;

fn cell(face: &Face, ch: char) -> Vec<u8> {
    (0..CELL_H)
        .flat_map(|y| (0..CELL_W).map(move |x| (x, y)))
        .map(|(x, y)| face.coverage(ch, x, y))
        .collect()
}

/// The tables are as long as [`FACE_BYTES`] says. [`Face`]'s doc says why this is a test and not
/// a type.
#[test]
fn the_tables_are_the_size_the_geometry_says() {
    for face in [&REGULAR, &BOLD, &OBLIQUE, &BOLD_OBLIQUE] {
        assert_eq!(face.0.len(), FACE_BYTES);
    }
}

/// Every printable character has a picture, no two share one, and none is the missing-glyph box.
///
/// A table generated with the repertoire shifted by one, a glyph index taken from the wrong
/// `cmap`, or a cell size the reader and the generator disagree on shows up here as a blank, a
/// collision or a box, and nowhere else until it is on a screen.
#[test]
fn every_printable_glyph_is_present_and_distinct() {
    for face in [&REGULAR, &BOLD, &OBLIQUE, &BOLD_OBLIQUE] {
        let missing = cell(face, '\u{e9}');
        assert!(
            missing.iter().any(|&a| a > 128),
            "the missing-glyph box is drawn"
        );
        assert!(
            cell(face, ' ').iter().all(|&a| a == 0),
            "the space is blank"
        );
        let cells: Vec<Vec<u8>> = (FIRST + 1..=LAST)
            .map(|cp| cell(face, char::from_u32(cp).unwrap()))
            .collect();
        for (i, c) in cells.iter().enumerate() {
            let ch = char::from_u32(FIRST + 1 + i as u32).unwrap();
            assert!(c.iter().any(|&a| a > 128), "{ch:?} has no ink");
            assert_ne!(*c, missing, "{ch:?} draws the missing-glyph box");
            for d in &cells[i + 1..] {
                assert_ne!(c, d, "{ch:?} shares its picture with a later glyph");
            }
        }
    }
}

/// **The four tables are four faces, in the order their names say.** A generator that wrote the
/// same font four times, or swapped two paths, would pass every other test here. Bold carries more
/// ink than its upright or oblique sibling, and an oblique `I`'s stem leans right: its ink near the top is
/// further right than near the baseline.
#[test]
fn each_face_is_the_one_its_name_says() {
    let ink = |face: &Face| cell(face, 'H').iter().map(|&a| a as u32).sum::<u32>();
    assert!(ink(&BOLD) > ink(&REGULAR) * 5 / 4);
    assert!(ink(&BOLD_OBLIQUE) > ink(&OBLIQUE) * 5 / 4);
    let center = |face: &Face, y: u32| {
        let row: Vec<u32> = (0..CELL_W)
            .map(|x| face.coverage('I', x, y) as u32)
            .collect();
        let total: u32 = row.iter().sum();
        row.iter()
            .enumerate()
            .map(|(x, &a)| x as u32 * a)
            .sum::<u32>()
            * 16
            / total
    };
    for (upright, leaning) in [(&REGULAR, &OBLIQUE), (&BOLD, &BOLD_OBLIQUE)] {
        let (top, bottom) = (ASCENT - 14, ASCENT - 3);
        assert!(
            center(leaning, top) > center(leaning, bottom) + 16,
            "the oblique leans right"
        );
        assert!(
            center(upright, top).abs_diff(center(upright, bottom)) < 16,
            "the upright does not"
        );
    }
}

/// **The way back is exact.** [`to_srgb`] searches [`SRGB_TO_LINEAR`] for the nearest code, which
/// is only sound if the table rises strictly; and every code must come back as itself, or a cell
/// at full coverage of an exotic color and one at 254 would disagree by more than the coverage.
#[test]
fn the_linear_table_rises_and_round_trips() {
    for c in 1..256 {
        assert!(
            SRGB_TO_LINEAR[c] > SRGB_TO_LINEAR[c - 1],
            "code {c} does not rise"
        );
    }
    for c in 0..=255u8 {
        assert_eq!(to_srgb(SRGB_TO_LINEAR[c as usize] as u32), c);
    }
    assert_eq!((SRGB_TO_LINEAR[0], SRGB_TO_LINEAR[255]), (0, u16::MAX));
}

/// **The rows the line renditions use are the font's**: the underline sits one row under the
/// baseline (`post`: -130 units, 90 thick) and the strikeout six rows above it (`OS/2`: 530
/// units, 102 thick), at 22 pixels per em. Pinned as numbers because nothing else in this crate
/// would notice them move, and the terminal draws its lines on them.
#[test]
fn the_line_rows_are_the_fonts() {
    assert_eq!((ASCENT, UNDERLINE_ROW, STRIKETHROUGH_ROW), (20, 21, 14));
    assert_eq!((GLYPHS, FACE_BYTES), (96, 34_944));
}

/// **A coordinate past the cell is background, in either direction**, never the next row's or
/// the next glyph's first pixel: the underscore's bottom-left pixel is ink, so reading one column
/// past its row above it would find ink if the bounds check let `x` through.
#[test]
fn a_coordinate_past_the_cell_has_no_coverage() {
    assert!(REGULAR.coverage('_', 0, ASCENT + 4) > 0);
    assert_eq!(REGULAR.coverage('_', CELL_W, ASCENT + 3), 0);
    assert_eq!(REGULAR.coverage('_', 0, CELL_H), 0);
}

/// **`cell_pixel` is the blend of the cell's coverage**, and the blend mixes every channel on its
/// own: a partial pixel of a three-channel foreground over a three-channel background lands on
/// each channel's own mix.
#[test]
fn a_pixel_is_its_coverage_blended_channel_by_channel() {
    let (fg, bg) = (0x00c8_6432, 0x0010_2030);
    for (x, y) in [(3, 10), (2, 10), (8, 10)] {
        let a = REGULAR.coverage('L', x, y);
        assert_eq!(REGULAR.cell_pixel('L', x, y, fg, bg), blend(fg, bg, a));
    }
    // 128 of 255 of each channel, in linear light: (200, 100, 50) over (16, 32, 48).
    assert_eq!(blend(fg, bg, 128), 0x0093_4b31);
}

/// **A value exactly between two codes rounds to the lower**, as [`to_srgb`]'s doc says, and a
/// value nearer either code rounds to that one. Ties are rare in a blend but deterministic, and the
/// three witnesses must agree on them too.
#[test]
fn a_tie_rounds_to_the_lower_code() {
    let tie = (1..255)
        .find(|&c| (SRGB_TO_LINEAR[c] + SRGB_TO_LINEAR[c + 1]) % 2 == 0)
        .expect("some pair of codes has an even sum");
    let mid = (SRGB_TO_LINEAR[tie] as u32 + SRGB_TO_LINEAR[tie + 1] as u32) / 2;
    assert_eq!(to_srgb(mid), tie as u8);
    assert_eq!(to_srgb(mid + 1), tie as u8 + 1);
    assert_eq!(to_srgb(mid - 1), tie as u8);
}
