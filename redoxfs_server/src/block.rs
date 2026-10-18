//! **The block layer beneath RedoxFS**: whole-block transfers ([`BlockIo`], [`BlockDisk`]) and the
//! single-block read cache ([`CachedDisk`]); [`crate::Server`] reaches it only through
//! [`redoxfs::Disk`]. Moved from `lib.rs` unchanged by milestone 846 (the file-server core moves
//! its tests out), as calef ruled on 2026-10-11 (UTC) in §275 (the file server core's tests move
//! into child modules by topic). The module name is provisional.

use alloc::vec::Vec;

use redoxfs::Disk;
use syscall::error::Result;

/// One filesystem block, in bytes (RedoxFS's `BLOCK_SIZE`): the unit [`BlockIo`] transfers.
pub const BLOCK: usize = 4096;

/// **How many blocks [`CachedDisk`] holds in the EL0 server**, here rather than in the binary so a
/// host test measures the cache the device runs.
///
/// 256 blocks, 1 MiB at most, allocated as they fill. Milestone 138 (close the read gap) chose 64
/// for one open file's
/// five-block tree spine. Milestone 606 (a directory walk costs what it does on Linux) measured a
/// walk (`examples/walk_replay.rs`, the walk
/// fixture): after a namespace change forgets the memo (`memo.rs`), the next walk went to the device 372
/// times at 64 slots, 212 at 128, 71 at 192 and 2 at 256, because the tree's metadata working set
/// is 207 blocks. A larger tree wants more; this is the smallest size that holds the one we price.
pub const CACHE_SLOTS: usize = 256;

/// **The one place in this tree that sees both the wire protocol and the store**, which is what
/// makes the two assertions below possible at all: `filesystem_contract` deliberately depends on nothing, and
/// the vendored engine has never heard of it. Milestone 138 wrote them because it moved the record
/// level, and `filesystem_fixture::throughput::RECORD`'s own comment named itself the soft spot a
/// change like that would break silently.
///
/// They are `const` assertions rather than tests so that they fire in the EL0 build too, where no
/// test harness runs. A build is the only artifact that can be wrong about this.
const _: () = {
    // `BLOCK` is the transfer unit for a store whose block size is `BLOCK_SIZE`. If they ever
    // disagree, every chunked read and write in this file is off by a factor.
    assert!(BLOCK as u64 == redoxfs::BLOCK_SIZE);

    // The record-aligned throughput phase reads at multiples of `RECORD`, and that means what it
    // claims only while every such multiple is also a record boundary. See that constant's comment.
    assert!(
        filesystem_fixture::throughput::RECORD
            .is_multiple_of(redoxfs::BLOCK_SIZE << redoxfs::RECORD_LEVEL)
    );

    // A file this build creates must be readable by this build. The creation level and the ceiling
    // the two `BlockTrait::empty` guards enforce are separate constants since milestone 138, and
    // this is what stops them being separated in the wrong direction.
    assert!(redoxfs::RECORD_LEVEL <= redoxfs::RECORD_LEVEL_MAX);

    // **The file channel is a whole number of blocks** (milestone 138 step 3). A `READ` or `WRITE`
    // now carries up to `fs::TRANSFER_MAX` bytes straight into the shared region, and RedoxFS reads
    // and writes it a record at a time from there; a channel that ended part way through a block
    // would make the last chunk of every full-channel request a partial one, which is the shape the
    // repeat-write bug had. Nothing else in the tree can check this: `filesystem_contract` has never heard of
    // the store and the store has never heard of `filesystem_contract`.
    assert!(filesystem_contract::fs::TRANSFER_MAX.is_multiple_of(BLOCK));
    assert!(filesystem_contract::fs::TRANSFER_MAX >= BLOCK);
};

/// A block-granular transport: read or write one whole filesystem block, or report the disk size.
/// The EL0 binary implements this over blk IPC; a host test implements it over a `Vec`.
///
/// This exists so the **chunking** that bridges RedoxFS's byte-addressed, arbitrary-length `Disk`
/// calls to whole-block transfers is written once and host-tested, instead of living only in the EL0
/// binary where no host test can reach it. That gap is what let the repeat-write bug hide.
pub trait BlockIo {
    /// Read filesystem block `block` into `buf` (exactly [`BLOCK`] bytes).
    fn read_block(&mut self, block: u64, buf: &mut [u8; BLOCK]) -> Result<()>;
    /// Write `buf` (exactly [`BLOCK`] bytes) to filesystem block `block`.
    fn write_block(&mut self, block: u64, buf: &[u8; BLOCK]) -> Result<()>;
    /// The disk size, in bytes.
    fn size_bytes(&mut self) -> Result<u64>;
}

