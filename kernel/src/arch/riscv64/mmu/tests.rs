//! Tests for the Sv39 MMU: the live page tables, W^X, the guard page, and TLB invalidation.
//!
//! These are the RISC-V twins of the aarch64 module's, written against the same *properties*
//! rather than the same mechanisms (DECISIONS §19). Where the two ISAs differ, the difference is
//! stated where it matters: one `satp` instead of the TTBR0/TTBR1 pair, three levels instead of
//! four, a single `X` bit whose privilege is decided by `U` instead of the PXN/UXN pair, and a
//! software RSW bit standing in for a memory type base Sv39 has no encoding for.
//!
//! `translate` walks the live kernel root, so these inspect the tables the hardware is
//! **actually walking**, not a copy of what we intended.
//!
//! Moved out of `mmu.rs` unchanged on 2026-10-12 (UTC) by lane maintainer/near-ceiling-splits,
//! with the Kani harnesses beside them in `proofs.rs`, so `mmu.rs` stays under §266 (a Rust source
//! file stays under 2,000 lines).

/// **A physical address certainly outside every RAM region this machine described**, for
/// tests that need a spare page to map and unmap without colliding with the direct map.
///
/// Two tests here used to hardcode `0x1_0000_0000` / `0x1_0100_0000` as "not RAM on QEMU
/// virt", true when written and false on the VisionFive 2 (bench, 2026-08-21): 8 GiB of RAM
/// starting at `0x4000_0000` reaches past `0x2_4000_0000`, so both addresses land inside the
/// direct map and the very first `map_page` in each test failed with `AlreadyMapped` before
/// the test's own logic ever ran. Reading the machine's own RAM regions and picking one gap
/// above the top of the highest region is the fix that works on any machine's memory map,
/// not just a wider guess.
fn a_physical_address_outside_every_ram_region() -> u64 {
    let top = crate::memory::ram_regions()
        .map(|(start, size)| start + size)
        .max()
        .expect("a booted kernel has at least one RAM region");
    // One gigabyte clear of the top: page-aligned by construction (RAM regions are), and far
    // enough that a region reported with generous rounding still leaves room.
    top + (1 << 30)
}

/// **The trust flag agrees with the measured width, whatever that width is.**
///
/// This used to assert `bits >= 8` outright, named for `crates/address_space_identifier`'s own justification:
/// "below even the smallest hardware ASID space (8-bit, 256)", true of aarch64 (which
/// mandates 8 bits) and **not guaranteed by RISC-V at all**, which permits zero. The
/// VisionFive 2's U74 measures exactly zero implemented bits (bench, 2026-08-21: the boot
/// summary reads `satp.ASID 0 bits measured`), which is not a hypothetical this test can
/// treat as unreachable; it is the live case [`asid_tagging_is_trusted`] exists to handle.
///
/// With `bits < ASID_BITS_NEEDED`, every address space would carry ASID 0 in hardware and
/// their TLB entries would alias, which is one process reading another's memory, EXCEPT that
/// [`asid_tagging_is_trusted`] being `false` keeps the unconditional `sfence.vma` in
/// [`write_satp`] doing the flush a narrow machine still needs. So the invariant this test
/// owes is not "the hardware is wide enough": it is "the kernel correctly knows whether the
/// hardware is wide enough", which is checkable on any width, including zero.
#[test_case]
fn the_hardware_has_at_least_the_asid_bits_the_allocator_assumes() {
    let bits = super::asid_bits();
    assert!(
        bits <= super::SATP_ASID_WIDTH as usize,
        "satp.ASID reported {bits} implemented bits, wider than the architectural 16",
    );
    assert_eq!(
        super::asid_tagging_is_trusted(),
        bits >= super::ASID_BITS_NEEDED as usize,
        "satp.ASID implements {bits} bits, address_space_identifier::ASIDS needs {}: the trust flag must agree \
         with whether this width holds every tag the allocator can hand out. If it does not, \
         either the flush stays where it should have been dropped, or address spaces would \
         alias in the TLB with no flush catching it.",
        address_space_identifier::ASIDS,
    );
}

