//! Page tables, portable across formats.
//!
//! Two hardware page-table formats live under this crate: the aarch64 descriptor format
//! ([`Aarch64`], four levels) and RISC-V Sv39 ([`Sv39`], three levels). They share everything that
//! matters and differ only in the bits: the *walk* (descend the levels, allocate tables, write a
//! leaf) is written once in [`Mapper`], and each format supplies the handful of encode/decode
//! operations the walk needs through the [`PageFormat`] trait. See notes/riscv-port.md (leak #2) for
//! why this seam exists and DECISIONS §17.
//!
//! Both formats use 512-entry, 4 KiB tables and 4 KiB pages, because both were designed around the
//! same arithmetic: 9 index bits selects one of 512 eight-byte entries, so **a table is exactly one
//! page** and the frame allocator can supply page tables and nothing else.
//!
//! # Blocks: a leaf one or two levels up
//!
//! The same arithmetic gives every format a second and third leaf size for free. An entry one level
//! above the bottom covers 512 pages (2 MiB), one level above that 512 of those (1 GiB), and all
//! three formats let such an entry be a **leaf** that maps the whole span directly instead of
//! pointing at a table. aarch64 calls it a *block* descriptor, Sv39 a megapage or gigapage, x86 a
//! large page (`PS`). [`PageSize`] names the three sizes and [`Mapper::map_span`] picks the largest
//! one that fits, which is what a direct map of physical memory wants: in 4 KiB leaves it costs 8
//! bytes of table per 4 KiB of RAM (0.2%), in 2 MiB leaves 8 bytes per 2 MiB.
//!
//! **A block is a leaf the walk meets early, and every walk has to know that.** `map` refuses to
//! descend through one (it would read a 2 MiB frame of somebody's data as a page table and write a
//! leaf into it), `unmap` refuses to take a 4 KiB bite out of one ([`MapError::InsideBlock`]), and
//! `translate` stops at one and adds the offset within it. None of the three splits a block: doing
//! that on a live table is valid-to-valid on aarch64, which is the break-before-make hazard
//! [`TlbFlush`] exists for, and nothing in this tree has needed it.
//!
//! # Why this is a separate crate
//!
//! It is pure logic: addresses in, descriptors out. The host tests build real page tables in real
//! memory (using host allocations as pretend physical frames, which works because the pointer
//! arithmetic is identical) and walk them back. Milliseconds, no emulator. DECISIONS §7.
//!
//! # Examples
//!
//! Real aarch64 page tables, in real memory, on the host. The trick is in the section above: a
//! `Box<PageTable>` is 4 KiB-aligned because the type says so, so its address serves as a
//! "physical" frame and `phys_to_ptr` is the identity cast. **The pointer arithmetic is bit-for-bit
//! what the kernel does.**
//!
//! ```
//! use std::cell::RefCell;
//!
//! use paging::{Aarch64, Flags, Half, MapError, Mapper, PageTable};
//!
//! fn phys_to_ptr(pa: u64) -> *mut PageTable {
//!     pa as *mut PageTable
//! }
//!
//! // The pretend frame allocator, with a receipt so nothing leaks.
//! let tables: RefCell<Vec<*mut PageTable>> = RefCell::new(Vec::new());
//! let mut fresh = || {
//!     let p = Box::into_raw(Box::new(PageTable::new()));
//!     tables.borrow_mut().push(p);
//!     p as u64
//! };
//! let root = fresh();
//!
//! // SAFETY: `root` is a fresh, zeroed, 4 KiB-aligned table, and `phys_to_ptr` is the identity,
//! // which is correct because these "physical" addresses ARE host addresses.
//! let mut m: Mapper<_, fn(u64) -> *mut PageTable, Aarch64> = unsafe {
//!     Mapper::new(
//!         root,
//!         Half::Low,
//!         || Some(fresh()),
//!         phys_to_ptr as fn(u64) -> *mut PageTable,
//!     )
//! };
//!
//! // A user text page. Note what is not available: there is no writable-and-executable
//! // constructor to pass here at all.
//! m.map(0x40_0000, 0x4000_0000, Flags::user_code()).unwrap();
//! let (pa, flags) = m.translate(0x40_0000).unwrap();
//! assert_eq!(pa, 0x4000_0000);
//! assert!(flags.is_user_executable() && !flags.is_writable());
//!
//! // **Break-before-make is forced, not documented.** Overwriting a live mapping would go valid to
//! // valid (which the hardware may reject) and leak the old frame, so it is refused.
//! assert_eq!(
//!     m.map(0x40_0000, 0x5000_0000, Flags::user_data()),
//!     Err(MapError::AlreadyMapped),
//! );
//!
//! // A kernel address in the user tables is a mapping the CPU would never consult, because the top
//! // bits pick the table set before any index is extracted. An error rather than a silent no-op.
//! assert_eq!(
//!     m.map(0xffff_0000_0000_0000, 0x4000_0000, Flags::kernel_data()),
//!     Err(MapError::WrongHalf),
//! );
//!
//! // The other half of break-before-make: `unmap` hands back a `TlbFlush` that must be discharged.
//! let (freed, flush) = m.unmap(0x40_0000).unwrap();
//! assert_eq!(freed, 0x4000_0000);
//! flush.flush(|va| assert_eq!(va, 0x40_0000)); // the kernel's `tlbi vaae1is` goes here
//! assert!(m.translate(0x40_0000).is_none());
//!
//! // Now the address is free again, which is what makes the refusal above a sequencing rule rather
//! // than a prohibition.
//! m.map(0x40_0000, 0x5000_0000, Flags::user_data()).unwrap();
//!
//! drop(m); // the mapper borrows the tables; it goes first
//! for p in tables.borrow_mut().drain(..) {
//!     // SAFETY: each `p` came from `Box::into_raw` above and is registered exactly once.
//!     unsafe { drop(Box::from_raw(p)) };
//! }
//! ```
//!
//! W^X is worth stating as a claim about what is **absent**, over every constructor there is:
//!
//! ```
//! use paging::Flags;
//!
//! for f in [
//!     Flags::kernel_code(),
//!     Flags::kernel_rodata(),
//!     Flags::kernel_data(),
//!     Flags::device(),
//!     Flags::user_code(),
//!     Flags::user_rodata(),
//!     Flags::user_data(),
//!     Flags::user_device(),
//! ] {
//!     assert!(!(f.is_writable() && (f.is_kernel_executable() || f.is_user_executable())));
//!     // And the privilege split: nothing user-reachable is kernel-executable, so a wild kernel
//!     // jump into a user page faults instead of running user-chosen instructions in EL1.
//!     if f.is_user_accessible() {
//!         assert!(!f.is_kernel_executable());
//!     }
//! }
//! ```
//!
//! The user-VA gate is a conjunction, and the aligned high-half address is the case that tells `&&`
//! from `||`: an or-gate here would admit a kernel address to a user `MAP` request.
//!
//! ```
//! use paging::{Aarch64, PAGE_SIZE, is_user_page_va};
//!
//! assert!(is_user_page_va::<Aarch64>(0x40_0000));
//! assert!(!is_user_page_va::<Aarch64>(0x40_0001)); // not page-aligned
//! assert!(!is_user_page_va::<Aarch64>(0xffff_0000_0000_0000)); // aligned, and the kernel's half
//! assert_eq!(PAGE_SIZE, 4096);
//! ```
//!
//! Name: ratified 2026-08-01 (calef, the naming tenet in CLAUDE.md). Named in the group of standard
//! terms that are already right and must not be touched, because a name a reader knows from outside
//! this project costs nothing to learn and renaming it would destroy the recognition the tenet
//! exists to buy.

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
