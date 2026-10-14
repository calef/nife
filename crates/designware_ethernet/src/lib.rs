#![cfg_attr(not(test), no_std)]
//! **The Synopsys `DesignWare` Ethernet `QoS` controller, as pure logic**: radon's network ports
//! (notes/designware-ethernet.md). Everything the driver's two halves compute and nothing either
//! touches: the kernel (`kernel/src/designware_ethernet.rs`) brings the controller and PHY up,
//! measures DMA coherence and starts the DMA, and `net_stack`
//! (`components/src/designware_ethernet_transport.rs`) fills descriptors and moves the tail
//! pointers, both as thin volatile layers over this crate. Modules: [`regs`], [`mdio`],
//! [`motorcomm`] (the YT8531 PHY), [`controller`], [`coherence`], [`jh7110`]; the root holds the
//! descriptors, the DMA region's layout, the spawn handoff and the data plane.
//!
//! Adapted from OpenBSD's `dwqe` driver (`sys/dev/ic/dwqe.c`, `dwqereg.h`,
//! `sys/dev/fdt/if_dwqe_fdt.c`, and `sys/dev/mii/ytphy.c`), which has a JH7110 path reading
//! `starfive,tx-use-rgmii-clk`, the transmit clock radon's board uses; FreeBSD's `eqos` was a
//! cross-check. GPL sources gave hardware facts only, no code. The ISC license requires this notice:
//!
//! ```text
//! Copyright (c) 2008, 2019 Mark Kettenis <kettenis@openbsd.org>
//! Copyright (c) 2017, 2022 Patrick Wildt <patrick@blueri.se>
//! Copyright (c) 2001 Theo de Raadt
//! Copyright (c) 2023 Mark Kettenis <kettenis@openbsd.org>
//!
//! Permission to use, copy, modify, and distribute this software for any
//! purpose with or without fee is hereby granted, provided that the above
//! copyright notice and this permission notice appear in all copies.
//!
//! THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
//! WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
//! MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
//! ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
//! WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
//! ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
//! OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
//! ```
//!
//! # BUGS
//!
//! - Nothing here has touched the device; every test runs against `sim`. One receive and one
//!   transmit queue, polled: no interrupt, offloads, jumbo frames or multicast.
//! - Only `gmac0` (`16030000`) is wired; `gmac1`'s SYS-domain clocks have no plan.
//!
//! Name: ratified 2026-10-07 (calef, #1806): Synopsys's name, spelled out per §154 (the acronym test).

#[cfg(test)]
extern crate std;

pub mod coherence;
pub mod controller;
pub mod jh7110;
pub mod mdio;
pub mod motorcomm;
pub mod regs;
#[cfg(test)]
mod sim;

/// **The controller's register window, as a driver reaches it.** The kernel's implementation is a
/// pair of volatile accesses into the direct map; the tests' is `sim`. Offsets are bytes from the
/// window's base, and every one this crate passes is a constant from [`regs`], below
/// [`regs::WINDOW`].
pub trait Hw {
    /// Read the 32-bit register at `off`.
    fn read(&mut self, off: u32) -> u32;
    /// Write `v` to the 32-bit register at `off`.
    fn write(&mut self, off: u32, v: u32);
    /// Wait at least `us` microseconds.
    fn delay_us(&mut self, us: u64);
}

const PAGE: u64 = 4096;

/// Receive descriptors. Sixteen 16-byte descriptors are 256 bytes, one page with room to grow.
pub const RX_ENTRIES: u16 = 16;
/// Transmit descriptors.
pub const TX_ENTRIES: u16 = 16;
/// Bytes per descriptor: four little-endian words.
pub const DESCRIPTOR_BYTES: u64 = 16;
/// Bytes per buffer. Written into the receive control register's buffer size field, so the
/// device never writes past it, and large enough that a 1518-byte frame never spans two.
pub const BUFFER_SIZE: u64 = 2048;
/// The largest frame either direction carries: 1500 bytes of payload and the 14-byte header, CRC
/// excluded (the device appends it on transmit, and the driver strips it on receive).
pub const MAX_FRAME: usize = 1514;
/// The frame check sequence the device leaves on every received frame.
pub const CRC_BYTES: usize = 4;

