//! **A program proving its own ELF image was loaded correctly**, using nothing but its own memory.
//!
//! None of this needs a capability: every byte it reads belongs to the process already, which is
//! why the milestone-7 test spawns the self-checker with no authority at all.
//!
//! # What each check would catch
//!
//! - `.rodata` holds the bytes the file said it did, so the read-only segment was mapped from the
//!   file rather than zeroed.
//! - `.data` holds its initializer **and** is writable, so the loader copied it rather than
//!   pointing at the file's pages read-only.
//! - `.bss` reads zero before anything writes it. Those bytes are not in the file, so a loader that
//!   forgot them would leave whatever the frame allocator handed over.
//! - The stack survives eight frames of recursion, so `sp` was set to real, writable memory with
//!   room under it.
//!
//! # How it says no
//!
//! It traps. A program whose loader is broken has no reliable way to report anything (the endpoint
//! it would report on may not be mapped either). The kernel turns the trap into a fault and kills
//! the thread, which the test counts. `fail` is a parameter rather than a call into
//! `user_mode_runtime`, which keeps this crate host-buildable: the caller hands over its own way to
//! die.
//!
//! # Bugs
//!
//! **The markers are `#[unsafe(no_mangle)]` statics, so exactly one copy of this crate may be
//! linked into a binary.** Two would be a duplicate symbol at link time rather than a silent
//! wrong answer, but it does mean this crate cannot be a transitive dependency of something a
//! program also depends on directly.
//!
//! **It cannot check the executable segment.** A loader that mapped `.text` writable would pass
//! every check here; the `W^X` assertions of DECISIONS §10 (process model: capability-based,
//! microkernel) in `system_tests/src/user/elf_loader_tests.rs` catch that instead.
//!

//! Name: ratified 2026-09-14 (calef, working the unratified worklist). Coined, not standard: the
//! published prior art verifies ELF sections loader-side, and `kselftest` and `POST` name a system
//! testing itself, where this program audits someone else's work on it. A crate because two
//! fixtures need it and AGENTS.md rule 7 admits no `#[path]` module. Refused `self_check` (a verb
//! phrase, and `self` names the caller rather than the thing checked). Refused `image_check`
//! ("image" is as often a disk image here: `target/nifefs.img`). Refused putting it in
//! `crates/user_mode_runtime` (a diagnostic nothing in a running system calls does not belong in
//! that surface, and a crate depending on it cannot build for the host).

#![no_std]

use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering::Relaxed;

#[unsafe(no_mangle)]
static RODATA_MARKER: [u8; 4] = [0xc0, 0xff, 0xee, 0xd0];
// Atomics rather than `static mut` (milestone 891 (a mutable static becomes an atomic, a lock or a
// recorded cell)). The markers land where the test needs them: an atomic is interior-mutable, so it
// can never be placed in `.rodata`, and it is laid out as a plain `u64`, so a nonzero initializer
// is `.data` and a zero one is `.bss`, exactly as the `u64`s were. An atomic load is not folded to
// the initializer, which is what the volatile reads used to guarantee.
#[unsafe(no_mangle)]
static DATA_MARKER: AtomicU64 = AtomicU64::new(0x0000_c0ff_ee00_d0d0);
#[unsafe(no_mangle)]
static BSS_MARKER: AtomicU64 = AtomicU64::new(0);

/// Check this process's own image, calling `fail` if any part of it is wrong.
///
/// Returns normally when everything held. There is no error value on purpose: see the module's
/// "How it says no". `fail` diverges, so nothing after a failed check runs.
///
/// # Examples
///
/// The whole of a self-checking program:
///
/// ```ignore
/// #[unsafe(no_mangle)]
/// pub extern "C" fn _start(_a: u64, _b: u64, _c: u64) -> ! {
///     loaded_image_check::verify(user_mode_runtime::trap);
///     user_mode_runtime::exit();
/// }
/// ```
pub fn verify(fail: fn() -> !) {
    let check = |ok: bool| {
        if !ok {
            fail()
        }
    };
    check(RODATA_MARKER == [0xc0, 0xff, 0xee, 0xd0]);
    check(DATA_MARKER.load(Relaxed) == 0x0000_c0ff_ee00_d0d0);
    check(BSS_MARKER.load(Relaxed) == 0); // .bss was zeroed
    BSS_MARKER.store(1, Relaxed);
    check(BSS_MARKER.load(Relaxed) == 1); // .bss is writable
    check(stack_works(7));
}

/// Recurse `n` frames, keeping a live array on each so the compiler cannot flatten the chain.
#[inline(never)]
fn stack_works(n: u64) -> bool {
    let local = [n; 8];
    if n == 0 {
        return local[0] == 0;
    }
    core::hint::black_box(&local);
    stack_works(n - 1)
}

#[cfg(test)]
mod tests {
    /// **The check passes on a correctly loaded image**, which on the host is every image: the
    /// point of running it here is that the *logic* is exercised by an ordinary `cargo test`
    /// rather than only inside an emulator. What it cannot prove on the host is the thing it
    /// exists for, a loader that got a segment wrong, so this is a smoke test and says so.
    #[test]
    fn a_correctly_loaded_image_passes_every_check() {
        super::verify(|| unreachable!("a host binary failed this crate's own image check"));
        // `verify` writes `.bss` to prove it is writable, and it is the only thing here that can
        // be seen from outside: a `verify` that did nothing would pass the line above. This test
        // is the one caller in this binary, so the marker is not raced.
        let marker = super::BSS_MARKER.load(super::Relaxed);
        assert_eq!(marker, 1, "verify did not run its checks");
    }

    /// The recursion really recurses: eight frames deep, and the answer depends on the deepest.
    #[test]
    fn the_stack_check_reaches_the_bottom() {
        assert!(super::stack_works(7));
        assert!(super::stack_works(0));
    }
}
