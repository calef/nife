#![cfg_attr(not(test), no_std)]
//! **The Synopsys `DesignWare` Mobile Storage Host Controller, as pure logic**: radon's microSD
//! slot and eMMC socket (notes/designware-mobile-storage.md). The whole driver sits behind
//! [`host::Registers`], a window and a clock; `kernel/src/designware_mobile_storage.rs` implements
//! it over a mapped window and nothing more, so every sequence is tested against `sim` on a host.
//! [`jh7110`] has the evidence for the part and the board's constants. Modules: [`regs`],
//! [`command`], [`host`], [`card`], [`sd`], [`partition`], [`bench`](mod@bench), [`serve`].
//!
//! Adapted from OpenBSD's `sys/dev/fdt/dwmmc.c` revision 1.33 (register map, reset, clock, bus
//! width, command flags and the polled FIFO transfer; identification after its `sdmmc_mem.c`),
//! under the ISC license, whose notice must be kept. GPL sources (Linux, U-Boot, the device trees)
//! were read only for hardware facts; no GPL code was copied. The note records why the crates.io
//! candidate was not taken.
//!
//! ```text
//! Copyright (c) 2017 Mark Kettenis
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
//! - Nothing here has touched the device; the bench step in the note makes it a measurement.
//! - The CPU moves every byte through the FIFO; the IDMAC is unused ([`host`] says why).
//! - Default speed only (25 MHz, 4-bit SD, 1-bit eMMC); no tuning, no 8-bit bus. Polled, with
//!   `INTMASK` zero. A 64-bit FIFO is refused. No card-detect or write-protect handling.
//! - Which part of radon's card the EL0 server serves ([`serve::Window`]) is an architect's call.
//!
//! Name: ratified 2026-10-07 (calef, #1806): the databook's name for the part, spelled out as §154
//! (the acronym test is whether the phrase is spoken) asks; permanent on first publication.

pub mod bench;
pub mod card;
pub mod command;
pub mod host;
pub mod jh7110;
pub mod partition;
pub mod regs;
pub mod sd;
pub mod serve;

#[cfg(test)]
mod sim;

#[cfg(test)]
mod tests;

#[cfg(kani)]
mod proofs {
    use crate::host::fifo_step;
    use crate::partition::{Entry, Mbr};
    use crate::sd::in_range;

    /// No partition table makes the write test's range touch a partition or sector 0.
    ///
    /// Falsification: replayable `crates/designware_mobile_storage/falsifications/proofs.the_scratch_range_never_touches_a_partition_or_sector_zero.patch`
    #[kani::proof]
    #[kani::unwind(5)]
    fn the_scratch_range_never_touches_a_partition_or_sector_zero() {
        let entry = || Entry {
            kind: kani::any(),
            start: kani::any(),
            sectors: kani::any(),
        };
        let mbr = Mbr {
            entries: [entry(), entry(), entry(), entry()],
        };
        let want: u32 = kani::any();
        if let Some((first, count)) = mbr.scratch(want) {
            assert!(first >= 1);
            assert!((1..=want).contains(&count));
            let k: u32 = kani::any();
            kani::assume(k < count);
            let sector = u64::from(first) + u64::from(k);
            for e in &mbr.entries {
                assert!(!e.contains(sector));
            }
        }
    }

    /// No block request the server translates reaches a sector outside its window.
    ///
    /// Falsification: replayable `crates/designware_mobile_storage/falsifications/proofs.a_served_block_never_leaves_its_window.patch`
    #[kani::proof]
    fn a_served_block_never_leaves_its_window() {
        let w = crate::serve::Window {
            first: kani::any(),
            sectors: kani::any(),
            writable: kani::any(),
        };
        kani::assume(w.first.checked_add(w.sectors).is_some());
        let block: u64 = kani::any();
        let count: u64 = kani::any();
        if let Some(lba) = w.translate(block, count) {
            assert!(lba >= w.first);
            let span = count * crate::serve::SECTORS_PER_BLOCK;
            assert!(lba + span <= w.first + w.sectors);
        }
    }

    /// A FIFO step never moves more than is owed, and a write step never more than the FIFO has
    /// room for, whatever `STATUS` says.
    ///
    /// Falsification: replayable `crates/designware_mobile_storage/falsifications/proofs.a_fifo_step_never_overruns_the_fifo_or_the_buffer.patch`
    #[kani::proof]
    fn a_fifo_step_never_overruns_the_fifo_or_the_buffer() {
        let status: u32 = kani::any();
        let depth: u32 = kani::any();
        kani::assume((1..=4096).contains(&depth));
        let write: bool = kani::any();
        let remaining: u32 = kani::any();
        let n = fifo_step(status, depth, write, remaining);
        assert!(n <= remaining);
        assert!(n <= depth * 4);
        let filled = crate::regs::fifo_count(status);
        if write && filled <= depth {
            assert!(n <= (depth - filled) * 4);
        }
    }

    /// A range the driver admits is inside the card and inside what the command's 32-bit address
    /// can name.
    ///
    /// Falsification: replayable `crates/designware_mobile_storage/falsifications/proofs.an_admitted_range_is_inside_the_card_and_the_address.patch`
    #[kani::proof]
    fn an_admitted_range_is_inside_the_card_and_the_address() {
        let blocks: u64 = kani::any();
        let block_addressed: bool = kani::any();
        let lba: u64 = kani::any();
        let count: u64 = kani::any();
        if in_range(blocks, block_addressed, lba, count) {
            assert!(count >= 1);
            assert!(lba + count <= blocks);
            let last = lba + count - 1;
            if block_addressed {
                assert!(last <= u64::from(u32::MAX));
            } else {
                assert!((last + 1) * 512 <= 1 << 32);
            }
        }
    }
}
