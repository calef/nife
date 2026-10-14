#![no_std]
//! **The Intel `e1000e` family, as pure logic** (milestone 494 (a driver for the network card a PC
//! actually has); notes/e1000e.md).
//!
//! Everything both halves of the driver compute, and nothing either of them touches. The kernel
//! (`kernel/src/e1000e.rs`) resets the controller, reads its MAC address and programs the ring base
//! registers; `net_stack` (`components/src/e1000e_transport.rs`) fills descriptors and moves the
//! two ring tails. Both are thin volatile layers over this crate, which is what makes the ring
//! logic host-testable against a simulated device ([`DataPlane`]'s tests) when the real device only
//! exists inside an emulator and on one desk.
//!
//! # Where the facts come from
//!
//! Read, not recalled, on 2026-10-04 (UTC): Linux's `drivers/net/ethernet/intel/e1000e/regs.h`,
//! `defines.h` and `hw.h` for offsets, bits, descriptor layouts and device ids, cross-checked the
//! same day against FreeBSD's BSD-licensed `sys/dev/e1000/` (`e1000_regs.h`, and `e1000_api.c`'s
//! SPT class, which is exactly [`DEVICE_IDS`]' I219 rows), and QEMU's `hw/net/e1000e_core.c` for
//! what the emulated 82574L does with them (legacy descriptors unless `RFCTL.EXTEN`; receive only
//! with `RCTL.EN`, link up and bus mastering). No Linux code was copied. **Intel's own I219
//! datasheet was not read**; where the I219 is believed to differ from the 82574L it is said so,
//! and marked, at the item.
//!
//! # The finding that decides the confinement story
//!
//! Milestone 494's block asked this to be read before the split was drawn, because NVMe had put a
//! page boundary exactly where the authority boundary belongs and this family might not. **It does
//! not.** Receive queue 0's base is at `0x2800` and its tail at `0x2818`; transmit queue 0's base
//! is at `0x3800` and its tail at `0x3818` ([`regs`]). A process that can move a ring tail can
//! also repoint the ring, so on this device **the IOMMU is the whole of the DMA confinement**, as
//! the block predicted. What the page split still buys is everything else: the process holds the
//! two queue pages ([`RX_QUEUE_PAGE`], [`TX_QUEUE_PAGE`]) and not page 0 (`CTRL`, `STATUS`, `RCTL`,
//! `TCTL`, the interrupt registers) or page 5 (the receive address filter), so it cannot reset the
//! controller, change its MAC address, turn on promiscuous receive, or touch the PHY.
//!

//! # BUGS
//!
//! - **The I219's bring-up is ported in [`pch`]** (MAC and PHY steps, with Intel's BSD licence)
//!   **but not its link-up half**, which [`pch`]'s own BUGS records; none of it has run on silicon.
//! - **Legacy descriptors on the I219 are an assumption.** The 82574L supports them (QEMU uses them
//!   unless `RFCTL.EXTEN`); the PCH parts are believed to, from the 8254x lineage, and that was not
//!   read anywhere.
//! - **One receive and one transmit queue, polled.** No interrupt, no checksum offload, no
//!   segmentation offload, no jumbo frames. Each is a later decision rather than an omission.
//!
//! Name: ratified 2026-10-07 (calef, #1806), permanent on first publication: the family name
//! QEMU's `-device e1000e` and Linux's driver directory use, kept across their interface
//! (design/naming.md, the acronym test). Milestone 815 (proven Intel e1000e ring logic, released on
//! its own) publishes it.

#[cfg(test)]
extern crate std;

pub mod pch;

/// Register offsets in BAR0. Read from Linux's `regs.h`; queue 0 only.
pub mod regs;

/// Offsets inside the DMA region, which the kernel allocates contiguous and confines whole.
pub mod layout;

/// Where the process finds what it was mapped, which the kernel's spawn and `net_stack` must
/// agree on (rule 7: a crate, not a pair of "must match" comments).
pub mod process;

#[cfg(test)]
mod tests;

mod control;
mod data_plane;
mod descriptor;
mod device;
mod handoff;

pub use control::{
    CTRL_ASDE, CTRL_EXT_DRV_LOAD, CTRL_RST, CTRL_SLU, RAH_AV, RFCTL_EXTEN, STATUS_LU,
    mac_from_receive_address, rctl, tctl, tipg,
};
pub use data_plane::{DataPlane, Region, Tails, TxError};
use descriptor::*;
pub use descriptor::{
    BUFFER_SIZE, MAX_FRAME, RX_ENTRIES, RX_ERRORS_FRAME, RX_QUEUE_PAGE, RX_STATUS_DD,
    RX_STATUS_EOP, RxVerdict, TAIL_IN_PAGE, TX_COMMAND, TX_ENTRIES, TX_QUEUE_PAGE, TX_STATUS_DD,
    rx_verdict, tx_word,
};
pub use device::{DEVICE_IDS, VENDOR_INTEL, is_supported, model};
pub use handoff::{Handoff, ROLE_TAG};

#[cfg(kani)]
mod proofs;
