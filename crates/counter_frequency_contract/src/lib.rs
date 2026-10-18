//! **The counter frequency page**: how many [`user_mode_runtime::now`](../user_mode_runtime/fn.now.html)
//! ticks make a second, on the architectures with no register that says so. aarch64 reads
//! `CNTFRQ_EL0` and maps no page. `x86_64`'s TSC rate comes from `CPUID` leaf `0x15` or is
//! measured, and `riscv64`'s is `/cpus/timebase-frequency` in a device tree no process can read:
//! either way the kernel learns it once, at boot. This crate is the page's one definition, so the
//! kernel (the one writer) and `user_mode_runtime::cntfrq` (every reader) cannot drift on its
//! layout or address, as `clock_contract` does for the wall clock. A hardcoded rate is not an
//! answer: radon's `time` CSR runs at 4 MHz, not QEMU's 10 MHz (calef, 2026-09-21: "the program
//! only returns accurate numbers").
//!
//! Every process that calls `now` needs the rate, with no capability to ask for, so the page is
//! mapped unconditionally: by `kernel::user::load` and `kernel::user::map_timebase_page` for spaces
//! the kernel builds, and by `supervision_protocol::build_child_space`, where the parent writes a
//! fresh page from its own `user_mode_runtime::cntfrq_checked` reading. A parent that does not know
//! the rate writes zeroes, and [`TimebasePage::hz`] reads that as `None`, never a fabricated rate.
//! The kernel writes the page once before mapping it read-only, so no seqlock is needed.
//!
//! # BUGS
//!
//! - The TSC is assumed invariant (`CPUID.80000007H:EDX[8]` is not checked, as in
//!   `arch::x86_64::timer`); a part that changes its TSC rate leaves every process reading a stale
//!   number. QEMU's TSC is invariant.
//! - A process spawned another way (the hand-built demo spaces that run raw machine code) has no
//!   page, and [`user_mode_runtime::cntfrq`](../user_mode_runtime/fn.cntfrq.html) there faults on
//!   the unmapped read. That is deliberate: dying says the rate is unknown, a plausible number
//!   would not. A new spawn path must map the page or not link anything that calls `cntfrq`.
//! - The rate is read once per process, so a later, better number could not reach it.
//!
//! Name: ratified 2026-10-17 (calef): "ratify counter_frequency_contract and fix the docs". The stem
//! `counter_frequency` was ruled on 2026-09-13. It mirrors `CNTFRQ_EL0`, the register this page
//! substitutes for. Refused `timebase_proto` (the oscillator, where the contents are the frequency),
//! `tick_rate_proto` (drops the `CNTFRQ_EL0` connection), `tsc_frequency_proto` and
//! `x86_64_timebase_proto` (one architecture's name for a neutral contract).

#![cfg_attr(not(test), no_std)]

/// The page's first eight bytes: an assembled page from an unrecognized or zeroed frame. ASCII,
/// unpadded, the same shape `clock_contract::MAGIC` and `environment_contract::MAGIC` use.
pub const MAGIC: [u8; 8] = *b"TIMEBAS1";

/// **The slowest counter this tree will believe**, 100 kHz.
///
/// Every rate check in this tree used to validate against *zero* and nothing else: aarch64 asserted
/// `freq > 0`, riscv64 asserted `hz > 0`, and `x86_64`'s PIT calibration had no band at all. A
/// firmware that reports 1 Hz, or a calibration that returns garbage because the host descheduled
/// the vCPU mid-measurement, passes all three and then poisons every number derived from it,
/// silently, which is the failure this whole file exists to prevent one layer up.
///
/// **The bound is deliberately far below anything real**, because a false refusal on somebody's
/// silicon is worse than the failure it prevents: it turns a machine that would have run into a
/// machine that will not boot. The slowest counter in this tree's world is radon's 4 MHz `time`
/// CSR; the 32.768 kHz watch crystal is the slowest thing anyone drives a timer from anywhere, and
/// 100 kHz sits above it on purpose, because nothing here is a watch. Anything under this is a
/// measurement that failed, not a part that is slow.
pub const MIN_PLAUSIBLE_HZ: u64 = 100_000;

