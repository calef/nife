//! **What the kernel hands the driver** at spawn.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// The role word's low half when `arg0` is an `e1000e` server handoff. Disjoint from `net_stack`'s
/// existing roles (0 is the virtio server, small integers are socket clients).
pub const ROLE_TAG: u64 = 0xe1e0;

/// **What the kernel hands `net_stack` at spawn**, in two of its three argument words (the third
/// stays the listen grant, as for the virtio server).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Handoff {
    /// The station address, read by the kernel from receive-address entry 0. The process has no
    /// way to read it itself (page 5 is not mapped) and no way to change it.
    pub mac: [u8; 6],
    /// The DMA region's physical base. Descriptors speak physical addresses (identity-mapped by
    /// the IOMMU domain), so the process cannot compute them from its virtual window.
    pub data_plane_phys: u64,
}

impl Handoff {
    /// `[arg0, arg1]`: the tag and the MAC address in the role word, the base in the second.
    pub fn pack(&self) -> [u64; 2] {
        let mut mac = [0u8; 8];
        mac[..6].copy_from_slice(&self.mac);
        [
            ROLE_TAG | u64::from_le_bytes(mac) << 16,
            self.data_plane_phys,
        ]
    }

    /// The inverse of [`Handoff::pack`], refusing a role word without the tag, a base that is
    /// zero or not page aligned, and a MAC address no station can own.
    pub fn unpack(arg0: u64, arg1: u64) -> Option<Handoff> {
        if arg0 & 0xffff != ROLE_TAG || arg1 == 0 || !arg1.is_multiple_of(PAGE) {
            return None;
        }
        let b = (arg0 >> 16).to_le_bytes();
        let mac = [b[0], b[1], b[2], b[3], b[4], b[5]];
        if mac == [0; 6] || mac[0] & 1 != 0 {
            return None;
        }
        Some(Handoff {
            mac,
            data_plane_phys: arg1,
        })
    }

    /// Is `arg0` an `e1000e` server's role word at all? `net_stack` dispatches on this.
    pub fn is_role(arg0: u64) -> bool {
        arg0 & 0xffff == ROLE_TAG
    }
}
