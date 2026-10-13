//! **The GUID Partition Table**: reading one, writing one, and refusing a bad one.
//!
//! (The three letters are the disk format from UEFI 2.10 §5.3, which predates the machine-learning
//! sense of them by about fifteen years. There are no language models in here.)
//!
//! A partition table is the map that says "the filesystem starts at block 2048 and stops at block
//! 32767". Milestone 57 needs it in both directions: **reading** one is how a kernel finds a
//! partition on a disk it has never seen, and there is no way around that; **writing** one is the
//! `parted mkpart` equivalent, the first of the three steps in calef's router setup that this OS
//! has no answer to.
//!
//! # This crate does no I/O, on purpose
//!
//! Nothing here reads or writes a block device. Every function takes a byte buffer the caller
//! filled and returns bytes the caller will place. That is the same discipline `device_tree_blob`
//! and `elf` follow, and the reason is not purity: it is that the whole crate then compiles for the
//! host and its tests run in milliseconds against real disks made by real tools, instead of inside
//! a QEMU boot (DECISIONS §7, §14).
//!
//! It also allocates nothing and has no `unsafe`. The entry array is a caller-supplied buffer, so a
//! kernel can hand it a stack array and a userspace tool can hand it a `Vec`.
//!
//! [`span`] does not break that rule and is worth saying why: it computes *where* to read, and
//! arithmetic between two block sizes is exactly the kind of thing this crate is for. A GPT counts
//! in 512-byte logical blocks; the block service a nife program holds moves 4096 bytes per
//! request. Somebody has to reconcile those, and doing it in a driver means doing it three times.
//!
//! # What a GPT actually is, in the order the bytes appear
//!
//! ```text
//!   LBA 0     protective MBR   one fake "the whole disk is in use" record, type 0xEE
//!   LBA 1     primary header   92 bytes: signature, geometry, two CRC32s, disk GUID
//!   LBA 2..   entry array      128 entries x 128 bytes = 16 KiB = 32 blocks
//!   ...       the partitions themselves, between first_usable_lba and last_usable_lba
//!   LBA n-33  backup array     a second copy of the same 16 KiB
//!   LBA n-1   backup header    the same header, with my_lba and alternate_lba swapped
//! ```
//!
//! Four things carry the integrity of that: the header CRC and the array CRC, each stored twice.
//! **A GPT is a format that can tell you it is broken**, which is exactly why every check in here
//! is an error and none of them is a warning. A table that half-validates is a table you are about
//! to write a filesystem on top of.
//!
//! # The layering, and why the errors are where they are
//!
//! - [`header::Header::decode`] judges one block on its own terms: is this a GPT header at all?
//! - [`entry::Entry::decode`] judges nothing. It is **total**, every bit pattern decodes, and the
//!   round trip is the identity. There is no such thing as a malformed entry, only an entry that
//!   makes no sense on a particular disk.
//! - [`GloballyUniqueIdentifierPartitionTable::parse`] is where the disk shows up, and therefore
//!   where every geometry rule lives: the array CRC, the usable range, partitions that run off the
//!   end, partitions that overlap.
//! - [`GloballyUniqueIdentifierPartitionTable::check_backup`] compares the two copies. A disk whose
//!   halves disagree has had something happen to it and is not to be trusted.
//!
//! # Using it
//!
//! Reading, given a way to fetch blocks:
//!
//! ```
//! # use globally_unique_identifier_partition_table::GloballyUniqueIdentifierPartitionTable;
//! # use globally_unique_identifier_partition_table::guid::types;
//! # let disk = globally_unique_identifier_partition_table::testing::sample_disk();
//! # let (header_block, entry_array) = (&disk[512..1024], &disk[1024..1024 + 16384]);
//! let table = GloballyUniqueIdentifierPartitionTable::parse(header_block, entry_array)?;
//! for (index, part) in table.partitions() {
//!     let mut label = [0u8; 4 * globally_unique_identifier_partition_table::entry::NAME_UNITS];
//!     let n = part.name_utf8(&mut label)?;
//!     println!(
//!         "{}: {} blocks of {}, {:?}",
//!         index + 1,
//!         part.blocks().unwrap(),
//!         types::name(part.type_guid).unwrap_or("unknown"),
//!         core::str::from_utf8(&label[..n]).unwrap(),
//!     );
//! }
//! # Ok::<(), globally_unique_identifier_partition_table::Error>(())
//! ```
//!
//! Writing, which is four block ranges the caller then puts on the disk:
//!
//! ```
//! # use globally_unique_identifier_partition_table::Entry;
//! # use globally_unique_identifier_partition_table::GloballyUniqueIdentifierPartitionTable;
//! # use globally_unique_identifier_partition_table::guid::types;
//! # use universally_unique_identifier::Uuid;
//! # let disk_guid = Uuid::NIL;
//! # let part_guid = Uuid::NIL;
//! let mut array = [0u8; globally_unique_identifier_partition_table::ENTRY_ARRAY_BYTES];
//! let table = GloballyUniqueIdentifierPartitionTable::create(
//!     disk_guid,
//!     512,
//!     131_072, // a 64 MiB disk
//!     &[Entry::new(types::NIFE_DATA, part_guid, 2048, 131_038).with_name("nife data")?],
//!     &mut array,
//! )?;
//!
//! let mut block = [0u8; 512];
//! table.write_protective_mbr(&mut block)?; // to LBA 0
//! table.write_primary_header(&mut block)?; // to LBA 1
//! table.write_backup_header(&mut block)?;  // to table.backup_header_lba()
//! let bytes = table.entry_array();          // to LBA 2 and to table.backup_entry_lba()
//! # Ok::<(), globally_unique_identifier_partition_table::Error>(())
//! ```
//!
//! See notes/globally-unique-identifier-partition-table.md for the format walk-through and the
//! surprises the fixtures pinned.
//!
//! Name: ratified 2026-09-18 (calef, `design/decisions/` §154), **deratifying the 2026-08-01
//! ratification** to do it, and performed on 2026-09-18. Refused `gpt` and
//! `guid_partition_table`, the one-level spelling; §154 asks the question again of `GUID`.
//!
//! §154's test is whether the expansion is a phrase people actually say. "GUID partition table" is,
//! so it goes, where `pci` stays because "peripheral component interconnect" is not. It expands
//! **all the way**, because §154 asks the same question of an acronym left inside an expansion and
//! "globally unique identifier" is spoken too.
//!
//! The 2026-08-01 block called this one of the standard terms "already right and must not be
//! touched". That was an exemption rather than a test, and §154 records the three-layer
//! contradiction the exemptions left behind.
//!
//! **Known cost**: datasheets, error messages and this kernel's own output say `gpt`, so a reader
//! grepping the word the machine printed will not find this identifier. Weighed and accepted.
//!
//! **What the rename did not move, and why.** 636 occurrences of the three letters before, 421
//! after, not counting this paragraph. Of the 421: 136 are UEFI's own vocabulary (`GPT`, the GUID
//! Partition Table, the GPT header), 130 are the `Gpt` type, which the ruling did not name, 31 are
//! the test disk image and the `xtask` helpers that build it (`nifefs-gpt.img`, `mkgptdisk`), 13
//! are the fuzz target `gpt_table`, 10 are `gptfdisk`, somebody else's tool, 30 sit in
//! `design/decisions/`, 3 in bench transcripts and photographs, and 68 are the old name inside an
//! account, a measurement or a refusal. The `Gpt` type and the fuzz target are the two a later
//! ruling might still move; both name the format rather than this crate, which is why they stayed.
//! A later ruling did move both (calef, 2026-09-19: a crate's own public type named for the
//! acronym, and a fuzz target named for the crate, follow the crate). The type is
//! `GloballyUniqueIdentifierPartitionTable` and the fuzz target
//! `globally_unique_identifier_partition_table`; the counts above were taken before that.

