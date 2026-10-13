//! **The shared walk**: descending a format's levels, allocating intermediate tables and writing
//! a leaf, and why a mapping was refused.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// Why [`Mapper::map`] or [`Mapper::unmap`] refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapError {
    /// Ran out of frames while building intermediate tables.
    OutOfPageFrames,
    /// The virtual or physical address is not 4 KiB aligned.
    Misaligned,
    /// Something is already mapped here.
    ///
    /// **Load-bearing, not defensive.** Refusing to overwrite is what forces break-before-make: to
    /// change a mapping you must `unmap` (which hands you a [`TlbFlush`]) and then `map`. Silently
    /// overwriting would go valid → valid (which the hardware may reject) *and* leak the old frame.
    AlreadyMapped,
    /// This address belongs to the *other* half, or is non-canonical. Mapping a kernel address into
    /// the user tables would silently do nothing: the hardware would never consult this table for
    /// that address. Catching it here turns a mystery into an error.
    WrongHalf,
    /// Nothing is mapped at this address, so there is nothing to unmap.
    NotMapped,
    /// **This page is part of a block**, so it cannot be unmapped on its own: `unmap` takes 4 KiB
    /// and a block is 2 MiB or 1 GiB of one entry. Splitting the block would be the way to honour
    /// the request, and nothing here does that (see the module header). Name provisional.
    InsideBlock,
    /// The format has no leaf of the size asked for ([`PageFormat::block_entry`] declined). Name
    /// provisional.
    UnsupportedSize,
    /// **A region that is not a region**: its `base + size` wraps `u64`, so it has no end. Only
    /// [`domain::grant_pages`] raises this, and only for an input no caller can construct today; it
    /// exists because refusing such a region is what stops the page enumeration from wrapping into
    /// addresses *outside* the grant. See [`domain::grant_pages`] for the whole argument.
    BadRegion,
}

/// Builds page tables, for any [`PageFormat`].
///
/// # Safety contract
///
/// The mapper dereferences physical addresses directly. That is sound in exactly two situations,
/// both of which apply to us:
///
/// - **the MMU is off**, so an address *is* a physical address (how we build the first table), or
/// - **physical memory is identity- or direct-mapped**, so a physical address can be turned into a
///   usable pointer by a known transform (how we edit tables afterwards).
///
/// `phys_to_ptr` is that transform. The host tests pass the identity function, and it works because
/// a host allocation's address is as good a "physical address" as any: the pointer arithmetic is
/// identical.
pub struct Mapper<A, P, F>
where
    A: FnMut() -> Option<u64>,
    P: Fn(u64) -> *mut PageTable,
    F: PageFormat,
{
    root: u64,
    half: Half,
    alloc_page_frame: A,
    phys_to_ptr: P,
    _fmt: PhantomData<F>,
}