/// A RedoxFS [`Disk`] over a [`BlockIo`]. RedoxFS calls `read_at`/`write_at` with a block index and a
/// buffer that may be shorter than a block (a compressed record), exactly a block, or many blocks
/// long (an uncompressed 128 KiB record), so this splits each call into whole-block transfers.
///
/// A write whose final chunk is short is **read-modify-written**: the block is read first, the chunk
/// overwrites its front, and the whole block goes back. That preserves the bytes past the chunk,
/// which is what a byte-addressed disk (`DiskFile`, where `write_at` writes exactly the buffer) does,
/// and RedoxFS relies on it for compressed records.
pub struct BlockDisk<T>(pub T);

impl<T: BlockIo> Disk for BlockDisk<T> {
    unsafe fn read_at(&mut self, block: u64, buffer: &mut [u8]) -> Result<usize> {
        let mut scratch = [0u8; BLOCK];
        for (i, chunk) in buffer.chunks_mut(BLOCK).enumerate() {
            self.0.read_block(block + i as u64, &mut scratch)?;
            chunk.copy_from_slice(&scratch[..chunk.len()]);
        }
        Ok(buffer.len())
    }

    unsafe fn write_at(&mut self, block: u64, buffer: &[u8]) -> Result<usize> {
        let mut scratch = [0u8; BLOCK];
        for (i, chunk) in buffer.chunks(BLOCK).enumerate() {
            let b = block + i as u64;
            if chunk.len() < BLOCK {
                self.0.read_block(b, &mut scratch)?;
            }
            scratch[..chunk.len()].copy_from_slice(chunk);
            self.0.write_block(b, &scratch)?;
        }
        Ok(buffer.len())
    }

    fn size(&mut self) -> Result<u64> {
        self.0.size_bytes()
    }
}

/// **A direct-mapped, write-through cache of single-block reads** (milestone 138 step 2). Every
/// slot holds at most one `(block number, block bytes)` pair, chosen by `block % capacity`; a
/// collision simply evicts, which is safe because a miss just falls through to the disk.
///
/// **Why this and not an LRU.** An LRU would hit more often at the same slot count, at the cost of
/// bookkeeping (a recency list, a pointer to update on every hit) this cache does not need to be
/// correct: [`BlockCache::get`] and [`BlockCache::insert`] are each one array index and one
/// comparison. The fewer-moving-parts option wins while it costs nothing measurable: at
/// [`CACHE_SLOTS`] the walk fixture's whole metadata working set (207 blocks) stays resident.
///
/// **A slot's 4 KiB is allocated the first time something lands in it** (milestone 606). The cache
/// is sized for the ordinary server's 8 MiB heap, and the crash test's servers run on 2 MiB and
/// touch a few dozen blocks; allocating on use lets one constant serve both, because what a server
/// pays is what it touched, never more than `capacity` blocks.
struct BlockCache {
    slots: Vec<Option<alloc::boxed::Box<Slot>>>,
}

/// One cached block: its number and its bytes.
type Slot = (u64, [u8; BLOCK]);

impl BlockCache {
    /// `capacity` slots, none of them allocated. `capacity` must be at least 1.
    fn new(capacity: usize) -> Self {
        let mut slots = Vec::with_capacity(capacity);
        slots.resize_with(capacity, || None);
        Self { slots }
    }

    fn slot(&self, block: u64) -> usize {
        (block as usize) % self.slots.len()
    }

    /// The cached bytes for `block`, or `None` on a miss (never cached, or evicted by a collision).
    fn get(&self, block: u64) -> Option<&[u8; BLOCK]> {
        match &self.slots[self.slot(block)] {
            Some(entry) if entry.0 == block => Some(&entry.1),
            _ => None,
        }
    }

    /// Record `block`'s current bytes, replacing whatever the slot held (its own entry to refresh,
    /// or a different block's to evict). `data` must be exactly [`BLOCK`] bytes.
    fn insert(&mut self, block: u64, data: &[u8]) {
        let i = self.slot(block);
        match &mut self.slots[i] {
            Some(entry) => {
                entry.0 = block;
                entry.1.copy_from_slice(data);
            }
            empty => {
                let mut entry = alloc::boxed::Box::new((block, [0u8; BLOCK]));
                entry.1.copy_from_slice(data);
                *empty = Some(entry);
            }
        }
    }