/// Offsets inside the DMA region, which the kernel allocates contiguous: `e1000e`'s layout,
/// because the two NICs' data planes have the same shape.
pub mod layout {
    use super::{BUFFER_SIZE, DESCRIPTOR_BYTES, PAGE, RX_ENTRIES, TX_ENTRIES};
    /// The receive descriptor ring.
    pub const RX_RING: u64 = 0;
    /// The transmit descriptor ring.
    pub const TX_RING: u64 = PAGE;
    /// The first receive buffer.
    pub const RX_BUFFERS: u64 = 2 * PAGE;
    /// The first transmit buffer.
    pub const TX_BUFFERS: u64 = RX_BUFFERS + RX_ENTRIES as u64 * BUFFER_SIZE;
    /// The region's size in bytes.
    pub const BYTES: u64 = TX_BUFFERS + TX_ENTRIES as u64 * BUFFER_SIZE;
    /// The region's size in pages.
    pub const PAGES: u64 = BYTES.div_ceil(PAGE);

    /// Receive buffer `i`'s offset. `i` is reduced modulo the ring so no input leaves the region.
    pub const fn rx_buffer(i: u16) -> u64 {
        RX_BUFFERS + (i % RX_ENTRIES) as u64 * BUFFER_SIZE
    }
    /// Transmit buffer `i`'s offset, reduced the same way.
    pub const fn tx_buffer(i: u16) -> u64 {
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
}

/// Where the process finds what it was mapped, which the kernel's spawn and `net_stack` must
/// agree on (rule 7: a crate, not a pair of "must match" comments).
pub mod process {
    use address_space_map::pair_page;
    /// The DMA region, [`super::layout::PAGES`] pages, normal memory. The same address as the
    /// virtio and `e1000e` servers' DMA windows; a `net_stack` drives one NIC, never two.
    pub const DATA_PLANE_VA: u64 = pair_page(0x0000_0000_0090_0000);
    /// The controller's DMA page ([`super::regs::DMA_PAGE`]), device memory.
    pub const DMA_PAGE_VA: u64 = pair_page(0x0000_0000_009c_0000);
}

/// Receive and transmit, read and write-back formats: descriptor word 3 is the one that carries
/// ownership, and the only one the data plane judges.
///
/// **Owned by the device.** Set by the driver to hand a descriptor over, cleared by the device's
/// write-back.
pub const RDES3_OWN: u32 = 1 << 31;
/// Receive, read format: interrupt on completion. Interrupts are masked at the channel; set
/// anyway so the channel status's receive bit reports completions, which the coherence probe reads.
pub const RDES3_IOC: u32 = 1 << 30;
/// Receive, read format: buffer 1's address is valid.
pub const RDES3_BUF1V: u32 = 1 << 24;
/// Receive, write-back: this is a context descriptor (a timestamp), not a frame.
pub const RDES3_CTXT: u32 = 1 << 30;
/// Receive, write-back: first descriptor of a frame.
pub const RDES3_FD: u32 = 1 << 29;
/// Receive, write-back: last descriptor of a frame.
pub const RDES3_LD: u32 = 1 << 28;
/// Receive, write-back: error summary.
pub const RDES3_ES: u32 = 1 << 15;
/// Receive, write-back: the frame length including the CRC, valid when [`RDES3_LD`] is set.
pub const RDES3_LENGTH: u32 = 0x7fff;
/// Transmit, read format: interrupt on completion (in word 2).
pub const TDES2_IC: u32 = 1 << 31;
/// Transmit, read format: buffer 1's length (in word 2).
pub const TDES2_BUF1_LEN: u32 = 0x3fff;
/// Transmit: first descriptor of a frame.
pub const TDES3_FS: u32 = 1 << 29;
/// Transmit: last descriptor of a frame.
pub const TDES3_LS: u32 = 1 << 28;
/// Transmit: the device owns it ([`RDES3_OWN`]'s bit, in the transmit ring).
pub const TDES3_OWN: u32 = 1 << 31;
/// Transmit, read format: the whole frame's length.
pub const TDES3_FRAME_LEN: u32 = 0x7fff;
/// Transmit, write-back: error summary.
pub const TDES3_ES: u32 = 1 << 15;

/// **A receive descriptor handed to the device**: the buffer's address in words 0 and 1 (low and
/// high 32 bits, as OpenBSD and Linux's `dwmac4_set_addr` both write it), word 2 clear, and word 3
/// owned, interrupt-on-completion, buffer 1 valid. As two little-endian `u64`s: words 0 and 1,
/// then words 2 and 3.
pub const fn rx_descriptor_words(buffer_phys: u64) -> [u64; 2] {
    [
        buffer_phys,
        ((RDES3_OWN | RDES3_IOC | RDES3_BUF1V) as u64) << 32,
    ]
}

/// **A transmit descriptor for one whole frame in one buffer**: the address, the length with
/// interrupt-on-completion, and word 3 owned, first and last, with the frame length. The device
/// inserts the CRC and pads a short frame (checksum and padding control zero).
pub const fn tx_descriptor_words(buffer_phys: u64, len: u16) -> [u64; 2] {
    let len = len as u32 & TDES3_FRAME_LEN;
    let des2 = (len & TDES2_BUF1_LEN) | TDES2_IC;
    let des3 = TDES3_OWN | TDES3_FS | TDES3_LS | len;
    [buffer_phys, des2 as u64 | (des3 as u64) << 32]
}

/// What a receive descriptor's word 3 says, judged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RxVerdict {
    /// The device still owns it.
    NotYet,
    /// A whole, good frame of this many bytes (CRC removed), which fits the buffer and the room.
    Frame(usize),
    /// The device wrote it and it is not a frame to deliver: an error, a context descriptor, a
    /// frame spanning descriptors, or a length it could not have meant. Consumed and reposted.
    Drop,
}

