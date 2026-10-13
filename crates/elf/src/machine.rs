//! **Which machines an ELF file may be for**: the `e_machine` values nife runs on, the one this
//! build accepts, and the ones it must refuse.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

/// `e_machine` for the three architectures nife runs on.
///
/// **No `allow(dead_code)` any more, and that is the tell that this got better.** Each of these used
/// to carry a `cfg_attr` silencing the two builds that did not name it, because a build could see
/// only its own. They are all live in every build now: [`KNOWN_MACHINES`] reads all three, and
/// everything host-dependent below is derived from that one list rather than written out per `cfg`.
pub(crate) const EM_AARCH64: u16 = 183;

pub(crate) const EM_RISCV: u16 = 243;

pub(crate) const EM_X86_64: u16 = 62;

/// Every `e_machine` nife has an opinion about, in one place. **This list is the single definition
/// the host-dependent constants below are computed from**, so adding a fourth architecture is one
/// edit here and one arm of `EXPECTED_MACHINE`, and forgetting either one fails the build rather
/// than the tests (see [`FOREIGN_MACHINES`]).
///
/// Exported because a caller can have a per-machine obligation of its own that this crate cannot
/// see. `crates/elf/tests/fuzz_seed.rs` is the one that exists: the fuzz corpus needs a seed for
/// every machine some build accepts, and checking that against this list means a missing seed fails
/// on **any** host rather than only on the host it is missing for.
pub const KNOWN_MACHINES: [u16; 3] = [EM_AARCH64, EM_RISCV, EM_X86_64];

/// The machine this build accepts. A kernel only ever loads binaries for its **own** architecture,
/// so the expected machine is a compile-time fact, not a runtime parameter: each ISA's build accepts
/// that ISA's ELFs, and **that includes the host build these tests run in**, whichever machine the
/// developer is sitting at. This keeps the "catch a
/// foreign binary" check without threading an expected-machine argument through every caller in the
/// kernel and in userspace init, both of which would only ever pass their own architecture anyway.
///
/// **This was a two-way split and the third architecture is why it is now three** (milestone 161,
/// roadmap item 4). It read `#[cfg(not(target_arch = "riscv64"))] EM_AARCH64`, and `not(riscv64)`
/// catches `x86_64`: the x86 kernel was compiled to accept **`aarch64`** binaries and to refuse its own.
/// A default arm that names one architecture is a trap the moment a third exists, which is the
/// general lesson worth taking from this line rather than the specific number.
#[cfg(target_arch = "riscv64")]
pub(crate) const EXPECTED_MACHINE: u16 = EM_RISCV;

#[cfg(target_arch = "x86_64")]
pub(crate) const EXPECTED_MACHINE: u16 = EM_X86_64;

#[cfg(not(any(target_arch = "riscv64", target_arch = "x86_64")))]
pub(crate) const EXPECTED_MACHINE: u16 = EM_AARCH64;

/// `EXPECTED_MACHINE`, exported. A test that forges an ELF header has to write *some* machine
/// number, and writing the native one is what lets the forgery get past the machine check and reach
/// the property actually under test (a bad load address, a writable-executable segment). Naming it
/// here rather than repeating 183/243 behind a `cfg` at each such test keeps one definition of
/// "which machine is this build".
pub const NATIVE_MACHINE: u16 = EXPECTED_MACHINE;

/// **The machines nife runs that this build does not**, derived by subtracting [`NATIVE_MACHINE`]
/// from [`KNOWN_MACHINES`] rather than listed.
///
/// The spelling is **provisional**, minted by milestone 288's lane on 2026-09-14, as are
/// [`KNOWN_MACHINES`] and [`machine_no_nife_build_accepts`]. calef names public items and has not
/// ruled on these three. `script/names` does not enumerate this surface (see design/naming.md's
/// `BUGS`), so this paragraph is the record rather than a `Name:` block.
///
/// The other half of what [`NATIVE_MACHINE`] is for. A test that forges a header and expects it to
/// be **refused** has the same problem as one that expects it to be accepted, in mirror image: it
/// has to write some machine number, and which numbers are foreign depends on the build. Writing
/// them out is the bug this exists to remove, because a hand-written pair is a statement about the
/// author's laptop (milestone 288).
///
/// Iterating this proves the property the test actually means: the machine check is **symmetric**,
/// refusing every nife architecture that is not this build's, rather than privileging one. That
/// sentence is now true on all three hosts instead of on the one the author had.
///
/// The length is `KNOWN_MACHINES.len() - 1`, and the two `assert!`s in the const initialiser are
/// what make that a fact rather than a hope. **A new architecture that reaches `EXPECTED_MACHINE`
/// without reaching [`KNOWN_MACHINES`] fails the build**, on the host that added it, because
/// nothing gets subtracted and the array overflows. That is milestone 161's trap (`not(riscv64)`
/// silently catching `x86_64`) closed by the compiler instead of by remembering.
///
/// **The mirror case is deliberately not an error**, and the asymmetry is the honest half: a
/// machine added to `KNOWN_MACHINES` with no `EXPECTED_MACHINE` arm compiles, because such a build
/// accepts `aarch64` and is right to list the newcomer as foreign. It is only wrong once someone
/// tries to *run* on it, which is the arm they will be writing at the time.
pub const FOREIGN_MACHINES: [u16; KNOWN_MACHINES.len() - 1] = foreign_machines();

pub(crate) const fn foreign_machines() -> [u16; KNOWN_MACHINES.len() - 1] {
    let mut out = [0u16; KNOWN_MACHINES.len() - 1];
    let mut read = 0;
    let mut written = 0;
    while read < KNOWN_MACHINES.len() {
        if KNOWN_MACHINES[read] != EXPECTED_MACHINE {
            // Before the write, so the build error names the cause rather than an array index.
            assert!(
                written < out.len(),
                "EXPECTED_MACHINE is not in KNOWN_MACHINES: an architecture reached the accept check \
                 without reaching the list it is derived from"
            );
            out[written] = KNOWN_MACHINES[read];
            written += 1;
        }
        read += 1;
    }
    assert!(
        written == out.len(),
        "KNOWN_MACHINES names EXPECTED_MACHINE more than once"
    );
    out
}

/// **A machine number no nife build accepts, checked at compile time.** Hands back the number it is
/// given, having proved it is in none of [`KNOWN_MACHINES`]; used in a `const` it is a build error
/// rather than a test that quietly stops testing.
///
/// This exists because the tree has already paid for its absence. A test asserting "a binary for
/// another machine is refused" used `EM_X86_64`, which was foreign until the day `x86_64` became a
/// target and then was not (milestone 161). The number had to move, and nothing but a reader
/// noticing would have moved it.
///
/// **It is deliberately stronger than "not this build's machine".** Checking against
/// `EXPECTED_MACHINE` alone would fail only on an `x86_64` build, which is to say on a machine
/// nobody working on this had; checking against all three fails on **every** host, including the
/// `aarch64` laptop where the mistake was made. A gate that fires only where nobody is standing is
/// not a gate.
pub const fn machine_no_nife_build_accepts(machine: u16) -> u16 {
    let mut i = 0;
    while i < KNOWN_MACHINES.len() {
        assert!(
            KNOWN_MACHINES[i] != machine,
            "this is one of the machines nife runs, so some build accepts it"
        );
        i += 1;
    }
    machine
}
