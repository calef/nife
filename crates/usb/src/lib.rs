#![no_std]
//! **What a USB device says, what a host asks it, and what a boot keyboard's report means**
//! (milestone 242, USB host and HID; notes/usb.md).
//!
//! The half of a USB keyboard driver that is about USB rather than about the host controller. The
//! controller half (rings, contexts, doorbells) is `crates/extensible_host_controller_interface`;
//! the volatile half that touches either is `components/src/usb_keyboard_driver.rs`. This crate
//! touches nothing: bytes in, a decision out.
//!
//! It is also **the part of the driver that reads untrusted input**. Every descriptor and every
//! report is whatever the device chose to send, and a device is whatever a stranger plugged in.
//! So the parsers here are total: they bound every walk, check every length before reading through
//! it, and answer a [`Refusal`] that names what was wrong rather than panicking or guessing. The
//! Kani harnesses at the bottom prove that for every input up to a bound, not for the inputs a
//! test happened to try.
//!
//! Three pieces:
//!
//! - [`descriptor`]: the device descriptor, the configuration header, and the one walk this
//!   driver needs, [`descriptor::find_boot_keyboard`], which finds a keyboard interface that
//!   speaks the boot protocol and its interrupt IN endpoint.
//! - [`request`]: the four standard and class requests the driver sends, as the eight-byte setup
//!   packet a control transfer carries.
//! - [`boot_keyboard`]: the eight-byte boot report, and the difference between two of them as key
//!   presses and releases in Linux's evdev numbering, which is what `video_terminal::keymap`
//!   already turns into terminal bytes for the virtio keyboard. One layout table in the tree, two
//!   devices feeding it.
//!
//! # BUGS
//!
//! - **Boot protocol only.** A keyboard is found by its interface triple (3, 1, 1) and read in the
//!   fixed eight-byte layout; nothing here parses a HID *report descriptor*. A keyboard that
//!   declares no boot interface is refused with [`Refusal::NoBootKeyboard`].
//! - **The first boot keyboard interface wins.** A device with two (rare) gets its first; a second
//!   keyboard on another port is not looked for by this crate, and the driver serves one.
//! - **No key repeat.** A boot keyboard reports state, not events, and does not repeat. Typematic
//!   repeat is the host's job and nothing here does it; holding a key types it once.
//! - **Caps Lock, Num Lock and the LEDs are not handled**: the lock keys are reported as key codes
//!   and `video_terminal::keymap` maps none of them, so they do nothing.
//!

//! Name: provisional, for milestone 242 (USB host and HID). `usb` and not `universal_serial_bus`
//! by DECISIONS §154 (the acronym test is whether the phrase is spoken), as `pci` was: nobody says
//! the expansion.

pub mod boot_keyboard;
pub mod descriptor;
pub mod request;

pub use descriptor::Refusal;

#[cfg(kani)]
mod verification;
