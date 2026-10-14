//! **The assembly this architecture links**: the S-mode entry, the context switch, the trap
//! vector and the floating-point save area.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

// The S-mode entry (_start), the .bss zeroing, and the stack handoff to `kernel_main`.
global_asm!(include_str!("boot.s"));

// The context switch and the two first-run trampolines (the asm half of context.rs).
global_asm!(include_str!("context.s"));

// The S-mode trap vector (the asm half of exceptions.rs): save the frame, dispatch, restore, sret.
global_asm!(include_str!("trap.s"));

// Saving and restoring `f0`-`f31` (milestone 447 (a thread's vector registers are its own)). Separate from
// context.s because it moves a
// register file rather than a calling convention's callee-saved set; see fp.rs.
global_asm!(include_str!("fp.s"));
