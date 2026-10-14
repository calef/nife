//! The hardware abstraction boundary.
//!
//! Everything below this module is architecture-specific. Everything above it
//! should be portable. The rest of the kernel talks to the hardware only through
//! what is re-exported here.
//!
//! This is the single most important structural rule in the codebase, and the one
//! that is easiest to erode by accident. If you find yourself writing `asm!` or
//! touching a system register outside `arch/`, that's the bug. See
//! notes/portability.md.

#[cfg(target_arch = "aarch64")]
mod aarch64;

#[cfg(target_arch = "aarch64")]
pub use aarch64::*;

// The second architecture (milestone 20, notes/riscv-port.md). Its presence here, dispatched by
// the same `cfg` and re-exported flat through the same surface, is the proof that rule #1 holds: a
// new ISA is a new directory, not a diff across the kernel.
#[cfg(target_arch = "riscv64")]
mod riscv64;

#[cfg(target_arch = "riscv64")]
pub use riscv64::*;

// **riscv64's IOMMU driver, compiled for the prover on an aarch64 host.** This is milestone 432
// (the RISC-V IOMMU driver has no counterpart to the SMMU's proofs), built as option 1 of
// design/roadmap/0650-riscv64-code-the-prover-can-already-compile.md. Kani compiles for
// the host, so the `cfg` above hides every riscv64 file from it; this reaches one of them by path.
// The inline module is named `riscv64` and holds `iommu`, so the harnesses keep the module path
// they would have natively (`arch::riscv64::iommu::proofs::...`) and `script/falsifications` finds
// their patches under the name it derives from the file. No `#[path]` is needed: an inline module
// in `arch/mod.rs` resolves `mod iommu;` to `arch/riscv64/iommu.rs` on its own.
//
// **A foot gun, deliberately taken, and read notes/kernel-proofs.md before extending it.** Inside
// this module `crate::arch` is the HOST's architecture, not riscv64's: `iommu.rs`'s
// `crate::arch::mmu::phys_to_virt` resolves to aarch64's here. Only code that never calls through
// `crate::arch` is proved by a harness in this module, and `script/lint` fails if a file listed
// below gains a `crate::arch` reference it has not recorded. Why only aarch64: exactly one verify
// host should run it, and whether x86_64 resolves the same names is unmeasured. Not riscv64
// either, where the real `mod riscv64` above already exists and nothing proves it.
//
// `dead_code` is allowed because nothing on the host calls `init` or `attach`; only the pure
// functions the harnesses reach are live here.
#[cfg(all(kani, target_arch = "aarch64"))]
#[allow(dead_code)]
mod riscv64 {
    mod iommu;
}

// The third architecture (milestone 161, notes/x86-port.md), and the one that tests whether the
// split above is real or an accident of two similar RISC machines. Same `cfg`, same flat
// re-export, no change anywhere else in this file: a new ISA is a new directory.
#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

mod halt_reason;
mod name;
mod user_fault;

pub use halt_reason::*;
// `NAME` is read only by boots that print a transcript, as its own `allow(dead_code)` says.
#[cfg_attr(
    any(test, feature = "system_tests", feature = "bench"),
    allow(unused_imports)
)]
pub use name::*;
pub use user_fault::*;
