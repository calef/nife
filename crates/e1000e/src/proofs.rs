use super::*;

/// **No word a device can write makes the driver read outside a buffer or past its caller's
/// space.** The descriptor's second word is entirely the device's; this is every value of it.
///
/// Falsification: replayable `crates/e1000e/falsifications/proofs.a_delivered_frame_always_fits_the_buffer_and_the_room.patch`
#[kani::proof]
fn a_delivered_frame_always_fits_the_buffer_and_the_room() {
    let word: u64 = kani::any();
    let room: usize = kani::any();
    if let RxVerdict::Frame(n) = rx_verdict(word, room) {
        assert!(n <= room);
        assert!(n as u64 <= BUFFER_SIZE);
        assert!(n <= MAX_FRAME);
    }
}

/// **The handoff round-trips, and anything it accepts is page aligned with a unicast MAC.**
///
/// Falsification: replayable `crates/e1000e/falsifications/proofs.the_handoff_round_trips_and_accepts_only_what_pack_can_make.patch`
#[kani::proof]
fn the_handoff_round_trips_and_accepts_only_what_pack_can_make() {
    let a0: u64 = kani::any();
    let a1: u64 = kani::any();
    if let Some(h) = Handoff::unpack(a0, a1) {
        assert!(h.data_plane_phys.is_multiple_of(PAGE) && h.data_plane_phys != 0);
        assert!(h.mac[0] & 1 == 0);
        let [p0, p1] = h.pack();
        assert!(Handoff::unpack(p0, p1) == Some(h));
    }
}

/// **Every descriptor and buffer offset, for any index, lies inside the region.**
///
/// Falsification: replayable `crates/e1000e/falsifications/proofs.every_ring_offset_is_inside_the_region.patch`
#[kani::proof]
fn every_ring_offset_is_inside_the_region() {
    let i: u16 = kani::any();
    assert!(layout::rx_descriptor(i) + 16 <= layout::TX_RING);
    assert!(layout::tx_descriptor(i) + 16 <= layout::RX_BUFFERS);
    assert!(layout::rx_buffer(i) + BUFFER_SIZE <= layout::TX_BUFFERS);
    assert!(layout::tx_buffer(i) + BUFFER_SIZE <= layout::BYTES);
}
