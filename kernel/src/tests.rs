//! Tests for the boot path itself: the things `boot.s` and the boot protocol had to get
//! right before any other code could run at all.
//!
//! Everything else lives beside the code it tests. `cargo test -p kernel` still collects
//! all of them: `custom_test_frameworks` gathers every `#[test_case]` in the crate,
//! wherever it is.

/// Proves the harness itself works. If this fails, nothing else is meaningful.
#[test_case]
fn harness_runs() {
    // black_box so this is a real runtime check, not a constant clippy folds to `2 == 2`.
    let two = core::hint::black_box(1) + core::hint::black_box(1);
    assert_eq!(two, 2);
}

/// Proves `boot.s` zeroed `.bss`.
///
/// A zero-initialized static lands in `.bss`, which occupies no bytes in the
/// ELF file. Nobody loaded it. If our zeroing loop were wrong, this would hold
/// whatever garbage was in RAM at power-on. See notes/elf.md.
#[test_case]
fn bss_was_zeroed() {
    use core::sync::atomic::{AtomicU64, Ordering};
    static CANARY: AtomicU64 = AtomicU64::new(0);
    assert_eq!(CANARY.load(Ordering::Relaxed), 0);
}

/// Proves `boot.s` gave us a usable, correctly aligned stack.
///
/// Both aarch64 and RISC-V require a 16-byte-aligned `sp`, and both fault or corrupt
/// silently on a misaligned one, so a bug here would show up as a mysterious early crash
/// rather than as anything legible. Reads `sp` through `arch::current_sp`, so it is portable.
/// See notes/stack.md.
#[test_case]
fn stack_pointer_is_16_byte_aligned() {
    let sp = crate::arch::current_sp();
    assert_eq!(sp % 16, 0, "sp = {sp:#x}");
}

/// Proves we are where we think we are.
///
/// QEMU's `virt` machine starts a kernel at EL1 by default, which is exactly where a kernel
/// belongs. **A machine can hand us EL2 instead, and then this test is the one that proves
/// the drop worked** (milestone 127, the seL4 machine): `virt,virtualization=on` starts the
/// kernel at EL2 the way U-Boot does on a board, `boot.s` reads `CurrentEL` and `eret`s down,
/// and the assertion below is unchanged either way. The older comment here said that if this
/// ever read EL2 "we will need to drop down ourselves"; that is now what happens, and the
/// entry level is recorded rather than lost (`arch::entry_el`). See notes/aarch64.md.
///
/// EL is an aarch64 concept, so this is gated. **RISC-V needs no twin, and the older
/// comment here promising one "with the RISC-V boot path" outlived the boot path it
/// was waiting for.** That ISA deliberately gives S-mode no way to read its own
/// privilege level, and it does not need one: `arch::riscv64::exceptions`'s
/// `breakpoint_is_caught_and_execution_resumes` proves the same thing sideways, because
/// the breakpoint arm it counts is guarded on the trap having come from S-mode, and an
/// M-mode `ebreak` would have gone to OpenSBI's `mtvec` and never reached us at all.
/// See notes/riscv-arch-tests.md.
#[cfg(target_arch = "aarch64")]
#[test_case]
fn running_at_el1() {
    use aarch64_cpu::registers::CurrentEL;
    use tock_registers::interfaces::Readable;
    assert_eq!(CurrentEL.read(CurrentEL::EL), 1);
}

/// Proves the boot protocol actually delivered a device tree.
///
/// This is the test that closes the correction from milestone 1. Back then we
/// shipped an ELF, QEMU took its bare-metal path, and `x0` arrived as zero. Now
/// we ship a flat binary carrying an arm64 Image header, QEMU recognizes it as a
/// kernel, follows the Linux boot protocol, and hands us a real pointer.
///
/// A zero here means we have silently regressed to the ELF path, which would be
/// easy to do by editing the runner script and hard to notice any other way.
#[test_case]
fn device_tree_pointer_was_provided() {
    use core::sync::atomic::Ordering;
    assert_ne!(
        crate::DTB.load(Ordering::Relaxed),
        0,
        "no DTB pointer in x0: did we fall back to booting as an ELF?"
    );
}

/// Proves the pointer points at an actual device tree, not just at something.
///
/// A nonzero pointer is necessary but not sufficient. Every flattened device tree
/// begins with the magic `0xd00dfeed`, stored **big-endian** (the format predates
/// the little-endian consensus and never changed), so we have to byte-swap on the
/// way in. If this passes, the machine is genuinely describing itself to us.
/// **Not on `x86_64`**, and the reason is the whole of what that architecture's boot handoff
/// differs by: what arrives in `kernel_main`'s one pointer there is PVH's `hvm_start_info`,
/// which carries the same *kind* of thing (the memory map, the root of everything else
/// discoverable) in an entirely different format. `machine_discovery::x86_64` decodes it and
/// is host-tested against a real dump, which is the stronger place for that check to live;
/// this test's subject genuinely does not exist there.
#[test_case]
fn device_tree_has_the_right_magic() {
    use core::sync::atomic::Ordering;

    if cfg!(target_arch = "x86_64") {
        crate::testing::skip!(
            "this machine hands over PVH hvm_start_info, not a device tree (see \
             machine_discovery::x86_64, host-tested)"
        );
    }

    // DTB holds the PHYSICAL address QEMU gave us in x0. Since the kernel moved to the
    // high half, TTBR0 is disabled and a low address does not exist: dereferencing it
    // directly faults, which is exactly what we want and exactly what this line used to
    // do. Name it through the direct map instead.
    let pa = crate::DTB.load(Ordering::Relaxed) as u64;
    let ptr = crate::arch::mmu::phys_to_virt(pa) as *const u32;

    // SAFETY: QEMU put a device tree at that physical address, and the direct map makes
    // it readable.
    let magic = unsafe { core::ptr::read_volatile(ptr) };

    assert_eq!(
        u32::from_be(magic),
        0xd00d_feed,
        "no device tree magic at {ptr:p}"
    );
}