#![no_std]
// milestone 68's ratchet is workspace-wide (§107); this crate opts out until its 23-item
// worklist (notes/doc-coverage.md) is burned down.
#![allow(missing_docs)]

pub mod crc;
pub mod entry;
pub mod guid;
pub mod header;
pub mod span;

use crc::crc32;
pub use entry::Entry;
pub use header::Header;
use universally_unique_identifier::Uuid;

/// The smallest logical block a GPT disk can have, and what almost every disk reports.
pub const MIN_BLOCK_SIZE: usize = 512;

/// The largest logical block this crate handles. 4096 is what a native-4K drive reports, and the
/// header still occupies exactly one block, so the only thing that changes is the LBA arithmetic.
pub const MAX_BLOCK_SIZE: usize = 4096;

/// The primary header always lives at LBA 1, immediately after the protective MBR. Not a
/// convention: the spec fixes it, because it is the one address a reader can know before it has
/// read anything.
pub const PRIMARY_HEADER_LBA: u64 = 1;

/// Entries in a table written by [`GloballyUniqueIdentifierPartitionTable::create`] with a
/// [`ENTRY_ARRAY_BYTES`]-sized buffer, and what every tool in the world writes.
pub const DEFAULT_ENTRY_COUNT: u32 = 128;

/// 16 KiB: `DEFAULT_ENTRY_COUNT * entry::SIZE`, and the minimum array size UEFI 2.10 §5.3.3
/// requires of a *conformant writer*.
///
/// [`GloballyUniqueIdentifierPartitionTable::create`] will build a smaller array if handed a
/// smaller buffer, and [`GloballyUniqueIdentifierPartitionTable::parse`] will read one, because
/// refusing a table we can read perfectly well would be the wrong kind of strict. Anything writing
/// a real disk should pass a buffer of exactly this size.
pub const ENTRY_ARRAY_BYTES: usize = DEFAULT_ENTRY_COUNT as usize * entry::SIZE;

