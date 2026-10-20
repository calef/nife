//! **The `x86_64` architecture layer.** The third implementation of the `arch/` contract (milestone
//! 161, notes/x86-port.md), and the one milestone 20 said would be the real test of it: the first
//! two architectures are both RISC machines with a device tree, weak memory and a similar MMU, and
//! this one is none of those things.
//!
//! **This is a partial port, and it says so in every remaining stub.** What is real: the boot path
//! (a 32-bit multiboot-style trampoline into long mode and the high half), the GDT and TSS, the IDT
//! and the trap frame, the console UART over port I/O, the page-table format, the fine-grained W^X
//! kernel map, the local APIC and a calibrated timer, the IO APIC and a routed device line, user
//! address spaces, the `syscall` pair and ring 3, the address arithmetic, the interrupt-masking
//! primitives, the context switch, and the test exit. What is not: VT-d and SMP bring-up, each an
//! `unimplemented!()` that names itself and the reason, so that nobody mistakes a stub for a working
//! port.
//!
//! **And one thing that is real only at this layer**, which is worth saying here because this module
//! is where a reader looks first: a program runs at ring 3, but there is no *process* behind it. The
//! scheduler, the kernel heap and the untyped budget have never been brought up on this
//! architecture, so `user::run` and `KernelStack::new` have nothing to stand on. See
//! design/roadmap/0161-x86-64-kernel-port.md, item 4.
//!
//! # What this port has already shown about the seam
//!
//! Two things, both worth more than the code. The `paging` crate's split into a generic level walk
//! plus a per-architecture entry codec **held**: `paging::x86_64::Ia32e` is 60 lines and nothing
//! above it changed. And the 16550 driver spans a *different address space* (x86 port I/O rather
//! than MMIO) as a type parameter rather than a second driver, which is the strongest evidence so
//! far that "a new ISA is a new directory" is true rather than aspirational.
//!
//! # And two places it did not hold, until calef ratified the rename
//!
//! **`arch::psci_cpu_on` was an aarch64 name that leaked**, and RISC-V already had to implement it
//! as an SBI call under an ARM firmware interface's name. x86 has no third mechanism to hide behind
//! it: SMP bring-up here is INIT-SIPI-SIPI through the local APIC, sent by the interrupt controller,
//! at a physical page below 1 MiB, in 16-bit real mode. Ratified as `arch::cpu_start`.
//!
//! **The single pointer `kernel_main` took was called `dtb`.** x86 has no device tree; what arrives
//! there is PVH's `hvm_start_info`, which carries the memory map and the ACPI RSDP address, so the
//! *shape* was right and only the name was wrong. Ratified as `boot_info_pointer`.

use core::arch::{asm, global_asm};

// The AP real-mode trampoline's copy-and-prepare step (milestone 161's SMP item). See its own
// header, and `boot.s`'s `secondary_boot` for what it prepares.
// AMD's IOMMU (lane `amd-vi`), beside VT-d's `iommu`, which forwards to it on an AMD machine.
pub mod amd_vi;
pub mod ap_boot;
pub mod context;
pub mod exceptions;
pub mod fp;
mod instructions;
pub mod interrupts;
pub mod iommu;
pub mod irq;
pub mod isa;
// The NMI, double-fault and machine-check stacks the TSS's IST names: milestone 915 (NMI, double
// fault and machine check run on their own stacks), provisional.
pub mod ist;
// What the loader said (milestone 161): the kernel side of `machine_discovery::x86_64`.
pub mod machine;
pub mod mmu;
// Unhalted core cycles (milestone 309), the x86_64 half of milestone 74's measurement side. It is
// here rather than folded into `timer` because it is a different counter answering a different
// question: `timer::now()` is `rdtsc`, a constant-rate clock, and this is what the core actually
// ran. Its own header has the table of all three architectures' two counters.
pub mod pmu;
pub mod port;
pub mod reset;
pub mod rtc;
// The Intel TCO watchdog (milestone 593 (a wedged kernel resets itself), provisional number). Only
// the watchdog soak drives it, so only that build compiles it; see its own header.
pub mod segments;
pub mod semihosting;
#[cfg(soak_watchdog)]
pub mod tco;
pub mod thread_pointer;
pub mod timer;
/// The TSC calibration's estimator and its two boot assertions. Test-only; see the file header for
/// why they run under QEMU rather than on the host.
#[cfg(test)]
mod timer_calibration_tests;
/// The TSC-vs-RTC measurement instrument. Off by default; see notes/tsc-under-tcg.md.
#[cfg(feature = "tsc_probe")]
pub mod tsc_probe;

// The saved thread context and how a new one is faked (the Rust half of context.s). Re-exported
// flat so `crate::arch::{Context, switch_to}` names them regardless of architecture.
pub use context::{Context, switch_to};
// How the console reaches its UART's registers on this architecture. Named flat through `arch`
// because `console.rs` picks it by `target_arch` and must not reach into `arch::x86_64::` directly.
pub use port::PortIo;
/// The arch contract for a kernel-initiated cold reboot (milestone 249 (the boot lottery is sampled by a person walking to the board)). See [`reset::reboot`].
pub use reset::reboot;

mod assembly;
mod bring_up;
mod cpu_init;
mod msr;
mod percpu;
mod primitives;

pub use bring_up::*;
pub use cpu_init::*;
pub use msr::*;
pub use percpu::*;
pub use primitives::*;