/// **Judge a receive descriptor's word 3**, where `room` is how many bytes the caller can take.
///
/// Every field is the device's statement and none is trusted (notes/shared-page-audit.md's
/// finding 6, applied to a third NIC): a `Frame(n)` always has `n <= room`, `n <= MAX_FRAME` and
/// `n + CRC_BYTES <= BUFFER_SIZE`, which the Kani harness below proves for every word.
///
/// # Examples
///
/// ```
/// use designware_ethernet::{RxVerdict, rx_verdict, RDES3_FD, RDES3_LD, RDES3_OWN};
///
/// // 64 bytes on the wire is 60 of frame and 4 of CRC, which the driver strips.
/// assert_eq!(rx_verdict(RDES3_FD | RDES3_LD | 64, 1514), RxVerdict::Frame(60));
/// assert_eq!(rx_verdict(RDES3_OWN | RDES3_FD | RDES3_LD | 64, 1514), RxVerdict::NotYet);
/// assert_eq!(rx_verdict(RDES3_FD | RDES3_LD | 0x7fff, 1514), RxVerdict::Drop);
/// ```
pub fn rx_verdict(des3: u32, room: usize) -> RxVerdict {
    if des3 & RDES3_OWN != 0 {
        return RxVerdict::NotYet;
    }
    let whole = des3 & (RDES3_FD | RDES3_LD) == RDES3_FD | RDES3_LD;
    if des3 & RDES3_CTXT != 0 || !whole || des3 & RDES3_ES != 0 {
        return RxVerdict::Drop;
    }
    let wire = (des3 & RDES3_LENGTH) as usize;
    let Some(len) = wire.checked_sub(CRC_BYTES) else {
        return RxVerdict::Drop;
    };
    let fits = (14..=MAX_FRAME).contains(&len) && len <= room && wire as u64 <= BUFFER_SIZE;
    if !fits {
        return RxVerdict::Drop;
    }
    RxVerdict::Frame(len)
}

