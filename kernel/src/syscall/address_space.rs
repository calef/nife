//! **The `AddressSpace` methods**: map into, unmap, the futex pair and the listing, with the run
//! arithmetic `PageFrame::MAP` shares.
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

/// **Every `AddressSpace` method, out of line** (milestone 812 (`std::thread::spawn` runs real
/// threads in one address space)): the futex arm it added put aarch64's flat `syscall_entry` over
/// `script/fastpath-footprint`'s 5% bound while the methods were folded into
/// [`invoke`](super::invoke), and none of them is a step of the IPC round trip that bound protects.
#[inline(never)]
pub(super) fn address_space_invoke(
    frame: &mut TrapFrame,
    cap: crate::cap::Cap,
    name: u64,
    method: u64,
    [a0, a1, a2]: [u64; 3],
) -> Result<i64, Error> {
    match method {
        abi::address_space::MAP_INTO => address_space_map_into(cap, name, a0, a1, a2),
        // List what this address space has mapped, one entry per call, without the ability
        // to change any of it (milestone 126's `pmap`, DECISIONS §114): `Rendezvous::SURVEY`'s
        // shape one object type over, and pointedly `ENUMERATE` rather than `WRITE`, which is
        // what `MAP_INTO` above takes. See `abi::address_space::LIST` for the wire contract and
        // DECISIONS §114 for why this method's mere existence is the thing that makes
        // `ENUMERATE` live on every address-space capability minted since 2026-08-17 (the
        // `Rights::ALL`-on-creation invariant): the audit that check required is in
        // notes/process-view.md.
        abi::address_space::LIST => {
            if !cap.rights.allows(Rights::ENUMERATE) {
                return Err(Error::NotPermitted);
            }
            address_space_list(frame, name, a0)
        }
        // This space gives up the page it maps at `va` (milestone 95 (an unmap primitive),
        // DECISIONS §162 (whether a holder can give up a mapping), option A).
        // `WRITE`, the authority `MAP_INTO` takes: shaping a space is one right in both
        // directions. See `abi::address_space::UNMAP` and notes/unmap.md.
        abi::address_space::UNMAP => {
            if !cap.rights.allows(Rights::WRITE) {
                return Err(Error::NotPermitted);
            }
            address_space_unmap(name, a0)
        }
        // Futex wait and wake (milestone 812 (`std::thread::spawn` runs real threads in one
        // address space), §269 (how threads share a process) fork 2), out of line: neither is
        // a step of the IPC round trip the fastpath footprint bounds.
        abi::address_space::WAIT | abi::address_space::WAKE => {
            address_space_futex(cap.rights, name, method, a0, a1, a2)
        }
        _ => Err(Error::BadMethod),
    }
}

