//! **`MemoryRegion::RETYPE_OBJ`**: a kernel object made in a region's memory, and the capability to
//! it filed in the caller's table. Moved out of `syscall.rs` unchanged but for the floor, by
//! milestone 812 (`std::thread::spawn` runs real threads in one address space), for §266 (a Rust
//! source file stays under 2,000 lines).
//!
//! **The third word is a floor** (milestone 812, provisional): the capability lands in the first
//! free slot at or above it, and `0` is the first free slot as before. A `std` program's low slots
//! are fixed by number and an empty one means "not granted" (`std_runtime_protocol`), so what it
//! makes for itself, a thread or a timer, must land above them.
//!
//! Name: provisional (milestone 812's lane, 2026-10-10 UTC).

use abi::Error;

use crate::cap::{Capability, Object, Rights};
use crate::sched;

/// `MemoryRegion::RETYPE_OBJ`: retype a page into a page-resident KERNEL OBJECT the caller now owns
/// (19a). The object lives in the carved page, and the region is pinned (a live endpoint's page must
/// never be freed under a blocked thread). `#[inline(never)]` for the reason `memory_region_map`
/// gives.
///
/// **The object carries the budget's rights and no others**, for every kind (milestone 824 (a
/// retype mints no right its budget lacks), calef's ruling (a) of 2026-10-07 UTC, which named
/// frames; milestone 903 (a seventh outsider pass attacks the confinement claim) showed this arm minted the same way). Each arm mints
/// through `budget.mint_child`, `SPLIT`'s inheriting mint, which `split_never_widens_rights`
/// proves. Until 824 every arm minted `Rights::ALL`, "full rights on its own object", so a
/// `WRITE`-only budget made `GRANT`-bearing objects its holder could lend. The root budget carries
/// `Rights::ALL` (`cap::memory_region_root_cap`), so the progenitor still gets every right on what
/// it makes, and a right added later reaches it without a hand-kept list.
#[inline(never)]
pub(super) fn memory_region_retype_obj(
    budget: Capability,
    region: u64,
    kind: u64,
    space_slot: u64,
    floor: u64,
) -> Result<i64, Error> {
    match kind {
        abi::objtype::RENDEZVOUS => {
            let ep = sched::create_rendezvous_from(region).ok_or(Error::OutOfMemory)?;
            // The budget's rights, never a hand-written list. Spelling a set out here once meant
            // "full" silently stopped being full the day `ENUMERATE` was added: the progenitor
            // could not narrow `deaths` to a right it did not itself hold, `CAP_INSERT` refused
            // the widen, and the spawn surfaced as `OutOfMemory` at a prompt. Inheriting from the
            // budget keeps that fixed, because the root budget is `Rights::ALL`.
            let slot = sched::grant_from(floor, budget.mint_child(Object::Rendezvous(ep, 0)))
                .map_err(|_| Error::OutOfMemory)?;
            Ok(slot as i64)
        }
        // An address space (19b): the page becomes the L0 root, the untyped becomes the space's
        // backing region for tables and records (one budget model; see the abi doc and
        // design/init-and-granular-spawn.md).
        abi::objtype::ADDRESS_SPACE => {
            let name = crate::user::user_address_space_create(region).ok_or(Error::OutOfMemory)?;
            // The budget's rights, for the RENDEZVOUS arm's reason.
            let slot = sched::grant_from(floor, budget.mint_child(Object::AddressSpace(name)))
                .map_err(|_| Error::OutOfMemory)?;
            Ok(slot as i64)
        }
        // A thread (19c.3): the page holds an embryo TCB, born in no queue and not runnable
        // until CONFIGURE + START. The page is the creator's region's.
        abi::objtype::THREAD_CONTROL_BLOCK => {
            let tid = sched::create_thread_control_block(region).ok_or(Error::OutOfMemory)?;
            let slot = sched::grant_from(floor, budget.mint_child(Object::ThreadControlBlock(tid)))
                .map_err(|_| Error::OutOfMemory)?;
            Ok(slot as i64)
        }
        // A notification (milestone 151, DECISIONS §101): a word and a wait queue in the page.
        // The budget's rights, for the RENDEZVOUS arm's reason.
        abi::objtype::NOTIFICATION => {
            let id = sched::create_notification_from(region).ok_or(Error::OutOfMemory)?;
            let slot = sched::grant_from(floor, budget.mint_child(Object::Notification(id)))
                .map_err(|_| Error::OutOfMemory)?;
            Ok(slot as i64)
        }
        // A timer (milestone 106, DECISIONS §147): one deadline and its target in the page.
        // The budget's rights, for the RENDEZVOUS arm's reason.
        abi::objtype::TIMER => {
            let id = sched::create_timer_from(region).ok_or(Error::OutOfMemory)?;
            let slot = sched::grant_from(floor, budget.mint_child(Object::Timer(id)))
                .map_err(|_| Error::OutOfMemory)?;
            Ok(slot as i64)
        }
        // A process (milestone 812 (`std::thread::spawn` runs real threads in one address space)),
        // holding the space whose `WRITE` capability is in `space_slot`, which is consumed as
        // `CONFIGURE` consumes one. The budget's rights, `BIND` among them when the budget has it,
        // for the RENDEZVOUS arm's reason.
        abi::objtype::PROCESS => {
            let space = sched::current_cap(space_slot).map_err(|_| Error::NoSuchSlot)?;
            let crate::cap::Object::AddressSpace(name) = space.object else {
                return Err(Error::WrongObject);
            };
            if !space.rights.allows(Rights::WRITE) {
                return Err(Error::NotPermitted);
            }
            let pid = sched::create_process_from(region, name)?;
            let _ = sched::delete_current_cap(space_slot);
            let slot = sched::grant_from(floor, budget.mint_child(Object::Process(pid)))
                .map_err(|_| Error::OutOfMemory)?;
            Ok(slot as i64)
        }
        _ => Err(Error::BadMethod), // no such object type
    }
}