/// True if `len` is a logical block size this crate accepts: a power of two from
/// [`MIN_BLOCK_SIZE`] to [`MAX_BLOCK_SIZE`].
///
/// Block size is never a parameter in this crate's API. A caller passes the block it read, and the
/// slice's own length is the block size, which removes a whole class of "we told it 512 and handed
/// it 4096" bug.
pub const fn is_block_size_ok(len: usize) -> bool {
    len >= MIN_BLOCK_SIZE && len <= MAX_BLOCK_SIZE && len.is_power_of_two()
}

/// The protective MBR at LBA 0.
///
/// # Why a GPT disk starts with a lie
///
/// The first block of a GPT disk holds a classic MBR partition table describing **one** partition,
/// of type `0xEE`, covering the whole disk. Nothing ever mounts it. It exists so that a tool old
/// enough to not know about GPT sees a disk that is entirely spoken for, and therefore does not
/// helpfully offer to initialise it. The failure mode it defends against is a partition editor from
/// 2001 deciding a 4 TB disk is empty.
///
/// It is genuinely a *protective* device rather than a description, which is why the checks here
/// are about the two fields that do the protecting (start at LBA 1, cover the disk) and not about
/// the CHS fields, which are vestigial: `sgdisk` writes the real cylinder/head/sector of the last
/// block, macOS writes `FE FF FF`, and both are correct because nothing reads them.
pub mod mbr;

/// Sample tables, for this crate's own doc examples and for anything downstream that wants a GPT
/// without a disk.
///
/// Not behind `#[cfg(test)]`: rustdoc examples compile against the public API, so a helper only the
/// tests can see is a helper the documentation cannot use.
pub mod testing;

// =================================================================================================
// Machine-checked proofs (Kani). Behind `#[cfg(kani)]`, so an ordinary build or test never sees
// them; only `cargo kani` (script/verify) sets that cfg. See notes/verification.md and
// notes/globally-unique-identifier-partition-table.md.
//
// The division of labour with the tests is deliberate and is the one `network_time_protocol` argued for:
// **where a domain is small enough to count, the tests count it, exhaustively, and that is a
// stronger result than any bounded solver run.** Every single-byte corruption of a real 512-byte
// header block is enumerated in tests/real_disks.rs; every value of every byte of a small table is
// enumerated in tests/table.rs.
//
// What enumeration cannot do is quantify over the *table*. Those tests say "every corruption of
// this table is caught"; the harnesses below say "for every buffer there is, a changed byte changes
// the CRC", which is the general fact the tests are instances of. That, and the round trips over
// 2^1024 entries and 2^128 GUIDs, is what a model checker is for.
// =================================================================================================
#[cfg(kani)]
mod verification;

mod error;
mod table;

pub use error::{BackupMismatch, Error, MbrProblem};
pub use table::GloballyUniqueIdentifierPartitionTable;
