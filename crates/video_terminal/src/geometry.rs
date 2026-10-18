//! The grid's capacity: the most columns and rows a `Vt` holds, and its scrollback budget.
//!
//! Name: provisional (the lane of milestone 874 (the terminal engine moves its palette and its
//! tests out), 2026-10-12 UTC), named for the "Geometry" banner it was under in `lib.rs`; calef
//! names things.

// Only the doc links below name the crate root's items, so the import exists only for rustdoc.
#[cfg(doc)]
use super::*;

/// The widest grid this engine can hold, in cells.
///
/// Fixed rather than allocated, because the terminal component has no allocator and this crate is
/// the same code the kernel and the host run. **Grown from 32 to 182 at milestone 142's increment
/// 1, then retargeted to 132 on 2026-08-27** alongside the scanout (128x64 -> 1280x720 -> 924x344,
/// `graphics_contract::WIDTH`'s doc comment has the full story). 132 is exactly
/// `graphics_contract::WIDTH / bitmap_font::GLYPH_W` (924 / 7), the classic VT100/VT220 "wide mode"
/// column count and roughly what a real terminal actually runs, unlike 182's near-double of any
/// terminal anyone uses. A screen bigger than that gets a bigger constant, and the terminal
/// component asserts its own geometry fits at compile time so the failure is a build error rather
/// than a truncated screen.
///
/// **Grown to 240 on 2026-10-04** (the screen terminal lane), when the terminal on a firmware
/// screen stopped being the contract's 924x344 and started filling the screen at
/// `screen_console::ScreenConsole::scale_for`'s scale. That rule never gives a screen
/// `2 * screen_console::MIN_COLUMNS` (240) columns or more, so 240 is every screen's width in
/// cells; QEMU's 1280x800 OVMF screen is 182, xenon's 1920x1080 is 137. The virtio-gpu scanout
/// still lays out 132 (`script::COLS`); only the capacity grew.
pub const MAX_COLS: usize = 240;
/// The tallest grid this engine can hold, in cells. See [`MAX_COLS`]. Grown from 16 to 90, then
/// retargeted to 43 (924x344's `HEIGHT` / 8, the font's row height), the VT100/VT220 "wide mode"
/// row count, then grown to 150 with [`MAX_COLS`] (2026-10-04): a 1200-pixel-tall screen at scale
/// one. **Not every pair is reachable**: a grid also fits in [`MAX_CELLS`], so 240 columns get 76
/// rows and 150 rows need 122 columns or fewer.
pub const MAX_ROWS: usize = 150;
/// Cells in the largest possible **live** grid (the on-screen viewport; see [`SCROLLBACK_ROWS`] for
/// the off-screen history alongside it). A real cost, paid once per terminal instance and
/// auto-provisioned from that program's own region (the kernel's `user::load` sizes a process's
/// address-space region from its ELF segments, `.bss` included), not from any shared budget.
///
/// **A budget rather than `MAX_COLS * MAX_ROWS`** (2026-10-04, the screen terminal lane). The grid
/// is stored at its own width, so any shape up to this many cells fits. 18,432 holds QEMU's
/// 1280x800 OVMF screen (182x100) and xenon's 1920x1080 at scale two (137x67). The first attempt
/// sized it 240x150 and grew a `Vt` to 1.73 MB, and `graphical_terminal`'s session, measured at
/// 490-495 of its 528 pages, could no longer be built. The live grid and the scrollback
/// ([`SCROLLBACK_CELLS`]) now sum to 44,832 cells, fewer than the 45,276 of 132x43 plus 300 rows
/// of 132 they replaced, so a `Vt` is no larger than it was.
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
/// at the grid's own width, so it is 300 rows deep up to 88 columns and shallower past that: 200
/// at the virtio scanout's 132, 192 at xenon's 137, 145 at OVMF's 182. See [`Vt::scrollback_depth`].
pub const SCROLLBACK_ROWS: usize = 300;
/// Cells in the scrollback ring: 200 rows of the virtio scanout's 132 columns. See
/// [`SCROLLBACK_ROWS`] and [`MAX_CELLS`] for why it is a cell budget.
pub const SCROLLBACK_CELLS: usize = 26_400;