/// `AddressSpace::MAP_INTO`: map an existing frame into *another* process's address space under
/// construction (19b), the other half of [`page_frame_map`](super::page_frame::page_frame_map).
/// `#[inline(never)]` for the reason `memory_region_map` gives, **found late** (milestone 142 (a
/// text display good enough that people use it instead of a GUI)'s review): this arm sat inline in
/// [`invoke`](super::invoke)'s dispatch since before milestone 156 (`syscall_entry`'s measured size
/// is every method combined)'s discipline existed, and nobody had retrofitted it. It stayed under
/// the `syscall_entry` footprint budget by luck until MAJOR 2 and MAJOR 3's real, necessary fixes
/// (the rollback loop, the tail-VA check) added enough code to an already-inline arm to trip the 5%
/// tripwire on riscv64 -- the fix was correct, it just landed in the wrong place structurally.
/// Extracted here rather than shrunk, the same move every sibling administrative arm in this file
/// already made.
#[inline(never)]
fn address_space_map_into(
    cap: crate::cap::Cap,
    name: u64,
    a0: u64,
    a1: u64,
    a2: u64,
) -> Result<i64, Error> {
    if !cap.rights.allows(Rights::WRITE) {
        return Err(Error::NotPermitted);
    }
    let va = a0;
    // **The frame is read under the hold that maps and records it** (the map-revocation-window
    // lane, 2026-10-04 UTC). It used to be read here with `sched::current_cap`, in a critical
    // section of its own, and a sweep could fall between that read and the record and leave the
    // mapping live: `revoke::MappingHold` has the whole account. The space's registry comes first
    // because it ranks above the mapping registry; neither touches `IPC_TABLES`.
    #[cfg(feature = "system_tests")]
    crate::delegation_pause::here(); // no lock held
    crate::user::with_user_address_space(name, |space| {
        let mut hold = crate::revoke::hold();
        let frame = hold.current_cap(a1).map_err(|_| Error::NoSuchSlot)?;
        // The mappable object is a PageFrame (normal memory) or a DeviceFrame (a device's
        // MMIO, device-typed): the driver a userspace progenitor builds gets its registers this
        // way (19d.2). a2 chooses the shape for a PageFrame; a DeviceFrame is always
        // device-typed read/write and needs WRITE on the cap.
        let (phys, count, flags) = match frame.object {
            Object::DeviceFrame(phys) => {
                if !frame.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                (phys, 1u64, paging::Flags::user_device())
            }
            // §102 (2026-08-20): `count` is the run's length. A single-page frame is
            // `count: 1`, so this arm's behavior for every existing caller is unchanged; a
            // run-capable frame maps the whole run in this one MAP_INTO call, exactly as
            // `page_frame::MAP` does below.
            Object::PageFrame(phys, count) => {
                // 0 read-only, 1 read/write, 2 executable code (a loader's child .text).
                // Code is W^X: user_code is RX, never writable, so it needs only READ.
                let flags = match a2 {
                    abi::address_space::MAP_RW => {
                        if !frame.rights.allows(Rights::WRITE) {
                            return Err(Error::NotPermitted);
                        }
                        paging::Flags::user_data()
                    }
                    abi::address_space::MAP_CODE => {
                        if !frame.rights.allows(Rights::READ) {
                            return Err(Error::NotPermitted);
                        }
                        paging::Flags::user_code()
                    }
                    _ => {
                        if !frame.rights.allows(Rights::READ) {
                            return Err(Error::NotPermitted);
                        }
                        paging::Flags::user_rodata()
                    }
                };
                (phys, count.get(), flags)
            }
            _ => return Err(Error::WrongObject),
        };
        // **The run's last page is checked, not only its first** (milestone 142's review,
        // MAJOR 3). This used to check `va` alone, which was right when a frame was one
        // page and wrong the moment `count` could exceed 1: a run placed near the top of
        // the low half passed the check and then walked out of it partway through the loop
        // below, refused three layers down by `Mapper::map`'s own `Half::Low` re-check
        // rather than here. Same guard `page_frame_map` uses, same reason: reject the whole
        // request before mapping any of it.
        let Some(last_va) = run_end_va(va, count) else {
            return Err(Error::BadPointer);
        };
        if !paging::is_user_page_va::<crate::arch::mmu::Format>(va)
            || !paging::is_user_page_va::<crate::arch::mmu::Format>(last_va)
        {
            return Err(Error::BadPointer);
        }
        // A `MAP_INTO` naming a space that does not exist fails before it spends a page
        // table, and the loop's own `NotMapped` would otherwise report that as a mapping
        // failure with nothing to roll back.
        let Some(space) = space else {
            return Err(Error::BadPointer);
        };
        let root = space.root();
        for k in 0..count {
            let (page_phys, page_va) = (phys + k * paging::PAGE_SIZE, va + k * paging::PAGE_SIZE);
            // `phys` is the run's base for a `PageFrame` and the page itself for a
            // `DeviceFrame`, so in both arms it is the object the invoked capability names
            // (§132). The device case never scopes a revoke by it: `DeviceFrame::REVOKE`
            // scopes by holder, not by capability, which `revoke_device_from_others`
            // explains.
            match space.map_physical_held(
                &mut hold,
                page_va,
                page_phys,
                flags,
                crate::revoke::PageMapSource::Capability(phys),
            ) {
                Ok(()) => {
                    // When userspace maps a frame it wrote executable (a spawner building a
                    // child's code, MAP_CODE), the instruction fetcher must be made to see
                    // the bytes the writer stored: RISC-V's `fence.i`, aarch64's
                    // dcache-clean + icache-invalidate, both behind `sync_icache`. The
                    // kernel-side ELF loader does this (user/loader.rs map_segments); this is the
                    // userspace-built path, which a fast spawn+reap loop (bench::spawn_el0)
                    // is the first thing to stress. A child that fetches unsynced code
                    // takes an illegal-instruction fault at its entry.
                    if flags.is_user_executable() {
                        crate::arch::sync_icache(
                            crate::arch::mmu::phys_to_virt(page_phys),
                            paging::PAGE_SIZE as usize,
                        );
                    }
                }
                // **All or nothing across the run** (milestone 142's review, MAJOR 2).
                // Whatever this loop mapped before failing is unmapped again, so a caller
                // that gets an error never has to wonder how much of its run landed: the
                // answer is always none of it. Before this the prefix stayed mapped and
                // recorded with no way to ask about it, which is the pre-§102 single-page
                // path's own rollback quietly narrowed to one page by the widening.
                Err(e) => {
                    unmap_run_prefix(&mut hold, root, phys, va, k);
                    return Err(match e {
                        paging::MapError::OutOfPageFrames => Error::OutOfMemory,
                        // misaligned, already mapped, unknown space
                        _ => Error::BadPointer,
                    });
                }
            }
        }
        Ok(0)
    })
}

