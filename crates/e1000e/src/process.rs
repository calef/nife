use address_space_map::pair_page;
/// The DMA region, [`super::layout::PAGES`] pages, normal memory. The virtio server's one DMA
/// page lives at the same address; a `net_stack` drives one NIC or the other, never both.
pub const DATA_PLANE_VA: u64 = pair_page(0x0000_0000_0090_0000);
/// BAR0's receive-queue page, device memory.
pub const RX_QUEUE_VA: u64 = pair_page(0x0000_0000_009c_0000);
/// BAR0's transmit-queue page, device memory.
pub const TX_QUEUE_VA: u64 = pair_page(0x0000_0000_009c_1000);
