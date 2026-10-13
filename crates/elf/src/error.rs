//! **Why a file was refused.**
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

/// Why [`Elf::parse`](crate::Elf::parse) refused a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Smaller than an ELF header. Not even worth looking at.
    TooSmall,
    /// No `\x7fELF`.
    BadMagic,
    /// 32-bit. We do not have a 32-bit anything.
    Not64Bit,
    /// Big-endian. `aarch64` can be, and ours is not.
    NotLittleEndian,
    /// `e_version` is not 1.
    BadVersion,
    /// Compiled for a machine this kernel does not run (`e_machine` is neither our own architecture
    /// nor is it the one we were built to accept). **This is the one that catches a riscv binary
    /// handed to the `aarch64` kernel, or an `aarch64` binary handed to the x86 one**, and it catches it
    /// *here* rather than as a mystery illegal-instruction fault the instant the program starts.
    WrongMachine,
    /// A PIE or shared object. It expects a dynamic linker to relocate it. We are not one.
    NeedsRelocation,
    /// Not an executable at all (a relocatable object, a core dump).
    NotExecutable,
    /// The program header table runs off the end of the file.
    BadProgramHeaders,
    /// A segment's file contents run off the end of the file.
    ///
    /// **The bounds check that matters.** `p_offset + p_filesz` is attacker-controlled, and a
    /// loader that trusts it reads whatever happens to be after the buffer, and then maps it
    /// into a process.
    SegmentOutOfBounds,
    /// `p_memsz < p_filesz`: the segment claims to occupy less memory than it has bytes.
    SegmentTruncated,
    /// **A segment that is both writable and executable.**
    ///
    /// Refused, and this is the same W^X rule that `paging::Flags` enforces by having no
    /// `writable_and_executable()` constructor. A page that is both is how a buffer overflow
    /// becomes code execution, and an ELF is perfectly capable of *asking* for one.
    WritableAndExecutable,
    /// A segment that is neither readable nor executable. Nothing can ever touch it.
    SegmentUnreachable,
    /// Two segments want the same page.
    ///
    /// A real loader handles this (it is legal, and common when `.text` and `.rodata` share a
    /// page). Ours refuses, because our own linker script page-aligns every segment, so if we
    /// ever see one it means something we did not expect. The linker script that keeps that true,
    /// and the two empty-section cases that once broke it, is `crates/user_mode_runtime/link.ld`.
    SegmentsOverlap,
    /// The entry point is not inside any executable segment. The program cannot start.
    EntryNotExecutable,
    /// **`p_vaddr + p_memsz` overflows.** A crafted segment can name a near-`u64::MAX` memsz to
    /// wrap the address arithmetic; caught here so the entry check and page math cannot overflow.
    AddressOverflow,
    /// More program headers than we will look at. A real static executable has a handful; a huge
    /// count is only good for making the O(n^2) overlap check stall.
    TooManyProgramHeaders,
}
