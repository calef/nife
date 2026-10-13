#![no_std]
//! **xHCI, as pure logic** (milestone 242, USB host and HID; notes/usb.md).
//!
//! Everything a USB host controller driver computes, with nothing it touches. The register file
//! and the DMA memory are the kernel's to find and confine (`kernel/src/extensible_host_controller_interface.rs`)
//! and the EL0 driver's to drive (`components/src/usb_keyboard_driver.rs`); what lives here is the
//! arithmetic those volatile accesses carry. It is the NVMe split again, one device over:
//! `crates/non_volatile_memory_express` is the precedent and rule 7 is the reason.
//!
//! The protocol in one paragraph (xHCI 1.2, chapters 4 and 6). **This is the first controller in
//! the tree whose data structures the device walks on its own.** The driver lays out, in memory
//! the controller reads by DMA, a *device context base address array* (one pointer per device
//! slot), a *command ring*, an *event ring* with its segment table, and one *transfer ring* per
//! endpoint. Every ring is an array of 16-byte **TRBs** (transfer request blocks) with a **cycle
//! bit** in each: the producer writes TRBs carrying its current cycle, the consumer accepts a TRB
//! only while the bit matches its own, and both flip at the end of the ring, where a producer ring
//! carries a **link TRB** back to its start. The driver rings a **doorbell** to say a ring has new
//! work; the controller writes **events** (command completions, transfer completions, port
//! changes) to the event ring and raises an interrupt.
//!
//! What is here:
//!
//! - [`regs`]: register offsets and bit names, and [`Capabilities`], the read-only facts the
//!   capability registers state.
//! - [`register_window`]: **which pages of the register file a confined driver is mapped**, the
//!   confinement decision this crate exists to make checkable.
//! - [`Handoff`]: the three spawn words, packed and unpacked.
//! - [`dma`]: the DMA region's layout, which both ends agree on.
//! - [`Trb`], [`Event`], [`ProducerRing`] and [`EventRing`]: the ring discipline.
//! - [`context`]: slot and endpoint contexts.
//! - [`port`]: the port status register, whose write-one-to-clear bits are a trap.
//!
//! # Examples
//!
//! The cycle bit is the mechanism worth showing. A producer ring of four entries has three usable
//! slots and a link TRB in the fourth; the third enqueue also writes the link, and the cycle the
//! producer stamps flips for the next lap:
//!
//! ```
//! use extensible_host_controller_interface::{ProducerRing, Trb};
//!
//! let mut ring = ProducerRing::new(0x1000, 4);
//! let a = ring.enqueue(Trb::enable_slot());
//! assert_eq!((a.index, a.trb.cycle(), a.link), (0, true, None));
//! ring.enqueue(Trb::enable_slot());
//! let c = ring.enqueue(Trb::enable_slot());
//! assert_eq!(c.index, 2);
//! let (at, link) = c.link.unwrap(); // the third TRB fills the lap, so the link is written too
//! assert_eq!(at, 3);
//! assert!(link.cycle()); // stamped with the lap it ends, so the controller follows it
//! let d = ring.enqueue(Trb::enable_slot());
//! assert_eq!((d.index, d.trb.cycle()), (0, false)); // second lap, cycle flipped
//! assert_eq!(ring.address_of(d.index), 0x1000);
//! ```
//!
//! And the register window: QEMU's `qemu-xhci` puts its MSI-X table in the fourth page of its
//! 16 KiB register file, so a driver is mapped the first three and never the fourth.
//!
//! ```
//! use extensible_host_controller_interface::{Capabilities, register_window};
//!
//! // qemu-xhci's capability registers: 0x40 bytes of them, 64 slots, 4 ports, runtime at 0x1000,
//! // doorbells at 0x2000.
//! let caps = Capabilities::decode(0x0100_0040, 0x0400_0840, 0, 0x0000_7000, 0x2000, 0x1000);
//! let window = register_window(&caps, 0x4000, &[(0x3000, 0x3000 + 16 * 16), (0x3800, 0x3808)]);
//! assert_eq!(window, Ok(0b0111));
//! ```
//!
//! # BUGS
//!
//! - **One page size.** [`dma`] assumes the controller accepts 4 KiB pages (`PAGESIZE` bit 0),
//!   which every xHCI this project has read about does; the driver checks and refuses otherwise.
//! - **No streams, no isochronous, no bulk.** The TRB builders cover the commands and transfers a
//!   keyboard needs and nothing more.
//! - **Hubs are not modelled.** The slot context built here always has a route string of 0, so a
//!   device behind a hub cannot be addressed. Milestone 242's own BUGS entry; the keyboards on
//!   the bench plug into a root port.
//!
//! Name: ratified 2026-10-07 (calef, pull request #1806's publish-ours review). The maintainer
//! comment reads: "one milestone per crate, minted now, each keeping its tree name (all free on
//! crates.io as of today): [...] `extensible_host_controller_interface` (no offer to rust-osdev's
//! dormant `xhci`, per today's only-our-dependencies rule) [...] Names are ratified now and
//! permanent on first publication." His words: "Yes, one per crate". Minted provisionally
//! 2026-10-04 for milestone 242 (USB host and HID): the expansion of xHCI, by DECISIONS §154 (the
//! acronym test is whether the phrase is spoken) as `crates/non_volatile_memory_express` was
//! named; too long for a program (`nifefs`'s 32-byte limit), so the program that drives it is
//! `usb_keyboard_driver`, which says what it is for. Milestone 820 (proven xHCI logic, released on
//! its own) publishes it.

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
