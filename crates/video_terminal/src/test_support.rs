//! Helpers more than one module's tests use.
//!
//! Name: provisional (the lane of milestone 874 (the terminal engine moves its palette and its
//! tests out), 2026-10-12 UTC), named for what it holds, as `grant_plan`'s is under §272 (the
//! grant-plan crate root splits into private modules); calef names things.

use super::*;

/// A plain rendition in two indexed colours.
pub(super) fn attr(fg: u8, bg: u8) -> Attr {
    Attr::new(Colour::Indexed(fg), Colour::Indexed(bg))
}
