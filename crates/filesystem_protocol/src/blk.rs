use super::PAGE;

/// Read [`req_blocks`] filesystem blocks starting at `w1` from the disk into the shared
/// region. Reply `r0` = 0 on success, or [`super::reply_err`] on failure; the bytes land at the
/// start of the shared region, block `w1` first.
pub const READ: u64 = 1;
/// Write [`req_blocks`] filesystem blocks, now in the shared region, to the disk starting at
/// `w1`. Reply `r0` = 0 on success, or [`super::reply_err`] on failure.
pub const WRITE: u64 = 2;
/// Ask the disk's size in bytes. `w1` ignored. Reply `r0` = the size (always non-negative here).
pub const SIZE: u64 = 3;
/// **Make everything the device has already acknowledged durable** (milestone 55): a
/// `VIRTIO_BLK_T_FLUSH` the block server does not reply to until the device completes it.
///
/// `w1` is 0 and is ignored. Reply `r0` = how many device flushes this block server has
/// completed since it started, **including this one**, so a success is always `>= 1`; or
/// [`super::reply_err`] on failure.
///
/// # Why a count rather than a zero
///
/// Every other verb here answers 0 for "done", and this one does not, because "the device was
/// told" is a claim that a caller cannot otherwise check and that a stub would satisfy just as
/// well. A count that **strictly increases across each successful call** is falsifiable: two
/// syncs that return the same number mean the second one did not reach the device, which is
/// exactly the lie this verb exists to make impossible. It is what milestone 55's gate
/// observes, and it costs one `u64` the server was incrementing anyway.
///
/// # Refusals, and why neither of them is a quiet success
///
/// - **`EOPNOTSUPP` (95)** when the device did not offer `VIRTIO_BLK_F_FLUSH` (feature bit 9),
///   so no flush was issued and none can be. This is the case a silent zero would ruin: a
///   server that answered "done" here would be telling a backup client its data is on the
///   platter when nothing asked the platter anything.
/// - **`EIO` (5)** when the device completed the request with a status other than
///   `VIRTIO_BLK_S_OK`, including `VIRTIO_BLK_S_UNSUPP` (2) from a device that negotiated the
///   feature and then refused the command anyway.
///
/// Both travel to a file-service client unchanged through [`super::fs::SYNC`], which is the one
/// place a block-protocol errno is visible above the FS server. See that verb for the argument.
pub const FLUSH: u64 = 4;

/// One filesystem block, in bytes: the transfer unit [`SIZE`] and [`FLUSH`] see and the unit
/// [`req_blocks`] counts in. Equal to `redoxfs::BLOCK_SIZE`; asserted against it in the block
/// server so a future RedoxFS bump that changed the block size would fail to build rather than
/// silently corrupt.
pub const BLOCK_SIZE: usize = PAGE;

/// The logical block size a partition table counts in: the 512-byte sector every disk this
/// project has met presents and the unit the virtio blk wire counts in. Nothing on the wire
/// reports it (`disk_surveyor`'s BUGS), so it is stated here once and every consumer takes it
/// from here. It left the test fixture for this crate in milestone 886 (the filesystem test
/// fixture moves to its own crate), so the shipping
/// installer could state its table's unit without depending on test data.
/// Name: provisional (milestone 886); calef names public items.
pub const SECTOR_BYTES: u64 = 512;

/// The number of sectors in one filesystem block (`BLOCK_SIZE / SECTOR_BYTES = 8`).
pub const SECTORS_PER_BLOCK: u64 = (BLOCK_SIZE as u64) / SECTOR_BYTES;

/// **How many blocks the shared region spans, and the most one [`READ`] or [`WRITE`] may carry**
/// (milestone 138 step 4). Chosen equal to `fs::TRANSFER_PAGES` on purpose: it is the same
/// number for the same reason, and a full 64 KiB file request now fans out to exactly one blk
/// request per RedoxFS `Disk::read_at`/`write_at` call up to this many contiguous blocks, rather
/// than one per block.
///
/// # Why this is the whole of the change, the same way step 3's was
///
/// The request word had 56 bits below the opcode and used none of them; [`req`] packs the count
/// there and nothing about the block index in the second word changes. **The shared region did
/// limit a transfer to one block**, because the device DMAs straight into it and a request
/// larger than the region would overrun a buffer nobody sized for it. Growing the region to
/// [`TRANSFER_BLOCKS`] contiguous pages, and reading the count out of the request word instead
/// of assuming one, is the entire mechanism: no new opcode, no descriptor beyond the one that
/// already carried a whole block, only a longer one.
///
/// # Compatibility, the same property step 3's channel has
///
/// A caller that has never heard of this constant sends [`super::req`]`(operation)`, whose low bits are
/// all zero, which [`req_blocks`] reads back as **one** block: the pre-step-4 behaviour exactly.
/// `disk_surveyor`, `disk_partitioner` and `mkfs` all call the block protocol this way and are
/// unmodified by this step, the same compatibility step 3 gave `swish` and the caretakers.
pub const TRANSFER_BLOCKS: usize = 16;

/// The largest payload one [`READ`] or [`WRITE`] may carry, in bytes: the whole shared region
/// ([`TRANSFER_BLOCKS`] blocks). The block server clamps every request's count to this.
pub const TRANSFER_MAX: usize = TRANSFER_BLOCKS * BLOCK_SIZE;

/// Pack a blk request's first word: opcode (bits 63:56, [`super::OPERATION_SHIFT`]) and a block count
/// (bits 7:0, stored as `blocks - 1` so the all-zero word every pre-step-4 caller already sends
/// still decodes as one block). `blocks` must be in `1..=TRANSFER_BLOCKS`.
pub const fn req(operation: u64, blocks: usize) -> u64 {
    assert!(blocks >= 1 && blocks <= TRANSFER_BLOCKS);
    (operation << super::OPERATION_SHIFT) | (blocks as u64 - 1)
}

/// The block count of a blk request word: [`req`]'s inverse. A word built by
/// [`super::req`]`(operation)`, which every caller before this step sends, decodes as 1.
pub const fn req_blocks(w0: u64) -> usize {
    ((w0 & 0xff) + 1) as usize
}

/// **The channel's arithmetic, checked by the build rather than by a test**, for
/// `fs::TRANSFER_MAX`'s reason: this contract has no test harness at EL0, and a build is the
/// only artifact that can be wrong about it there.
const _: () = {
    assert!(TRANSFER_BLOCKS >= 1);
    assert!(TRANSFER_BLOCKS <= 256); // the 8-bit count field's range
    assert!(TRANSFER_MAX == TRANSFER_BLOCKS * BLOCK_SIZE);
    // `req`'s all-zero-low-bits compatibility case: a caller that never heard of `req_blocks`
    // sends `super::req(operation)`, whose low bits are 0, and that must still mean one block.
    assert!(req_blocks(super::req(READ)) == 1);
};

#[cfg(test)]
mod tests;