/// Paging is on, and we are alive to say so.
///
/// Weaker than it looks on this ISA, and worth saying so: the kernel runs at `KERNEL_VA_BASE`,
/// which does not exist unless Sv39 is on, so a machine that reached this line has paging. What
/// the assertion adds is that [`is_enabled`](super::is_enabled) reads the right field: `satp`'s
/// MODE is bits 63:60, and a helper that looked at the wrong bits would answer "paging is off"
/// on a paging machine, which is the sort of quiet wrongness that only shows up in whatever
/// decides to trust it.
#[test_case]
fn mmu_is_enabled() {
    assert!(crate::arch::mmu::is_enabled(), "satp.MODE reads as Bare");
}

/// The kernel is running in the Sv39 high half.
///
/// The reason is the same as aarch64's, arrived at differently. There the kernel lives in
/// `TTBR1`, which a process switch never touches. Here there is one `satp` per address space and
/// every process root carries a **copy of the kernel's top-level entries** (`share_kernel_half`),
/// so the kernel is reachable from every space. Either way the kernel must be in the half that
/// is not the process's, or the first switch into userspace deletes the kernel.
#[test_case]
fn the_kernel_lives_in_the_high_half() {
    use crate::arch::mmu::KERNEL_VA_BASE;

    // Our own code.
    let pc = crate::kernel_main as *const () as u64;
    assert!(
        pc >= KERNEL_VA_BASE,
        "kernel .text is at {pc:#x}, not in the high half"
    );

    // Our stack.
    let sp = crate::arch::current_sp();
    assert!(
        sp >= KERNEL_VA_BASE,
        "the stack is at {sp:#x}, not in the high half"
    );

    // And a static.
    static IN_BSS: u64 = 0;
    let addr = (&raw const IN_BSS) as u64;
    assert!(
        addr >= KERNEL_VA_BASE,
        "a static is at {addr:#x}, not in the high half"
    );
}

/// **The low half is empty when no process is running**, and on RISC-V that is a stronger claim
/// than on aarch64.
///
/// aarch64 can point `TTBR0` at an empty reserved table, so "no user space installed" is a
/// separate register. RISC-V has *one* `satp`: the kernel runs on the kernel root, and that root's
/// own low half is what a low address resolves through. Nothing but discipline stops
/// `map_everything` from leaving something down there, and if it did, a stray low pointer in the
/// kernel would silently succeed instead of faulting, and a process would inherit the mapping
/// through `share_kernel_half`'s copy path the moment the entry moved up a level.
///
/// The addresses are all inside Sv39's low half (`va >> 38 == 0`). That is deliberate: the
/// `Mapper` returns `None` for anything outside its half *before it walks anything*, so a test
/// address above 2^38 would pass without a single page-table read and prove nothing at all.
#[test_case]
fn a_low_address_does_not_translate_when_no_process_is_running() {
    use crate::arch::mmu::translate_user;

    for va in [
        0x1000u64,
        0x8020_0000, // where OpenSBI loaded us: the identity map, if it survived
        0x0000_003f_ffff_f000, // the top page of the Sv39 low half
    ] {
        assert!(
            translate_user(va).is_none(),
            "{va:#x} translates through the live satp: the boot table's identity gigapages \
             may still be live",
        );
    }
}

/// The direct map: every physical address is nameable at `pa + KERNEL_VA_BASE`.
///
/// This is how the kernel touches a frame the allocator just handed it. Without it, a physical
/// address the kernel cannot NAME is a physical address it cannot use.
#[test_case]
fn the_direct_map_reaches_physical_memory() {
    use crate::arch::mmu::{phys_to_virt, virt_to_phys};

    let frame = crate::memory::alloc().expect("out of memory");
    let va = phys_to_virt(frame.addr());

    assert_eq!(
        virt_to_phys(va),
        frame.addr(),
        "the transform is not reversible"
    );

    let (pa, flags) = crate::arch::mmu::translate(va).expect("frame is NOT in the direct map");
    assert_eq!(pa, frame.addr());
    assert!(flags.is_writable());

    // And it is real memory: write through the virtual name, read it back.
    // SAFETY: the allocator just gave us this frame exclusively.
    unsafe {
        core::ptr::write_volatile(va as *mut u64, 0xfeed_face_cafe_f00d);
        assert_eq!(
            core::ptr::read_volatile(va as *const u64),
            0xfeed_face_cafe_f00d
        );
    }

    crate::memory::free(frame);
}

