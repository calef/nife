//! **The bench build's no-op twin of `canary`**, so the call sites carry no `cfg` of their own.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

pub(crate) fn arm(_ranges: &[(usize, usize)]) {}
pub(crate) fn disarm() {}
pub(crate) fn check() -> bool {
    false
}
