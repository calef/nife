//! **The seam between the shared walk and a hardware page-table format**, the address-space
//! halves it splits on, and the user-VA gate built from them.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// Which half of the address space a set of tables serves: the low half (user) or the high half
/// (kernel). The bit boundary between them is per-format ([`PageFormat::SPLIT_SHIFT`]); this enum is
/// just the marker, and the format supplies the geometry via [`PageFormat::is_in_half`] and
/// [`PageFormat::half_base`].
///
/// # The thing that is easy to get wrong
///
/// **The top bits of a virtual address are not translated.** They are not part of any index; they
/// select *which set of tables to use* (aarch64 `TTBR0`/`TTBR1`; on RISC-V a single `satp` per
/// address space, with the sign-extended top bit distinguishing the halves). The index is extracted
/// from the lower bits **identically** for both halves, so a low address and its high-half twin are
/// the same entry within a table and differ only in which tables the hardware consults. A
/// higher-half kernel works because it is a *separate set of tables*, not because high addresses
/// index differently. A test discovered this the hard way, which is what host tests are for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Half {
    /// The low half: user space.
    Low,
    /// The high half: the kernel.
    High,
}

/// **The user-VA gate: may a user process ask for a mapping at `va` at all?**
///
/// True exactly when `va` is page-aligned and in the low half for the given format. The syscall
/// layer runs this before spending anything on a user `MAP` request, for two reasons:
///
/// - **Isolation.** An admitted address is in the low half, so the request can only ever walk the
///   process's own tables. No address that passes this gate lands in the kernel's half. (The halves
///   are disjoint; that is proved per format.)
/// - **Budget.** A rejected address is rejected *before* a page is retyped for it.
///
/// # Examples
///
/// A conjunction: the aligned high-half address is what tells `&&` from `||`.
///
/// ```
/// use paging::{Aarch64, PAGE_SIZE, is_user_page_va};
///
/// assert!(is_user_page_va::<Aarch64>(0x40_0000));
/// assert!(!is_user_page_va::<Aarch64>(0x40_0001)); // not page-aligned
/// assert!(!is_user_page_va::<Aarch64>(0xffff_0000_0000_0000)); // aligned, and the kernel's half
/// assert_eq!(PAGE_SIZE, 4096);
/// ```
///
/// Generic over the format so the split lands at the right bit for the running architecture.
pub fn is_user_page_va<F: PageFormat>(va: u64) -> bool {
    F::is_in_half(Half::Low, va) && va.is_multiple_of(PAGE_SIZE)
}

/// **The seam between the shared walk and a hardware page-table format.**
///
/// [`Mapper`] descends the levels, allocates intermediate tables, and writes a leaf, using nothing
/// architecture-specific except what a format supplies here: how many levels, where the halves
/// split, and the four bit operations (is-present, extract-address, encode-a-table-pointer,
/// encode-and-decode-a-leaf). Both hardware formats we support are 512-entry / 4 KiB / 4 KiB, so
/// [`ENTRIES`] and [`PAGE_SIZE`] are shared and not part of the trait.
pub trait PageFormat {
    /// Translation levels. aarch64: 4. Sv39: 3.
    const LEVELS: usize;

    /// The bit position that splits the low (user) half from the high (kernel) half. aarch64: 48
    /// (bits 63:48 select the half). Sv39: 38 (bits 63:38, the sign-extended top VA bit and above).
    const SPLIT_SHIFT: u32;

    /// Is this entry present (valid)?
    fn is_present(entry: u64) -> bool;

    /// The physical address a table-pointer or leaf entry refers to.
    fn entry_pa(entry: u64) -> u64;

    /// Encode an intermediate entry pointing at `pa` (a next-level table). No permissions: a leaf
    /// is the single source of truth for access rights, so intermediate entries carry none.
    ///
    /// `level` is the level of the table this entry is written INTO, counted the way
    /// [`index`](Self::index) counts it (0 = the top), so the table `pa` names is at `level + 1`.
    /// Three of the four formats ignore it. **AMD-Vi cannot**: its directory entry carries a
    /// `NextLevel` field that must name the level of the table it points at, and a wrong value is
    /// not a slightly different encoding but a skipped level (AMD 48882 rev 2.62, section 2.2.3).
    /// Required rather than a second method with a default, for the reason
    /// [`block_entry`](Self::block_entry) gives: a default that dropped the level would let a new
    /// format inherit the wrong encoding without saying so.
    fn table_entry(pa: u64, level: usize) -> u64;

    /// Encode a leaf entry mapping physical `pa` with `flags`.
    fn leaf_entry(pa: u64, flags: Flags) -> u64;

    /// Decode a leaf entry's permission/attribute bits back into portable [`Flags`]. Works on a
    /// block as well as a page: every format keeps its permission bits in the same positions at
    /// every level.
    fn leaf_flags(entry: u64) -> Flags;

    /// **Encode a block**: a leaf at the level `size` names (one level up for 2 MiB, two for
    /// 1 GiB), mapping the naturally aligned span at `pa` with `flags`. `None` when this format has
    /// no leaf of that size, which is how a format declines rather than being mis-encoded.
    ///
    /// Given [`PageSize::Size4KiB`] this is [`leaf_entry`](Self::leaf_entry), so a caller need
    /// not special-case the bottom level.
    ///
    /// **Required, with no default, on purpose**: a default of `None` would let a new format
    /// silently decline blocks, and a default that reused `leaf_entry` would silently write a
    /// table pointer's bit pattern where the hardware expects a block's on aarch64 (`0b11` at L2
    /// is a table; a block is `0b01`). Every format has to say.
    fn block_entry(pa: u64, flags: Flags, size: PageSize) -> Option<u64>;

    /// **Is this present entry, found above the bottom level, a block rather than a table
    /// pointer?** Only ever asked of an entry above the bottom level: at the bottom every present
    /// entry is a page, and on x86 the bit this reads there means something else entirely (PAT).
    fn is_block(entry: u64) -> bool;

    /// Which 9-bit slice of `va` selects an entry at `level` (0 = the top level). The top level
    /// covers the highest translated bits; each lower level shifts down by 9. Derived from
    /// [`LEVELS`](Self::LEVELS) so it is right for both a 4- and a 3-level walk.
    fn index(va: u64, level: usize) -> usize {
        let shift = 9 * (Self::LEVELS - 1 - level) as u32 + 12;
        ((va >> shift) & 0x1ff) as usize
    }

    /// Does `va` lie in `half` for this format? The top bits (above [`SPLIT_SHIFT`](Self::SPLIT_SHIFT))
    /// must be all-zero for the low half or all-one for the high half; anything between is
    /// non-canonical and faults.
    fn is_in_half(half: Half, va: u64) -> bool {
        let top = va >> Self::SPLIT_SHIFT;
        match half {
            Half::Low => top == 0,
            Half::High => top == (u64::MAX >> Self::SPLIT_SHIFT),
        }
    }

    /// The base virtual address of `half` (0 for the low half; the sign-extended high base for the
    /// high half). Exists to make a reader's mental model explicit; the walk does not use it.
    fn half_base(half: Half) -> u64 {
        match half {
            Half::Low => 0,
            Half::High => (u64::MAX >> Self::SPLIT_SHIFT) << Self::SPLIT_SHIFT,
        }
    }
}
