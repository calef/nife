//! **A loadable segment**: its `p_flags`, and what the loader reads off one.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

/// `p_type`: a segment the loader must actually put in memory. The only one we care about.
pub(crate) const PT_LOAD: u32 = 1;

/// `p_flags`.
pub const PF_X: u32 = 1;

/// `p_flags`: writable.
pub const PF_W: u32 = 2;

/// `p_flags`: readable.
pub const PF_R: u32 = 4;

/// One `PT_LOAD` segment: what to map, where, and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment<'a> {
    /// The virtual address the program wants this at. Its choice, and we honour it or refuse.
    pub vaddr: u64,

    /// `p_paddr`: where the segment wants to sit in **physical** memory. Only a loader placing an
    /// image at fixed physical addresses (a firmware loader, a bootloader) has any use for this.
    ///
    /// **Read the hazard before you reach for it, because the field is redundant in every case but
    /// the one that matters.** For every user program in this tree `paddr == vaddr`, because the
    /// linker scripts say so and nothing here builds a program that separates them. So a consumer
    /// that reaches for the wrong one of the two behaves *correctly* in every test it will ever run
    /// and wrongly on the single path where the distinction is real: **the kernel image**. Measured
    /// on the `x86_64-unknown-none` release build, `.text` is `p_vaddr` `0xffffffff80109000`
    /// against `p_paddr` `0x109000`, a fixed offset; but `.ap_trampoline` is `p_vaddr` `0x8000`
    /// against `p_paddr` `0x12b000`, which is a *different* relationship in the same file, so no
    /// caller can recover one from the other by subtracting anything.
    ///
    /// That segment is worth understanding rather than memorising, because it inverts the usual
    /// sense of both words. Its bytes **ship** at `0x12b000` (`AT()` puts them after `.rodata`,
    /// in ordinary file-backed memory nothing writes at runtime) and they **execute** at `0x8000`,
    /// because a STARTUP IPI can only name a physical page below 1 MiB and the secondary core
    /// arrives there in real mode with no paging at all. So its VMA was chosen to be its eventual
    /// execution address and its LMA is merely where the image carries it. See
    /// `kernel/link-x86_64.ld`.
    ///
    /// **A loader mapping into a fresh address space wants [`vaddr`](Self::vaddr) and never this.**
    /// That is `kernel/src/user.rs`, and it is the common case: it allocates frames wherever the
    /// allocator likes and maps them at the virtual address the program asked for, so the physical
    /// address in the file is not merely unused, it is a lie about a decision the loader is making
    /// itself.
    ///
    /// This is a plain `u64` on purpose, and it is a stopgap at rung three of AGENTS.md's ladder: a
    /// doc comment, read by whoever is already looking. Milestone 200 is what makes the mistake
    /// unsayable, by giving a virtual address and a physical address different types across the
    /// whole tree. A newtype on these two fields alone would claim a distinction the rest of the
    /// tree does not make, and one every consumer would immediately unwrap.
    pub paddr: u64,

    /// How much **memory** it occupies. May exceed `data.len()`.
    pub memsz: u64,

    /// `PF_R | PF_W | PF_X`.
    pub flags: u32,

    /// The bytes from the file. **`data.len()` is `p_filesz`, which can be less than `memsz`.**
    ///
    /// The difference is `.bss`, and **the loader must zero it**. This is the classic ELF loader
    /// bug: copy `filesz` bytes, forget the tail, and hand the program a `.bss` full of whoever
    /// used that frame last. Our loader zeroes every page before copying, so the tail is free,
    /// but only because we thought about it.
    pub data: &'a [u8],
}

impl Segment<'_> {
    /// Whether `PF_R` is set in [`flags`](Self::flags).
    pub fn is_readable(&self) -> bool {
        self.flags & PF_R != 0
    }
    /// Whether `PF_W` is set in [`flags`](Self::flags).
    pub fn is_writable(&self) -> bool {
        self.flags & PF_W != 0
    }
    /// Whether `PF_X` is set in [`flags`](Self::flags).
    pub fn is_executable(&self) -> bool {
        self.flags & PF_X != 0
    }

    /// The page-aligned range this segment touches: `[start, end)`.
    pub fn page_range(&self, page_size: u64) -> (u64, u64) {
        let start = self.vaddr & !(page_size - 1);
        // Saturating, so a hostile `memsz` cannot overflow this even though `Elf::parse` already
        // rejects `vaddr + memsz` overflow (this type is `pub`, so it must be panic-free alone).
        let end = self
            .vaddr
            .saturating_add(self.memsz)
            .div_ceil(page_size)
            .saturating_mul(page_size);
        (start, end)
    }
}
