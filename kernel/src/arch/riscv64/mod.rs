//! **The RISC-V (rv64) architecture layer.** The second implementation of the `arch/` contract
//! (milestone 20, notes/riscv-port.md), the one that proves rule #1: the rest of the kernel calls
//! everything here through `crate::arch`, exactly the names it calls on aarch64.
//!
//! This is a scaffold. The pieces that are pure and portable-in-spirit (the per-CPU register, the
//! halt/idle/barrier primitives, the initial thread contexts) are real. The pieces that are the
//! work of later steps (MMU/Sv39, traps, the timer, SMP bring-up, the test-exit) are loud
//! `unimplemented!()` stubs, each naming the step that fills it, so nobody mistakes a stub for a
//! working port. What is proved *today* is that the boundary is complete: a second architecture
//! compiles and links against the whole kernel with no change above `arch/`.

use core::arch::global_asm;

pub mod context;
pub mod exceptions;
#[cfg(feature = "fastpath_pad")]
mod fastpath_pad;
pub mod fp;
mod instructions;
pub mod interrupts;
pub mod iommu;
pub mod irq;
pub mod isa;
pub mod machine;
pub mod mmu;
pub mod pmu;
mod sbi;
pub mod semihosting;
pub mod thread_pointer;
pub mod timer;

// The saved thread context and how a new one is faked (the Rust half of context.s). Re-exported
// flat so `crate::arch::{Context, switch_to}` names them regardless of architecture.
use core::sync::atomic::{AtomicUsize, Ordering};

pub use context::{Context, switch_to};
// E3's padding sled (milestone 134); see kernel/src/fastpath_pad.rs.
#[cfg(feature = "fastpath_pad")]
pub use fastpath_pad::fastpath_pad_body;
/// The arch contract for a kernel-initiated cold reboot (milestone 249 (the boot lottery is sampled by a person walking to the board)). See [`semihosting::reboot`].
pub use semihosting::reboot;

mod assembly;
mod cpu_init;
mod harts;
mod percpu;
mod primitives;
mod remote_fence;

pub use cpu_init::*;
pub use harts::*;
pub use percpu::*;
pub use primitives::*;
pub use remote_fence::*;
