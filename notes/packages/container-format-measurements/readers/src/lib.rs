//! The readers this measurement wrote: NIFEPKG2 and a ustar reader with pax `path` records, each
//! borrowing from its buffer and allocation-free. Prototypes for measuring, not for the tree.
#![no_std]
pub mod manifest;
pub mod n2;
pub mod tar_own;