/// **The guard page must not be mapped.** That is its entire job.
///
/// link-riscv64.ld has reserved the page since the port landed, and `map_everything` skips it by
/// mapping `.data..bss_end` and `stack_bottom..stack_top` as two ranges with a hole between
/// them. Nothing asserted the hole was where it was supposed to be, so a linker-script edit that
/// moved `__stack_guard` would have left the stack still mapped and the protection silently
/// gone. `verify` now checks it at boot as well, as aarch64's always has.
#[test_case]
fn the_guard_page_is_a_hole() {
    use crate::arch::mmu;
    assert_eq!(
        mmu::translate(mmu::stack_guard()),
        None,
        "the guard page IS mapped: a stack overflow would silently eat .bss"
    );

    // And the pages either side of it must be mapped, or the hole is in the wrong place and is
    // protecting nothing.
    assert!(
        mmu::translate(mmu::stack_guard() - 4096).is_some(),
        "below the guard"
    );
    assert!(
        mmu::translate(mmu::stack_bottom()).is_some(),
        "the stack itself"
    );
}

/// W^X, checked against the tables the hardware is actually walking.
///
/// **The `!is_user_executable` line is weaker here than on aarch64, and deliberately kept
/// anyway.** aarch64 has two independent execute-never bits (PXN and UXN), so asserting both is
/// two claims. Sv39 has one `X` bit whose privilege is decided by `U`, and `Sv39::leaf_flags`
/// reports kernel-exec or user-exec accordingly, so given kernel-exec the user-exec assertion
/// cannot fail. It stays because the *property* is what the kernel cares about and the format
/// under it may change (Svpbmt, or a future format with separate bits); it is not carrying
/// weight today.
#[test_case]
fn kernel_text_is_executable_and_not_writable() {
    use crate::arch::mmu;

    let (pa, flags) = mmu::translate(mmu::text_start()).expect(".text is not mapped");
    assert_eq!(
        pa,
        mmu::virt_to_phys(mmu::text_start()),
        ".text maps to the wrong frame"
    );

    assert!(flags.is_kernel_executable(), ".text is not executable");
    assert!(!flags.is_writable(), ".text is WRITABLE: W^X is broken");
    assert!(!flags.is_user_executable(), ".text is executable by U-mode");
}

/// Constants are read-only, and not executable by anyone.
#[test_case]
fn kernel_rodata_is_read_only_and_not_executable() {
    use crate::arch::mmu;

    let (_, flags) = mmu::translate(mmu::rodata_start()).expect(".rodata is not mapped");
    assert!(!flags.is_writable(), ".rodata is writable");
    assert!(!flags.is_kernel_executable(), ".rodata is executable");
}

/// The stack is writable and NOT executable.
#[test_case]
fn the_stack_is_writable_and_not_executable() {
    use crate::arch::mmu;

    let (_, flags) = mmu::translate(mmu::stack_bottom()).expect("stack is not mapped");
    assert!(flags.is_writable());
    assert!(
        !flags.is_kernel_executable(),
        "the stack is EXECUTABLE: data on the stack could be run as code"
    );
}

/// The UART is device-typed.
///
/// **And the honest caveat, because this is the one place the two ISAs are not the same claim.**
/// On aarch64 the device type is an architectural PTE field, and getting it wrong lets the CPU
/// cache, reorder, merge and *speculatively read* MMIO; a speculative read of a UART FIFO
/// register consumes the byte. Base Sv39 has no such field (that is the Svpbmt extension), so
/// `paging::Sv39` carries the flag in an RSW software bit and QEMU's `virt` derives the real
/// memory type from the physical address instead. So this asserts the kernel's *bookkeeping* is
/// right, not that the hardware was told. It still fails for the mistake worth catching (mapping
/// the UART with `Flags::kernel_data()`), and on a board with Svpbmt the same flag is what would
/// drive the architectural bits. See notes/page-tables.md and crates/paging/src/sv39.rs.
#[test_case]
fn the_uart_is_mapped_as_device_memory() {
    use crate::arch::mmu;

    // The UART lives in the direct map, like every other physical address the kernel names: its
    // raw physical address stopped existing when the boot table's identity gigapages went away.
    let (_, flags) =
        mmu::translate(mmu::phys_to_virt(super::UART_BASE)).expect("the UART is not mapped");

    assert!(flags.is_device(), "the UART is not device memory");
    assert!(flags.is_writable(), "we do need to write to it");
    assert!(!flags.is_kernel_executable());
}

