//! **Bringing this CPU into a state where the kernel can safely run.**
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// Bring the CPU into a state where the kernel can safely run.
///
/// Right now that means one thing: install the exception vectors, so that a fault
/// produces a report instead of a silent death. Note the ordering constraint in
/// `main.rs`: the console has to come up first, because the fault handler's whole
/// job is to *print*.
pub fn init() {
    exceptions::init();
}
