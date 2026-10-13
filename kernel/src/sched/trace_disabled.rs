//! **The bench build's no-op twin of `trace`**: the same names and signatures, nothing
//! recorded, so the benchmark numbers the tripwire watches never measure the instrument.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

/// Mirrors the real module's [`Event`](super::trace::Event) variants; carried only so the
/// call sites name the same paths in both configurations.
#[derive(Clone, Copy)]
pub enum Event {
    SwitchTo,
    BlockSelf,
    Wake,
    WakeDeferred,
    WakeCompleted,
    PlaceRemote,
    StealServe,
    InboxDrain,
    WakeRefused,
    Served,
    #[cfg_attr(not(feature = "soak_test"), allow(dead_code))]
    Migrated,
}

#[inline]
pub fn record(_kind: Event, _tid: u64, _aux: u8) {}

/// Always zero here, because nothing is recorded. The bench boot runs no soak (both diverge
/// before the other could start), so no caller can be misled by it.
#[cfg(feature = "soak_test")]
pub fn counted(_kind: Event) -> u64 {
    0
}

pub fn dump(_cpu: usize) {}
