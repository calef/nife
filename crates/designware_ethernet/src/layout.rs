use super::{BUFFER_SIZE, DESCRIPTOR_BYTES, PAGE, RX_ENTRIES, TX_ENTRIES};
/// The receive descriptor ring.
pub(crate) const RX_RING: u64 = 0;
/// The transmit descriptor ring.
pub(crate) const TX_RING: u64 = PAGE;
/// The first receive buffer.
pub(crate) const RX_BUFFERS: u64 = 2 * PAGE;
/// The first transmit buffer.
pub(crate) const TX_BUFFERS: u64 = RX_BUFFERS + RX_ENTRIES as u64 * BUFFER_SIZE;
/// The region's size in bytes.
pub const BYTES: u64 = TX_BUFFERS + TX_ENTRIES as u64 * BUFFER_SIZE;
/// The region's size in pages.
pub const PAGES: u64 = BYTES.div_ceil(PAGE);

/// Receive buffer `i`'s offset. `i` is reduced modulo the ring so no input leaves the region.
pub(crate) const fn rx_buffer(i: u16) -> u64 {
    RX_BUFFERS + (i % RX_ENTRIES) as u64 * BUFFER_SIZE
}
/// Transmit buffer `i`'s offset, reduced the same way.
pub(crate) const fn tx_buffer(i: u16) -> u64 {
    TX_BUFFERS + (i % TX_ENTRIES) as u64 * BUFFER_SIZE
}
/// Receive descriptor `i`'s offset.
pub const fn rx_descriptor(i: u16) -> u64 {
    RX_RING + (i % RX_ENTRIES) as u64 * DESCRIPTOR_BYTES
}
/// Transmit descriptor `i`'s offset.
pub const fn tx_descriptor(i: u16) -> u64 {
    TX_RING + (i % TX_ENTRIES) as u64 * DESCRIPTOR_BYTES
}
