//! **The `MemoryRegion` methods**: map, retype, split, destroy and the usage records, and since
//! 2026-10-12 (UTC) the arm of [`invoke`](super::invoke) that decodes them.
//!
//! Cut out of `syscall.rs` unchanged on 2026-10-12 (UTC) by lane maintainer/near-ceiling-splits,
//! so the parent stays under §266 (a Rust source file stays under 2,000 lines) and moves toward
//! §282 (when a file is split, the parent becomes an index). Each method keeps its
//! `#[inline(never)]`, so [`invoke`](super::invoke) reaches it through one call, as before, and
//! `script/fastpath-footprint` measures the same code under a longer symbol name. `pub(super)`
//! is the visibility a private item of `syscall` had: `syscall` and its children.
//!
//! Name: provisional (lane maintainer/near-ceiling-splits, 2026-10-12 UTC), for the module and the
//! file.

use super::*;
use crate::arch::mmu;

/// **The `MemoryRegion` arm of [`invoke`](super::invoke)**: the WRITE check each spending method
/// asks for, then the method's out-of-line body.
///
/// Cut out of [`invoke`](super::invoke) on 2026-10-12 (UTC) by lane
/// maintainer/long-function-paydown-invoke, for milestone 888 (a ratchet on function length), with
/// its rights checks and its order unchanged. **`#[inline(always)]`, unlike every body below**,
/// because this is decoder rather than method: the checks sat inline in `invoke` and are counted in
/// `script/fastpath-footprint`'s flat `syscall_entry`, and this crate builds in several codegen
/// units, where a plain function in a child module may not be inlined at all. Out of line, the
/// arm would leave the measured set under a symbol the gate does not count and read as a shrink
/// that nothing on the syscall path earned. Each body keeps the `#[inline(never)]` it had.
///
/// Name: provisional (lane maintainer/long-function-paydown-invoke, 2026-10-12 UTC).
#[inline(always)]
pub(super) fn memory_region_invoke(
    capability: crate::cap::Cap,
    region: u64,
    method: u64,
    [a0, a1, a2]: [u64; 3],
) -> Result<i64, Error> {
    match method {
        // Body extracted (milestone 156): all five `MemoryRegion` methods are memory-management
        // administration a spawner runs while building a process, never a step of the IPC round
        // trip `script/fastpath-footprint` bounds, so each stays out of `invoke`'s own bytes and
        // `#[inline(never)]` on purpose. See `address_space_list`, the pattern this copies.
        abi::memory_region::MAP => {
            if !capability.rights.allows(Rights::WRITE) {
                return Err(Error::NotPermitted);
            }
            memory_region_map(region, a0)
        }
        abi::memory_region::RETYPE_OBJ => {
            if !capability.rights.allows(Rights::WRITE) {
                return Err(Error::NotPermitted);
            }
            super::retype::memory_region_retype_obj(region, a0, a1, a2)
        }
        abi::memory_region::RETYPE => {
            if !capability.rights.allows(Rights::WRITE) {
                return Err(Error::NotPermitted);
            }
            memory_region_retype(region, a0)
        }
        abi::memory_region::SPLIT => {
            if !capability.rights.allows(Rights::WRITE) {
                return Err(Error::NotPermitted);
            }
            memory_region_split(capability, region, a0)
        }
        abi::memory_region::DESTROY => {
            if !capability.rights.allows(Rights::WRITE) {
                return Err(Error::NotPermitted);
            }
            memory_region_destroy(region)
        }
        // What the region was spent on (milestone 126's `free`, DECISIONS §225 (`free` sees the
        // machine and your share) part 1). Under `ENUMERATE` alone, `address_space::LIST`'s rule
        // one object type over: learning what a budget went to is not the authority to spend it.
        // An unknown record is refused before the region is looked up, `SURVEY`'s order.
        abi::memory_region::USAGE => memory_region_usage(capability, region, a0),
        _ => Err(Error::BadMethod),
    }
}

