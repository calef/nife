//! **One 4 KiB table of 512 entries**, the unit every format's walk descends through.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

/// Entries per table. 4096 bytes / 8 bytes. The same for every format we support.
pub const ENTRIES: usize = 512;

/// A page table: one 4 KiB frame of entries.
///
/// `repr(C, align(4096))` matters. Every format takes the entry's high bits as the next table's
/// address and assumes the low 12 are zero, so a table must be page-aligned. This alignment is also
/// what lets the host tests allocate real, correctly-aligned tables.
#[repr(C, align(4096))]
#[derive(Clone)]
pub struct PageTable {
    /// The raw entries, in the format's own encoding. Decode with the `PageFormat` in use rather
    /// than reading these directly.
    pub entries: [u64; ENTRIES],
}

impl Default for PageTable {
    fn default() -> Self {
        Self::new()
    }
}

impl PageTable {
    /// A table of all-zero entries: every slot not-present.
    pub const fn new() -> Self {
        Self {
            entries: [0; ENTRIES],
        }
    }
}
