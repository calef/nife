#![no_std]
//! **The Intel `e1000e` family, as pure logic** (notes/e1000e.md): everything both halves of the
//! driver compute, nothing either touches. The kernel (`kernel/src/e1000e.rs`) resets the
//! controller, reads its MAC and programs the ring bases; `net_stack`
//! (`components/src/e1000e_transport.rs`) fills descriptors and moves the ring tails. Both are thin
//! volatile layers, so the ring logic is tested against a simulated device ([`DataPlane`]'s tests).
//! Facts come from Linux's `e1000e` headers (no code copied), FreeBSD's `sys/dev/e1000/`, and
//! QEMU's `e1000e_core.c` for the emulated 82574L; Intel's I219 datasheet was not read, and each
//! believed difference is marked at its item.
//!
//! **On this device the IOMMU is the whole of the DMA confinement.** Each queue's base and tail
//! share a page (receive 0 at `0x2800` and `0x2818`, transmit 0 at `0x3800` and `0x3818`,
//! [`regs`]), so a process that moves a tail can repoint the ring. The page split still withholds
//! everything else: the process holds the two queue pages ([`RX_QUEUE_PAGE`], [`TX_QUEUE_PAGE`])
//! and not page 0 (`CTRL`, `STATUS`, `RCTL`, `TCTL`, interrupts) or page 5 (the receive address
//! filter), so it cannot reset the controller, change its MAC, receive promiscuously, or reach the
//! PHY.
//!
//! # BUGS
//!
//! - The I219's bring-up is ported from FreeBSD in [`pch`], which records the half that is not (the
//!   link-up reconfiguration); xenon's bench step is the first test of any of it.
//! - Legacy descriptors on the I219 are an assumption from the 8254x lineage, not read anywhere.
//! - One receive and one transmit queue, polled: no interrupt, offloads or jumbo frames.
//!
//! Name: ratified 2026-10-07 (calef, #1806), permanent on first publication: the family name QEMU's
//! `-device e1000e` and Linux's driver both use, kept as the outside world spells it.

#[cfg(test)]
extern crate std;

pub mod pch;

/// Intel's PCI vendor id.
pub const VENDOR_INTEL: u16 = 0x8086;

/// The device ids this driver claims, with what each is. Read from Linux's `hw.h` on 2026-10-04.
///
/// **Deliberately narrow.** The 82574L is what QEMU's `-device e1000e` presents, and proves the
/// sequence. The I219 entries are the Sunrise Point (SPT) generation only, which is what an `OptiPlex`
/// 7050 carries (Dell's specification sheet: "Integrated Intel i219-LM"). xenon's PCI survey
/// settled which: `00:1f.6 8086:15e3`, the I219-LM5 (bench/xenon-2026-10-04/). The other SPT rows
/// stay because FreeBSD runs one sequence for the whole class. Later PCH generations (Cannon Lake onward,
/// dozens of ids) need the bench to prove one before the family is widened, because the
/// generation-specific workarounds are exactly what this crate does not do (`BUGS`).
pub const DEVICE_IDS: &[(u16, &str)] = &[
    (0x10d3, "82574L (QEMU's e1000e)"),
    (0x156f, "I219-LM (SPT)"),
    (0x1570, "I219-V (SPT)"),
    (0x15b7, "I219-LM2 (SPT-H)"),
    (0x15b8, "I219-V2 (SPT-H)"),
    (0x15b9, "I219-LM3 (LBG)"),
    (0x15d7, "I219-LM4 (SPT)"),
    (0x15d8, "I219-V4 (SPT)"),
    (0x15e3, "I219-LM5 (SPT)"),
    (0x15d6, "I219-V5 (SPT)"),
];

/// Is `vendor:device` a part this driver claims? See [`DEVICE_IDS`] for why the list is short.
pub fn is_supported(vendor: u16, device: u16) -> bool {
    vendor == VENDOR_INTEL && DEVICE_IDS.iter().any(|&(id, _)| id == device)
}

/// What a claimed device id is, for a boot line; `None` for one this driver does not claim.
pub fn model(device: u16) -> Option<&'static str> {
    DEVICE_IDS
        .iter()
        .find(|&&(id, _)| id == device)
        .map(|&(_, m)| m)
}

