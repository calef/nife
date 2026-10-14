//! **The I219's own bring-up, ported from FreeBSD's `e1000` shared code** (milestone 494 (a driver
//! for the network card a PC actually has), after calef's amendment of 2026-10-04 to §46 (thin
//! primitives or whole subsystems): outside the kernel and the Kani-proved crates, take or adapt
//! first).
//!
//! The I219 is not a PCIe NIC in the 82574L's sense. It is a MAC inside the platform controller hub
//! (PCH) talking to a separate PHY, shared with the Management Engine, and it carries workarounds
//! that only field exposure finds. FreeBSD's Intel shared code (BSD-3-Clause, notice below) is the
//! only source this tree reads for them; Linux's `e1000e` is GPL and was not consulted for any of
//! it. Each item names the FreeBSD function it comes from, read from `sys/dev/e1000/` on FreeBSD's
//! `main`, on 2026-10-04 for the first three and 2026-10-05 for the rest:
//!
//! - [`ulp`]: leaving ultra-low-power mode through the Management Engine, from
//!   `e1000_disable_ulp_lpt_lp` (`e1000_ich8lan.c`), its ME branch.
//! - [`flush`]: the descriptor-ring flush SPT parts need before a reset, from `em_flush_desc_rings`,
//!   `em_flush_tx_ring` and `em_flush_rx_ring` (`if_em.c`).
//! - [`reset`]: the MAC-register constants around the global reset, from `e1000_reset_hw_ich8lan`
//!   (`e1000_ich8lan.c`) and `e1000_disable_pcie_master_generic` (`e1000_mac.c`).
//! - [`phy`]: MDIO through `MDIC`, the software/firmware semaphore, and the PCH PHY's paged
//!   registers.
//! - [`nvm`]: reading the `GbE` region of the flash through BAR0, as Sunrise Point does it.
//! - [`sequence`]: the bring-up itself, in FreeBSD's order: the PHY workarounds at attach
//!   (`e1000_init_phy_workarounds_pchlan`, which holds the ULP exit without an ME, the
//!   `SMBus`-to-PCIe switch and the `LANPHYPC` power cycle), the global reset with the PHY
//!   (`e1000_reset_hw_ich8lan`, `e1000_post_phy_reset_ich8lan`), and the hardware and copper-link
//!   setup (`e1000_init_hw_ich8lan`, `e1000_setup_copper_link_pch_lpt`).
//!
//! Every sequence is generic over [`Hw`], so it runs on the host against a simulated I219 (this
//! module's `sim`) as well as in the kernel. QEMU's 82574L is not a PCH part and never reaches
//! [`sequence`]; it does reach [`phy::id_at`], which shares [`phy::read_mdic`] with everything
//! here, so the MDIO primitive is the one piece the QEMU gates prove.
//!
//! The constants are facts about the hardware; the sequences are FreeBSD's, adapted, and carry
//! Intel's licence below as its first condition requires.
//!
//! ```text
//! SPDX-License-Identifier: BSD-3-Clause
//!
//! Copyright (c) 2001-2020, Intel Corporation
//! All rights reserved.
//!
//! Redistribution and use in source and binary forms, with or without
//! modification, are permitted provided that the following conditions are met:
//!
//!  1. Redistributions of source code must retain the above copyright notice,
//!     this list of conditions and the following disclaimer.
//!
//!  2. Redistributions in binary form must reproduce the above copyright
//!     notice, this list of conditions and the following disclaimer in the
//!     documentation and/or other materials provided with the distribution.
//!
//!  3. Neither the name of the Intel Corporation nor the names of its
//!     contributors may be used to endorse or promote products derived from
//!     this software without specific prior written permission.
//!
//! THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
//! AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
//! IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
//! ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE
//! LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
//! CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
//! SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
//! INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
//! CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
//! ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
//! POSSIBILITY OF SUCH DAMAGE.
//! ```
//!
//! # BUGS
//!
//! - **None of it has run on silicon.** QEMU's 82574L is not a PCH part, so [`sequence`] is
//!   skipped under the gates ([`is_pch`] is false for it) and is proved only against `sim`, which
//!   is this crate's reading of FreeBSD and not the device. xenon's bench boot is its first
//!   execution (notes/e1000e.md).
//! - **The link-up half is not ported.** FreeBSD reconfigures the PHY each time link comes up
//!   (`e1000_check_for_copper_link_ich8lan`: the EMI receive configuration, the PLL clock gate,
//!   the 776.20 pointer gap at 1000 Mb/s, the beacon duration, LTR and OBFF), from a link-change
//!   interrupt this driver does not take. A link that comes up and then loses frames, especially
//!   at 10 or 100 Mb/s, points here first. About 250 lines of FreeBSD; proposed in milestone 494's
//!   Follow-on.
//! - **Flow control is not configured, so it is not advertised.** FreeBSD advertises symmetric
//!   pause and programs the watermarks to match (`e1000_set_fc_watermarks_generic`); this driver
//!   programs neither, and advertises neither ([`sequence::setup_copper_link`]).
//! - **The transmit flush diverges from FreeBSD in one place**, said at [`flush::tail_after`].
//! - **One read in FreeBSD's `SMBus` unforce is not mirrored.** In `e1000_phy_is_accessible_pchlan`,
//!   a failed read of `CV_SMB_CTRL` leaves the previous register's value in the variable that is
//!   then written back with one bit cleared. This port writes only when the read succeeded.
//! - **The licence's second condition is not met by any binary this tree ships yet.** A binary
//!   redistribution must reproduce Intel's notice "in the documentation and/or other materials".
//!   The source carries it here and in each adapted file; nothing that ships an image (the stick, a
//!   package) carries a third-party notices file, and none exists in the tree. Milestone 494's
//!   block records it.

pub mod nvm;
pub mod phy;
pub mod sequence;
#[cfg(test)]
mod sim;

/// MAC registers the I219 adds. Offsets from FreeBSD's `e1000_regs.h` and `e1000_ich8lan.h`.
pub mod regs;

/// Leaving ultra-low-power (ULP) mode through the Management Engine.
pub mod ulp;

/// The SPT descriptor-ring flush. FreeBSD's comment: "In I219, the descriptor rings must be
/// emptied before resetting the HW ... Failure to do this will cause the HW to enter a unit hang
/// state which can only be released by PCI reset on the device."
pub mod flush;

/// The MAC-register steps around the global reset.
pub mod reset;

#[cfg(test)]
mod tests;

mod hardware;

use hardware::*;
pub use hardware::{Hw, is_pch};