/// A frame from the allocator is still real, writable memory *through the MMU*.
///
/// Proves the direct map covers everything the allocator can hand out. With paging on, a
/// physical address the kernel cannot name is a physical address it cannot use, and the riscv
/// direct map is built from `memory::ram_regions()` with the kernel image punched out of it,
/// which is exactly the kind of arithmetic that can leave a hole nobody notices.
#[test_case]
fn an_allocated_frame_is_reachable_through_the_mmu() {
    use crate::arch::mmu;

    let frame = crate::memory::alloc().expect("out of memory");
    let va = mmu::phys_to_virt(frame.addr());
    let (pa, flags) = mmu::translate(va).expect("allocated frame is NOT MAPPED");

    assert_eq!(pa, frame.addr());
    assert!(flags.is_writable());
    assert!(!flags.is_kernel_executable(), "RAM is executable");

    crate::memory::free(frame);
}

/// **Prove the TLB is actually invalidated on unmap.**
///
/// The landmine, and it is the same landmine on both ISAs even though the instruction differs
/// (`sfence.vma` here, `tlbi` there). Change a mapping without discharging the flush and the CPU
/// keeps using the *cached* translation: memory reads back as the previous owner's data. It is a
/// security hole and it is close to undebuggable, because the page tables **in memory are
/// correct**; the lie lives in a CPU structure you cannot inspect.
///
/// So we make it observable:
///
///   1. map a spare VA to frame A, which holds 0xAAAA...
///   2. **read it**, which is what populates the TLB
///   3. unmap, and invalidate
///   4. map the *same VA* to frame B, which holds 0xBBBB...
///   5. read it again
///
/// If step 5 returns 0xAAAA, the TLB is stale and we have exactly the bug. It must return
/// 0xBBBB.
#[test_case]
fn unmap_invalidates_the_tlb() {
    use paging::Flags;

    use crate::arch::mmu::{self, phys_to_virt};

    const PATTERN_A: u64 = 0xaaaa_aaaa_aaaa_aaaa;
    const PATTERN_B: u64 = 0xbbbb_bbbb_bbbb_bbbb;

    // A high-half address well clear of everything the kernel maps: outside every RAM region
    // this machine described, and above every device window on both QEMU virt and the
    // VisionFive 2. Sv39's high half is 256 GiB, so it is a nameable address.
    let test_va = mmu::phys_to_virt(a_physical_address_outside_every_ram_region());
    assert_eq!(
        mmu::translate(test_va),
        None,
        "test address is already in use"
    );

    let a = crate::memory::alloc().expect("out of memory");
    let b = crate::memory::alloc().expect("out of memory");

    // SAFETY: two frames the allocator just gave us exclusively, reached via the direct map.
    unsafe {
        core::ptr::write_volatile(phys_to_virt(a.addr()) as *mut u64, PATTERN_A);
        core::ptr::write_volatile(phys_to_virt(b.addr()) as *mut u64, PATTERN_B);
    }

    mmu::map_page(test_va, a.addr(), Flags::kernel_data()).expect("map A");

    // SAFETY: just mapped, writable.
    let seen = unsafe { core::ptr::read_volatile(test_va as *const u64) };
    assert_eq!(seen, PATTERN_A, "the mapping didn't take");
    // ^ that read is the point: it pulls the translation into the TLB.

    let returned = mmu::unmap_page(test_va).expect("unmap");
    assert_eq!(returned, a.addr(), "unmap returned the wrong frame");

    mmu::map_page(test_va, b.addr(), Flags::kernel_data()).expect("map B");

    // SAFETY: mapped again, to a different frame.
    let seen = unsafe { core::ptr::read_volatile(test_va as *const u64) };

    assert_eq!(
        seen, PATTERN_B,
        "STALE TLB: the same virtual address still reads the OLD frame's data. \
         This is the bug that reads back another process's memory."
    );

    mmu::unmap_page(test_va).expect("cleanup");
    crate::memory::free(a);
    crate::memory::free(b);
}

