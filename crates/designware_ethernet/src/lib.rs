#![cfg_attr(not(test), no_std)]
//! **The Synopsys `DesignWare` Ethernet `QoS` controller, as pure logic**: radon's network ports
//! (milestone 53 (the board's own peripherals: network and storage on real silicon);
//! notes/designware-ethernet.md).
//!
//! The `StarFive` JH7110 has two of these, `snps,dwmac-5.20` at `0x1603_0000` and `0x1604_0000`,
//! each behind a Motorcomm YT8531 PHY on a VisionFive 2. This crate is everything the driver's
//! halves compute and nothing either of them touches. The kernel
//! (`kernel/src/designware_ethernet.rs`) ungates the clocks, resets the controller, configures the
//! PHY, measures whether device DMA is coherent with the caches, programs the rings and starts
//! the DMA; `net_stack` (`components/src/designware_ethernet_transport.rs`) fills descriptors and
//! moves the two tail pointers. Both are thin volatile layers over this crate, which is what lets
//! the logic be tested on a host when the device exists on one desk and in no emulator.
//!
//! | module | what it holds |
//! |---|---|
//! | [`regs`] | the register map, and which page holds what |
//! | [`mdio`] | the MAC's MDIO pair and the clause 22 PHY registers |
//! | [`motorcomm`] | the YT8531's board configuration and its link status |
//! | [`controller`] | the reset, configuration, start and stop sequences |
//! | [`coherence`] | the loopback probe that measures DMA coherence, and its verdict |
//! | [`jh7110`] | the device-tree query and the syscon interface select |
//! | this root | descriptors, the DMA region's layout, the spawn handoff, the data plane |
//!
//! # Where the facts come from, and the license that comes with them
//!
//! Read, not recalled, on 2026-10-06 (UTC). **OpenBSD's `dwqe` driver is the source this crate
//! adapts**: `sys/dev/ic/dwqe.c`, `sys/dev/ic/dwqereg.h` and `sys/dev/fdt/if_dwqe_fdt.c` at commit
//! `59ec3ab4d107` (2026-07-31), and `sys/dev/mii/ytphy.c` at `9d7a05f92003` (2024-03-12). It is ISC
//! licensed and has a JH7110 path (`if_dwqe_fdt.c` matches `starfive,jh7110-dwmac`). FreeBSD has
//! a second permissive one, `sys/dev/eqos/if_eqos_starfive.c` (BSD-2-Clause, `351fad05e075`) with
//! `sys/dev/mii/mcommphy.c`, read as an independent cross-check: it agrees on the YT8531's
//! extended registers and on writing a 2048-byte receive buffer size. OpenBSD's was adapted
//! because its glue reads `starfive,tx-use-rgmii-clk`, the transmit clock arrangement radon's
//! v1.3B board has, where FreeBSD's sets the `gtx` rate per speed and does not. Linux's stmmac
//! and `dwmac-starfive.c`, U-Boot's `dwc_eth_qos*.c` and both JH7110 device trees are GPL and were
//! read only for facts about the hardware (a register's offset, a bit's meaning, what a board
//! wires where), each named where it is used. No GPL code was copied.
//!
//! The register map, the MDIO sequence, the descriptor bits, the controller sequences and the PHY
//! sequence are OpenBSD's, adapted, and carry its notice as the ISC license requires:
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
//! # Examples
//!
//! A receive descriptor is the device's statement, and a length it could not have meant is a
//! dropped frame rather than a read past the buffer:
//!
//! ```
//! use designware_ethernet::{RxVerdict, rx_verdict, RDES3_FD, RDES3_LD, RDES3_OWN};
//!
//! // 64 bytes on the wire is 60 of frame and 4 of CRC, which the driver strips.
//! assert_eq!(rx_verdict(RDES3_FD | RDES3_LD | 64, 1514), RxVerdict::Frame(60));
//! assert_eq!(rx_verdict(RDES3_OWN | RDES3_FD | RDES3_LD | 64, 1514), RxVerdict::NotYet);
//! assert_eq!(rx_verdict(RDES3_FD | RDES3_LD | 0x7fff, 1514), RxVerdict::Drop);
//! ```
//!
//! The spawn handoff refuses what the kernel did not make:
//!
//! ```
//! use designware_ethernet::Handoff;
//!
//! let h = Handoff { mac: [0x6c, 0xcf, 0x39, 0x00, 0x47, 0x2c], data_plane_phys: 0x4012_3000 };
//! let [role, phys] = h.pack();
//! assert_eq!(Handoff::unpack(role, phys), Some(h));
//! assert_eq!(Handoff::unpack(0, phys), None);
//! assert_eq!(Handoff::unpack(role, phys + 8), None);
//! ```
//!
//! # BUGS
//!
//! - **Nothing here has touched the device.** Every test runs against `sim`, which is this
//!   crate's reading of OpenBSD and the databook-derived headers, not the silicon. The bench
//!   runbook in notes/designware-ethernet.md is the first run of any of it.
//! - **One receive and one transmit queue, polled.** No interrupt, no checksum offload, no
//!   segmentation offload, no jumbo frames, no multicast filter: the same set of later decisions
//!   `e1000e` records.
//! - **`gmac0` only is wired.** [`jh7110`] knows both ports' addresses, and the kernel brings up
//!   the first the tree names, which is `gmac0` (`16030000`) in both trees and the port U-Boot
//!   netboots radon over. `gmac1`'s clocks are all in the SYS domain and no plan for them exists.
//!
//! Name: ratified 2026-10-07 (calef, pull request #1806's publish-ours review). The maintainer
//! comment reads: "one milestone per crate, minted now, each keeping its tree name (all free on
//! crates.io as of today) [...] Names are ratified now and permanent on first publication." His
//! words: "Yes, one per crate". Minted provisionally 2026-10-06 (UTC) by milestone 53's lane:
//! "`DesignWare` Ethernet" is how Synopsys's own documentation and both trees' `snps,dwmac`
//! compatible read aloud, and §154 (the acronym test is whether the phrase is spoken, applied
//! recursively) spells out `dwmac`. The provisional block offered
//! `designware_ethernet_quality_of_service`, which says which generation this is: the 4.x and 5.x
//! "`QoS`" controllers have these descriptors, while the older 3.x GMAC (`snps,dwmac-3.70a`, which
//! milestone 655 (DMA on a non-coherent RISC-V machine) names for the TH1520) has a different
//! descriptor format and would not fit here. The ruling kept the tree name. Milestone 816 (proven
//! `DesignWare` Ethernet `QoS` logic, released on its own) publishes it.

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