/// The role word's low half when `arg0` is this server's handoff. Disjoint from `net_stack`'s
/// existing roles: 0 is the virtio server, small integers are socket clients, `0xe1e0` is
/// `e1000e`'s.
pub const ROLE_TAG: u64 = 0xd3e0;

/// **What the kernel hands `net_stack` at spawn**, in two of its three argument words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Handoff {
    /// The station address the kernel programmed. The process cannot read or change it: the
    /// MAC's page is not mapped.
    pub mac: [u8; 6],
    /// The DMA region's physical base. Descriptors speak physical addresses (there is no IOMMU
    /// on the JH7110), so the process cannot compute them from its virtual window.
    pub data_plane_phys: u64,
}

impl Handoff {
    /// `[arg0, arg1]`: the tag and the MAC address in the role word, the base in the second.
    pub fn pack(&self) -> [u64; 2] {
        let mut mac = [0u8; 8];
        mac[..6].copy_from_slice(&self.mac);
        [
            ROLE_TAG | u64::from_le_bytes(mac) << 16,
            self.data_plane_phys,
        ]
    }

    /// The inverse of [`Handoff::pack`], refusing a role word without the tag, a base that is
    /// zero or not page aligned, and a MAC address no station can own.
    ///
    /// # Examples
    ///
    /// ```
    /// use designware_ethernet::Handoff;
    ///
    /// let h = Handoff { mac: [0x6c, 0xcf, 0x39, 0x00, 0x47, 0x2c], data_plane_phys: 0x4012_3000 };
    /// let [role, phys] = h.pack();
    /// assert_eq!(Handoff::unpack(role, phys), Some(h));
    /// assert_eq!(Handoff::unpack(0, phys), None);
    /// assert_eq!(Handoff::unpack(role, phys + 8), None);
    /// ```
    pub fn unpack(arg0: u64, arg1: u64) -> Option<Handoff> {
        if arg0 & 0xffff != ROLE_TAG || arg1 == 0 || !arg1.is_multiple_of(PAGE) {
            return None;
        }
        let b = (arg0 >> 16).to_le_bytes();
        let mac = [b[0], b[1], b[2], b[3], b[4], b[5]];
        if mac == [0; 6] || mac[0] & 1 != 0 {
            return None;
        }
        Some(Handoff {
            mac,
            data_plane_phys: arg1,
        })
    }

    /// Is `arg0` this server's role word at all? `net_stack` dispatches on this.
    pub fn is_role(arg0: u64) -> bool {
        arg0 & 0xffff == ROLE_TAG
    }
}

/// The DMA region, as the data plane reads and writes it. Offsets are bytes from the region's
/// base; an implementation bounds them. `e1000e::Region`'s shape.
pub trait Region {
    /// Read the little-endian `u64` at `off`.
    fn read_u64(&self, off: u64) -> u64;
    /// Write `v` at `off`, little-endian.
    fn write_u64(&mut self, off: u64, v: u64);
    /// Copy `out.len()` bytes starting at `off` into `out`.
    fn read_bytes(&self, off: u64, out: &mut [u8]);
    /// Copy `src` to `off`.
    fn write_bytes(&mut self, off: u64, src: &[u8]);
}

/// The two tail pointers, and the ordering between them and the region: the only register writes
/// the data plane makes.
pub trait Tails {
    /// Write the receive tail pointer's low 32 bits ([`regs::CHAN_RX_TAIL`]).
    fn set_rx_tail(&mut self, addr: u32);
    /// Write the transmit tail pointer's low 32 bits ([`regs::CHAN_TX_TAIL`]).
    fn set_tx_tail(&mut self, addr: u32);
    /// Order every region access before this against every access after it, as seen by the
    /// device. Called between filling a descriptor and the tail write that announces it, and
    /// between reading an ownership bit and reading the bytes it covers.
    fn barrier(&mut self);
}

/// Why a frame was not queued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxError {
    /// Empty, or longer than [`MAX_FRAME`].
    BadLength,
    /// Every descriptor is still the device's.
    Full,
}