/// `AddressSpace::UNMAP` (milestone 95, DECISIONS §162 option A; the two semantics §162 left open
/// are provisional, and `notes/unmap.md` argues each): take the one page at `va` out of the space
/// `name`, out of every core's TLB, and out of the space's mapping record. `BadPointer` when nothing
/// is mapped there. No capability is read, consumed or changed. `#[inline(never)]` for the reason
/// `memory_region_map` gives: giving up a window is setup work, never a step of the IPC round trip.
///
/// **Under the space registry and then the mapping hold, the order `MAP_INTO` takes them**
/// (`ADDRESS_SPACES` above `MAPPINGS`), so an `UNMAP` and a `MAP_INTO` of the same `va` are ordered
/// whole, and a revoke's unmap pass (which takes the same hold) either finds the record and unmaps
/// the page itself, or finds neither. The table walk and the record are changed in the same hold,
/// so no sweep can see one without the other.
///
/// **The TLB obligation is `mmu::unmap_user_at`'s**, the function every revoke already unmaps with:
/// `tlbi vaae1is` on aarch64 (every ASID, every core in the inner-shareable domain), a local
/// `sfence.vma` plus an SBI remote fence on riscv64, `invlpg` plus the NMI shootdown on x86_64.
#[inline(never)]
fn address_space_unmap(name: u64, va: u64) -> Result<i64, Error> {
    if !paging::is_user_page_va::<crate::arch::mmu::Format>(va) {
        return Err(Error::BadPointer);
    }
    crate::user::with_user_address_space(name, |space| {
        // A space that no longer resolves (its thread reaped, or its region reclaimed) has nothing
        // this capability can reach, which is what `MAP_INTO` answers for it too. A running space
        // does resolve since §249, and `unmap_user_at`'s flush is what reaches its thread's core.
        let Some(space) = space else {
            return Err(Error::BadPointer);
        };
        let root = space.root();
        let mut hold = crate::revoke::hold();
        // Both halves, and neither is allowed to short-circuit the other: a page in the tables with
        // no record (none should exist since 2026-09-21, when every route began recording) is
        // still a window to close, and a record with no page behind it is still a record a later
        // revoke would act on.
        let unmapped = mmu::unmap_user_at(root, va);
        let forgotten = hold.forget_mapping_at(root, va);
        if unmapped.is_none() && forgotten.is_none() {
            return Err(Error::BadPointer);
        }
        Ok(0)
    })
}

/// The **last** virtual address a `count`-page run starting at `va` covers, or `None` if the run
/// does not fit in a `u64`.
///
/// Both mapping paths guard on this, and both used to compute it inline as
/// `va.checked_add((count - 1) * paging::PAGE_SIZE)`, where only the *addition* was checked: the
/// multiply was not, so a `count` large enough to wrap `(count - 1) * 4096` back around to a small
/// number produced a `last_va` near `va` and the guard cheerfully passed a run that spans the
/// address space (milestone 142's review, MAJOR 4). `count` is a `NonZeroU64` on the capability
/// now, so the subtraction cannot underflow; this closes the other half.
pub(super) fn run_end_va(va: u64, count: u64) -> Option<u64> {
    va.checked_add(count.checked_sub(1)?.checked_mul(paging::PAGE_SIZE)?)
}

/// **Undo the `mapped` pages a failed run-map had already established**, in the space rooted at
/// `root`: unmap each page and tombstone its revocation record, leaving the space exactly as the
/// call found it.
///
/// The rollback the pre-§102 code got for free by mapping one page. A partially mapped run is
/// worse than a failed one: the caller is told `OutOfMemory` or `BadPointer` with no way to ask
/// how much of its request survived, and a shared surface half-mapped is a peer reading pixels
/// that are not there.
///
/// # BUGS
///
/// **The page tables the mapped prefix retyped are not returned**, so a failed multi-page map
/// still costs the caller whatever L3s (and their parents) the prefix needed, permanently. A
/// region is spend-only (`MemoryRegion::RETYPE` never un-retypes), so giving them back is not a
/// matter of calling something; it is the reverse of the model. The mapping is undone, the budget
/// is not. Recorded in notes/frames.md.
pub(super) fn unmap_run_prefix(
    hold: &mut crate::revoke::MappingHold,
    root: u64,
    phys: u64,
    va: u64,
    mapped: u64,
) {
    for k in 0..mapped {
        let (page_phys, page_va) = (phys + k * paging::PAGE_SIZE, va + k * paging::PAGE_SIZE);
        mmu::unmap_user_at(root, page_va);
        hold.forget_mapping(page_phys, root, page_va);
    }
}