/// Register offsets in BAR0. Read from Linux's `regs.h`; queue 0 only.
pub mod regs {
    /// Device control.
    pub const CTRL: u64 = 0x0_0000;
    /// Device status (read only).
    pub const STATUS: u64 = 0x0_0008;
    /// Extended device control.
    pub const CTRL_EXT: u64 = 0x0_0018;
    /// Interrupt cause read (read to clear).
    pub const ICR: u64 = 0x0_00c0;
    /// Interrupt mask clear (write only).
    pub const IMC: u64 = 0x0_00d8;
    /// Receive control.
    pub const RCTL: u64 = 0x0_0100;
    /// Transmit control.
    pub const TCTL: u64 = 0x0_0400;
    /// Transmit inter-packet gap.
    pub const TIPG: u64 = 0x0_0410;
    /// Receive descriptor base, low 32 bits, queue 0.
    pub const RDBAL: u64 = 0x0_2800;
    /// Receive descriptor base, high 32 bits.
    pub const RDBAH: u64 = 0x0_2804;
    /// Receive ring length in bytes (a multiple of 128).
    pub const RDLEN: u64 = 0x0_2808;
    /// Receive head (the device's cursor).
    pub const RDH: u64 = 0x0_2810;
    /// Receive tail (the driver's cursor). **The same 4 KiB page as [`RDBAL`].**
    pub const RDT: u64 = 0x0_2818;
    /// Transmit descriptor base, low 32 bits, queue 0.
    pub const TDBAL: u64 = 0x0_3800;
    /// Transmit descriptor base, high 32 bits.
    pub const TDBAH: u64 = 0x0_3804;
    /// Transmit ring length in bytes.
    pub const TDLEN: u64 = 0x0_3808;
    /// Transmit head.
    pub const TDH: u64 = 0x0_3810;
    /// Transmit tail. **The same 4 KiB page as [`TDBAL`].**
    pub const TDT: u64 = 0x0_3818;
    /// Receive filter control (`EXTEN` selects extended descriptors).
    pub const RFCTL: u64 = 0x0_5008;
    /// Multicast table array, 128 dwords.
    pub const MTA: u64 = 0x0_5200;
    /// How many dwords [`MTA`] has.
    pub const MTA_DWORDS: u64 = 128;
    /// Receive address low, entry 0 (the MAC address's first four octets).
    pub const RAL0: u64 = 0x0_5400;
    /// Receive address high, entry 0 (the last two octets, and [`super::RAH_AV`]).
    pub const RAH0: u64 = 0x0_5404;
}

/// `CTRL`: set link up.
pub const CTRL_SLU: u32 = 0x0000_0040;
/// `CTRL`: auto-speed detection.
pub const CTRL_ASDE: u32 = 0x0000_0020;
/// `CTRL`: global reset, self-clearing.
pub const CTRL_RST: u32 = 0x0400_0000;
/// `CTRL_EXT`: tells management firmware a driver has the device.
pub const CTRL_EXT_DRV_LOAD: u32 = 0x1000_0000;
/// `STATUS`: link up.
pub const STATUS_LU: u32 = 0x0000_0002;
/// `RFCTL`: extended receive descriptors. Cleared, so the device writes the legacy format.
pub const RFCTL_EXTEN: u32 = 0x0000_8000;
/// `RAH`: the address in this entry is valid.
pub const RAH_AV: u32 = 0x8000_0000;

/// `RCTL` as the kernel programs it: enabled, broadcast accepted (DHCP's offer is broadcast),
/// 2048-byte buffers (size field zero), the CRC stripped. **Not** promiscuous: unicast is filtered
/// to receive-address entry 0, which the process cannot write.
pub const fn rctl() -> u32 {
    const EN: u32 = 0x0000_0002;
    const BAM: u32 = 0x0000_8000;
    const SECRC: u32 = 0x0400_0000;
    EN | BAM | SECRC
}

/// `TCTL` as the kernel programs it: enabled, short frames padded, collision threshold 15 and
/// distance 63 (Linux's full-duplex values, `defines.h`).
pub const fn tctl() -> u32 {
    const EN: u32 = 0x0000_0002;
    const PSP: u32 = 0x0000_0008;
    EN | PSP | 15 << 4 | 63 << 12
}

