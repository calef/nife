//! **A wait the system tests share**, kept outside any one test module.
//!
//! Cut out of `user.rs` unchanged by milestone 872 (the kernel's user-mode file is split along its
//! seams), which left the parent an index.
//!
//! Name: provisional (milestone 872's lane, 2026-10-11 UTC; §281 (the kernel's user-mode file
//! splits into child modules of `user`)).

#[cfg(any(test, feature = "system_tests"))]
#[cfg_attr(not(feature = "system_tests"), allow(dead_code))] // the system tests call it; a unit-test boot on some ISAs does not
/// Spin the scheduler until `done()`, or give up after a wall-clock deadline. Returns whether it
/// happened. **Time-based, not a fixed yield count** (DECISIONS §28): with work spread across
/// cores, the test thread's own core is often idle, so a yield returns at once and a fixed count
/// of them elapses in almost no real time, timing out before a parallel result on another core
/// lands. A ~2 s deadline gives the other cores real time to finish while staying far under the
/// 60 s hang watchdog, so a genuine hang still fails.
///
/// It lives **here** rather than in `tests` because six sibling modules use it and that one does
/// not compile on every architecture: `user::tests` needs a real ELF program out of the initrd
/// and is `#[cfg(all(test, initrd))]`, which would have taken this helper down with it on a
/// target that packs none (milestone 161, roadmap item 4). A helper every module uses does not
/// belong inside one of them. Milestone 81 needed it in two of them: running on the physical core makes the
/// yield-count version fail for the *mirror* reason it fails on a loaded host, since a yield on an
/// idle core costs nanoseconds there. See notes/hvf-leg.md.
pub fn wait_for(mut done: impl FnMut() -> bool) -> bool {
    let deadline = crate::arch::timer::now() + 2 * crate::arch::timer::frequency();
    while crate::arch::timer::now() < deadline {
        if done() {
            return true;
        }
        crate::sched::yield_now();
    }
    done()
}
