/// `invoke(cap, MAP, va, writable, memory_region_slot)` -> 0. Map this frame at `va` in the caller's
/// own address space. `writable` != 0 maps it read/write (needs `WRITE` on the frame); `0` maps
/// it read-only (needs `READ`). Page tables to reach `va` come from the untyped named by
/// `memory_region_slot`, so the kernel allocates nothing. `BadPointer` for a misaligned or high `va`,
/// `OutOfMemory` when that untyped is exhausted.
///
/// The tables belong to that untyped, so its `DESTROY` cuts them out of this address space,
/// and every page mapped beneath them stops translating, this frame's included. A mapping
/// meant to outlive the untyped needs its tables from a region that lives as long.
pub const MAP: u64 = 0;

/// `invoke(cap, REVOKE, _, _, _)` -> 0. **Un-share this page** (milestone 13). Unmap it from every
/// address space that mapped it and delete every capability to it, including the caller's own, so
/// no holder can reach or re-map it. Needs `GRANT` (you were trusted to lend the frame, so you may
/// take it back; a read-only consumer handed it without `GRANT` cannot revoke the owner). It does
/// **not** reclaim the page: the untyped is spend-only, and `memory_region::destroy` reclaims a whole
/// region. See DECISIONS §13 and notes/capability-lifecycle.md.
///
/// **On a device capability the same method means take-back, not un-share** (milestone 23,
/// DECISIONS §39). It deletes every `DeviceFrame` capability to the page and unmaps it
/// everywhere **except the caller's own**, so exactly one process can still reach the
/// registers: the one that asked. That is what live replacement needs between tearing a driver
/// down and endowing its replacement, and the asymmetry is forced: the kernel mints a device
/// capability once, at boot, so a symmetric revoke would strand the device forever.
pub const REVOKE: u64 = 1;

/// `invoke(cap, SLICE, first, count, _)` -> the slot the new capability landed in, or a
/// negative [`crate::Error`]. The slot comes back in x0, the way [`super::rendezvous::BADGE`]
/// answers.
///
/// **Derive a capability naming a sub-run of this one** (milestone 599 (a frame per filesystem
/// client channel), calef's ruling of 2026-09-27 on option 4; name and number provisional): the
/// `count` pages starting `first` pages into this capability's run, with this capability's
/// rights. It is how one holder of a pool hands a client exactly its window: slice window `w`,
/// `MAP_INTO` the client, delete the slice. §102 (a Frame names a run of pages) anticipated the
/// shape, "two capabilities: `Frame(phys, 401)` and `Frame(phys + 401 * 4096, 74)`"; this is the
/// method that makes the second from the first.
///
/// - Needs `GRANT`. A slice is a new object as far as `REVOKE` is concerned: under §132
///   (what `PageFrame::REVOKE` owes an overlapping run) revocation is scoped to the exact run, so
///   revoking the source does not reach a slice. Minting a capability its source's revocation
///   cannot reach is a delegation-class power, the same argument `BADGE` makes.
/// - Never wider: a slice names pages inside the source's run and carries the source's rights,
///   which the holder narrows further as ever when it delegates.
/// - [`crate::Error::BadPointer`] for `count` of 0 or a range past the run's end: those pages are
///   not memory the holder could have named.
pub const SLICE: u64 = 2;
