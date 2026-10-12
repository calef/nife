//! Measured boot (milestone 104 (the measurement continues past init)): `measured`, the one call
//! every component load goes through. Moved out of `lib.rs` unchanged but for visibility by
//! milestone 843 (the system-initializer crate root is split along its seams), under §279 (the
//! system-initializer crate root splits into private modules).
//!
//! Name: provisional (milestone 843's lane, 2026-10-11 UTC).

// -------------------------------------------------------------------------------------------
// The measurement (milestone 104): the progenitor measures what the progenitor loads.
// -------------------------------------------------------------------------------------------

/// Read a program out of the archive and measure it against the table, or refuse it.
///
/// **The decision itself is [`measured_boot::verdict`], not this function** (milestone 246). It used
/// to be eighteen lines here, and this crate cannot be reached by a host test, so the branch that
/// sets `unvouched` on a substituted program was proved by nothing: changing its `true` to `false`
/// started an unvouched binary and turned nothing in the tree red. Moving the three-way answer into
/// `measured_boot` put it beside `verify_in_manifest`, which is where the rest of that rule already
/// lives, and left this line: an archive read and a call. `script/swish-check` still proves that the
/// boot *makes* the call, on both ISAs, with the table the kernel measured, and nothing about that
/// changed.
pub(super) fn measured<'a>(
    fs: &nifefs::Fs<'a>,
    table: &str,
    name: &str,
) -> measured_boot::Verdict<'a> {
    measured_boot::verdict(table, name, fs.read(name))
}
