//! **The GTDT**: an Arm machine's architected timers and their interrupts.
//!
//! Moved out of `acpi.rs` unchanged but for visibility by milestone 875 (the ACPI table
//! parser is split along its seams), under §293 (the ACPI table parser splits into one private
//! module per table).
//!
//! Name: provisional (milestone 875's lane, 2026-10-12 UTC).

use super::*;

/// **The Generic Timer Description Table** (signature `GTDT`), which is where an Arm machine states
/// the architected timer's interrupts.
///
/// x86 has no counterpart: its timers are the HPET and the local APIC, both described elsewhere. On
/// a device-tree machine the same facts are the `arm,armv8-timer` node's `interrupts` property, and
/// the two describe the same four timers in the same order, which is what makes a tree writer able
/// to carry one into the other.
///
/// **A GSIV is an absolute INTID**, not a bank-relative number: the virtual timer's 27 here is the
/// same 27 the GIC delivers, where the device tree spells it `<1 11 ...>` (PPI 11, and PPIs start
/// at 16). Converting between the two is the writer's job, not this parser's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gtdt {
    /// The secure EL1 timer's interrupt, and its flags. A kernel at EL1 non-secure never takes it.
    pub secure_el1: (u32, u32),
    /// The non-secure EL1 physical timer (`CNTP_*`).
    pub non_secure_el1: (u32, u32),
    /// **The virtual timer** (`CNTV_*`), which is the one this kernel arms.
    pub virtual_el1: (u32, u32),
    /// The EL2 physical timer (`CNTHP_*`).
    pub el2: (u32, u32),
}

/// The GTDT flag bit that says the line is edge-triggered rather than level-triggered.
pub const GTDT_EDGE_TRIGGERED: u32 = 1 << 0;
/// The GTDT flag bit that says the line is asserted low rather than high.
pub const GTDT_ACTIVE_LOW: u32 = 1 << 1;

/// The bytes of a GTDT body this decoder needs, which is everything up to the platform-timer list.
const GTDT_FIXED_LEN: usize = 60;

/// Decode the GTDT's four architected timers. `body` begins after the SDT header.
///
/// The platform timers past the fixed part (the memory-mapped `CNTCTLBase` blocks and the watchdog
/// entries) are **not** decoded: nothing here drives them, and the device tree binding this feeds
/// has no place to put them.
pub fn parse_gtdt(body: &[u8]) -> Result<Gtdt, AcpiError> {
    if body.len() < GTDT_FIXED_LEN {
        return Err(AcpiError::Truncated);
    }
    Ok(Gtdt {
        secure_el1: (u32(body, 12), u32(body, 16)),
        non_secure_el1: (u32(body, 20), u32(body, 24)),
        virtual_el1: (u32(body, 28), u32(body, 32)),
        el2: (u32(body, 36), u32(body, 40)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acpi::tests::every_short_prefix_is_refused;

    /// The GTDT's four timers, in the order the device tree binding also states them. The numbers
    /// are QEMU `virt`'s: secure 29, non-secure 30, virtual 27, EL2 26, all level-triggered.
    #[test]
    fn the_gtdt_names_four_timers_and_their_trigger_modes() {
        let mut body = [0u8; GTDT_FIXED_LEN];
        for (at, gsiv) in [(12usize, 29u32), (20, 30), (28, 27), (36, 26)] {
            body[at..at + 4].copy_from_slice(&gsiv.to_le_bytes());
        }
        // The virtual timer, and only it, marked edge-triggered and active low, so a decoder that
        // read one flags word for all four would fail here.
        body[32..36].copy_from_slice(&(GTDT_EDGE_TRIGGERED | GTDT_ACTIVE_LOW).to_le_bytes());
        let g = parse_gtdt(&body).expect("a full-length GTDT");
        assert_eq!(g.secure_el1, (29, 0));
        assert_eq!(g.non_secure_el1, (30, 0));
        assert_eq!(g.virtual_el1, (27, GTDT_EDGE_TRIGGERED | GTDT_ACTIVE_LOW));
        assert_eq!(g.el2, (26, 0));
        every_short_prefix_is_refused(&body, GTDT_FIXED_LEN, parse_gtdt);
    }

    // ---- survivor triage of the 2026-10-03 census ---------------------------------------------

    /// The GTDT interrupt flags are the bits ACPI numbers: bit 0 edge, bit 1 active low.
    #[test]
    fn the_gtdt_flag_bits_are_the_ones_acpi_numbers() {
        assert_eq!(GTDT_EDGE_TRIGGERED, 0b01);
        assert_eq!(GTDT_ACTIVE_LOW, 0b10);
    }
}
