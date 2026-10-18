/// `invoke(cap, MAP, va, _, _)` -> 0. Retype one page out of the untyped and map it, writable,
/// at `va` in the caller's own address space. The page and any page tables it needs both come
/// from the untyped; the kernel allocates nothing. Returns `OutOfMemory` when the untyped is
/// exhausted (the *process* is out of budget, not the kernel).
pub const MAP: u64 = 0;

/// `invoke(cap, RETYPE, pages, _, _)` -> slot. Retype `pages` contiguous pages out of the untyped
/// into one **`PageFrame` capability** naming the run,
/// per §102 (a Frame names a run of pages), and return the slot it landed in.
/// `pages == 0` means one page, which is what every caller passed before the count existed
/// (calef's ruling of 2026-09-26). Nothing is mapped: the caller decides where to map it, and
/// may delegate it first. This is the split that makes a page a first-class, delegatable object
/// rather than something mapped in one shot. `OutOfMemory` when the run does not fit, and then
/// the region's budget is untouched; also when the caller's capability table is full, and then
/// the run is spent, exactly as a single page always was.
pub const RETYPE: u64 = 1;

/// `invoke(cap, RETYPE_OBJ, objtype, space, floor)` -> slot. Retype one page out of the untyped into
/// a **kernel object** (milestone 19a; design/init-and-granular-spawn.md): the object lives
/// in that page, in the caller's own memory, and the returned slot holds a full-rights
/// capability to it. One object per page, deliberately (one memory rule for the whole object
/// family; packing is a later placement optimization).
///
/// `objtype` names what to make (see [`objtype`](super::objtype)); 19a implements
/// `RENDEZVOUS`. A region that has produced a kernel object is **pinned**: [`DESTROY`] reclaims
/// it (object revocation) once the objects are torn down. `BadMethod` for an unknown objtype;
/// `OutOfMemory` when the untyped is exhausted, the object registry is full, or the capability table is.
///
/// `space` is read only for [`objtype::PROCESS`](super::objtype::PROCESS): the slot of the
/// address space the process holds (milestone 812 (`std::thread::spawn` runs real threads in one
/// address space)). **`floor`**, for every object type, is the lowest slot the new capability may
/// land in, `0` meaning the first free slot as before (milestone 812, provisional): a `std`
/// program passes `std_runtime_contract::SLOTS` so nothing it makes lands in a fixed slot it was
/// not given.
pub const RETYPE_OBJ: u64 = 2;

/// `invoke(cap, SPLIT, pages, _, _)` -> slot. Carve `pages` off this untyped's unspent budget
/// into a **new child untyped**, and return the slot holding a full-rights capability to it.
/// seL4's untyped-retype-into-untyped: a spawner splits a child its own region so it can be
/// reclaimed independently. This untyped is then marked as having children and can no longer be
/// `DESTROY`ed (its pages are committed; the child frees them at its own `DESTROY`). `OutOfMemory`
/// when the budget or region table is exhausted or the capability table is full; `NotPermitted` without
/// `WRITE`.
pub const SPLIT: u64 = 3;

/// `invoke(cap, DESTROY, _, _, _)` -> 0. **Reclaim this region and every object retyped from
/// it** (object revocation, the region-owner's half). The objects are torn down and their pages
/// returned; every capability to them goes stale on next use (generational names, no derivation
/// tree to walk). `NotPermitted` while a live thread still occupies the region, or if it has
/// been `SPLIT` into children (destroy the children first), or without `WRITE`.
pub const DESTROY: u64 = 4;

/// `invoke(cap, USAGE, record, _, _)` -> pages. **How much of this region has been spent, and
/// on what** (milestone 126, DECISIONS §225 (`free` sees the machine and your share) part 1: `free`'s "yours" line, and `slabtop`
/// asked per object type). `record` is one of [`usage`](super::usage)'s selectors; the answer
/// is a page count in x0.
///
/// Needs `ENUMERATE` and nothing else, the rule §114 (`ENUMERATE` extends to the address-space
/// object) set for `pmap`: a holder learns what the region is spent on without being able to
/// spend, split or destroy it. `BadMethod` for an unknown record, checked before the region is
/// looked up; `NoSuchSlot`-style staleness reads as [`crate::Error::Gone`] for a region already
/// reclaimed.
///
/// Name and number ratified 2026-09-27 (calef, #1360's table).
///
/// # BUGS
///
/// **Every subtree record walks the whole region table under the region lock, with interrupts
/// masked.** [`FRAMES`](super::usage::FRAMES) and the three object records sum every live
/// region that descends from this one, and the table (`kernel/src/memory_region.rs`
/// `MAX_REGIONS`, 256) is machine-wide, so one call costs every region's parent chain. Measured
/// on the host by the 2026-10-03 security audit: 8 ns for a root with eight children, 42 µs for
/// a root over the deepest chain the table can hold (256 regions, each split from the last) and
/// 62 µs for that chain's leaf, which rejects every candidate after a full walk. A holder of
/// `ENUMERATE` on any region can repeat it, so this is a bounded interrupt-latency cost a
/// hostile process can impose, of the same shape as the region reclaim scans and the registry
/// bounds the 2026-09-29 audit accepted. Accepted: nothing multi-domain splits that deep, and
/// the bound is `MAX_REGIONS`, not the caller's choice.
pub const USAGE: u64 = 5;