/// **The data plane**: ring cursors and the region's physical base. Everything `net_stack` does
/// to the device goes through this, so all of it runs on the host against a simulated device.
///
/// # The tail pointers, and the one slot that is never the device's
///
/// A tail pointer is the address one past the last descriptor the device may use; the device
/// works from its current descriptor up to the tail and suspends there (OpenBSD's
/// `GMAC_CHAN_RX_END_ADDR` and `GMAC_CHAN_TX_END_ADDR` writes). On receive, one descriptor is
/// always held back so that "the device has every descriptor" and "the device has none" are never
/// the same tail address: OpenBSD's `if_rxr_init(..., DWQE_NRXDESC - 1)` makes the same choice.
/// The held-back slot is the one just behind [`DataPlane::rx_cursor`], and each received frame
/// reposts it and holds back the slot just consumed.
#[derive(Debug)]
pub struct DataPlane {
    phys: u64,
    rx_next: u16,
    tx_next: u16,
    tx_clean: u16,
    /// Frames the device wrote that were not delivered, for a report line.
    pub dropped: u32,
}

impl DataPlane {
    /// A data plane over the region at `data_plane_phys`. Call [`DataPlane::start`] first.
    pub fn new(data_plane_phys: u64) -> DataPlane {
        DataPlane {
            phys: data_plane_phys,
            rx_next: 0,
            tx_next: 0,
            tx_clean: 0,
            dropped: 0,
        }
    }

    /// The receive descriptor the next [`DataPlane::receive`] looks at.
    pub fn rx_cursor(&self) -> u16 {
        self.rx_next
    }

    /// The receive tail address for a ring whose held-back slot is `slot`.
    fn rx_tail(&self, slot: u16) -> u32 {
        (self.phys + layout::rx_descriptor(slot)) as u32
    }

    /// The transmit tail address when `next` is the next free slot.
    fn tx_tail(&self, next: u16) -> u32 {
        (self.phys + layout::tx_descriptor(next)) as u32
    }

    /// **Post every receive descriptor but the last and clear the transmit ring.** The kernel
    /// programmed both ring bases and lengths and started the DMA with both tails at the ring
    /// bases (nothing available); the receive tail write here is what hands the device its
    /// buffers.
    pub fn start(&mut self, region: &mut impl Region, tails: &mut impl Tails) {
        for i in 0..RX_ENTRIES {
            let d = layout::rx_descriptor(i);
            let [w01, w23] = if i == RX_ENTRIES - 1 {
                [0, 0]
            } else {
                rx_descriptor_words(self.phys + layout::rx_buffer(i))
            };
            region.write_u64(d, w01);
            region.write_u64(d + 8, w23);
        }
        for i in 0..TX_ENTRIES {
            let d = layout::tx_descriptor(i);
            region.write_u64(d, 0);
            region.write_u64(d + 8, 0);
        }
        tails.barrier();
        tails.set_rx_tail(self.rx_tail(RX_ENTRIES - 1));
    }

    /// Take the next received frame into `out`, returning its length, or `None` when the device
    /// has written nothing new. A descriptor judged [`RxVerdict::Drop`] is consumed and counted
    /// and the next tried, so one bad frame cannot stall the ring.
    pub fn receive(
        &mut self,
        region: &mut impl Region,
        tails: &mut impl Tails,
        out: &mut [u8],
    ) -> Option<usize> {
        loop {
            let d = layout::rx_descriptor(self.rx_next);
            let des3 = (region.read_u64(d + 8) >> 32) as u32;
            let verdict = rx_verdict(des3, out.len());
            if verdict == RxVerdict::NotYet {
                return None;
            }
            // The ownership bit was read before the bytes it covers, and nothing else stops a
            // weakly ordered CPU reading the buffer first.
            tails.barrier();
            let got = match verdict {
                RxVerdict::Frame(n) => {
                    region.read_bytes(layout::rx_buffer(self.rx_next), &mut out[..n]);
                    Some(n)
                }
                _ => {
                    self.dropped = self.dropped.wrapping_add(1);
                    None
                }
            };
            // Repost the held-back slot behind this one, then hold this one back: its descriptor
            // is the device's write-back and not owned, so leaving it as it is keeps it out of the
            // device's reach until it is reposted in turn.
            let behind = (self.rx_next + RX_ENTRIES - 1) % RX_ENTRIES;
            let [w01, w23] = rx_descriptor_words(self.phys + layout::rx_buffer(behind));
            let b = layout::rx_descriptor(behind);
            region.write_u64(b, w01);
            region.write_u64(b + 8, w23);
            tails.barrier();
            tails.set_rx_tail(self.rx_tail(self.rx_next));
            self.rx_next = (self.rx_next + 1) % RX_ENTRIES;
            if got.is_some() {
                return got;
            }
        }
    }

