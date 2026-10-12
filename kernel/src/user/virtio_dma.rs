//! **Where a boot virtio device's DMA region records its own physical base.**
//!
//! Cut out of `user.rs` unchanged by milestone 872 (the kernel's user-mode file is split along its
//! seams), which left the parent an index.
//!
//! Name: provisional (milestone 872's lane, 2026-10-11 UTC; §281 (the kernel's user-mode file
//! splits into child modules of `user`)).

use super::*;

/// Where [`boot_virtio_mmio_device`](super::device_starters) (and, since milestone 600
/// (provisional), the gpu and keyboard wiring in [`display_service`] and [`keyboard_service`])
/// writes a DMA region's own physical base, inside that same region. Safe for every device granted
/// this way for one reason: no driver's layout reaches the first page's tail. Entropy's ring
/// (`components/src/entropy.rs`'s `Q_DESC`/`Q_AVAIL`/`Q_USED`) and its one pool buffer (`POOL_OFF`
/// 0x400, `POOL_LEN` 256 bytes) end at byte 0x500; `net_stack`'s two rings and four frame buffers
/// (`components/src/virtio_net_transport.rs`'s `BUF_BASE` 0x400 plus four `BUF`s of 0x2C0) end at
/// 0xF00. This sits in the last eight bytes, past all of them, so a future widening of any has room
/// to move without colliding. The value is `abi::virtio::DMA_PHYS_OFFSET`, which every reader
/// shares.
pub(crate) const VIRTIO_DMA_PHYS_OFFSET: u64 = abi::virtio::DMA_PHYS_OFFSET;
const _: () = assert!(VIRTIO_DMA_PHYS_OFFSET + 8 <= FRAME_SIZE);

/// **Write a DMA region's own physical base into its first page**, at [`VIRTIO_DMA_PHYS_OFFSET`].
/// The one place every boot virtio device's region learns where it is; see that constant.
pub(crate) fn write_dma_phys(dma: u64) {
    // SAFETY: `dma` is a fresh frame the caller just allocated, direct-mapped and owned by nobody
    // else yet, and `VIRTIO_DMA_PHYS_OFFSET + 8` is inside `FRAME_SIZE` (the assertion above), so
    // the write stays in the frame.
    unsafe {
        core::ptr::write_unaligned(
            (mmu::phys_to_virt(dma) as *mut u8)
                .add(VIRTIO_DMA_PHYS_OFFSET as usize)
                .cast::<u64>(),
            dma,
        );
    }
}
