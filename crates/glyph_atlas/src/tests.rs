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
    let centre = |face: &Face, y: u32| {
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
            centre(leaning, top) > centre(leaning, bottom) + 16,
            "the oblique leans right"
        );
        assert!(
            centre(upright, top).abs_diff(centre(upright, bottom)) < 16,
            "the upright does not"
        );
    }
}
