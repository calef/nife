//! **The MMU, `x86_64`.** `crates/paging` owns the level walk and `paging::x86_64::Ia32e` the entry
//! format; this module names `CR3`, the direct map, and where this machine's devices are.
//!
//! **Two bases**, the one structural difference from aarch64 and RISC-V. `code-model: kernel`
//! pins every symbol in the top 2 GiB, so the image cannot share a base with a direct map of more
//! than 2 GiB of RAM. So, as on Linux:
//!
//! | What | Base | Who needs it there |
//! |---|---|---|
//! | The kernel **image** | [`KERNEL_VA_BASE`] `0xffffffff80000000` | the code model, non-negotiably |
//! | The **direct map** of physical memory | [`DIRECT_MAP_BASE`] `0xffff888000000000` | [`phys_to_virt`], and it has room for 64 TiB |
//!
//! They are separate PML4 entries (511 and 273), both canonically high. [`virt_to_phys`] inverts
//! both bases, because the kernel asks it about image addresses too (Linux's `__pa()`).
//!
//! **`phys_to_virt` means one thing for the machine's whole life**: `boot.s` gives the boot tables
//! a PML4[273] entry over the low 4 GiB before the first Rust instruction, so the frame bitmap,
//! PVH structure and ACPI tables can keep the pointers they took early, and [`init`] widens the
//! direct map rather than introducing it. [`init`] drops the boot identity map, an alias of all
//! physical memory in the half user programs get.
//!
//! # BUGS
//!
//! - RAM is mapped with the largest leaf that fits ([`largest_leaf`]) without reading the MTRRs,
//!   trusting firmware to keep usable RAM write-back throughout; device windows and firmware
//!   reservations stay in 4 KiB leaves, where mixed types live.
//! - `mmu::init` must allocate its tables below 4 GiB, all the boot map reaches; a machine whose
//!   RAM starts above 4 GiB would fault in `map_everything`.
//! - `CR4.PGE` is off, so every `mov cr3` drops the kernel's translations too. TCG flushes on every
//!   `CR3` write regardless and icount cannot see a refill, so its cost is unmeasured
//!   (notes/benchmarks.md); turning it on is two lines in [`init`] and [`init_secondary`].
//! - `CR4.PCIDE` is off, so [`ttbr0_value`] drops the address-space number and [`flush_asid`]
//!   flushes everything. It is not a bit to flip: with PCIDE on, `invlpg` and `CR3` writes act on
//!   the current PCID only, so [`unmap_user_at`], [`flush_asid`] and the NMI shootdown would leave
//!   other spaces' stale entries alive until `INVPCID` covers every path.
//! - The cacheable fill follows the firmware map only while it is contiguous
//!   ([`firmware_fill_ceiling`]); ACPI tables above an undescribed gap inside low DRAM would fault.
//! - An aperture firmware also reports as usable RAM panics, naming both ranges, on purpose: the
//!   fix is excluding it from both the direct map and the allocator, not a second mapping.

use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering};

use paging::x86_64::Ia32e;
use paging::{Flags, Half, MapError, Mapper, PAGE_SIZE, PageSize, PageTable};

// The `x86_64` modules the children name as `super::isa` and the like. Imported here so that a
// path written when this code lived in `mmu.rs` still means what it meant: the children's `super`
// is this module, and these are what it would have reached.
use super::{ap_boot, current_sp, instructions, irq, isa, machine, segments};
use crate::memory;

/// This architecture's page-table format. Portable code names it as `arch::mmu::Format`.
pub type Format = Ia32e;

// **Every type and function of `mmu` lives in a child named for it**, and this file is the index
// (§282 (when a file is split, the parent becomes an index)). `Format` stays: the arch contract's
// one alias, a single line. Each child takes `use super::*` and is re-exported whole, so every
// `arch::mmu::` path a caller names still resolves, and `map_tests` keeps its path. Cut by lane
// `maintainer/last-three-over-2000` (2026-10-12 UTC), which brought this file under §266 (a Rust
// source file stays under 2,000 lines).
mod layout;
pub use layout::*;
mod kernel_map;
pub use kernel_map::*;
mod shootdown;
pub use shootdown::*;
mod claims;
use claims::*;
mod memory_mapped_io_window;
pub use memory_mapped_io_window::*;
mod fine_map;
use fine_map::*;
mod image_bounds;
pub use image_bounds::*;
mod user_space;
pub use user_space::*;
mod device_windows;
pub use device_windows::*;

#[cfg(test)]
mod map_tests;