/// `TIPG` for copper: IPGT 8, IPGR1 8, IPGR2 6 (Linux's `DEFAULT_82543_TIPG_*`).
pub const fn tipg() -> u32 {
    8 | 8 << 10 | 6 << 20
}

/// The MAC address in receive-address entry 0, or `None` when the entry is not marked valid, which
/// on a real part means the NVM load did not happen and on QEMU does not occur.
pub fn mac_from_receive_address(ral: u32, rah: u32) -> Option<[u8; 6]> {
    if rah & RAH_AV == 0 {
        return None;
    }
    let l = ral.to_le_bytes();
    let h = rah.to_le_bytes();
    let mac = [l[0], l[1], l[2], l[3], h[0], h[1]];
    // All zeros, or a group address, is not an address a station can own.
    (mac != [0; 6] && mac[0] & 1 == 0).then_some(mac)
}

/// The page of BAR0 holding receive queue 0's registers, mapped into the process.
pub const RX_QUEUE_PAGE: u64 = 0x2000;
/// The page of BAR0 holding transmit queue 0's registers, mapped into the process.
pub const TX_QUEUE_PAGE: u64 = 0x3000;
/// Where `RDT` sits inside [`RX_QUEUE_PAGE`], and `TDT` inside [`TX_QUEUE_PAGE`]: the same offset.
pub const TAIL_IN_PAGE: u64 = regs::RDT - RX_QUEUE_PAGE;

const PAGE: u64 = 4096;

/// Receive descriptors. Sixteen 16-byte descriptors are 256 bytes, which satisfies `RDLEN`'s
/// multiple-of-128 rule and fits one page with room to grow.
pub const RX_ENTRIES: u16 = 16;
/// Transmit descriptors.
pub const TX_ENTRIES: u16 = 16;
/// Bytes per buffer, matching `RCTL`'s size field of zero.
pub const BUFFER_SIZE: u64 = 2048;
/// The largest frame either direction carries: 1500 bytes of payload and the 14-byte Ethernet
/// header, no CRC (the device strips it on receive and appends it on transmit). smoltcp's
/// `max_transmission_unit` means exactly this for an Ethernet device.
pub const MAX_FRAME: usize = 1514;

/// Offsets inside the DMA region, which the kernel allocates contiguous and confines whole.
pub mod layout {
    use super::{BUFFER_SIZE, PAGE, RX_ENTRIES, TX_ENTRIES};
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
        RX_RING + (i % RX_ENTRIES) as u64 * 16
    }
    /// Transmit descriptor `i`'s offset.
    pub const fn tx_descriptor(i: u16) -> u64 {
        TX_RING + (i % TX_ENTRIES) as u64 * 16
    }
}

/// Where the process finds what it was mapped, which the kernel's spawn and `net_stack` must
/// agree on (rule 7: a crate, not a pair of "must match" comments).
pub mod process {
    use address_space_map::pair_page;
    /// The DMA region, [`super::layout::PAGES`] pages, normal memory. The virtio server's one DMA
    /// page lives at the same address; a `net_stack` drives one NIC or the other, never both.
    pub const DATA_PLANE_VA: u64 = pair_page(0x0000_0000_0090_0000);
    /// BAR0's receive-queue page, device memory.
    pub const RX_QUEUE_VA: u64 = pair_page(0x0000_0000_009c_0000);
    /// BAR0's transmit-queue page, device memory.
    pub const TX_QUEUE_VA: u64 = pair_page(0x0000_0000_009c_1000);
}

/// Receive status: descriptor done.
pub const RX_STATUS_DD: u8 = 0x01;
/// Receive status: end of packet. A frame without it spans buffers, which a 2048-byte buffer and
/// a 1514-byte maximum never need, so such a frame is dropped rather than reassembled.
pub const RX_STATUS_EOP: u8 = 0x02;
/// Receive errors that mean the frame is bad: CRC, symbol, sequence, carrier extension, RX data.
/// Linux's `E1000_RXD_ERR_FRAME_ERR_MASK`.
pub const RX_ERRORS_FRAME: u8 = 0x01 | 0x02 | 0x04 | 0x10 | 0x80;
/// Transmit command: end of packet, insert FCS, report status.
pub const TX_COMMAND: u8 = 0x01 | 0x02 | 0x08;
/// Transmit status: descriptor done.
pub const TX_STATUS_DD: u8 = 0x01;