impl<A, P, F> Mapper<A, P, F>
where
    A: FnMut() -> Option<u64>,
    P: Fn(u64) -> *mut PageTable,
    F: PageFormat,
{
    /// # Safety
    /// `root` must be a zeroed, page-aligned frame, and `phys_to_ptr` must satisfy the contract in
    /// the type's docs.
    pub unsafe fn new(root: u64, half: Half, alloc_page_frame: A, phys_to_ptr: P) -> Self {
        Self {
            root,
            half,
            alloc_page_frame,
            phys_to_ptr,
            _fmt: PhantomData,
        }
    }

    /// The physical address of the top-level table.
    pub fn root(&self) -> u64 {
        self.root
    }

    /// Which half of the address space this mapper builds into.
    pub fn half(&self) -> Half {
        self.half
    }

    /// Map one 4 KiB page. Walks the levels, creating tables as needed, and writes a leaf at the
    /// bottom.
    pub fn map(&mut self, va: u64, pa: u64, flags: Flags) -> Result<(), MapError> {
        // The hardware selects the table set from the top bits before it touches an index, so
        // mapping a high address into the low tables would build a mapping the CPU never consults.
        if !F::is_in_half(self.half, va) {
            return Err(MapError::WrongHalf);
        }

        if !va.is_multiple_of(PAGE_SIZE) || !pa.is_multiple_of(PAGE_SIZE) {
            return Err(MapError::Misaligned);
        }

        let mut table_pa = self.root;

        // Descend through every level but the last, creating intermediate tables as we go.
        for level in 0..F::LEVELS - 1 {
            let i = F::index(va, level);

            // SAFETY: `table_pa` is a page-aligned table, per the type's contract.
            let entry = unsafe { &mut (*(self.phys_to_ptr)(table_pa)).entries[i] };

            if !F::is_present(*entry) {
                let new = (self.alloc_page_frame)().ok_or(MapError::OutOfPageFrames)?;

                // SAFETY: a fresh frame from the allocator. Zero it before it becomes reachable by
                // the hardware, or the walk reads whatever garbage was in RAM and follows it.
                unsafe {
                    (*(self.phys_to_ptr)(new)).entries = [0; ENTRIES];
                }

                *entry = F::table_entry(new, level);
            } else if F::is_block(*entry) {
                // A block already maps this address. Descending would take the block's frame for
                // a page table and write a leaf into somebody's data.
                return Err(MapError::AlreadyMapped);
            }

            table_pa = F::entry_pa(*entry);
        }

        // The leaf.
        let i = F::index(va, F::LEVELS - 1);
        // SAFETY: as above.
        let entry = unsafe { &mut (*(self.phys_to_ptr)(table_pa)).entries[i] };

        if F::is_present(*entry) {
            return Err(MapError::AlreadyMapped);
        }

        *entry = F::leaf_entry(pa, flags);

        Ok(())
    }

    /// **Map one leaf of `size`**: a 4 KiB page (exactly [`map`](Self::map)), or a 2 MiB or 1 GiB
    /// block written one or two levels up, with the tables above it created as needed.
    ///
    /// Refuses a misaligned `va` or `pa` (both must be multiples of `size`), a size the format
    /// cannot encode ([`MapError::UnsupportedSize`]), and anything already present at or above the
    /// block's slot, **including an empty table**: taking over a table slot would orphan the table
    /// frame, and the mapper does not own frames to free them.
    pub fn map_block(
        &mut self,
        va: u64,
        pa: u64,
        size: PageSize,
        flags: Flags,
    ) -> Result<(), MapError> {
        if size == PageSize::Size4KiB {
            return self.map(va, pa, flags);
        }
        if !F::is_in_half(self.half, va) {
            return Err(MapError::WrongHalf);
        }
        let bytes = size.bytes();
        if !va.is_multiple_of(bytes) || !pa.is_multiple_of(bytes) {
            return Err(MapError::Misaligned);
        }
        let leaf = F::block_entry(pa, flags, size).ok_or(MapError::UnsupportedSize)?;
        // Every CPU format here has at least three levels, so a 1 GiB block (two up) always has a
        // level to sit at; a format with fewer would have declined above.
        let leaf_level = F::LEVELS - 1 - size.levels_above_bottom();

        let mut table_pa = self.root;
        for level in 0..leaf_level {
            let i = F::index(va, level);
            // SAFETY: `table_pa` is a page-aligned table, per the type's contract.
            let entry = unsafe { &mut (*(self.phys_to_ptr)(table_pa)).entries[i] };
            if !F::is_present(*entry) {
                let new = (self.alloc_page_frame)().ok_or(MapError::OutOfPageFrames)?;
                // SAFETY: a fresh frame, zeroed before it becomes reachable (see `map`).
                unsafe {
                    (*(self.phys_to_ptr)(new)).entries = [0; ENTRIES];
                }
                *entry = F::table_entry(new, level);
            } else if F::is_block(*entry) {
                return Err(MapError::AlreadyMapped);
            }
            table_pa = F::entry_pa(*entry);
        }

        let i = F::index(va, leaf_level);
        // SAFETY: as above.
        let entry = unsafe { &mut (*(self.phys_to_ptr)(table_pa)).entries[i] };
        if F::is_present(*entry) {
            return Err(MapError::AlreadyMapped);
        }
        *entry = leaf;
        Ok(())
    }

    /// **Map `len` bytes from `va` to `pa` in the largest leaves that fit**, none larger than
    /// `largest`. The direct map's shape: a 4 KiB head up to the first 2 MiB boundary, blocks
    /// through the middle, a 4 KiB tail.
    ///
    /// A block is used only where it lies wholly inside the span and both addresses are aligned to
    /// it ([`PageSize::largest_fitting`], which is where that is proved), so this maps exactly the
    /// pages [`map_range`](Self::map_range) would, in fewer entries. `largest` is the caller's
    /// because only the caller knows what the running machine supports: x86 has 1 GiB leaves only
    /// where `CPUID` says so, and a format cannot ask.
    ///
    /// `va`, `pa` and `len` must be page-aligned. Not atomic: on an error, what was mapped before
    /// it stays mapped, exactly as with `map_range`.
    pub fn map_span(
        &mut self,
        va: u64,
        pa: u64,
        len: u64,
        flags: Flags,
        largest: PageSize,
    ) -> Result<(), MapError> {
        if !len.is_multiple_of(PAGE_SIZE) {
            return Err(MapError::Misaligned);
        }
        let mut done = 0;
        while done < len {
            let size = PageSize::largest_fitting(va + done, pa + done, len - done, largest);
            self.map_block(va + done, pa + done, size, flags)?;
            done += size.bytes();
        }
        Ok(())
    }

    /// Map `count` consecutive pages.
    pub fn map_range(
        &mut self,
        va: u64,
        pa: u64,
        count: u64,
        flags: Flags,
    ) -> Result<(), MapError> {
        for i in 0..count {
            self.map(va + i * PAGE_SIZE, pa + i * PAGE_SIZE, flags)?;
        }
        Ok(())
    }

    /// Remove a mapping, and return the physical frame it pointed at.
    ///
    /// The frame is returned rather than freed, because the mapper does not own it: the caller took
    /// it from the frame allocator and must give it back. This clears only the leaf and leaves the
    /// intermediate tables standing, on purpose: break-before-make unmaps and immediately remaps the
    /// same address, so freeing the tables here would only reallocate them a line later. Address-
    /// space teardown does not use `unmap` at all (see notes/teardown.md); it frees the recorded
    /// frame set wholesale.
    ///
    /// Returns a [`TlbFlush`] you cannot ignore.
    pub fn unmap(&mut self, va: u64) -> Result<(u64, TlbFlush), MapError> {
        if !F::is_in_half(self.half, va) {
            return Err(MapError::WrongHalf);
        }
        if !va.is_multiple_of(PAGE_SIZE) {
            return Err(MapError::Misaligned);
        }

        let mut table_pa = self.root;

        for level in 0..F::LEVELS - 1 {
            let i = F::index(va, level);
            // SAFETY: per the type's contract.
            let entry = unsafe { (*(self.phys_to_ptr)(table_pa)).entries[i] };
            if !F::is_present(entry) {
                return Err(MapError::NotMapped);
            }
            if F::is_block(entry) {
                return Err(MapError::InsideBlock);
            }
            table_pa = F::entry_pa(entry);
        }

        let i = F::index(va, F::LEVELS - 1);
        // SAFETY: per the type's contract.
        let entry = unsafe { &mut (*(self.phys_to_ptr)(table_pa)).entries[i] };

        if !F::is_present(*entry) {
            return Err(MapError::NotMapped);
        }

        let pa = F::entry_pa(*entry);

        // Break: the entry becomes invalid *before* anything else happens.
        *entry = 0;

        Ok((pa, TlbFlush { va }))
    }

    /// **Cut the intermediate table at physical `table` out of the walk that reaches `va`**, and
    /// say what span of addresses went with it. Name provisional (the page-tables-outlive-destroy
    /// lane, 2026-10-05 UTC).
    ///
    /// The walk descends from the root along `va`; the first present table entry that points at
    /// `table` is cleared, and the answer is `(base, span)`: the naturally aligned range of virtual
    /// addresses that `table` translated, all of which now translate to nothing. `None` when the
    /// walk to `va` never passes through `table` (it was never linked there, or something above it
    /// has already been cut), which is not an error: a caller cutting a set of tables top-down
    /// meets exactly that for every table below the first.
    ///
    /// **Why a table, and not every leaf beneath it.** The kernel uses this when the memory a table
    /// lives in is about to be handed back (`revoke::revoke_region`). Clearing the leaves would leave
    /// the table itself still linked, and the next owner of that page would be writing this space's
    /// translations. Cutting the one entry that points at it is the only change that makes the page
    /// unreachable, and it takes the whole subtree with it in one store.
    ///
    /// Nothing is freed: the table is the caller's (it came from the allocator the caller passed),
    /// and so is everything below it. The root is never a candidate, since no entry points at it.
    ///
    /// The [`TlbFlush`] covers the whole span and every level of cached walk, not one page: its
    /// [`address`](TlbFlush::address) is `base`, and the caller must discharge it with an
    /// invalidation that reaches non-leaf entries for the whole range (a by-ASID flush does).
    pub fn unlink_table(&mut self, va: u64, table: u64) -> Option<(u64, u64, TlbFlush)> {
        if !F::is_in_half(self.half, va) {
            return None;
        }
        let mut table_pa = self.root;
        for level in 0..F::LEVELS - 1 {
            let i = F::index(va, level);
            // SAFETY: `table_pa` is a page-aligned table, per the type's contract.
            let entry = unsafe { &mut (*(self.phys_to_ptr)(table_pa)).entries[i] };
            if !F::is_present(*entry) || F::is_block(*entry) {
                return None;
            }
            if F::entry_pa(*entry) == table {
                // The table one level down translates the span one entry at `level` covers.
                let span = PAGE_SIZE << (9 * (F::LEVELS - 1 - level));
                let base = va & !(span - 1);
                // Break: no walk can reach `table` from here after this store.
                *entry = 0;
                return Some((base, span, TlbFlush { va: base }));
            }
            table_pa = F::entry_pa(*entry);
        }
        None
    }

    /// Walk the tables and report what a virtual address actually maps to. This is what the hardware
    /// does on every access, in silicon, and it is worth having in software: it is the only way to
    /// *check* that the tables say what you think they say.
    pub fn translate(&self, va: u64) -> Option<(u64, Flags)> {
        if !F::is_in_half(self.half, va) {
            return None;
        }

        let mut table_pa = self.root;

        for level in 0..F::LEVELS - 1 {
            let i = F::index(va, level);
            // SAFETY: per the type's contract.
            let entry = unsafe { (*(self.phys_to_ptr)(table_pa)).entries[i] };

            if !F::is_present(entry) {
                return None;
            }
            if F::is_block(entry) {
                // The walk ends early: this entry maps the whole span it covers. The span is
                // what the remaining index bits and the page offset address, together.
                let span = PAGE_SIZE << (9 * (F::LEVELS - 1 - level));
                let base = F::entry_pa(entry) & !(span - 1);
                return Some((base + (va & (span - 1)), F::leaf_flags(entry)));
            }
            table_pa = F::entry_pa(entry);
        }

        let i = F::index(va, F::LEVELS - 1);
        // SAFETY: per the type's contract.
        let entry = unsafe { (*(self.phys_to_ptr)(table_pa)).entries[i] };

        if !F::is_present(entry) {
            return None;
        }

        let offset = va % PAGE_SIZE;
        Some((F::entry_pa(entry) + offset, F::leaf_flags(entry)))
    }
}
