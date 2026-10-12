/// `invoke(cap, MAP_INTO, va, frame_slot, writable)` -> 0. Map the frame in `frame_slot`
/// into THIS address space at `va`, read-only or read/write. Needs `WRITE` on the
/// address-space capability; the frame capability needs `READ` for a read-only mapping, `WRITE` for a
/// writable one, the `frame::MAP` rule verbatim. Page tables and the revocation record are
/// paid from the space's own backing region; a mapping whose record cannot be afforded is
/// refused and unmapped (`OutOfMemory`), never left invisible to revocation.
pub const MAP_INTO: u64 = 0;

/// `MAP_INTO`'s third argument: how to map the frame. Read-only, read/write, or executable
/// code (milestone 19d, so a loader can lay down a child's `.text`). Code is W^X: mapped RX,
/// never writable, and the kernel makes the I-cache coherent with the loader's data writes.
pub const MAP_RO: u64 = 0;
/// Map read/write.
pub const MAP_RW: u64 = 1;
/// Map executable code: RX, never writable (W^X).
pub const MAP_CODE: u64 = 2;

/// `invoke(cap, LIST, cursor, 0, 0) -> (next_cursor, va, kind)` (milestone 126, `pmap`,
/// DECISIONS §114). List what this address space has mapped, one entry per call, without the
/// ability to change any of it. `Rendezvous::SURVEY`'s shape one object type over: same cursor
/// protocol, same reason for one entry per call (the space's own mapping log is held only
/// long enough to read one row, never for the whole walk), same right.
///
/// - `next_cursor` returns in x0 (a0 on RISC-V), `va` in x1, `kind` in x2.
/// - Start with `cursor = 0`. Feed each `next_cursor` back. `abi::survey::DONE` (zero) means
///   finished; the constant is reused rather than a second one minted, because "no more
///   entries" means the same thing on both objects.
/// - A negative first word is an [`crate::Error`]. **A cursor this space's own log never
///   minted answers [`crate::Error::BadPointer`]** (milestone 779 (fuzz the surface a
///   confined process can reach)'s confined fuzzer drew a random cursor and the kernel
///   followed it as a log page, 2026-10-06 UTC; ruled `BadPointer` over reading it as
///   `DONE` the way a dead space does, DECISIONS §114's 2026-10-06 addendum: a caller that
///   corrupts its own cursor learns it is broken rather than seeing a truncated listing).
/// - `kind` is one of [`MAP_RO`], [`MAP_RW`], [`MAP_CODE`] above: the same three words
///   `MAP_INTO`'s third argument takes, reused rather than a second vocabulary invented for
///   what is, read back, the same fact about the same page. A `DeviceFrame` mapping (always
///   read/write, never executable) reads as `MAP_RW`; the listing does not distinguish device
///   memory from ordinary data, which is recorded where a reader meets `pmap`.
///
/// **Needs [`rights::ENUMERATE`](super::rights::ENUMERATE), and pointedly not `WRITE`.**
/// `WRITE` on an address-space capability is what `MAP_INTO` takes; a viewer holding
/// `ENUMERATE` alone can list every mapping and change none of them, the same split
/// `Rendezvous::SURVEY` drew between looking and acting. See `capability::Rights::ENUMERATE`
/// and DECISIONS §114's delegation-audit caveat: every address-space capability minted since
/// 2026-08-17 already carries the bit (the `Rights::ALL`-on-creation invariant), so this
/// method's existence is what turns that bit from inert to live.
pub const LIST: u64 = 1;

/// `invoke(cap, UNMAP, va, _, _)` -> 0 (milestone 95 (an unmap primitive), DECISIONS §162
/// (whether a holder can give up a mapping), option A). **This space gives up the one page it
/// maps at `va`, and no other holder is touched.** The inverse of one
/// page of [`MAP_INTO`], and of any other route that put a page there.
///
/// - Needs `WRITE` on the address-space capability, the authority `MAP_INTO` takes.
/// - One page per call. A run `MAP_INTO` laid down in one call is given up a page at a time;
///   unmapping one page of it leaves the rest mapped.
/// - **A `va` with nothing mapped answers [`crate::Error::BadPointer`]** and changes nothing,
///   as does a misaligned or kernel-half one. Not idempotent, deliberately: the caller is
///   closing a window, and "it was never open" is a different fact from "it is now shut".
/// - **No capability is consumed or changed.** A frame capability the caller still holds for
///   the page survives and can map it again; one already deleted is not needed.
/// - The page is gone from every core's TLB before this returns, and from the space's mapping
///   record, so [`LIST`] stops reporting it and a later revoke of the frame does not reach `va`.
///
/// Both semantics above were ruled by calef on 2026-10-05 (UTC) and are §162's amendment of
/// that date; `notes/unmap.md` has the argument for each.
///
/// **It reaches a running space** since §249 (a running address space stays nameable): a
/// capability to a space keeps naming it after `ThreadControlBlock::CONFIGURE`, so a holder
/// can take a page out from under the thread running there, which faults on its next touch of
/// it wherever it runs.
pub const UNMAP: u64 = 2;

/// `invoke(cap, WAIT, va, flags, expected)` -> `0` woken, `1` the word did not hold
/// `expected` (milestone 812 (`std::thread::spawn` runs real threads in one address space),
/// §269 (how threads share a process) fork 2). **Futex wait: park the calling thread on the
/// word at `va` in its own space, if that word still holds `expected`.** The check and the park
/// are one step against [`WAKE`], so a waker that changes the word and then wakes cannot slip
/// between them.
///
/// - `flags` is [`super::futex::PRIVATE`] `|` [`super::futex::SIZE_U32`], and nothing else
///   is admitted today: a shared futex or another size is `BadMethod` (§269 reserves both).
/// - The capability must name the caller's own address space, else `WrongObject`, and carry
///   `READ`, else `NotPermitted`. The key is a word in memory the caller already has, so it
///   carries no authority beyond that memory.
/// - `va` must be 4-aligned, in the user half and mapped readable, else `BadPointer`.
/// - `Gone` if the wait was ended without a wake (the thread's region was destroyed under it).
/// - A return of `0` may be spurious: a caller re-checks its word, as every futex user does.
///
/// No timeout yet; see `notes/futex.md`'s `BUGS`. *(Name and number provisional, milestone 812's
/// lane, 2026-10-10 UTC.)*
pub const WAIT: u64 = 3;

/// `invoke(cap, WAKE, va, flags, count)` -> the number of threads woken (milestone 812, §269
/// fork 2). **Futex wake: release up to `count` threads parked by [`WAIT`] on the word at `va`
/// in the caller's own space**, oldest first. `u64::MAX` wakes all. Refuses what [`WAIT`]
/// refuses, except that the word need not be mapped: a wake reads no memory. *(Name and number
/// provisional, milestone 812's lane, 2026-10-10 UTC.)*
pub const WAKE: u64 = 4;
