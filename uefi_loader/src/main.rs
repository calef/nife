//! **The boot entry real firmware can start** (milestone 87; aarch64 and riscv64 since the stick
//! milestone, 2026-09-19).
//!
//! A UEFI application the firmware loads from the removable-media path of a FAT volume
//! (`\EFI\BOOT\BOOTX64.EFI`, `BOOTAA64.EFI`, `BOOTRISCV64.EFI`), carrying the kernel and the
//! userspace archive inside itself, which asks the firmware the questions a kernel cannot ask once
//! the firmware is gone and hands over. One source, three architectures; what differs is how the
//! kernel is told about the machine, and that lives under `src/arch/`:
//!
//! | Architecture | The kernel's entry contract | Told about the machine by |
//! |---|---|---|
//! | `x86_64` | PVH: 32-bit protected mode, `ebx` = `hvm_start_info` | a synthesised `hvm_start_info` (`arch/x86_64`) |
//! | aarch64 | Linux arm64 `booti`: MMU off, `x0` = device tree | the firmware's device tree, with `/chosen` naming the archive (`arch/aarch64`) |
//! | riscv64 | Linux riscv `booti`: paging off, `a0` = hart, `a1` = device tree | the same, plus the boot hart from the firmware (`arch/riscv64`) |
//!
//! In every case **the kernel is not modified and cannot tell which loader started it**: the state
//! it is entered in is the state QEMU's `-kernel` or U-Boot's `booti` would have entered it in.
//!
//! **Read notes/x86-uefi-boot.md and notes/boot-stick.md before changing anything here.**
//!
//! # The order of operations is rigid, and two rules are why
//!
//! `ExitBootServices` takes a `map_key` from a `GetMemoryMap` with **no allocation in between**, so
//! every buffer is allocated before that call and filled in after it. And **after
//! `ExitBootServices` there is no console**: a failure past that point can only halt, so everything
//! checkable is checked while there is still something to report it on.
//!

//! # BUGS
//!
//! - **Secure Boot must be off.** This image is unsigned, and by calef's ruling on milestone 500 (a
//!   stick that boots with Secure Boot on, or a page that says how to turn it off) nothing signs it:
//!   a stranger turns Secure Boot off by `notes/turning-secure-boot-off.md`. Under QEMU it is the
//!   difference between `edk2-x86_64-code.fd` and `edk2-x86_64-secure-code.fd`.
//! - **The kernel is embedded rather than loaded from the filesystem.** One file on the stick
//!   instead of three, and no `SimpleFileSystem` protocol to speak, at the cost of rebuilding this
//!   loader whenever the kernel changes. `cargo xtask uefi-image` and `cargo xtask stick` do both
//!   in one command, and the stick carries a `NIFE.TXT` naming its build, so a stale file is no
//!   longer silent; it is still stale.
//! - **Nothing validates the UEFI tables' header CRCs.** The firmware is the trust root at this
//!   point in boot; there is nothing to check it against that is not also the firmware.
//! - **Nothing here verifies what it hands over at boot.** The kernel and the archive are bytes this
//!   binary was compiled with, so the trust boundary is the build. `build.rs` does refuse to embed
//!   an archive the kernel does not vouch for, which is the one check that matters at build time.
//! - **AP bring-up under UEFI is proved on OVMF and nowhere else** (milestone 195); see
//!   `arch/x86_64`.
//! - **The `hvm_start_info` command line carries exactly one thing** (milestone 243): where the
//!   screen is. Nothing reads a configuration file off the stick, so anything on that line is
//!   something this binary decided.
//! - **The screen is taken as the firmware left it**, and on aarch64 and riscv64 it is not handed
//!   over at all: those kernels learn about a display from the device tree, and the firmware's GOP
//!   framebuffer is not in it.
//! - **A `PixelBitMask` or `PixelBltOnly` adapter gets no screen** on `x86_64`, and the loader says
//!   so before it hands over.

#![no_std]
#![no_main]

use core::ptr;

// **UEFI through `r-efi`** (milestone 784 (reuse what the tree wrote), item (e), ruled by calef on
// 2026-10-20 UTC under §46 (thin primitives or whole subsystems)). Its tables are complete and in
// specification order, so no field here is found by counting. Every function pointer in them is an
// `unsafe extern "efiapi" fn`. `boot_services` wraps the calls made more than once; every other
// call site states why its arguments are what the specification asks for. All of them run before
// `ExitBootServices`, through the table the firmware handed to `efi_main`.
use r_efi::efi::{
    self, ALLOCATE_ADDRESS, ALLOCATE_MAX_ADDRESS, BootServices, ConfigurationTable, Handle, Status,
    SystemTable,
};
#[cfg(target_arch = "x86_64")]
use r_efi::protocols::{file, loaded_image, simple_file_system};
#[cfg(not(target_arch = "x86_64"))]
use uefi_loader::device_tree_patch;
use uefi_loader::image;

mod arch;
mod boot_services;
/// **Which of the two boot images on the disk to start**, so a bad upgrade cannot brick the machine (milestone 198's rung 2b).
#[cfg(target_arch = "x86_64")]
mod chooser;

mod console;
mod loading;
mod placement;

use console::*;
use loading::*;
pub use placement::Placed;
use placement::*;

/// The kernel ELF and the userspace archive, embedded by `build.rs`.
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

/// `EFI_SUCCESS`, spelled once for the comparisons every boot-service call ends in.
const SUCCESS: Status = Status::SUCCESS;

/// UEFI's page size, and the granularity of every `AllocatePages` call.
const PAGE: u64 = 4096;

/// Slack, in descriptors, added to the memory map buffer between the sizing call and the real one.
///
/// The UEFI specification says so outright: allocating memory to hold the map can itself change the
/// map. This loader allocates exactly once between the two calls, and that allocation can split one
/// free range into as many as three; thirty-two is far more than that and costs 4 KiB.
const MAP_SLACK_DESCRIPTORS: usize = 32;