/// Offsets inside the DMA region, which the kernel allocates contiguous: `e1000e`'s layout,
/// because the two NICs' data planes have the same shape.
pub mod layout;

/// Where the process finds what it was mapped, which the kernel's spawn and `net_stack` must
/// agree on (rule 7: a crate, not a pair of "must match" comments).
pub mod process;

#[cfg(test)]
mod tests;

mod data_plane;
mod descriptor;
mod handoff;
mod hardware;

pub use data_plane::{DataPlane, Region, Tails, TxError};
use descriptor::*;
pub use descriptor::{
    BUFFER_SIZE, CRC_BYTES, DESCRIPTOR_BYTES, MAX_FRAME, RDES3_BUF1V, RDES3_CTXT, RDES3_ES,
    RDES3_FD, RDES3_IOC, RDES3_LD, RDES3_LENGTH, RDES3_OWN, RX_ENTRIES, RxVerdict, TDES2_BUF1_LEN,
    TDES2_IC, TDES3_ES, TDES3_FRAME_LEN, TDES3_FS, TDES3_LS, TDES3_OWN, TX_ENTRIES,
    rx_descriptor_words, rx_verdict, tx_descriptor_words,
};
pub use handoff::{Handoff, ROLE_TAG};
pub use hardware::Hw;

#[cfg(kani)]
mod proofs;
