use super::*;

/// **No page that overlaps a withheld range is ever in the mask**, for any capability values,
/// any BAR size and any one withheld range: the MSI-X table is never mapped into the driver.
/// Falsification: replayable `crates/extensible_host_controller_interface/falsifications/verification.the_window_never_maps_a_withheld_page.patch`
#[kani::proof]
#[kani::unwind(18)]
fn the_window_never_maps_a_withheld_page() {
    let caps = Capabilities::decode(
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
    );
    let bar: u64 = kani::any();
    let a: u64 = kani::any();
    let len: u64 = kani::any();
    kani::assume(len > 0 && len <= 0x1_0000 && a <= u64::MAX - len);
    if let Ok(mask) = register_window(&caps, bar, &[(a, a + len)]) {
        for page in 0..MAX_REGISTER_PAGES {
            if mask & (1 << page) != 0 {
                let (s, e) = (page * PAGE, (page + 1) * PAGE);
                assert!(!(a < e && s < a + len));
            }
        }
        assert!(mask >> MAX_REGISTER_PAGES == 0);
    }
}

/// **A producer ring never places a TRB on its link slot or past the end**, and places the link
/// exactly on the last slot, from every reachable state.
/// Falsification: replayable `crates/extensible_host_controller_interface/falsifications/verification.a_producer_ring_stays_in_bounds.patch`
#[kani::proof]
fn a_producer_ring_stays_in_bounds() {
    let entries: u16 = kani::any();
    let enqueue: u16 = kani::any();
    kani::assume(entries >= 2 && enqueue < entries - 1);
    let mut ring = ProducerRing {
        base: kani::any(),
        entries,
        enqueue,
        cycle: kani::any(),
    };
    let p = ring.enqueue(Trb(kani::any()));
    assert!(p.index < entries - 1);
    if let Some((at, link)) = p.link {
        assert_eq!(at, entries - 1);
        assert_eq!(link.kind(), trb_type::LINK);
    }
    assert!(ring.enqueue < entries - 1);
}

/// **The handoff round-trips, and anything `unpack` accepts is a layout the driver can use.**
/// Falsification: replayable `crates/extensible_host_controller_interface/falsifications/verification.the_handoff_round_trips.patch`
#[kani::proof]
fn the_handoff_round_trips() {
    let w: [u64; 3] = kani::any();
    if let Some(h) = Handoff::unpack(w) {
        assert_eq!(h.pack(), w);
        assert!(u64::from(h.dma_pages) >= dma::WORK_PAGES);
        assert!(h.registers & 1 != 0);
    }
}