    /// Queue `frame` for transmission.
    pub fn transmit(
        &mut self,
        region: &mut impl Region,
        tails: &mut impl Tails,
        frame: &[u8],
    ) -> Result<(), TxError> {
        if frame.is_empty() || frame.len() > MAX_FRAME {
            return Err(TxError::BadLength);
        }
        self.reclaim(region);
        let next = (self.tx_next + 1) % TX_ENTRIES;
        if next == self.tx_clean {
            return Err(TxError::Full);
        }
        region.write_bytes(layout::tx_buffer(self.tx_next), frame);
        let d = layout::tx_descriptor(self.tx_next);
        let [w01, w23] = tx_descriptor_words(
            self.phys + layout::tx_buffer(self.tx_next),
            frame.len() as u16,
        );
        region.write_u64(d, w01);
        // Word 3, with the ownership bit, goes last and in one store with word 2.
        region.write_u64(d + 8, w23);
        tails.barrier();
        self.tx_next = next;
        tails.set_tx_tail(self.tx_tail(next));
        Ok(())
    }

    /// Is there a free transmit descriptor right now? Reclaims first.
    pub fn can_transmit(&mut self, region: &mut impl Region) -> bool {
        self.reclaim(region);
        (self.tx_next + 1) % TX_ENTRIES != self.tx_clean
    }

    /// Move the clean cursor past every descriptor the device has handed back.
    fn reclaim(&mut self, region: &impl Region) {
        while self.tx_clean != self.tx_next {
            let des3 = (region.read_u64(layout::tx_descriptor(self.tx_clean) + 8) >> 32) as u32;
            if des3 & TDES3_OWN != 0 {
                break;
            }
            self.tx_clean = (self.tx_clean + 1) % TX_ENTRIES;
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(kani)]
mod proofs {
    use super::*;

    /// **No word a device can write makes the driver read outside a buffer or past its caller's
    /// space.** Word 3 of a receive descriptor is entirely the device's; this is every value of it.
    ///
    /// Falsification: replayable `crates/designware_ethernet/falsifications/proofs.a_delivered_frame_always_fits_the_buffer_and_the_room.patch`
    #[kani::proof]
    fn a_delivered_frame_always_fits_the_buffer_and_the_room() {
        let des3: u32 = kani::any();
        let room: usize = kani::any();
        if let RxVerdict::Frame(n) = rx_verdict(des3, room) {
            assert!(n <= room);
            assert!(n <= MAX_FRAME);
            assert!(n as u64 + CRC_BYTES as u64 <= BUFFER_SIZE);
        }
    }

    /// **Every descriptor and buffer offset, for any index, lies inside the region.**
    ///
    /// Falsification: replayable `crates/designware_ethernet/falsifications/proofs.every_ring_offset_is_inside_the_region.patch`
    #[kani::proof]
    fn every_ring_offset_is_inside_the_region() {
        let i: u16 = kani::any();
        assert!(layout::rx_descriptor(i) + DESCRIPTOR_BYTES <= layout::TX_RING);
        assert!(layout::tx_descriptor(i) + DESCRIPTOR_BYTES <= layout::RX_BUFFERS);
        assert!(layout::rx_buffer(i) + BUFFER_SIZE <= layout::TX_BUFFERS);
        assert!(layout::tx_buffer(i) + BUFFER_SIZE <= layout::BYTES);
    }
}
