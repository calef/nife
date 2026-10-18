//! **Is a filesystem reachable at all?**
//!
//! This has to be answerable WITHOUT touching the shared page, because a program that was not
//! granted a directory capability has no page mapped and a probe that wrote a name into it would
//! fault instead of returning an error. So the probe is a request that carries no payload: `FSTAT`
//! on a handle number the server's table can never contain.
//!
//!   - no capability in the slot: the kernel refuses the invoke itself and the first reply word is
//!     one of its own small negatives (`NoSuchSlot` = -1, `WrongObject` = -2, `NotPermitted` = -3).
//!   - a real server: it answers `-EBADF` (-9) for the impossible handle, which is a *reply*, so a
//!     filesystem is reachable.
//!
//! Cached, because the answer cannot change: a capability table slot's contents are fixed at spawn on this
//! ABI (0 = not yet asked, 1 = granted, 2 = not granted).
//!
//! Cut out of `sys/fs/nife.rs` unchanged on 2026-10-12 (UTC) by lane
//! maintainer/near-ceiling-splits, for §266 (a Rust source file stays under 2,000 lines) and §282
//! (when a file is split, the parent becomes an index). What was private to `nife` is `pub(super)`
//! here, which reaches the same modules: `nife` and its children.
//!
//! Name: provisional (lane maintainer/near-ceiling-splits, 2026-10-12 UTC), for the module and the
//! file.

use super::*;

pub(super) static REACHABLE: AtomicU8 = AtomicU8::new(0);

/// True if this process holds a directory capability. See the note above for why the probe is an
/// `FSTAT` on an impossible handle rather than anything that touches the shared page.
///
/// `pub(crate)` for `sys/paths/nife.rs`, which needs the same fact for the same reason: naming a
/// place a process has no capability for is the answer-instead-of-refusing failure this platform's
/// own notes keep recording.
pub(crate) fn is_reachable() -> bool {
    match REACHABLE.load(Ordering::Relaxed) {
        1 => true,
        2 => false,
        _ => {
            let (r0, _) = rt::call(FS, proto::req(proto::FSTAT, proto::MAX_HANDLE, 0), 0);
            // A payload-free probe cannot draw `ENOENT` (whose -2 collides with the kernel's
            // `WrongObject`), so all three of the kernel's refusals are unambiguous here.
            let ok = !matches!(r0 as i64, -1 | -2 | -3);
            REACHABLE.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
            ok
        }
    }
}

/// Whether a reply word is the *kernel* refusing the invoke rather than the server answering.
///
/// The kernel's own errors are -1..-8 (`abi::Error`); the server's are negated errnos. The two
/// spaces overlap, which is a wart of the contract recorded in notes/std.md.
///
/// **This used to read -1 as "you hold no such capability" too, and that was wrong** (milestone
/// 122). The comment justifying it said `EPERM` (1) is not in the FS server's vocabulary; it has
/// been since milestone 47, where `dir::EPERM` is what a directory capability answers when the
/// right a verb needs is withheld and when a descent asks for more than the parent holds. So the
/// one reply that says "this capability does not carry that right" was being reported to a std
/// program as "this platform cannot do that", which is the silent-degradation shape DECISIONS §42
/// forbids and which would have made a narrowed grant indistinguishable from no grant at all.
///
/// What makes -1 unambiguous here is [`is_reachable`]: every entry point in this module checks it
/// first and returns `Unsupported` when the slot is empty, so by the time a `request` is issued a
/// server has already answered one message. A kernel `NoSuchSlot` cannot follow that. -3 stays,
/// because `ESRCH` really is not in the FS server's vocabulary and `NotPermitted` from the kernel
/// is what a WRITE-less endpoint answers.
///
/// The clean fix is the contract's, not this function's: a tag or an offset in the reply word so
/// the two error spaces stop overlapping. See the BUGS section of notes/std.md.
///
/// Name: provisional, flagged 2026-09-24 by the boolean-predicate pass
/// (design/naming/functions-that-answer-yes-or-no-worklist.md). It does not yet follow the Rust predicate rule
/// calef ratified 2026-09-24; recommended `is_missing_capability`, because `is_no_capability` does
/// not parse.
pub(super) fn no_capability(r0: u64) -> bool {
    matches!(r0 as i64, -3)
}