/// **The fastest counter this tree will believe**, 100 GHz, and the same reasoning as
/// [`MIN_PLAUSIBLE_HZ`] read from the other end. No part ticks a counter at 100 GHz and none will
/// for a long time (the fastest here is a few GHz of TSC), so a number above this came from a
/// division that went the wrong way or a register that was never written. Generous by two orders of
/// magnitude on purpose: this band exists to catch nonsense, not to police hardware.
pub const MAX_PLAUSIBLE_HZ: u64 = 100_000_000_000;

/// Is `hz` a rate a real machine could plausibly have? See [`MIN_PLAUSIBLE_HZ`] and
/// [`MAX_PLAUSIBLE_HZ`] for where the bounds come from and why they are so wide.
///
/// One definition rather than three, because a band copied into each architecture's timer is a band
/// that drifts: the kernel's aarch64, riscv64 and `x86_64` boot paths all call this, at the point
/// each first learns its number.
///
/// # Examples
///
/// ```
/// use counter_frequency_contract::is_plausible;
///
/// assert!(is_plausible(4_000_000)); // radon's `time` CSR
/// assert!(is_plausible(62_500_000)); // QEMU's aarch64 virtual counter under TCG
/// assert!(!is_plausible(0)); // firmware never wrote it
/// assert!(!is_plausible(1)); // firmware wrote nonsense, and `> 0` would have believed it
/// ```
pub fn is_plausible(hz: u64) -> bool {
    (MIN_PLAUSIBLE_HZ..=MAX_PLAUSIBLE_HZ).contains(&hz)
}

const OFF_MAGIC: usize = 0;
const OFF_HZ: usize = OFF_MAGIC + 8;

/// The whole page's size in bytes: the magic, then one little-endian `u64`. Far under one frame
/// (4096 bytes), which is the unit this is mapped as.
pub const PAGE_BYTES: usize = OFF_HZ + 8;

/// **The fixed virtual address `kernel::user::load` maps this page at**, in every `x86_64` and
/// every `riscv64` process. Both sides of the page (the kernel's writer,
/// `user_mode_runtime::cntfrq`'s reader) agree on
/// this number through this crate rather than through two copies of a magic constant (CLAUDE.md
/// rule 7).
///
/// **`riscv64` gets its own address, because Sv39's low half is not big enough for this one.**
/// `x86_64` splits at bit 47 and the value below sits comfortably inside the low half; RISC-V under
/// Sv39 splits at bit 38 (`crates/paging/src/sv39.rs`), so every address at or above
/// `0x40_0000_0000` is not a user address at all and a mapping at the `x86_64` value would be
/// refused rather than merely unusual. See the `riscv64` arm's own comment for where its number
/// comes from.
///
/// **Deliberately far above every other low-half address this tree hands out.** When this was
/// sited, every program's ELF loaded at `0x40_0000`, its stack sat at `0x50_0000`, and individual
/// test fixtures mapped their own extra windows in the low few megabytes above them (a first
/// attempt at `0x60_0000` collided with `fixtures/src/window.rs`'s own `CTL_VA`, which a full-suite
/// run under `script/test --arch x86_64` caught as `AlreadyMapped`). Because this page is mapped
/// **unconditionally into every `x86_64` process** rather than opted into by one program's own
/// wiring, it could not share that convention's address space at all: any low-MiB address might be
/// exactly what some future program's own segments or some future fixture's own window wants.
///
/// **That collision is why `crates/address_space_map` exists** (milestone 206 (a program image has under 896 KiB), DECISIONS §171 (where a program image starts) option
/// D). Under the map, `0x60_0000` is a pair page by definition and a page like this one belongs in
/// the map's `PROCESS_PAGES`, beside the stack, where it would share the stack's page tables
/// instead of paying for three of its own. It has not moved there: this file is generated into
/// std's PAL, so the move is a farm rebuild and a benchmark re-baseline, and it is recorded in the
/// map's roadmap block as the next step rather than folded into the change that drew the map. The
/// test module pins that it stays above every band on the map meanwhile. `0x0000_7000_0000_0000` sits at seven-eighths of the low half's own ceiling
/// (`x86_64`'s `SPLIT_SHIFT` is 47, so the low half is every address under `0x0000_8000_0000_0000`;
/// see `crates/paging/src/x86_64.rs`), leaving roughly 26 TiB of headroom below the boundary and
/// none of this tree's other conventions anywhere near it.
#[cfg(not(target_arch = "riscv64"))]
pub const PAGE_VA: u64 = 0x0000_7000_0000_0000;