/// What a receive descriptor's second word says, judged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RxVerdict {
    /// The device has not written this descriptor yet.
    NotYet,
    /// A whole, good frame of this many bytes, which fits the buffer and the caller's space.
    Frame(usize),
    /// The device wrote it and it is not a frame to deliver: an error bit, no end-of-packet, or a
    /// length it could not have meant. Consumed and the buffer reposted.
    Drop,
}

/// **Judge a receive descriptor's status word**, where `word` is the descriptor's second
/// little-endian `u64` (length, checksum, status, errors, special) and `room` is how many bytes the
/// caller can take.
///
/// Every field is the device's statement and none is trusted (notes/shared-page-audit.md's finding
/// 6, applied to a second NIC): a `Frame(n)` always has `n <= room` and `n <= BUFFER_SIZE`, which is
/// the property the Kani harness below proves for every word.
///
/// # Examples
///
/// ```
/// use e1000e::{RxVerdict, rx_verdict, RX_STATUS_DD, RX_STATUS_EOP};
///
/// let word = |len: u64, status: u8| len | (status as u64) << 32;
/// assert_eq!(rx_verdict(word(60, RX_STATUS_DD | RX_STATUS_EOP), 1514), RxVerdict::Frame(60));
/// assert_eq!(rx_verdict(word(60, 0), 1514), RxVerdict::NotYet);
/// assert_eq!(rx_verdict(word(0xffff, RX_STATUS_DD | RX_STATUS_EOP), 1514), RxVerdict::Drop);
/// ```
pub fn rx_verdict(word: u64, room: usize) -> RxVerdict {
    let status = (word >> 32) as u8;
    if status & RX_STATUS_DD == 0 {
        return RxVerdict::NotYet;
    }
    let errors = (word >> 40) as u8;
    let len = (word & 0xffff) as usize;
    let fits = (14..=MAX_FRAME).contains(&len) && len <= room && len as u64 <= BUFFER_SIZE;
    if status & RX_STATUS_EOP == 0 || errors & RX_ERRORS_FRAME != 0 || !fits {
        return RxVerdict::Drop;
    }
    RxVerdict::Frame(len)
}

/// A transmit descriptor's second word: `len` bytes, [`TX_COMMAND`], status cleared.
pub const fn tx_word(len: u16) -> u64 {
    len as u64 | (TX_COMMAND as u64) << 24
}

/// The role word's low half when `arg0` is an `e1000e` server handoff. Disjoint from `net_stack`'s
/// existing roles (0 is the virtio server, small integers are socket clients).
pub const ROLE_TAG: u64 = 0xe1e0;

/// **What the kernel hands `net_stack` at spawn**, in two of its three argument words (the third
/// stays the listen grant, as for the virtio server).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Handoff {
    /// The station address, read by the kernel from receive-address entry 0. The process has no
    /// way to read it itself (page 5 is not mapped) and no way to change it.
    pub mac: [u8; 6],
    /// The DMA region's physical base. Descriptors speak physical addresses (identity-mapped by
    /// the IOMMU domain), so the process cannot compute them from its virtual window.
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
    /// use e1000e::Handoff;
    ///
    /// let h = Handoff { mac: [0x52, 0x54, 0x00, 0x12, 0x34, 0x57], data_plane_phys: 0x4012_3000 };
    /// let [role, phys] = h.pack();
    /// assert_eq!(Handoff::unpack(role, phys), Some(h));
    /// // Role 0 is the virtio server and 1..=6 are socket clients; neither unpacks as this.
    /// assert_eq!(Handoff::unpack(0, phys), None);
    /// assert_eq!(Handoff::unpack(2, phys), None);
    /// // A data plane that is not page aligned was not made by the kernel.
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

    /// Is `arg0` an `e1000e` server's role word at all? `net_stack` dispatches on this.
    pub fn is_role(arg0: u64) -> bool {
        arg0 & 0xffff == ROLE_TAG
    }
}

