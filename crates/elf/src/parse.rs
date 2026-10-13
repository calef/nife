//! **The parse**: the ELF64 header and program-header table, checked before anything is loaded,
//! and the little-endian reads it is built from.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// `\x7fELF`.
pub(crate) const MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

pub(crate) const ELFCLASS64: u8 = 2;

pub(crate) const ELFDATA2LSB: u8 = 1;

pub(crate) const EV_CURRENT: u8 = 1;

/// `e_type`. We accept only this one: a **static executable**, loaded where it says.
pub(crate) const ET_EXEC: u16 = 2;

/// `e_type` for a PIE / shared object. Needs relocation, which we do not do.
pub(crate) const ET_DYN: u16 = 3;

/// 64 bytes of ELF64 header, then program headers of 56 bytes each.
pub(crate) const EHDR_SIZE: usize = 64;

pub(crate) const PHDR_SIZE: usize = 56;

/// The most program headers a file may have. Bounds the header count before the O(n^2) overlap
/// check: a legitimate static executable has a few `PT_LOAD` segments, and 65535 headers exist only
/// to make validation stall. The note reader walks the same bound.
pub(crate) const MAX_PHNUM: usize = 64;

/// A parsed, **fully validated** ELF64 executable.
///
/// Everything is checked in [`Elf::parse`], not lazily while iterating. A loader that validates
/// as it maps has already mapped half a bad program by the time it finds out.
pub struct Elf<'a> {
    pub(crate) bytes: &'a [u8],
    entry: u64,
    phoff: usize,
    phnum: usize,
    phentsize: usize,
}

impl<'a> Elf<'a> {
    /// Validate the whole file up front: header, program header table bounds, and every segment's
    /// bounds, permissions and address arithmetic. A `Ok(Elf)` has nothing left to check; every
    /// later accessor and [`segments`](Self::segments) iteration step trusts this pass completely.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() < EHDR_SIZE {
            return Err(Error::TooSmall);
        }

        if bytes[0..4] != MAGIC {
            return Err(Error::BadMagic);
        }
        if bytes[4] != ELFCLASS64 {
            return Err(Error::Not64Bit);
        }
        if bytes[5] != ELFDATA2LSB {
            return Err(Error::NotLittleEndian);
        }
        if bytes[6] != EV_CURRENT {
            return Err(Error::BadVersion);
        }

        let e_type = u16le(bytes, 16);
        let e_machine = u16le(bytes, 18);

        if e_machine != EXPECTED_MACHINE {
            return Err(Error::WrongMachine);
        }
        match e_type {
            ET_EXEC => {}
            ET_DYN => return Err(Error::NeedsRelocation),
            _ => return Err(Error::NotExecutable),
        }

        let entry = u64le(bytes, 24);
        let phoff = u64le(bytes, 32) as usize;
        let phentsize = u16le(bytes, 54) as usize;
        let phnum = u16le(bytes, 56) as usize;

        if phentsize < PHDR_SIZE {
            return Err(Error::BadProgramHeaders);
        }
        if phnum > MAX_PHNUM {
            return Err(Error::TooManyProgramHeaders);
        }

        // The bounds check on the program header table itself. `phoff` and `phnum` come out of
        // the file, so they are hostile input, and `phoff + phnum * phentsize` is exactly the
        // kind of arithmetic that wraps.
        let table_len = phnum
            .checked_mul(phentsize)
            .ok_or(Error::BadProgramHeaders)?;
        let table_end = phoff
            .checked_add(table_len)
            .ok_or(Error::BadProgramHeaders)?;
        if table_end > bytes.len() {
            return Err(Error::BadProgramHeaders);
        }

        let elf = Elf {
            bytes,
            entry,
            phoff,
            phnum,
            phentsize,
        };

        elf.validate()?;
        Ok(elf)
    }

    /// Every check, before the caller maps a single page.
    fn validate(&self) -> Result<(), Error> {
        let mut entry_ok = false;

        for i in 0..self.phnum {
            let Some(seg) = self.segment_at(i)? else {
                continue;
            };

            if seg.is_writable() && seg.is_executable() {
                return Err(Error::WritableAndExecutable);
            }
            if !seg.is_readable() && !seg.is_executable() {
                return Err(Error::SegmentUnreachable);
            }

            if seg.is_executable()
                && (self.entry >= seg.vaddr && self.entry < seg.vaddr + seg.memsz)
            {
                entry_ok = true;
            }

            // No two segments may claim the same page. See `Error::SegmentsOverlap`.
            for j in 0..i {
                if let Some(other) = self.segment_at(j)? {
                    let (a0, a1) = seg.page_range(4096);
                    let (b0, b1) = other.page_range(4096);
                    if a0 < b1 && b0 < a1 {
                        return Err(Error::SegmentsOverlap);
                    }
                }
            }
        }

        if !entry_ok {
            return Err(Error::EntryNotExecutable);
        }
        Ok(())
    }

    /// The `i`th program header, if it is a `PT_LOAD`.
    fn segment_at(&self, i: usize) -> Result<Option<Segment<'a>>, Error> {
        let off = self.phoff + i * self.phentsize;
        let ph = &self.bytes[off..off + PHDR_SIZE];

        if u32le(ph, 0) != PT_LOAD {
            return Ok(None);
        }

        let flags = u32le(ph, 4);
        let p_offset = u64le(ph, 8) as usize;
        let vaddr = u64le(ph, 16);
        let paddr = u64le(ph, 24);
        let filesz = u64le(ph, 32) as usize;
        let memsz = u64le(ph, 40);

        // All the arithmetic a hostile program header can weaponize (offset + filesz within the
        // file, vaddr + memsz without overflow) lives in `check_segment_bounds`, factored out as a
        // pure, loopless function so it can be proved panic-free on its own. See the verification
        // module and notes/verification.md.
        let end = check_segment_bounds(self.bytes.len(), p_offset, filesz, vaddr, memsz)?;

        Ok(Some(Segment {
            vaddr,
            paddr,
            memsz,
            flags,
            data: &self.bytes[p_offset..end],
        }))
    }

    /// Where execution begins. Validated to be inside an executable segment.
    pub fn entry(&self) -> u64 {
        self.entry
    }

    /// The segments to map, in file order.
    pub fn segments(&self) -> impl Iterator<Item = Segment<'a>> + '_ {
        (0..self.phnum).filter_map(|i| self.segment_at(i).ok().flatten())
    }
}