/// The same page's fixed address under Sv39, where the low half ends at `0x40_0000_0000` and the
/// value above is not addressable. `0x38_0000_0000` is seven-eighths of that ceiling, which is the
/// same rule the `x86_64` arm follows one address width down: 224 GiB in, with 8 GiB of headroom
/// below the boundary and every other user address this tree hands out (the highest is
/// `address_space_map::CODE_MODEL_CEILING`, 2 GiB, below which the whole map lies) more than two
/// hundred gigabytes below it.
///
/// The host build of this crate takes the `x86_64` arm, which is harmless: nothing in the `cfg(test)`
/// suite below depends on the value, only on the layout.
#[cfg(target_arch = "riscv64")]
pub const PAGE_VA: u64 = 0x0000_0038_0000_0000;

/// Build a counter frequency page's bytes for `hz`, the calibrated (or `CPUID`-reported) TSC rate in
/// hertz. The kernel writes the result into a fresh frame before mapping it read-only into any
/// process; nothing else ever constructs one of these.
///
/// # Examples
///
/// ```
/// use counter_frequency_contract::{TimebasePage, build_page};
///
/// let bytes = build_page(1_000_000_000);
/// // SAFETY: `bytes` is a live, aligned buffer of exactly `PAGE_BYTES` for this block.
/// let page = unsafe { TimebasePage::new(bytes.as_ptr() as u64) };
/// assert_eq!(page.hz(), Some(1_000_000_000));
/// ```
pub fn build_page(hz: u64) -> [u8; PAGE_BYTES] {
    let mut page = [0u8; PAGE_BYTES];
    page[OFF_MAGIC..OFF_MAGIC + 8].copy_from_slice(&MAGIC);
    page[OFF_HZ..OFF_HZ + 8].copy_from_slice(&hz.to_le_bytes());
    page
}

/// **The counter frequency page**, as seen through one process's read-only mapping of it at [`PAGE_VA`].
#[derive(Debug, Clone, Copy)]
pub struct TimebasePage {
    base: *const u8,
}

// SAFETY: the page has one writer (the kernel, before the page is ever mapped into a second
// address space) and every reader after that sees only immutable bytes, so there is no mutable
// aliasing across the `Send`/`Sync` boundary to protect against. Same reasoning as
// `environment_contract::ConfigPage`.
unsafe impl Send for TimebasePage {}
// SAFETY: as `Send` above.
unsafe impl Sync for TimebasePage {}

impl TimebasePage {
    /// Name the counter frequency page mapped at `va`.
    ///
    /// # Safety
    ///
    /// `va` must be a mapped, byte-aligned buffer of at least [`PAGE_BYTES`] bytes (either a page
    /// [`build_page`] wrote, or a zeroed frame, which reads as "unknown"), and it must stay
    /// mapped and unwritten by this process for as long as this value is used. `kernel::user::load`
    /// maps exactly such a page, read-only, at [`PAGE_VA`], into every `x86_64` process; see the
    /// crate's own `BUGS` section for the one spawn path that does not.
    pub const unsafe fn new(va: u64) -> Self {
        TimebasePage {
            base: va as *const u8,
        }
    }

    /// # Safety
    ///
    /// Upheld by [`new`](Self::new)'s contract: `base` names at least `PAGE_BYTES` mapped,
    /// stable bytes.
    fn bytes(&self) -> &[u8] {
        // SAFETY: `new`'s contract.
        unsafe { core::slice::from_raw_parts(self.base, PAGE_BYTES) }
    }