/// Changing a mapping is forced through break-before-make.
///
/// aarch64 needs this because a valid -> valid change can raise a TLB conflict abort. RISC-V is
/// more forgiving about the hardware consequence, but the API is the same on both because the
/// *software* hazard is the same: overwrite a leaf in place and the old frame is leaked with no
/// record that it ever existed. The mapper makes it unrepresentable rather than merely unwise.
#[test_case]
fn the_kernel_mapper_refuses_to_overwrite() {
    use paging::{Flags, MapError};

    use crate::arch::mmu;

    let va = mmu::phys_to_virt(a_physical_address_outside_every_ram_region());
    let f = crate::memory::alloc().unwrap();

    mmu::map_page(va, f.addr(), Flags::kernel_data()).unwrap();

    assert_eq!(
        mmu::map_page(va, f.addr(), Flags::kernel_data()),
        Err(MapError::AlreadyMapped)
    );

    mmu::unmap_page(va).unwrap();
    crate::memory::free(f);
}

/// **`satp` carries the address space's ASID, in the field the hardware reads.**
///
/// This is as much of aarch64's `asid_tagging_keeps_address_spaces_apart_without_flushes` as is
/// true on RISC-V today, and the gap is worth naming rather than papering over.
///
/// That test proves two things: distinct spaces get distinct tags, *and* switching between them
/// flushes nothing, so their TLB entries coexist. The second half cannot be proved here, because
/// `write_satp` follows every `csrw satp` with a bare `sfence.vma`, which discards the whole TLB
/// on every user switch. So the ASID is written and then immediately made irrelevant: an
/// isolation test would pass with the tagging removed entirely, which makes it a test that
/// cannot fail for its stated reason, and we do not ship those.
///
/// What is left is real and untested until now: `ttbr0_value` must place the ASID at bits 59:44,
/// where `satp` keeps it. It sits directly above the PPN, so a wrong shift either corrupts the
/// root pointer (loud) or drops the tag on the floor (silent, and about to matter). Dropping the
/// unconditional `sfence` is a change to the switching model, not a test fix; see
/// notes/riscv-arch-tests.md.
#[test_case]
fn the_satp_carries_the_address_spaces_asid() {
    use crate::user::AddressSpace;

    let a = AddressSpace::new(2).expect("no space A");
    let b = AddressSpace::new(2).expect("no space B");

    // satp.ASID is bits 59:44 (16 bits) on rv64 Sv39.
    let asid_of = |satp: u64| (satp >> 44) & 0xffff;
    let (asid_a, asid_b) = (asid_of(a.ttbr0()), asid_of(b.ttbr0()));

    assert_ne!(asid_a, asid_b, "two live spaces share an ASID");
    assert_ne!(asid_a, 0, "a user space got the kernel's ASID 0");
    assert_ne!(asid_b, 0, "a user space got the kernel's ASID 0");

    // And the tag did not eat the rest of the word. `satp` packs MODE (63:60), ASID (59:44) and
    // PPN (43:0) with no slack between them, so a shift that is off by four in either direction
    // lands the ASID in the mode field or in the root pointer. Both other fields must still be
    // what they were: Sv39 mode, and two distinct root tables.
    for satp in [a.ttbr0(), b.ttbr0()] {
        assert_eq!(satp >> 60, 8, "satp.MODE is not Sv39: {satp:#x}");
    }
    assert_ne!(
        a.ttbr0() & super::SATP_PPN_MASK,
        b.ttbr0() & super::SATP_PPN_MASK,
        "two address spaces share a root table",
    );
}
