//! Name: provisional, minted 2026-10-19 (UTC) by the lane for milestone 23.

use super::*;
use crate::Vt;
use crate::geometry::{MAX_ROWS, SCROLLBACK_ROWS};
extern crate std;
use std::vec;

/// A terminal with everything a swap must carry set to something other than its default: a
/// scrollback ring `lines` deep (past 200 at 132 columns it has wrapped), a rendition in force, a wrap pending, a scrolled-back view,
/// and a 24-bit color sequence cut off after its `2;`, so the parser is mid-CSI with three
/// parameters taken.
fn busy(cols: u32, rows: u32, lines: u32) -> Vt {
    let mut t = Vt::new(cols, rows);
    for i in 0..lines {
        t.feed(std::format!("\x1b[3{}mline {i}\x1b[0m\r\n", i % 8).as_bytes());
    }
    t.feed(b"\x1b[1;4m");
    t.feed(&vec![b'x'; cols as usize]);
    t.feed(b"\x1b[38;2;");
    t.scroll_up(17);
    t
}

fn blob_of(t: &Vt) -> std::vec::Vec<u8> {
    let mut out = vec![0u8; t.handoff_len()];
    assert_eq!(t.write_handoff(&mut out), Some(out.len()));
    out
}

/// **What a replacement absorbs behaves as the incumbent would have**, which is the claim a
/// swap makes to the person at the screen. Compared on every pixel of the scrolled-back view, on
/// every row of history, and after feeding both the rest of the cut sequence and more text, so a
/// field the blob dropped shows as a difference somewhere a person would see it.
#[test]
fn an_absorbed_terminal_carries_on_exactly_where_the_incumbent_stopped() {
    // A ring that has wrapped, and one that has not, whose tail is not at slot zero.
    carries_on(400);
    carries_on(90);
}

fn carries_on(lines: u32) {
    let mut old = busy(132, 43, lines);
    let blob = blob_of(&old);
    let mut new = Vt::new(132, 43);
    new.absorb_handoff(&blob).expect("refused its own blob");
    assert_eq!(blob_of(&new), blob, "the blob does not read back to itself");

    let same = |a: &Vt, b: &Vt, what: &str| {
        for y in 0..a.height() {
            for x in 0..a.width() {
                assert_eq!(a.pixel(x, y), b.pixel(x, y), "{what}: pixel ({x}, {y})");
            }
        }
    };
    same(&old, &new, "the scrolled-back view");
    for depth in [0, 1, 150, old.scrollback_len()] {
        old.scroll_down(u32::MAX);
        new.scroll_down(u32::MAX);
        old.scroll_up(depth);
        new.scroll_up(depth);
        same(&old, &new, "history");
    }
    // The rest of the sequence: a red-orange `R`, which needs the parameters taken before the
    // swap, then more lines, which need the rendition, the pending wrap and the ring's tail.
    for t in [&mut old, &mut new] {
        t.feed(b"255;64;0mR\x1b[0m");
        for i in 0..60 {
            t.feed(std::format!("after {i}\r\n").as_bytes());
        }
    }
    assert_eq!(new.cell(1, 0), old.cell(1, 0));
    assert_eq!(blob_of(&new), blob_of(&old));
    same(&old, &new, "after more output");
}

/// A refused blob leaves the terminal as it was, so a replacement that refuses can still be told
/// apart from one that absorbed half: the supervisor collects it either way, but nothing it
/// reports is a lie about the state it held.
#[test]
fn every_refusal_leaves_the_terminal_untouched() {
    let blob = blob_of(&busy(132, 43, 400));
    let mut t = Vt::new(132, 43);
    t.feed(b"mine\r\n");
    let before = blob_of(&t);

    let mut bad = blob.clone();
    bad[0] ^= 1;
    assert_eq!(t.absorb_handoff(&bad), Err(Refusal::NoBlob));
    let mut bad = blob.clone();
    bad[4] = 9;
    assert_eq!(t.absorb_handoff(&bad), Err(Refusal::Layout(9)));
    // The last cell's character, a surrogate: no `char` is that.
    let mut bad = blob.clone();
    let last = bad.len() - 12;
    bad[last..last + 4].copy_from_slice(&0xd800u32.to_le_bytes());
    assert_eq!(t.absorb_handoff(&bad), Err(Refusal::Malformed));
    assert_eq!(
        t.absorb_handoff(&blob[..blob.len() - 1]),
        Err(Refusal::Malformed)
    );
    let mut wide = Vt::new(133, 43);
    assert_eq!(wide.absorb_handoff(&blob), Err(Refusal::Geometry));

    assert_eq!(blob_of(&t), before, "a refusal changed the terminal");
}

/// **The largest blob fits what a supervisor retypes for it.** A full live grid and a full ring at
/// the shape that fills both budgets exactly, which is the worst case [`HANDOFF_BYTES`] claims.
#[test]
fn the_largest_blob_fits_the_handoff_pages() {
    // 96 columns divides both MAX_CELLS (192 rows, clamped to MAX_ROWS) and SCROLLBACK_CELLS
    // (275 rows); 88 is the widest ring that is a full SCROLLBACK_ROWS deep.
    for cols in [88, 96, 132] {
        let mut t = Vt::new(cols, u32::MAX);
        for _ in 0..SCROLLBACK_ROWS + MAX_ROWS {
            t.feed(b"\r\n");
        }
        assert_eq!(t.scrollback_len(), t.scrollback_depth());
        assert!(t.handoff_len() <= HANDOFF_BYTES);
        assert!(t.write_handoff(&mut vec![0u8; HANDOFF_BYTES]).is_some());
    }
    assert!(HANDOFF_BYTES as u64 <= HANDOFF_PAGES * address_space_map::PAGE);
}