    /// The calibrated TSC rate in hertz, or `None` if this page is unrecognized (a zeroed frame,
    /// the state before calibration has run, or a page belonging to something else). Never a
    /// fabricated default: a parent that does not know the rate writes zeroes.
    ///
    /// # Examples
    ///
    /// A zeroed frame reads as unknown, never as a fabricated rate:
    ///
    /// ```
    /// use counter_frequency_contract::TimebasePage;
    ///
    /// let zeroed = [0u8; counter_frequency_contract::PAGE_BYTES];
    /// // SAFETY: as above.
    /// let page = unsafe { TimebasePage::new(zeroed.as_ptr() as u64) };
    /// assert_eq!(page.hz(), None);
    /// ```
    pub fn hz(&self) -> Option<u64> {
        let bytes = self.bytes();
        if bytes[OFF_MAGIC..OFF_MAGIC + 8] != MAGIC {
            return None;
        }
        let mut raw = [0u8; 8];
        raw.copy_from_slice(&bytes[OFF_HZ..OFF_HZ + 8]);
        Some(u64::from_le_bytes(raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The round trip: build a page for a real-looking rate, read it back through a raw pointer
    /// the way a mapped frame would be named.
    /// **The page stays above every band of the address-space map**, on the architecture the
    /// host build stands in for and on Sv39's own arm, so a program's image, stack, heap or windows
    /// can never be placed over it.
    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn the_page_is_above_every_band_of_the_address_space_map() {
        assert!(PAGE_VA >= address_space_map::CODE_MODEL_CEILING);
        assert!(0x0000_0038_0000_0000 >= address_space_map::CODE_MODEL_CEILING);
    }

    #[test]
    fn a_built_page_round_trips_its_rate() {
        let bytes = build_page(2_400_000_000);
        // SAFETY: `bytes` is a live, aligned buffer of exactly PAGE_BYTES for this call's scope.
        let page = unsafe { TimebasePage::new(bytes.as_ptr() as u64) };
        assert_eq!(page.hz(), Some(2_400_000_000));
    }

    /// A zeroed frame (nothing has ever written a page into it, or calibration has not run) reads
    /// as "unknown", not as a fabricated 0 Hz or a stale value.
    #[test]
    fn a_zeroed_frame_reads_as_unknown() {
        let zeroed = [0u8; PAGE_BYTES];
        // SAFETY: as above.
        let page = unsafe { TimebasePage::new(zeroed.as_ptr() as u64) };
        assert_eq!(page.hz(), None);
    }

    /// An unrecognized magic refuses the rate even when the bytes after it look plausible: the
    /// magic check runs first, the same discipline `environment_contract`'s equivalent test pins.
    #[test]
    fn an_unrecognized_magic_reads_as_unknown() {
        let mut bytes = build_page(1_000_000_000);
        bytes[OFF_MAGIC] ^= 0xff;
        // SAFETY: as above.
        let page = unsafe { TimebasePage::new(bytes.as_ptr() as u64) };
        assert_eq!(page.hz(), None);
    }

    /// The band accepts every rate this tree has actually met and refuses the two shapes the old
    /// `> 0` assertions let through: a register firmware never wrote, and a register firmware wrote
    /// nonsense into. The real rates are pinned by value so that a later reader narrowing the band
    /// for tidiness fails here rather than on somebody's board.
    #[test]
    fn the_band_accepts_real_rates_and_refuses_nonsense() {
        for hz in [
            4_000_000,     // radon, the VisionFive 2's `time` CSR
            10_000_000,    // QEMU virt, riscv64
            62_500_000,    // QEMU virt, aarch64 under TCG
            24_000_000,    // a common ARM board crystal
            3_000_000_000, // a TSC
        ] {
            assert!(is_plausible(hz), "{hz} Hz is a rate a real machine has");
        }
        for hz in [0, 1, 1_000, u64::MAX] {
            assert!(!is_plausible(hz), "{hz} Hz is a measurement that failed");
        }
    }

    /// The layout constants do not overlap and the page fits in one frame, pinned so a mutant
    /// swapping an offset is caught here rather than by a torn read on real hardware. The same
    /// shape `environment_contract::the_layout_offsets_do_not_overlap` and
    /// `clock_contract`'s own layout test use.
    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn the_layout_offsets_do_not_overlap() {
        assert_eq!(OFF_HZ, 8);
        assert_eq!(PAGE_BYTES, 16);
        assert!(PAGE_BYTES < 4096, "the page must fit in one frame");
    }
}