/// The DMA region, as the data plane reads and writes it. Offsets are bytes from the region's
/// base; an implementation bounds them (the EL0 one is a `MappedWindow`, which panics on an
/// out-of-window offset rather than touching memory it was not given).
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

/// The two tail registers, and the ordering between them and the region. The only register writes
/// the data plane makes.
pub trait Tails {
    /// Write the receive tail (`RDT`).
    fn set_rx_tail(&mut self, v: u32);
    /// Write the transmit tail (`TDT`).
    fn set_tx_tail(&mut self, v: u32);
    /// Order every region access before this against every access after it, as seen by the device.
    /// Called between filling a descriptor and the tail write that announces it, and between
    /// reading a status and reading the bytes it covers.
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

/// **The data plane**: two ring cursors and the region's physical base. Everything `net_stack`
/// does to the device goes through this, so all of it runs on the host against a simulated
/// device.
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
    /// A data plane over the region at `data_plane_phys`. Call [`DataPlane::start`] before
    /// anything else.
    pub fn new(data_plane_phys: u64) -> DataPlane {
        DataPlane {
            phys: data_plane_phys,
            rx_next: 0,
            tx_next: 0,
            tx_clean: 0,
            dropped: 0,
        }
    }

    /// The receive descriptor the next [`DataPlane::receive`] looks at, for a caller that wants to
    /// peek at its status without consuming it.
    pub fn rx_cursor(&self) -> u16 {
        self.rx_next
    }

    /// Post every receive buffer and clear the transmit ring. The kernel left both heads and both
    /// tails at zero; the receive tail goes to the last slot, which gives the device every
    /// descriptor but one (a ring with head equal to tail is empty, so one slot always separates
    /// them).
    pub fn start(&mut self, region: &mut impl Region, tails: &mut impl Tails) {
        for i in 0..RX_ENTRIES {
            let d = layout::rx_descriptor(i);
            region.write_u64(d, self.phys + layout::rx_buffer(i));
            region.write_u64(d + 8, 0);
        }
        for i in 0..TX_ENTRIES {
            let d = layout::tx_descriptor(i);
            region.write_u64(d, 0);
            region.write_u64(d + 8, 0);
        }
        tails.barrier();
        tails.set_rx_tail(u32::from(RX_ENTRIES - 1));
    }

    /// Take the next received frame into `out`, returning its length, or `None` when the device
    /// has written nothing new. A descriptor judged [`RxVerdict::Drop`] is consumed and counted and
    /// the next one is tried, so one bad frame cannot stall the ring.
    pub fn receive(
        &mut self,
        region: &mut impl Region,
        tails: &mut impl Tails,
        out: &mut [u8],
    ) -> Option<usize> {
        loop {
            let d = layout::rx_descriptor(self.rx_next);
            let verdict = rx_verdict(region.read_u64(d + 8), out.len());
            if verdict == RxVerdict::NotYet {
                return None;
            }
            // The status was read before the bytes it covers, and nothing else stops a weakly
            // ordered CPU reading the buffer first.
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
            // Repost: the address again (a device may write back over it), the status cleared,
            // then hand the slot back by making it the tail.
            region.write_u64(d, self.phys + layout::rx_buffer(self.rx_next));
            region.write_u64(d + 8, 0);
            tails.barrier();
            tails.set_rx_tail(u32::from(self.rx_next));
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
        region.write_u64(d, self.phys + layout::tx_buffer(self.tx_next));
        region.write_u64(d + 8, tx_word(frame.len() as u16));
        tails.barrier();
        self.tx_next = next;
        tails.set_tx_tail(u32::from(next));
        Ok(())
    }

    /// Is there a free transmit descriptor right now? Reclaims first.
    pub fn can_transmit(&mut self, region: &mut impl Region) -> bool {
        self.reclaim(region);
        (self.tx_next + 1) % TX_ENTRIES != self.tx_clean
    }

    /// Move the clean cursor past every descriptor the device has reported done. `RS` is set on
    /// every frame, so the device writes `DD` on each in order.
    fn reclaim(&mut self, region: &impl Region) {
        while self.tx_clean != self.tx_next {
            let word = region.read_u64(layout::tx_descriptor(self.tx_clean) + 8);
            if (word >> 32) as u8 & TX_STATUS_DD == 0 {
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
}
