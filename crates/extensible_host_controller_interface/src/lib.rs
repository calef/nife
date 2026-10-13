#![no_std]
//! **xHCI, as pure logic** (notes/usb.md): everything a USB host controller driver computes, with
//! nothing it touches. The kernel finds and confines the register file and DMA memory
//! (`kernel/src/extensible_host_controller_interface.rs`) and the EL0 driver drives them
//! (`components/src/usb_keyboard_driver.rs`); this is the arithmetic between, the NVMe split again.
//!
//! The protocol in one paragraph (xHCI 1.2, chapters 4 and 6). **This is the first controller in
//! the tree whose data structures the device walks on its own.** The driver lays out, in memory
//! the controller reads by DMA, a *device context base address array* (one pointer per device
//! slot), a *command ring*, an *event ring* with its segment table, and one *transfer ring* per
//! endpoint. Every ring is an array of 16-byte **TRBs** (transfer request blocks) with a **cycle
//! bit** in each: the producer writes TRBs carrying its current cycle, the consumer accepts a TRB
//! only while the bit matches its own, and both flip at the end of the ring, where a producer ring
//! carries a **link TRB** back to its start. The driver rings a **doorbell** to say a ring has new
//! work; the controller writes **events** to the event ring and raises an interrupt.
//!
//! [`regs`] and [`Capabilities`] name the registers; [`register_window`] decides **which pages of
//! the register file a confined driver is mapped**, the confinement this crate makes checkable;
//! [`Handoff`] packs the spawn words; [`dma`] is the DMA region's layout; [`Trb`], [`Event`],
//! [`ProducerRing`] and [`EventRing`] are the ring discipline; [`context`] holds slot and endpoint
//! contexts; [`port`] is the port status register, whose write-one-to-clear bits are a trap.
//!
//! # BUGS
//!
//! - One page size: [`dma`] assumes 4 KiB (`PAGESIZE` bit 0), and the driver refuses otherwise.
//! - No streams, isochronous or bulk: the TRBs a keyboard needs and nothing more.
//! - Hubs are not modelled (route string 0), so a device behind a hub cannot be addressed.
//!
//! Name: ratified 2026-10-07 (calef, #1806), permanent on first publication: xHCI expanded per §154
//! (the acronym test is whether the phrase is spoken). Too long for a program name, so the program
//! is `usb_keyboard_driver`.

pub mod context;
pub mod dma;
pub mod port;
pub mod regs;
pub mod report;

pub use regs::Capabilities;

/// **How many pages of the register file a driver can be mapped**: the bits of [`Handoff`]'s
/// register mask. 64 KiB, which is the BAR of every Intel xHCI this project has read about; a
/// larger BAR is mapped this far and no further, and [`register_window`] refuses a controller
/// whose needed registers lie beyond it.
pub const MAX_REGISTER_PAGES: u64 = 16;

/// The page size every layout here is in.
pub const PAGE: u64 = 4096;

/// TRB type codes (xHCI 1.2 Table 6-91).
pub mod trb_type;

/// Completion codes (xHCI 1.2 Table 6-90), the ones a keyboard driver tells apart.
pub mod completion;

#[cfg(test)]
mod tests;

mod event;
mod handoff;
mod ring;
mod trb;
mod window;

pub use context::interrupt_interval;
pub use event::Event;
pub use handoff::Handoff;
pub use ring::{EventRing, Placement, ProducerRing};
pub use trb::Trb;
pub use window::{WindowRefusal, register_window};

#[cfg(kani)]
mod verification;