/// The per-segment bounds and overflow checks, factored out of [`Elf::segment_at`] as pure
/// arithmetic over a program header's raw fields and the file length. No slicing and no loop, which
/// is what lets the verification module prove it panic-free, and its result in-bounds, for *every*
/// field combination. The whole-parse proof cannot reach this (the header loop and symbolic slice
/// offsets defeat bounded model checking); this is the decomposition that can. See
/// notes/verification.md.
///
/// Returns the exclusive end offset of the segment's file data, so `[p_offset, end)` lies within a
/// `file_len`-byte file, or the error the fields earn. Checks run in the same order as before the
/// extraction, so the errors are identical and the tests are unchanged.
pub(crate) fn check_segment_bounds(
    file_len: usize,
    p_offset: usize,
    filesz: usize,
    vaddr: u64,
    memsz: u64,
) -> Result<usize, Error> {
    // The segment cannot claim less memory than it has bytes on disk.
    if (memsz as usize) < filesz {
        return Err(Error::SegmentTruncated);
    }

    // The file range must lie within the file. `p_offset` and `filesz` are hostile, so the add is
    // checked rather than trusted.
    let end = p_offset
        .checked_add(filesz)
        .ok_or(Error::SegmentOutOfBounds)?;
    if end > file_len {
        return Err(Error::SegmentOutOfBounds);
    }

    // The virtual range must not overflow: `vaddr + memsz` feeds the entry check and `page_range`,
    // and a near-`u64::MAX` memsz would wrap them, a PANIC under the dev profile's overflow checks,
    // i.e. a crafted binary halting the kernel.
    vaddr.checked_add(memsz).ok_or(Error::AddressOverflow)?;

    Ok(end)
}

pub(crate) fn u16le(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

pub(crate) fn u32le(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

pub(crate) fn u64le(b: &[u8], at: usize) -> u64 {
    let mut v = [0u8; 8];
    v.copy_from_slice(&b[at..at + 8]);
    u64::from_le_bytes(v)
}