/// `MemoryRegion::MAP`: retype a page out of the untyped and map it, writable, at `va` in the caller's
/// own address space. Both the page and any page tables come from the untyped, so the KERNEL
/// ALLOCATES NOTHING: the leaf is retyped here and the tables by the closure below, both bumping
/// the untyped's watermark.
///
/// Pulled out of [`invoke`](super::invoke) and marked `#[inline(never)]` (milestone 156
/// (`syscall_entry`'s measured size is every method combined), the pattern `address_space_list`
/// proved on milestone 126's `LIST`): `syscall_entry` is measured flat, so a rare administrative
/// arm inlined into the hot dispatcher grows every syscall's footprint for a method that never runs
/// on an IPC round trip.
///
/// **Retype, map and record are one [`MappingHold`](crate::revoke::MappingHold)** (the
/// map-revocation-window lane, 2026-10-04 UTC). A region is generational, so the window here was
/// narrower than `PageFrame::MAP`'s, and it was there: a retype that succeeded before
/// `MemoryRegion::DESTROY` claimed the region (the claim is the generation bump, after which a
/// retype refuses) and recorded after `revoke_region` had scanned left a live mapping of a page the
/// allocator then reused. Under the hold, the claim lands before the retype (it refuses) or the
/// scan lands after the record (it finds it).
/// `map_revocation_window_tests::a_destroy_inside_a_memory_region_map_leaves_no_mapping` drives it.
#[inline(never)]
pub(super) fn memory_region_map(region: u64, va: u64) -> Result<i64, Error> {
    // Reject the cheap failures BEFORE retyping a page for them: a non-page-aligned or
    // non-low-half address can never be mapped, and without this pre-check each such attempt
    // would silently spend a page of the process's own untyped (a self-inflicted budget leak the
    // audit noted). An already-mapped `va` still costs one page, which is process-local and
    // bounded by the untyped. The gate itself is proved: every address it admits is aligned and
    // in the low half (see `paging::is_user_page_va` and its harness).
    if !paging::is_user_page_va::<crate::arch::mmu::Format>(va) {
        return Err(Error::BadPointer);
    }
    #[cfg(feature = "system_tests")]
    crate::delegation_pause::here(); // no lock held
    let mut hold = crate::revoke::hold();
    let root = mmu::current_user_root();
    // The leaf first, then the tables, recorded as tables as they are retyped: the region pays
    // for both, and its `DESTROY` has to find the tables to cut them out of this walk before the
    // pages go back (`revoke::revoke_region`). An already-mapped `va` spends the leaf, as before.
    let Some(phys) = crate::memory_region::retype_page(region) else {
        return Err(Error::OutOfMemory);
    };
    match mmu::map_current_user_page_frame(va, phys, paging::Flags::user_data(), || {
        hold.retype_table(region, root, va)
    }) {
        Ok(()) => {
            // Record the mapping so it can be revoked before the region is ever reclaimed (§13).
            // MemoryRegion::MAP pages are process-private, but they still must be unmapped before
            // memory_region::destroy frees the region under them. The record is paid from the caller's
            // own address-space budget (phase C); if it cannot afford the record, it cannot keep
            // the mapping: an unrecorded mapping is invisible to revocation, the §13 hole.
            // No capability names this page (§132): `MemoryRegion::MAP` retypes and maps in one
            // step, so there is never a `PageFrame` capability and no derivation family to scope a
            // revoke to. Reclamation finds the record regardless, because `revoke_region`'s unmap
            // sweep is object-blind by design.
            if !hold.record_mapping(phys, root, va, crate::revoke::PageMapSource::NoCapability) {
                mmu::unmap_user_at(root, va);
                return Err(Error::OutOfMemory);
            }
            Ok(0)
        }
        Err(paging::MapError::OutOfPageFrames) => Err(Error::OutOfMemory),
        Err(_) => Err(Error::BadPointer), // misaligned, already mapped, or wrong half
    }
}

/// `MemoryRegion::RETYPE`: retype `a0` pages (`0` meaning one) into one `PageFrame` capability the
/// caller now holds, instead of mapping them in one shot. The caller gets full rights on its own
/// frame (read, write, and the right to pass it on); delegation is where those narrow. Nothing is
/// mapped yet. The count arrived with calef's ruling of 2026-09-26 and a run is the `PageFrame` run of
/// §102 (a Frame names a run of pages); every caller before it passed `0`. `#[inline(never)]` for the reason `memory_region_map`
/// gives.
#[inline(never)]
pub(super) fn memory_region_retype(region: u64, requested: u64) -> Result<i64, Error> {
    let (phys, count) =
        crate::memory_region::retype_run(region, requested).ok_or(Error::OutOfMemory)?;
    // The run is non-empty by construction (`retype_pages` never returns zero).
    let count = core::num::NonZeroU64::new(count).ok_or(Error::OutOfMemory)?;
    // Capability table full. **BUGS: the run stays retyped** (recorded by milestone 757 (a test
    // kernel fails a process on its Nth retype), provisional): the watermark has moved and no
    // capability names the pages, so each call against a full table spends `count` pages until the
    // region is destroyed. `memory_region_split` below gives its child back in the same position;
    // a run has no such inverse today. `memory_region_retype_obj`'s arms are the same shape, an
    // object nobody can name. Bounded by the caller's own region, so a self-inflicted cost, not a
    // leak into anyone else's budget.
    let slot = sched::grant(crate::cap::page_frame_run_cap(phys, count, Rights::ALL))
        .map_err(|_| Error::OutOfMemory)?;
    Ok(slot as i64)
}

