//! How a builder fails: `must`, `must_ok` and `fail`. A half-built system is not worth limping
//! along, so every failed step traps, and the kernel names the pc. Moved out of `lib.rs` unchanged
//! but for visibility by milestone 843 (the system-initializer crate root is split along its
//! seams), under §279 (the system-initializer crate root splits into private modules).
//!
//! Name: provisional (milestone 843's lane, 2026-10-11 UTC).

/// Unwrap a `Result<_, ()>` or fault: a half-built system is not worth limping along. Generic since
/// DECISIONS §142, because `build_child` now returns a `Child` (a tcb plus what this builder keeps
/// over it) where it used to return a bare slot.
pub(super) fn must<T>(r: Result<T, ()>) -> T {
    match r {
        Ok(v) => v,
        Err(()) => fail(),
    }
}

/// Fault unless the syscall succeeded.
pub(super) fn must_ok(ok: bool) {
    if !ok {
        fail();
    }
}

/// Trap. The kernel prints the pc and kills the process, which is the legible way for a builder to
/// say that the system it was asked to build cannot exist.
pub(super) fn fail() -> ! {
    supervision_protocol::fail()
}
