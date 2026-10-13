//! **The bench build's no-op twin of `canary`**, so the call sites carry no `cfg` of their own.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

pub fn arm(_ranges: &[(usize, usize)]) {}
pub fn disarm() {}
pub fn check() -> bool {
    false
}
