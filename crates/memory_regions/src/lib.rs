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
//! # Examples
//!
//! ```
//! use memory_regions::split_new_watermark;
//!
//! // A 100-page parent that has spent 40: carving 10 more moves the watermark to 50.
//! assert_eq!(split_new_watermark(100, 40, 10), Some(50));
//!
//! // Exactly exhausting the budget is fine.
//! assert_eq!(split_new_watermark(100, 90, 10), Some(100));
//!
//! // One page beyond it is not. A child can never be bigger than what is left.
//! assert_eq!(split_new_watermark(100, 90, 11), None);
//! ```
//!
//! The two refusals that are not about the budget at all, and are the reason this is a function
//! rather than a subtraction written at the call site:
//!
//! ```
//! use memory_regions::split_new_watermark;
//!
//! // An empty carve would mint a zero-page region, which is not a thing.
//! assert_eq!(split_new_watermark(100, 40, 0), None);
//!
//! // And a wrap would forge budget out of arithmetic.
//! assert_eq!(split_new_watermark(100, 40, u64::MAX), None);
//! ```
//!
//! Name: ratified 2026-08-23 (calef, a kernel-dependency crate naming review). Renamed from
//! `regions`: DECISIONS §113 already rejected bare "Region" for the identical `crates/compositor`
//! collision (damaged-screen-rectangle "region") when choosing `MemoryRegion` for the kernel's
//! own object; this crate is that type's accounting logic. The kernel type itself was `Untyped`
//! until this same DECISIONS §113 entry, built out by milestone 158: this crate's own name and
//! vocabulary anticipated the rename by a session.

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