    /// Drop `block`'s entry if it is the one occupying its slot. A no-op if the slot holds a
    /// different block (already evicted) or nothing. The allocation is kept for the next insert.
    fn invalidate(&mut self, block: u64) {
        let i = self.slot(block);
        if let Some(entry) = &mut self.slots[i]
            && entry.0 == block
        {
            // `u64::MAX` is no block this disk has, so the slot now answers every `get` with a miss.
            entry.0 = u64::MAX;
        }
    }
}

/// **Whether `block` is one [`CachedDisk`] keeps** (milestone 606): everything but the header ring.
///
/// The ring's 256 blocks are read once, all of them, when the image is mounted, and after that
/// only written, one per commit. Caching them bought nothing and cost the whole cache: at mount
/// they filled every slot they mapped to, and a 256-slot cache on a 2 MiB crash-test server would
/// have spent half its heap on blocks nobody reads again. RedoxFS puts the ring at the start of
/// the filesystem, and this server opens images whose filesystem starts at block 0
/// (`FileSystem::open(disk, None, ..)`), so the ring is blocks `0..HEADER_RING`.
fn cacheable(block: u64) -> bool {
    block >= redoxfs::HEADER_RING
}

/// **A [`Disk`] wrapping another, caching single-block reads** (milestone 138 step 2).
/// `Transaction::read_tree_and_addr`'s four-level tree walk plus the target node is five
/// single-block reads on every `Server::read`/`write`/`fstat` call, even for the node the last call
/// resolved (notes/benchmarks/read-path-block-contract-and-metadata-cache.md).
///
/// **Only a `buffer.len() == BLOCK` read consults the cache.** A multi-block record body is read
/// once per access, so caching it would spend this process's small heap for little repetition; the
/// cache stays proportional to the metadata working set.
///
/// **A write updates the cache only after the inner write succeeded**, so the cache never holds
/// bytes a failed write did not land. A short final chunk is a read-modify-write below this type
/// whose merged result it never sees, so that block's slot is invalidated instead.
///
/// **Safe against RedoxFS's copy-on-write allocator without a generation counter**: RedoxFS never
/// rewrites a live block in place, so a cached address goes stale only by being **written**, and
/// this type's [`Disk::write_at`] is the only path a write reaches the platter through.
pub struct CachedDisk<D> {
    inner: D,
    cache: BlockCache,
}

impl<D> CachedDisk<D> {
    /// The disk this wraps, so a host test can read a counter below the cache.
    #[cfg(feature = "hosttest")]
    pub fn inner(&self) -> &D {
        &self.inner
    }

    /// Wrap `inner`, caching up to `capacity` distinct single-block reads. `capacity` is the
    /// caller's call, against its own heap budget: milestone 37's crash-test FS servers run a
    /// fraction of the ordinary heap and must pick a smaller number than the ordinary server does.
    pub fn new(inner: D, capacity: usize) -> Self {
        Self {
            inner,
            cache: BlockCache::new(capacity),
        }
    }
}

impl<D: Disk> Disk for CachedDisk<D> {
    unsafe fn read_at(&mut self, block: u64, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() == BLOCK && cacheable(block) {
            if let Some(cached) = self.cache.get(block) {
                buffer.copy_from_slice(cached);
                return Ok(BLOCK);
            }
            // SAFETY: forwarded from this method's own caller, on a buffer of the same shape.
            let n = unsafe { self.inner.read_at(block, buffer)? };
            if n == BLOCK {
                self.cache.insert(block, buffer);
            }
            return Ok(n);
        }
        // A record body or larger: already batched by step 4, and not this cache's target. See the
        // type's own doc for why caching it would cost more than it is worth.
        unsafe { self.inner.read_at(block, buffer) }
    }

    unsafe fn write_at(&mut self, block: u64, buffer: &[u8]) -> Result<usize> {
        // SAFETY: forwarded from this method's own caller, on a buffer of the same shape.
        let n = unsafe { self.inner.write_at(block, buffer)? };
        // Only the bytes the inner disk confirmed writing are cached; a short count from a
        // hypothetical partial write must not be papered over with an entry that claims more
        // landed than did.
        let mut off = 0;
        let mut b = block;
        while off < n {
            let chunk = (n - off).min(BLOCK);
            if !cacheable(b) {
                // A header-ring block: never cached, so there is nothing to keep fresh.
            } else if chunk == BLOCK {
                self.cache.insert(b, &buffer[off..off + BLOCK]);
            } else {
                // A short final chunk: the whole-block bytes that actually landed are a merge
                // (read-modify-write) this type never saw, so it cannot vouch for a cached copy.
                self.cache.invalidate(b);
            }
            off += chunk;
            b += 1;
        }
        Ok(n)
    }

    fn size(&mut self) -> Result<u64> {
        self.inner.size()
    }
}

#[cfg(test)]
mod tests;
