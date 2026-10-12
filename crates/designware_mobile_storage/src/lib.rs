#![cfg_attr(not(test), no_std)]
//! **The Synopsys `DesignWare` Mobile Storage Host Controller, as pure logic**: radon's microSD
//! slot and eMMC socket (milestone 53 (the board's own peripherals: network and storage on real
//! silicon); notes/designware-mobile-storage.md).
//!
//! The `StarFive` JH7110 has two of these, at `0x1601_0000` (the VisionFive 2's eMMC socket) and
//! `0x1602_0000` (its microSD slot, the card radon boots from). [`jh7110`] has the evidence for
//! what the part is. This crate is the whole driver behind one small trait,
//! [`host::Registers`]: a window to read and write, and a clock. The kernel's
//! `kernel/src/designware_mobile_storage.rs` implements that trait over a mapped device window, and
//! that is all it does. Nothing here dereferences a pointer, which is what lets every sequence be
//! tested on a host against `sim` when the device exists on one desk and in no emulator.
//!
//! | module | what it holds |
//! |---|---|
//! | [`regs`] | the register map, and what `VERID` and `HCON` say about the FIFO |
//! | [`command`] | SD and eMMC commands, and the `CMD` word that sends each |
//! | [`host`] | reset, clock, bus width, and one command with a polled data phase |
//! | [`card`] | the OCR, CID, CSD, R1 and `EXT_CSD`, decoded |
//! | [`sd`] | identification, and block reads and writes |
//! | [`partition`] | the MBR, and the only sectors a write test may touch |
//! | [`bench`](mod@bench) | the bench step's read-only probe and scratch write test |
//! | [`serve`] | what the kernel and the EL0 block server agree on: windows, the card window, the spawn words |
//! | [`jh7110`] | the device-tree query and the board's constants |
//!
//! # Examples
//!
//! The card clock never runs faster than asked, and a divider the field cannot hold is refused
//! rather than written as zero (the fastest setting there is):
//!
//! ```
//! use designware_mobile_storage::host::{card_clock, clock_divider};
//!
//! assert!(card_clock(50_000_000, clock_divider(50_000_000, 400_000).unwrap()) <= 400_000);
//! assert_eq!(clock_divider(50_000_000, 50_000), None);
//! ```
//!
//! A write test on radon's boot card may only land before the first partition:
//!
//! ```
//! use designware_mobile_storage::partition::{Entry, Mbr};
//!
//! let fat = Entry { kind: 0x0b, start: 2048, sectors: 1 << 20 };
//! let mbr = Mbr { entries: [fat, Entry::default(), Entry::default(), Entry::default()] };
//! assert_eq!(mbr.scratch(8), Some((2040, 8)));
//! ```
//!
//! # Where the code comes from, and the license that comes with it
//!
//! Read, not recalled, on 2026-10-06 (UTC). **OpenBSD's `dwmmc` driver is the source this crate
//! adapts**: `sys/dev/fdt/dwmmc.c` revision 1.33 (2026-03-11), which matches both
//! `starfive,jh7110-mmc` and `snps,dw-mshc` and so drives exactly this part on exactly this board.
//! The register map, the reset, clock and bus-width sequences, the command flags and the polled
//! FIFO transfer are its, adapted; the identification order follows its `sdmmc_mem.c`. It is ISC
//! licensed and carries this notice, which the license requires be kept:
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
//! The crates.io candidate, `starfive-jh7110-dwmmc` 0.1.8 (Apache-2.0, `rcore-os/tgoskits`), was
//! read and **not taken**, and the reason is recorded in notes/designware-mobile-storage.md: it is
//! a 333-line wrapper over `dwmmc-host` 0.4.2 (5,253 lines) and three more crates from the same
//! tree (`sdmmc-host`, `sdmmc-protocol`, `dma-api`), sixteen releases since August, with an
//! IDMAC-only data path and volatile accesses inside the driver core, where this tree's split puts
//! them outside it. Its JH7110 constants (a 50 MHz reference, a 32-word, 32-bit FIFO) agree with
//! the device trees and were used as a third cross-check. Linux's `dw_mmc.c` and
//! `dw_mmc-starfive.c`, U-Boot's `dw_mmc.c` and both device trees are GPL and were read only for
//! facts about the hardware (where the FIFO is by release, which clocks and resets the part takes),
//! each named where it is used. No GPL code was copied.
//!
//! # BUGS
//!
//! - **Nothing here has touched the device.** Every test runs against `sim`, which models the
//!   contract as the databook and OpenBSD describe it. The bench step in
//!   notes/designware-mobile-storage.md is what makes it a measurement.
//! - **The CPU moves every byte.** The IDMAC (the controller's own DMA) is not used; see
//!   [`host`]'s header for why the first silicon read goes through the FIFO. Throughput is
//!   whatever polled MMIO gives, and the bench step prints it.
//! - **Default speed only**: 25 MHz, 4-bit on SD, 1-bit on eMMC. No high-speed switch, no UHS-I,
//!   no HS200 and so none of the JH7110's sample-phase tuning (`dw_mmc-starfive.c`'s
//!   `execute_tuning`), no 8-bit eMMC bus.
//! - **Polled, not interrupt-driven.** `INTMASK` stays zero. The vendor tree names no interrupt for
//!   either controller; mainline names 74 and 75.
//! - **A 64-bit FIFO is refused.** OpenBSD handles one; whether radon's is 32-bit is the first
//!   thing the bench step reads (`HCON`).
//! - **Which part of radon's card the EL0 server serves is undecided.** [`serve::Window`] carries it
//!   and the server cannot reach past it; who chooses it for the booted system is an architect's
//!   call, in the 53 block.
//! - **No card detect or write-protect handling.** The probe prints `CDETECT` and proceeds; radon's
//!   slot has a card in it or there is nothing to boot from.
//!
//! Name: ratified 2026-10-07 (calef, pull request #1806's publish-ours review). The maintainer
//! comment reads: "one milestone per crate, minted now, each keeping its tree name (all free on
//! crates.io as of today) [...] Names are ratified now and permanent on first publication." His
//! words: "Yes, one per crate". Minted provisionally 2026-10-06 (UTC) by milestone 53's storage
//! lane: "`DesignWare` Mobile Storage" is the Synopsys databook's name for the part (`DW_mshc`, the
//! `snps,dw-mshc` compatible read aloud), spelled out as §154 (the acronym test is whether the
//! phrase is spoken, applied recursively) asks and as the same milestone's `designware_ethernet`
//! spells its own. The provisional block offered `designware_mmc`, after Linux's `dw_mmc`, and a
//! name for what the crate does (`sd_card`), which would hide that the eMMC socket is driven by it
//! too. The ruling kept the tree name. Milestone 817 (a proven `DesignWare` SD and eMMC host,
//! released on its own) publishes it.

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
