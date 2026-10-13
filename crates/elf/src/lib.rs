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
//! # Examples
//!
//! Forging an ELF64 image by hand is twenty lines, and **that is the argument for this crate being a
//! host crate rather than kernel code.** Writing a malicious binary costs nothing here; producing one
//! from a real toolchain, getting it into an initial ramdisk, and booting QEMU to watch it be
//! refused would be a day's work and a slower test.
//!
//! ```
//! use elf::{Elf, Error, FOREIGN_MACHINES, NATIVE_MACHINE, PF_R, PF_W, PF_X};
//!
//! /// One `PT_LOAD` segment at 0x40_0000, four kilobytes of it, with whatever flags you like.
//! fn image(flags: u32, entry: u64) -> Vec<u8> {
//!     const EHDR: usize = 64;
//!     const PHDR: usize = 56;
//!     let mut v = vec![0u8; EHDR + PHDR];
//!     v[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
//!     v[4] = 2; // ELFCLASS64
//!     v[5] = 1; // ELFDATA2LSB
//!     v[6] = 1; // EV_CURRENT
//!     v[16..18].copy_from_slice(&2u16.to_le_bytes()); // ET_EXEC
//!     v[18..20].copy_from_slice(&NATIVE_MACHINE.to_le_bytes());
//!     v[24..32].copy_from_slice(&entry.to_le_bytes()); // e_entry
//!     v[32..40].copy_from_slice(&(EHDR as u64).to_le_bytes()); // e_phoff
//!     v[54..56].copy_from_slice(&(PHDR as u16).to_le_bytes()); // e_phentsize
//!     v[56..58].copy_from_slice(&1u16.to_le_bytes()); // e_phnum
//!
//!     let p = EHDR;
//!     v[p..p + 4].copy_from_slice(&1u32.to_le_bytes()); // PT_LOAD
//!     v[p + 4..p + 8].copy_from_slice(&flags.to_le_bytes());
//!     v[p + 8..p + 16].copy_from_slice(&((EHDR + PHDR) as u64).to_le_bytes()); // p_offset
//!     v[p + 16..p + 24].copy_from_slice(&0x40_0000u64.to_le_bytes()); // p_vaddr
//!     v[p + 24..p + 32].copy_from_slice(&0x40_0000u64.to_le_bytes()); // p_paddr, equal here
//!     v[p + 40..p + 48].copy_from_slice(&4096u64.to_le_bytes()); // p_memsz; p_filesz stays 0
//!     v
//! }
//!
//! // What a loader gets back: where to map, how big, and with what permissions.
//! let bytes = image(PF_R | PF_X, 0x40_0000);
//! let elf = Elf::parse(&bytes).unwrap();
//! assert_eq!(elf.entry(), 0x40_0000);
//!
//! let seg = elf.segments().next().unwrap();
//! assert!(seg.is_readable() && seg.is_executable() && !seg.is_writable());
//! assert_eq!(seg.page_range(4096), (0x40_0000, 0x40_1000));
//!
//! // `paddr` is carried but is almost never what you want: it equals `vaddr` for every program in
//! // this tree, and a loader mapping into a fresh address space wants `vaddr`. See `Segment::paddr`.
//! assert_eq!(seg.paddr, seg.vaddr);
//!
//! // The refusal that matters most, and an ELF is perfectly capable of *asking* for it: a segment
//! // both writable and executable is how a buffer overflow becomes code execution. Same W^X rule
//! // `paging::Flags` keeps by having no writable-and-executable constructor.
//! let bad = image(PF_R | PF_W | PF_X, 0x40_0000);
//! assert!(matches!(Elf::parse(&bad), Err(Error::WritableAndExecutable)));
//!
//! // An entry point outside every executable segment is a program that cannot start.
//! let nowhere = image(PF_R | PF_X, 0x99_0000);
//! assert!(matches!(Elf::parse(&nowhere), Err(Error::EntryNotExecutable)));
//!
//! // And a foreign binary is caught here rather than as a mystery illegal instruction later.
//! // `FOREIGN_MACHINES`, not a literal: which numbers are foreign depends on the build, and this
//! // line said `62` (`x86_64`) until milestone 288 ran it on an `x86_64` host and watched it fail.
//! let mut foreign = image(PF_R | PF_X, 0x40_0000);
//! foreign[18..20].copy_from_slice(&FOREIGN_MACHINES[0].to_le_bytes());
//! assert!(matches!(Elf::parse(&foreign), Err(Error::WrongMachine)));
//! ```
//!
//! Name: ratified 2026-09-18 (calef, `design/decisions/` §154), re-ratified under the acronym
//! test rather than grandfathered by the 2026-08-01 tenet that first blessed it. §154's question is
//! whether the expansion is a phrase people actually say. "executable and linkable format" is not: an
//! expert meeting those words has to translate them back to `elf` to know what they are looking
//! at, so spelling it out would cost the reader who knows the term and help nobody. It stays.
//! Refused `executable_and_linkable_format` (§154).
//!
//! Originally ratified 2026-08-01 in the group of standard terms "already right and must not be
//! touched". That reasoning (recognition bought from outside this project) survives; what changed is
//! that it is now the outcome of a test rather than an exemption from one.

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
