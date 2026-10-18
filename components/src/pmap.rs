//! **`pmap`: what is mapped, and who is allowed to ask** (milestone 126, notes/process-view.md,
//! DECISIONS §114).
//!
//! The whole program is: walk one address space with `abi::address_space::LIST`, then say what it
//! found on one stream and what went wrong on the other, `ps`'s shape one object type over. The
//! listing itself is `crates/pmap`, host-tested; what lives here is the syscall and the two sinks.
//!
//! # `ENUMERATE`, not `WRITE`, is the whole demonstration
//!
//! `abi::address_space::MAP_INTO` needs `WRITE` on the address-space capability: the authority to
//! shape it. `abi::address_space::LIST` needs `ENUMERATE`: the authority to look. A `pmap` granted
//! `ENUMERATE` alone can list every mapping and cannot add, remove or change one.
//!
//! | slot | what | why |
//! |---|---|---|
//! | 0 | the output sink, `WRITE` | where the table goes |
//! | [`ADDRESS_SPACE_SLOT`] | the address space, `ENUMERATE` | the space whose mappings it may **name** |
//! | [`DIAG_SLOT`](grant_plan::DIAGNOSTICS_SLOT) | the diagnostics sink, `WRITE` | where a refusal goes |
//!
//! # BUGS
//!
//! **Nothing at the interactive prompt can grant this program an address space.** `ps` reaches the
//! shell because `Manifest::domain` tells `system_initializer` which live supervision endpoint to
//! place; there is no such manifest field or wiring for an address space. Since §249 (a running
//! address space stays nameable) a builder can keep a copy, so the gap is the wiring and the design
//! for handing a narrowed view to a third party (`crates/pmap`'s `BUGS`).
//! `kernel::user::pmap_tests` proves this program end to end against a real space:
//!
//! ```text
//! $ pmap
//!               VA  PERM
//!       0x00400000  r-x
//!       0x00500000  rw-
//!
//! $ pmap        pmap: this address space may be mapped into, but not looked at: no ENUMERATE
//! ```
//!
//! Name: ratified 2026-09-14 (calef, working the unratified worklist), on milestone 126 and
//! design/naming.md: the name every reader knows from outside, shared with `crates/pmap` as `ps`
//! and `crates/ps` share theirs.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and this is one more
// of the 58 that document an OS-facing ABI entry point rather than a library the lint is for.
#![allow(missing_docs)]
#![no_main]

use core::sync::atomic::{AtomicBool, Ordering};

use user_mode_runtime::{exit, is_granted, list, send};

/// The output sink: where the table goes. Slot 0 is where every spawned program's output lands.
const REPORT: u64 = 0;

/// **The address space under view**: an `AddressSpace` capability, `ENUMERATE`. Everything this program
/// can see comes through here and nothing else in the capability table names a space.
///
/// Not a `grant_plan` constant like `ps`'s [`grant_plan::DOMAIN_SLOT`], on purpose: `grant_plan` is
/// the shell's command-line planning surface, and there is no command line that can grant this
/// program anything (see the module docs). The number is chosen to sit in the same reserved-low-slot
/// range `DOMAIN_SLOT` (7) and `DIAGNOSTICS_SLOT` (8) already use, so a caller that does eventually
/// wire this up is not the first program to want a fixed low slot for "the thing under view."
const ADDRESS_SPACE_SLOT: u64 = 7;

/// The declared second stream (DECISIONS §67): complaints about the run, never about the listing.
/// Reused from `grant_plan` rather than a second constant minted for the same number: every program
/// with a diagnostics stream in this tree uses slot 8, whether or not the shell can reach it.
const DIAG_SLOT: u64 = grant_plan::DIAGNOSTICS_SLOT;

/// Whether this process was granted a second stream, decided once in [`_start`].
static HAS_DIAG: AtomicBool = AtomicBool::new(false);

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    HAS_DIAG.store(is_granted(DIAG_SLOT), Ordering::Relaxed);

    // Collect first, complain second, print third (DECISIONS §67), `ps`'s shape verbatim: a
    // listing cannot know its complaints up front (the space may be refused on the first call or
    // vanish on a later one), so the whole thing goes into a buffer before either stream is
    // touched.
    let mut rows = [pmap::Row::default(); pmap::MAX_ROWS];
    let found = pmap::collect(&mut rows, &mut |cursor| list(ADDRESS_SPACE_SLOT, cursor));

    found.write_diagnostics(&mut |bytes| write_on(diag_slot(), bytes));
    diag_end();

    found.write_report(&mut |bytes| write_on(REPORT, bytes));
    send(REPORT, byte_sink_contract::eof(), 0, 0);
    exit();
}

/// Which endpoint a complaint goes to: the declared second stream when there is one, and the output
/// otherwise. `ps`'s fallback, verbatim: honest rather than a silent drop, and what a test harness
/// that wires no second sink gets.
fn diag_slot() -> u64 {
    if HAS_DIAG.load(Ordering::Relaxed) {
        DIAG_SLOT
    } else {
        REPORT
    }
}

/// **Close the second stream**, which is not tidiness: its reader waits for the end before it reads
/// the first, so a `pmap` that exited without this would leave the prompt blocked.
fn diag_end() {
    if HAS_DIAG.load(Ordering::Relaxed) {
        send(DIAG_SLOT, byte_sink_contract::eof(), 0, 0);
    }
}

/// Write bytes to an endpoint under the sink contract, sixteen at a time.
fn write_on(slot: u64, bytes: &[u8]) {
    let mut rest = bytes;
    while !rest.is_empty() {
        let (w0, w1, w2, n) = byte_sink_contract::pack(rest);
        send(slot, w0, w1, w2);
        rest = &rest[n..];
    }
}

user_mode_runtime::panic_handler!();
