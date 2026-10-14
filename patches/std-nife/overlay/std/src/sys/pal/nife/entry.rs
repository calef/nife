//! **The process entry point**: `_start` on aarch64 and riscv64, the `x86_64`
//! entry that keeps the SysV stack contract, and the call into `main`.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// The process entry point. The kernel (or a userspace loader) enters the ELF here per the
/// native ABI (notes/abi.md §3): three free argument registers, a mapped stack, a populated
/// capability table, no libc and no argv. rustc's generated C `main` wraps the user's `fn main` in
/// `lang_start`, which runs std's rt init and catches the exit code; all this shim does is call
/// it and turn the return into `SYS_EXIT`.
///
/// The linker finds this symbol in std's rlib because the link script's `ENTRY(_start)` makes it
/// an undefined reference, the same way a libc's crt0.o gets pulled in.
#[cfg(all(not(test), not(target_arch = "x86_64")))]
#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    run_main()
}

/// **`x86_64` enters through one `call`, because the SysV stack contract is about the instruction
/// before the function, not the function** (milestone 184).
///
/// The kernel drops a new thread to ring 3 with `rsp` at the top of its stack page, which is a page
/// boundary and so `rsp % 16 == 0`. The x86-64 psABI promises a function that `rsp % 16 == 8` on
/// entry, because it assumes a `call` just pushed an 8-byte return address. aarch64 and RISC-V have
/// no such offset (neither pushes on call), which is why only this architecture needs a shim. The
/// cheapest correct fix is to make the assumption true: `call` the Rust entry, which pushes exactly
/// the eight bytes the ABI expects. The argument registers pass through untouched.
///
/// Without SSE nothing in this target *faults* on a misaligned stack (there is no `movaps` to trip),
/// so getting this wrong would pass every test and stay latent until someone compiled with vector
/// features. That is the reason to do it properly rather than to rely on the soft-float target
/// being forgiving. The `ud2` is unreachable: the Rust entry diverges.
#[cfg(all(not(test), target_arch = "x86_64"))]
core::arch::global_asm!(
    ".pushsection .text._start, \"ax\", @progbits",
    ".globl _start",
    "_start:",
    "call {entry}",
    "ud2",
    ".popsection",
    entry = sym x86_entry,
);

#[cfg(all(not(test), target_arch = "x86_64"))]
pub(super) extern "C" fn x86_entry(_a0: u64, _a1: u64, _a2: u64) -> ! {
    run_main()
}

#[cfg(not(test))]
pub(super) fn run_main() -> ! {
    unsafe extern "C" {
        fn main(argc: isize, argv: *const *const u8, sigpipe: u8) -> i32;
    }
    // No argv on this ABI (arguments are three registers, unused by std programs for now), and
    // sigpipe is a Unix concept passed as 0, the same as every non-Unix port.
    let code = unsafe { main(0, core::ptr::null(), 0) };
    rt::exit(code as i64)
}
