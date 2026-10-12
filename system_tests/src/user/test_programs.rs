//! `test_programs`: the program images and the bare spawn and reap that more than one topic file
//! of the userspace suite uses. Each is `pub(super)`, so it reaches the other test modules under
//! `user` and nothing outside `cfg(test)`.
//!
//! **The in-kernel userspace suite, on both instruction sets** (milestone 19 (run a real
//! workload)'s user-test port).
//!
//! It was aarch64-only for most of this project's life, and the module comment used to say the
//! reason was the tests: "every test drives a hand-written aarch64 program through `exec` and reads
//! aarch64 fault registers". That was true, and it was the wrong thing to fix. The tests were fine;
//! their *scaffolding* was aarch64. Three things moved and the tests came along unchanged:
//!
//! 1. The hand-assembled programs became real ELFs the toolchain builds for both targets (the
//!    `outlaw` binary and the `interrupt_ignorer` that already existed). See the note above
//!    `OUTLAW_ROUND_TRIP`.
//! 2. `ESR`/`FAR` became `arch::UserFault`, the same fact in words RISC-V can say, which is what
//!    keeps "a PERMISSION fault at exactly this address" assertable rather than softened to "a fault
//!    happened".
//! 3. `hello`, which carries the milestone 7-19 role catalogue (milestone 7 (user mode: EL0,
//!    capabilities, the ELF loader, and IPC) onward), was found to build for RISC-V once
//!    six syscalls it had hand-rolled in aarch64 `asm!` were routed through `user_mode_runtime`, which already
//!    had portable versions of all six.
//!
//! **What is still gated, and why, is written at each test rather than here**, because a blanket
//! module comment is how the old claim survived past the point of being true. Two kinds of gate
//! appear below: a property that has no RISC-V analogue at all (`el1_runs_on_sp_el1`), and a
//! property whose RISC-V twin lives in `riscv_virtio_tests` and would be duplicated rather than
//! gained. See notes/riscv-parity-scope.md.
//!
//! Name: provisional, minted 2026-10-12 (UTC) by the lane for milestone 873 (the catch-all
//! userspace test file is split by topic).

use super::*;
use crate::sched;

/// **The `image_self_checker` program's ELF bytes**: the loader's subject.
///
/// Four tests want *a real user ELF to load* rather than a particular program, and they all want
/// the same one, which is the point of naming it once. Three read its segment table (three
/// segments, none writable-and-executable, one with a `.bss`) and the fourth runs it and asserts
/// it did not fault, which is that program checking those very properties from the inside. Naming
/// two different binaries there would let the structural half and the behavioural half drift apart.
///
/// **The `.bss` requirement is load-bearing and nearly went missing.** It was `hello`'s bytes until
/// milestone 291, and splitting hello's roles out took its last `static mut X = 0` with them, so
/// the binary stopped having a `.bss` at all and `the_initrd_holds_a_native_executable` said so:
/// "no segment has a .bss, so the zero-fill is untested". The vacuity guard fired exactly as
/// written, which is why it is there.
///
/// **`x86_64` packs no initrd at all**, because no user program is built for
/// `x86_64-unknown-none` (`crates/user_mode_runtime` has no arms for this ISA; see notes/x86-port.md). Every
/// test that reaches for this skips instead.
pub(super) fn loader_subject_image() -> &'static [u8] {
    program("image_self_checker").expect("no image_self_checker program in the initrd archive")
}

/// **The `block_driver` binary's ELF bytes**: the virtio driver and its two DMA attackers, one
/// program on every architecture since milestone 291.
///
/// This read a role of `hello` on aarch64 until then: that binary carried seven virtio roles which
/// dispatched into the same `crates/virtio` the dedicated binary uses. `riscv_virtio_tests` has
/// always named `block_driver` here; the two now name the same thing.
///
/// aarch64 and `x86_64`: the riscv64 leg drives these same properties through
/// `riscv_virtio_tests`, which has its own `blk_image` naming the same program, and `x86_64` runs
/// the two PCIe tests in this module (it packs RISC-V's archive but has no `riscv_virtio_tests`).
#[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
pub(super) fn blk_image() -> &'static [u8] {
    program("block_driver").expect("no block_driver program in the initrd archive")
}

/// The `outlaw` program's ELF bytes: the two privilege-boundary behaviours that used to be
/// hand-assembled aarch64 machine code in this file. See [`OUTLAW_ROUND_TRIP`].
pub(super) fn outlaw_image() -> &'static [u8] {
    program("outlaw").expect("no outlaw program in the initrd archive")
}

/// **An address in the kernel's own memory: mapped, readable by the kernel, forbidden to
/// userspace.** The address of a kernel static, rather than a per-ISA constant for the kernel's
/// text base, because taking the address of something the kernel is demonstrably using is true
/// on any ISA and any link layout, and needs no table of magic numbers to keep current.
pub(super) fn a_kernel_address() -> u64 {
    static SENTINEL: u64 = 0x00C0_FFEE_D00D;
    &raw const SENTINEL as u64
}

/// Run `image` at user mode with `arg0`/`arg1` and no authority at all: no capabilities, no
/// extra mappings. It can run its own code and touch its own memory and name nothing else.
pub(super) fn spawn_bare(
    image: &'static [u8],
    arg0: u64,
    arg1: u64,
) -> Option<crate::thread::ThreadId> {
    sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0,
                arg1,
                arg2: 0,
                grants: &[],
                maps: &[],
            },
        )
    })
}

/// **Take back a thread from [`spawn_bare`] that will never exit on its own.**
///
/// A bare thread belongs to no reclaimable region, so nothing tears it down when the test that
/// started it returns. For a subject whose whole point is that it never exits (a spinner, or a
/// child holding a measurement still), "the test ended" and "the thread ended" are unrelated
/// events, and the thread just keeps running.
///
/// That is not merely untidy. Two such threads spinning on a four-hart machine is a load every
/// later test in the suite then runs under, and it starved
/// `reclaim_frees_a_started_then_exited_childs_regions` into its watchdog on CI, intermittently,
/// nowhere near the tests responsible. See `notes/riscv-parity-scope.md`.
///
/// The kill is armed rather than immediate (DECISIONS §16), so this waits for the name to stop
/// resolving instead of assuming the thread is gone on return. Returns whether it actually went;
/// callers assert on it, because a silent failure here restores the leak this exists to remove.
pub(super) fn reap_bare(tid: crate::thread::ThreadId) -> bool {
    if !sched::kill_thread(tid) {
        // Already gone. Nothing to wait for, and not a failure.
        return true;
    }
    wait_for(|| !sched::is_thread_present(tid))
}

/// The `std_exerciser` std program's ELF bytes. The same binary the offline std test spawns; given
/// the network here, its `UdpSocket::bind` probe succeeds and it runs the net transcript.
#[cfg(target_arch = "aarch64")]
pub(super) fn std_exerciser_image() -> &'static [u8] {
    program("std_exerciser").expect("no std_exerciser program in the initrd archive")
}
