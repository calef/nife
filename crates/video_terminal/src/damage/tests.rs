use super::*;

// ---- survivor triage of the 2026-10-03 census ---------------------------------------------

/// A report with nothing scrolled repaints what changed, not the whole grid.
#[test]
fn an_unscrolled_report_repaints_its_own_rectangle() {
    let rect = CellRect {
        col: 2,
        row: 1,
        cols: 3,
        rows: 1,
    };
    assert_eq!(Damage { scrolled: 0, rect }.repaint_rect(80, 24), rect);
    let all = Damage { scrolled: 1, rect }.repaint_rect(80, 24);
    assert_eq!((all.col, all.row, all.cols, all.rows), (0, 0, 80, 24));
}

/// **The damage box's arithmetic, at the edges where a bounding box is decided.** A rectangle
/// that already contains the other must be its own union either way round, which is the case
/// that reads both operands' far edges; the cell-by-cell unions elsewhere only ever grow the
/// second operand's, so half of `union` was never consulted.
#[test]
fn a_damage_box_contains_a_rectangle_it_already_covered() {
    let big = CellRect {
        col: 1,
        row: 2,
        cols: 4,
        rows: 3,
    };
    let inside = CellRect {
        col: 2,
        row: 3,
        cols: 1,
        rows: 1,
    };
    assert_eq!(big.union(inside), big);
    assert_eq!(inside.union(big), big);
    // Cells are 14 by 26, so a rect at (1, 2) starts at (14, 52) and is 56 by 78. The two axes
    // differ on purpose: a single constant used for both would pass whatever it was.
    assert_eq!(big.to_pixels(), (14, 52, 56, 78));
}
