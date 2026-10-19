//! The grid's capacity: the most columns and rows a `Vt` holds, and its scrollback budget.
//!
//! Name: provisional (the lane of milestone 874 (the terminal engine moves its palette and its
//! tests out), 2026-10-12 UTC), named for the "Geometry" banner it was under in `lib.rs`; calef
//! names things.

// Only the doc links below name the crate root's items, so the import exists only for rustdoc.
#[cfg(doc)]
use super::*;

/// A cell's width in pixels: `glyph_atlas`'s, since every pixel this engine draws comes from that
/// atlas. A consumer laying a `Vt` out over a surface divides by this, never by a font's own
/// constant, so the grid and the picture cannot disagree about where a cell ends.
pub const CELL_W: u32 = glyph_atlas::CELL_W;
/// A cell's height in pixels. See [`CELL_W`].
pub const CELL_H: u32 = glyph_atlas::CELL_H;

/// The widest grid this engine can hold, in cells.
///
/// Fixed rather than allocated, because the terminal component has no allocator and this crate is
/// the same code the kernel and the host run. The terminal component asserts its own geometry fits
/// at compile time, so a screen too wide is a build error rather than a truncated screen.
///
/// 240 is every screen's width in cells: the terminal on a firmware screen fills it at
/// `screen_console::ScreenConsole::scale_for_cell`'s scale for [`CELL_W`], and that rule never gives
/// a screen `2 * screen_console::MIN_COLUMNS` (240) columns or more. QEMU's 1280x800 OVMF screen is
/// 91, xenon's 1920x1080 is 137. The virtio-gpu scanout lays out 80 (`script::COLS`).
pub const MAX_COLS: usize = 240;
/// The tallest grid this engine can hold, in cells. See [`MAX_COLS`]. **Not every pair is
/// reachable**: a grid also fits in [`MAX_CELLS`], so 240 columns get 76 rows and 150 rows need 122
/// columns or fewer.
pub const MAX_ROWS: usize = 150;
/// Cells in the largest possible **live** grid (the on-screen viewport; see [`SCROLLBACK_ROWS`] for
/// the off-screen history alongside it). A real cost, paid once per terminal instance and
/// auto-provisioned from that program's own region (the kernel's `user::load` sizes a process's
/// address-space region from its ELF segments, `.bss` included), not from any shared budget.
///
/// **A budget rather than `MAX_COLS * MAX_ROWS`**: the grid is stored at its own width, so any
/// shape up to this many cells fits. 18,432 is far more than any screen this tree drives needs at
/// the 14x26 cell (xenon's 137x41 is 5,617). It stays this size because it also sizes the handoff
/// blob (`handoff::HANDOFF_PAGES`) and `graphical_terminal`'s measured session budget, so a smaller
/// one is a change to both. The live grid and the scrollback ([`SCROLLBACK_CELLS`]) sum to 44,832
/// cells.
pub const MAX_CELLS: usize = 18_432;

/// **Off-screen history**, in whole rows, kept alongside the live grid (milestone 142 increment 2).
///
/// A fixed ring rather than a `Vec`, for the same reason the live grid is a fixed array: this crate
/// reaches no allocator, so the capacity is a constant three parties (the terminal, the kernel test,
/// the host-side check) already agree on the same way they agree on [`MAX_COLS`]/[`MAX_ROWS`].
///
/// **300, chosen as a working depth rather than derived from anything.** At 132 columns and
/// sixteen bytes a [`Cell`] the ring was 633,600 bytes of `.bss` and a whole `Vt` 724,416 (362,208
/// before the truecolour pass widened the cell, 2026-09-26); since 2026-10-04 it is the
/// [`SCROLLBACK_CELLS`] budget instead, and a `Vt`'s cells are 717,312 bytes. That is
/// 177 page frames per terminal instance, a small fraction of the free page-frame pool a terminal's
/// own region draws from (see notes/frames.md's measurement that hundreds of page frames are "under
/// one percent of the free pool"). The kernel's test image holds eight `Vt` statics as witnesses
/// (`system_tests/src/user/display_tests.rs` and `compositor_tests.rs`), so the widening cost it about
/// 2.9 MB of `.bss` against QEMU's 256 MiB. There is no principled reason it could not be larger or smaller; it is a constant a
/// future lane can change without touching the shape of the ring around it.
///
/// **The most rows, not always the rows** (2026-10-04): the ring holds [`SCROLLBACK_CELLS`] cells
/// at the grid's own width, so it is 300 rows deep up to 88 columns, which covers the virtio
/// scanout's 80 and OVMF's 91 (290), and shallower past that: 192 at xenon's 137. See
/// [`Vt::scrollback_depth`].
pub const SCROLLBACK_ROWS: usize = 300;
/// Cells in the scrollback ring: 200 rows of 132 columns. See
/// [`SCROLLBACK_ROWS`] and [`MAX_CELLS`] for why it is a cell budget.
pub const SCROLLBACK_CELLS: usize = 26_400;
