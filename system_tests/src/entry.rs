//! **Where the kernel's boot hands over to this suite.**
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

#[cfg(test)]
use super::*;

/// Called by the kernel's boot where a `cargo test` kernel calls its own `test_main`
/// (`kernel/src/boot.rs`, `run_test_suite`). The name and signature are the whole contract.
#[unsafe(no_mangle)]
pub fn system_tests_main() {
    #[cfg(test)]
    test_main();
}