/// `MemoryRegion::SPLIT`: carve a child untyped off this one (subdivision), so a spawner can give each
/// child its own reclaimable region. `count` is the child's page count. `#[inline(never)]` for
/// the reason `memory_region_map` gives.
#[inline(never)]
pub(super) fn memory_region_split(
    cap: crate::cap::Cap,
    region: u64,
    count: u64,
) -> Result<i64, Error> {
    let child = crate::memory_region::split(region, count).ok_or(Error::OutOfMemory)?;
    // The child inherits THIS capability's rights, never more (milestone 31). SPLIT is a fresh
    // mint, so it must honor the derive-never-widens invariant by hand: a process holding a
    // spend-only (GRANT-less) untyped must not SPLIT itself a GRANT-bearing child over the same
    // memory and manufacture the right its capability withheld. `Cap::mint_child` is that
    // inheriting mint, and `split_never_widens_rights` (crates/capability) proves it never widens,
    // at the one mint site outside `derive` the caps proofs otherwise miss (milestone 35). Rights
    // narrow monotonically from the delegable root budget down; the progenitor holds that root with GRANT
    // and hands narrowed budgets on. See DECISIONS §16.
    //
    // **A full capability table gives the child back rather than orphaning it**, milestone 601 (the
    // region table prints its peak), a provisional number. This used to be a bare `?`, which
    // returned `OutOfMemory` with the child still live and no capability anywhere naming it: its region slot was held until reboot, and so was
    // the parent, because the orphan's count on it could never come down (the same consequence as
    // `RegionTable::split`'s BUGS entry, by a different road). The child is unpinned, childless and
    // at the top of the parent's watermark, so `destroy` returns its pages to the parent LIFO and
    // drops the count, leaving the parent exactly as it was before the call.
    match sched::grant(cap.mint_child(crate::cap::Object::MemoryRegion(child))) {
        Ok(slot) => Ok(slot as i64),
        Err(_) => {
            crate::memory_region::destroy(child);
            Err(Error::OutOfMemory) // capability table full
        }
    }
}

/// `MemoryRegion::DESTROY`: reclaim this region and every object retyped from it (object revocation):
/// tear the objects down and return the memory. Refused (`NotPermitted`) while a live thread still
/// occupies it, or if it has been split into children (destroy those first). Generational names
/// make every capability to the reclaimed objects stale on next use. `#[inline(never)]` for the
/// reason `memory_region_map` gives.
#[inline(never)]
pub(super) fn memory_region_destroy(region: u64) -> Result<i64, Error> {
    sched::reclaim_region(region).map_err(|_| Error::NotPermitted)?;
    Ok(0)
}

/// The body of `abi::memory_region::USAGE` (milestone 126 (the `procps` package), `free`),
/// out of line and `#[inline(never)]` for `address_space_list`'s reason: `syscall_entry` is
/// measured flat, and a method only `free`, `vmstat` and `slabtop` call must not grow every
/// syscall's footprint. The first CI run with it inline measured `syscall_entry` 5.6% larger on
/// riscv64 and 8.8% on `x86_64`, over the 5% bound.
/// The rights check lives here too, `page_frame_map`'s shape, so the arm in `invoke` is one call.
#[inline(never)]
pub(super) fn memory_region_usage(
    cap: crate::cap::Cap,
    region: u64,
    record: u64,
) -> Result<i64, Error> {
    if !cap.rights.allows(Rights::ENUMERATE) {
        return Err(Error::NotPermitted);
    }
    if !abi::usage::is_known(record) {
        return Err(Error::BadMethod);
    }
    crate::memory_region::usage_record(region, record)
        .map(|pages| pages as i64)
        .ok_or(Error::Gone)
}
