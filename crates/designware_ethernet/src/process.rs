use address_space_map::pair_page;
/// The DMA region, [`super::layout::PAGES`] pages, normal memory. The same address as the
/// virtio and `e1000e` servers' DMA windows; a `net_stack` drives one NIC, never two.
pub const DATA_PLANE_VA: u64 = pair_page(0x0000_0000_0090_0000);
/// The controller's DMA page ([`super::regs::DMA_PAGE`]), device memory.
pub const DMA_PAGE_VA: u64 = pair_page(0x0000_0000_009c_0000);