/// `AddressSpace::WAIT` and `WAKE`: the checks every futex method shares, then the scheduler.
/// The order of the refusals is the contract `abi::address_space::WAIT` states: the flags first
/// (`BadMethod`, a form this kernel does not answer), then the right (`NotPermitted`), the address
/// (`BadPointer`), and last whether the space is the caller's own (`WrongObject`).
#[inline(never)]
fn address_space_futex(
    rights: Rights,
    space: u64,
    method: u64,
    va: u64,
    flags: u64,
    word: u64,
) -> Result<i64, Error> {
    if !abi::futex::admitted(flags) {
        return Err(Error::BadMethod);
    }
    if !rights.allows(Rights::READ) {
        return Err(Error::NotPermitted);
    }
    if !va.is_multiple_of(4)
        || !<crate::arch::mmu::Format as paging::PageFormat>::is_in_half(paging::Half::Low, va)
    {
        return Err(Error::BadPointer);
    }
    if !sched::is_current_space(space) {
        return Err(Error::WrongObject);
    }
    if method == abi::address_space::WAIT {
        // The expected value is the low 32 bits; the size admitted is 32 (§269).
        sched::futex_wait(space, va, word as u32).map(|r| r as i64)
    } else {
        Ok(sched::futex_wake(space, va, word) as i64)
    }
}

/// The body of `abi::address_space::LIST` (milestone 126's `pmap`, DECISIONS §114), pulled out of
/// [`invoke`](super::invoke) and marked `#[inline(never)]` on purpose: `syscall_entry` is measured
/// flat (`script/fastpath-footprint`), so a rare administrative loop inlined into the hot
/// dispatcher grows every syscall's instruction footprint for a method almost nothing calls. One
/// call-site's worth of bytes in `invoke` costs far less than this loop's own bytes would.
#[inline(never)]
fn address_space_list(frame: &mut TrapFrame, name: u64, cursor: u64) -> Result<i64, Error> {
    // The capability names a registry entry by generation; once the space dies (its thread reaped
    // or its region destroyed, §249 (a running address space stays nameable)), the entry is gone
    // and `root` is `None` from here on for every capability that pointed at it. That is not a refusal (the capability is real and was never
    // widened past what it always held): it reads as an empty listing, symmetric to `SURVEY`'s
    // "before the scheduler exists there is no domain to report."
    let Some(root) = crate::user::user_address_space_root(name) else {
        frame.set_arg(1, 0);
        frame.set_arg(2, 0);
        return Ok(abi::survey::DONE as i64);
    };
    // The cursor is the caller's word, so it is checked before it is followed, in the same
    // SPACES hold that walks the chain (revoke::list_mapping): only a cursor this space's own
    // log minted may name a log page. Milestone 779 (fuzz the surface a confined process can
    // reach)'s confined fuzzer drew a random cursor here and the kernel walked it as a log page
    // (a data abort on a kernel-mapped address; anything the page held would have come back as
    // a mapping record), 2026-10-06 UTC.
    // Skip an entry whose `va` no longer translates (a race with revocation of a shared page this
    // space had mapped) rather than report a fabricated `kind` for it; bounded by the log's own
    // finite length, so this cannot loop forever.
    let mut cursor = cursor;
    loop {
        match crate::revoke::list_mapping(root, cursor) {
            crate::revoke::Listing::Done => {
                frame.set_arg(1, 0);
                frame.set_arg(2, 0);
                return Ok(abi::survey::DONE as i64);
            }
            // Only the caller's own word can be foreign: every cursor after the first is
            // minted by the arm below. BadPointer, not DONE, per DECISIONS §114 (`pmap` gets
            // its listing: `ENUMERATE` extends to the address-space object)'s 2026-10-06
            // addendum: a dead space's stale cursor is DONE, a live space's foreign word is
            // garbage, and the boundary answers garbage with BadPointer everywhere else.
            crate::revoke::Listing::ForeignCursor => return Err(Error::BadPointer),
            crate::revoke::Listing::Entry(next, va) => {
                if let Some((_, flags)) = crate::arch::mmu::translate_at(root, va) {
                    let kind = if flags.is_user_executable() {
                        abi::address_space::MAP_CODE
                    } else if flags.is_writable() {
                        abi::address_space::MAP_RW
                    } else {
                        abi::address_space::MAP_RO
                    };
                    frame.set_arg(1, va);
                    frame.set_arg(2, kind);
                    return Ok(next as i64);
                }
                cursor = next;
            }
        }
    }
}
