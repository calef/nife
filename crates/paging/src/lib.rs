//! Page tables, portable across formats. Each hardware format ([`Aarch64`], [`Sv39`], and x86's)
//! supplies the few encode and decode operations the walk needs through [`PageFormat`], and the walk
//! itself (descend the levels, allocate tables, write a leaf) is written once in [`Mapper`]: §17
//! (the second architecture: RISC-V, and the page-table format trait), notes/riscv-port.md.
//!
//! Every format uses 512-entry, 4 KiB tables and 4 KiB pages, so a table is exactly one page and
//! the frame allocator supplies page tables and nothing else. The same arithmetic gives 2 MiB and
//! 1 GiB leaves one and two levels up (a block, a megapage, a large page): [`PageSize`] names
//! them, and [`Mapper::map_span`] picks the largest that fits, which a direct map of RAM wants.
//!
//! **A block is a leaf the walk meets early, and every walk has to know that.** `map` refuses to
//! descend through one (it would write a leaf into somebody's data), `unmap` refuses to take a
//! 4 KiB bite out of one ([`MapError::InsideBlock`]), and `translate` stops at one and adds the
//! offset. Nothing splits a block: on a live table that is the break-before-make hazard
//! [`TlbFlush`] exists for, and nothing here has needed it.
//!
//! It is pure logic, so the host tests build real tables in host memory, using allocations as
//! pretend physical frames (a `Box<PageTable>` is 4 KiB-aligned by its type), and walk them back
//! with pointer arithmetic identical to the kernel's. [`Mapper::map`] and [`Mapper::unmap`] show
//! it.
//!
//! Name: ratified 2026-08-01 (calef, the naming tenet): a standard term that is already right.

#![cfg_attr(not(test), no_std)]

use core::marker::PhantomData;

pub mod aarch64;
pub mod domain;
pub mod sv39;
// The third format (milestone 161). Named `x86_64` for the architecture, like `aarch64` above,
// rather than for the mode; the type inside is `Ia32e`, which is Intel's own name for it.
pub mod x86_64;

pub use aarch64::Aarch64;
pub use domain::{DmaRegion, build_identity_domain};
pub use sv39::{Sv39, Sv39Mae};
pub use x86_64::{AmdVi, Ia32e, Vtd};

// --- Portable access flags ---
//
// `Flags` used to *be* the aarch64 descriptor's permission bits. It is now a format-neutral set of
// capabilities: what access this mapping grants, said in terms every format can encode. Each
// `PageFormat` translates these to and from its own descriptor bits (see `leaf_entry`/`leaf_flags`).
// The constructor and predicate API is unchanged, so consumers that ask for `Flags::user_code()` or
// test `is_writable()` did not move.

#[cfg(test)]
mod flag_tests;

#[cfg(test)]
mod geometry_tests;

/// Machine-checked proofs of the leaf-size choice [`Mapper::map_span`] makes for every block it
/// writes. **This is the property that makes a block mapping safe**: a block that ran past the span
/// it was asked for would map bytes nobody granted, which on the direct map means a second,
/// writable alias of the kernel image or a cacheable view of a device window. Loopless arithmetic,
/// so it is proved for every address and length rather than tested through a built table (the
/// "prefer refactoring the logic to shrinking the proof" move notes/verification.md records for
/// `domain::grant_pages`). The formats' own block encodings are proved in their modules.
#[cfg(kani)]
mod verification;

mod flags;
mod mapper;
mod page_format;
mod page_size;
mod page_table;
mod tlb_flush;

pub use flags::Flags;
use flags::*;
pub use mapper::{MapError, Mapper};
pub use page_format::{Half, PageFormat, is_user_page_va};
pub use page_size::{PAGE_SIZE, PageSize};
pub use page_table::{ENTRIES, PageTable};
pub use tlb_flush::TlbFlush;

/// **The eighth outsider pass's host attacks on the geometry gates** (claims 10 and 11): the
/// split-edge sweep the cited per-format tests do not shape, and the cross-format W^X sweep over
/// every public constructor. Name: provisional (this pass's lane, 2026-10-13 UTC).
#[cfg(test)]
mod eighth_pass_attacks {
    use super::*;

    /// **The user-va gate is exact at every format's own split, from both sides.**
    ///
    /// The cited test checks one aligned low address, one unaligned one and one kernel-half address
    /// per format. This sweeps the boundary itself: the last page of the low half is admitted, the
    /// first page of the high half (page-aligned, so alignment alone cannot save the gate) is
    /// refused, the unaligned neighbours on both sides are refused, and the all-ones address is
    /// refused. A split constant off by one page, or a gate testing `>=` where it means `>`, moves
    /// one of these and nothing else.
    #[test]
    fn the_user_va_gate_is_exact_at_each_formats_own_split() {
        fn edges<F: PageFormat>() {
            let split = 1u64 << F::SPLIT_SHIFT;
            assert!(is_user_page_va::<F>(0), "the null page is low and aligned");
            assert!(is_user_page_va::<F>(PAGE_SIZE));
            assert!(!is_user_page_va::<F>(1));
            assert!(
                is_user_page_va::<F>(split - PAGE_SIZE),
                "the last page of the low half must be admitted",
            );
            assert!(
                !is_user_page_va::<F>(split - 1),
                "unaligned at the top of the low half"
            );
            assert!(
                !is_user_page_va::<F>(split),
                "the first page of the high half is page-aligned: only the half test refuses it",
            );
            assert!(!is_user_page_va::<F>(split | PAGE_SIZE));
            assert!(!is_user_page_va::<F>(u64::MAX));
            assert!(
                !is_user_page_va::<F>(0x7fff_ffff_ffff_f000),
                "a high-ish address with bit 63 clear is not in the low half",
            );
        }
        edges::<Aarch64>();
        edges::<Sv39>();
        edges::<Ia32e>();
    }

    /// **No leaf of any format is both writable and executable, over every public constructor.**
    ///
    /// The cited tests assert this per format against that format's own raw bits. This one decodes
    /// through `leaf_flags`, the portable read side, so it holds the whole set at once: a
    /// constructor or encoder that manufactures the combination in one format only is caught by
    /// the same loop. `Vtd` is in the set too, where the claim is vacuous but free: VT-d has no
    /// execute bit to set and the assertion says so.
    #[test]
    fn no_leaf_of_any_format_is_both_writable_and_executable() {
        fn sweep<F: PageFormat>() {
            for flags in [
                Flags::kernel_code(),
                Flags::kernel_rodata(),
                Flags::kernel_data(),
                Flags::device(),
                Flags::write_combining(),
                Flags::user_code(),
                Flags::user_rodata(),
                Flags::user_data(),
                Flags::user_device(),
                Flags::user_write_combining(),
            ] {
                let leaf = F::leaf_entry(0x10_0000, flags);
                let decoded = F::leaf_flags(leaf);
                assert!(
                    !(decoded.is_writable()
                        && (decoded.is_kernel_executable() || decoded.is_user_executable())),
                    "{flags:?} encoded a leaf both writable and executable: {leaf:#x}",
                );
            }
        }
        sweep::<Aarch64>();
        sweep::<Sv39>();
        sweep::<Ia32e>();
        sweep::<Vtd>();
    }
}
