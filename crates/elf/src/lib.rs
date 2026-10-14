//! An ELF64 loader's front half: parse, validate, and hand back the segments to map.
//!
//! **Pure logic, so it compiles for the host and its tests run in milliseconds** with no
//! emulator (DECISIONS §7). Nothing in here knows what a page table is. It answers one
//! question: *what does this file want me to put where, and with what permissions?*
//!
//! Deliberately narrow. We parse **static, little-endian, `ET_EXEC`** binaries for **this build's
//! own machine** and nothing else. No dynamic linking, no relocations, no interpreter, no PIE. Every one of those
//! is a real feature and every one of them is also a way for a file to ask us to do something
//! surprising, and we would rather say "no" in eleven lines than "maybe" in a thousand.
//!
//! See notes/elf.md.
//!
//! Name: ratified 2026-09-18 (calef, `design/decisions/` §154 (the acronym test is whether the
//! phrase is spoken)): "executable and linkable format" is not a phrase people say, and an expert
//! meeting it translates it back to `elf`, so the name stays. Refused
//! `executable_and_linkable_format` (§154).

#![no_std]

mod note;
pub use note::{NoteError, NoteSearch, NoteSegment, NoteSegments};

/// Machine-checked proofs of the loader's front half (DECISIONS §14, milestone 18).
///
/// This crate exists to be panic-free on hostile input, and one piece of that is proved here. The
/// larger claim, that `Elf::parse` is *total* (never panics) over every byte string, turned out to
/// be past what bounded model checking can do at a useful input size, and that is written up rather
/// than hidden. See notes/verification.md ("Where BMC hit a wall: the ELF parser").
///
/// The short version: `parse` has an `O(n^2)` overlap loop over up to `MAX_PHNUM = 64` program
/// headers, and Kani bounds that loop by the linear 64 cap it can see rather than the tighter
/// nonlinear table-size bound, so it must unroll 64 deep. Combined with symbolic slice offsets
/// (`phoff` and `p_offset` place reads at symbolic positions in a symbolic array), the solver did
/// not return in minutes, even after pinning the header count to a single segment. Totality here
/// wants a loop-invariant tool (Verus) or the leaf arithmetic factored into a pure function provable
/// on its own, both noted in the write-up. The tests below still cover the specific hostile files by
/// example.
#[cfg(kani)]
mod verification;

#[cfg(test)]
mod tests;

mod error;
mod machine;
mod parse;
mod segment;

pub use error::Error;
use machine::*;
pub use machine::{
    FOREIGN_MACHINES, KNOWN_MACHINES, NATIVE_MACHINE, machine_no_nife_build_accepts,
};
pub use parse::Elf;
use parse::*;
use segment::*;
pub use segment::{PF_R, PF_W, PF_X, Segment};
