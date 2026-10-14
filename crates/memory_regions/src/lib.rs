//! **The memory-region accounting, as pure logic** (object revocation, DECISIONS §16).
//!
//! The kernel's memory-region allocator (`kernel/src/memory_region.rs`) is a bump watermark per region, with a
//! parent/child tree from `SPLIT` and a LIFO return-of-pages on `DESTROY`. The scary property is
//! **no double-free**: a physical page must reach the frame allocator at most once across any
//! sequence of create/split/destroy. That safety rests on two arithmetic decisions, and this crate
//! is those decisions, extracted so Kani can prove them and the kernel then *calls* the proved code
//! rather than keeping a parallel copy (the discipline in notes/verification.md).
//!
//! The two decisions:
//!
//! - [`split_new_watermark`]: carving a child off a parent's budget. It advances the parent's
//!   watermark, and the property is that a successful carve stays within budget, does not overflow,
//!   and strictly progresses, so consecutive carves are disjoint runs within the parent.
//! - [`destroy_outcome`]: reclaiming a region. The property is that a region with a live object
//!   (`pinned`) or a live child refuses; a **root** frees its run to the allocator; and a **child**
//!   *never* frees to the allocator, its pages return to the parent (LIFO un-bump, or a hole). "A
//!   child never frees to global" is the no-double-free crux: a page reaches the allocator only when
//!   the root that owns it is destroyed, and each root owns one contiguous run from one allocation.
//!
//! Everything here works in **page units** (a watermark and page counts), so it is `FRAME_SIZE`- and
//! address-agnostic; the kernel supplies the base addresses and does the byte arithmetic and the I/O.
//!
//! Name: ratified 2026-08-23 (calef, a kernel-dependency crate naming review). Renamed from
//! `regions`, which collides with `crates/compositor`'s damaged-screen "region". DECISIONS §113
//! (eleven kernel object and identifier names move) found that when naming `MemoryRegion`, the
//! kernel object whose accounting this crate is.

#![cfg_attr(not(test), no_std)]

mod table;

pub use table::{DestroyClaim, NO_PARENT, ObjectKind, PageUse, RegionTable};

mod destroy;
mod watermark;

pub use destroy::{DestroyOutcome, destroy_outcome};
pub use watermark::{coalesced_watermark, retype_new_watermark, retype_pages, split_new_watermark};

#[cfg(kani)]
mod proofs;

#[cfg(test)]
mod tests;
