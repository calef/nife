//! **The MMU, `x86_64`.** The third implementation of the seam milestone 20 built: `crates/paging`
//! owns the level walk and `paging::x86_64::Ia32e` owns the entry format, and this module is the
//! glue that names `CR3`, the direct map, and where this machine's devices are.
//!
//! # Two bases, which is the one structural difference of the three architectures
//!
//! aarch64 and RISC-V put the kernel image and the direct map at a single base, so one constant and
//! one subtraction serve both. `x86_64` cannot: `x86_64-unknown-none` uses `code-model: kernel`,
//! which promises LLVM that every symbol is in the **top 2 GiB**, so the image is pinned at
//! [`KERNEL_VA_BASE`] (`0xffffffff80000000`) and there is no room above it to name more than 2 GiB
//! of physical memory. `VA = PA | KERNEL_VA_BASE` was therefore never going to reach a real
//! machine's RAM, and it is not even invertible above the line: the local APIC at `0xfee00000` maps
//! to a distinct address whose inverse is not `0xfee00000`.
//!
//! So the two jobs that constant was doing are split, exactly as Linux splits them (milestone 161,
//! item 1 of the roadmap's open list):
//!
//! | What | Base | Who needs it there |
//! |---|---|---|
//! | The kernel **image** | [`KERNEL_VA_BASE`] `0xffffffff80000000` | the code model, non-negotiably |
//! | The **direct map** of physical memory | [`DIRECT_MAP_BASE`] `0xffff888000000000` | [`phys_to_virt`], and it has room for 64 TiB |
//!
//! They are separate PML4 entries (511 and 273), so nothing about them interferes, and both are
//! canonically high, so `Ia32e::is_in_half(Half::High, ..)` admits both by the same bit-47 test.
//! The `paging` crate needed no change for either, which is the claim of milestone 20 (a portable
//! HAL, proven on a second architecture) holding a second time.
//!
//! [`phys_to_virt`] is the direct map's arithmetic and [`virt_to_phys`] inverts **both** bases,
//! because the kernel asks it about image addresses (`memory::image_start`, linker symbols) as well
//! as about direct-map ones. That two-branch shape is Linux's `__pa()`, for the same reason.
//!
//! # Why `boot.s` installs the direct map, and not this module
//!
//! The roadmap flagged one hazard here and it is real: `phys_to_virt` must not change meaning under
//! anything that already used it. The frame allocator's bitmap is the sharp case, because
//! `memory::bring_up_page_frames` turns a physical address into a `&'static mut [u8]` and **stores it**,
//! long before there is a fine map; the PVH structure and the ACPI tables are read the same way.
//!
//! Rather than sequence around that (map the old alias too, switch, then drop it), `boot.s` gives
//! the boot tables a PML4[273] entry pointing at the same low PDPT the identity map uses. The
//! direct map therefore exists from before the first Rust instruction, covering the low 4 GiB, and
//! [`init`] widens it rather than introducing it. **`phys_to_virt` means one thing for the
//! machine's whole life**, so the hazard stops existing instead of being documented for the next
//! reader to remember. The cost is one PML4 entry and no memory at all.
//!
//! What [`init`] *does* drop is the identity map, which was a complete alias of physical memory
//! sitting in the half user programs will get.
//!
//! # BUGS
//!
//! - **The direct map's RAM is built out of blocks, but its memory type is not checked against the
//!   MTRRs.** Each RAM claim is mapped with the largest leaf that fits ([`largest_leaf`]: 1 GiB
//!   where `CPUID` offers `Page1GB`, else 2 MiB), which took QEMU's 256 MiB from 560 KiB of page
//!   tables to 60 KiB (milestone 161; 4 KiB leaves cost 0.2% of RAM, ~64 MiB on a 32 GiB
//!   machine). The SDM leaves the memory type undefined when one large page spans two MTRR ranges
//!   of different types, and this kernel never reads the MTRRs, so it relies on firmware keeping
//!   every range it calls usable RAM write-back throughout, which is what firmware does and what
//!   Linux checks rather than assumes (`mtrr_type_lookup`'s "uniform" answer). Device windows and
//!   firmware reservations stay in 4 KiB leaves for the same reason: they are where mixed types
//!   live, and they are a few hundred pages.
//!
//! - **`mmu::init` must allocate its page tables from the low 4 GiB**, because that is all the boot
//!   tables' direct map reaches and the mapper writes every table through it. The frame allocator
//!   hands out the lowest free frame first, so this holds today on any machine; a machine whose RAM
//!   *starts* above 4 GiB would fault inside `map_everything`. Widening the boot map to 1 GiB
//!   leaves (one PDPT covers 512 GiB) is the fix, and it needs a `CPUID` check for `PDPE1GB`.
//!
//! - **`CR4.PGE` is off, so the `G` bit the kernel's `Flags` set is ignored.** Every `mov cr3`
//!   therefore discards the kernel's own translations as well as the outgoing process's, which is
//!   correct and costs a TLB refill of whatever kernel pages the next thread touches. **Measured
//!   2026-09-19 (milestone 161) and left off, because the only instrument this tree has cannot see
//!   the thing PGE changes.** QEMU 11.0.2's `cpu_x86_update_cr3` (`target/i386/helper.c`) calls
//!   `tlb_flush` on every `CR3` write, whatever `PGE` and `PCIDE` say, so under TCG a global entry
//!   never survives a switch; and icount counts retired guest instructions, which a TLB refill is
//!   not. The x86 bench with an initrd (so `ctx_switch`, two processes and a `CR3` write per
//!   switch, runs) printed **byte-identical tick counts on every line** with PGE on and off, which
//!   is the proof of the first half rather than a result about the second. notes/benchmarks.md's
//!   2026-09-19 section has the numbers and what would measure it: KVM on cordoba (blocked on one
//!   `usermod`) or a bench boot on xenon. Turning it on is a two-line change after `install` in
//!   [`init`] and [`init_secondary`] (after the fine map, or the boot map's entries are the ones
//!   pinned), and nothing else here depends on it being off: kernel mappings are already global,
//!   user mappings already are not, and every kernel unmap already ends in `invlpg`, which does
//!   invalidate a global entry.
//!
//! - **`CR4.PCIDE` is off, so an address space has no hardware tag.** `crates/address_space_identifier` hands every
//!   space a number and this architecture has nowhere to put it: PCID is `CR3[11:0]`, and with
//!   PCIDE clear those bits are reserved-zero rather than a tag. [`ttbr0_value`] therefore drops the
//!   number and [`flush_asid`] flushes the whole TLB rather than one space's entries. Both say so in
//!   their own words; neither pretends to a selectivity the hardware does not have. **Unmeasured for
//!   the same reason as PGE, and not a bit to flip**: with PCIDE on, `invlpg` and a plain `CR3`
//!   write act on the *current* PCID only, so [`unmap_user_at`] on a space that is not installed,
//!   [`flush_asid`] and the NMI shootdown's discard-everything arm would each leave another space's
//!   tagged entries alive. That is a stale-translation defect (memory reading back as its previous
//!   owner's), and closing it is `INVPCID` (checked in `CPUID`) on every one of those paths plus a
//!   rule for tag reuse, which is a milestone rather than a line.
//!
//! - **The cacheable fill follows the firmware's map only as far as that map describes memory
//!   *contiguously*** ([`firmware_fill_ceiling`]). That is a claim about how firmware writes
//!   memory maps, not an architectural guarantee: DRAM and the carve-outs taken out of it abut
//!   each other, and the 32-bit MMIO hole above them is a gap. A machine whose map had an
//!   undescribed gap *inside* low DRAM, with ACPI tables above it, would have those tables left
//!   out of the direct map and would fault reading one. No map in this tree has that shape and
//!   xenon's does not (`notes/x86-uefi-boot.md` transcribes it). The bound it replaced was "the
//!   top of RAM", which was the same number on every machine whose RAM ends below the hole and
//!   swallowed every MMIO window on the first machine whose RAM does not.
//!
//! - **An aperture that firmware reports as usable RAM would panic, and that is deliberate.** It
//!   was the leading hypothesis for the 2026-09-05 `AlreadyMapped` and it was wrong: xenon's
//!   framebuffer is in the MMIO hole, in no RAM region. If a machine ever did report both, the fix
//!   is to exclude the aperture from the RAM direct map *and* from the frame allocator, not to
//!   skip the second mapping and not to remap the frames device-typed. Skipping leaves the
//!   aperture cacheable, and this architecture leaves the effective memory type **undefined** when
//!   one frame is reachable through two mappings with conflicting types, so that is wrong rather
//!   than merely slow. Remapping would give one correct mapping and leave the allocator handing
//!   out frames the display adapter answers at. Until a machine forces the question, the panic
//!   names both ranges and is the better answer.

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
