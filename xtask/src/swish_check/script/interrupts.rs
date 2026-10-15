//! **A supervised job, interrupted** (DECISIONS §24): the heeder that stops cleanly on one
//! `^C`, and the ignorer the grace timeout escalates to a `DESTROY`.
//!
//! Cut from the table in `script.rs` verbatim by milestone 900 (the swish-check script
//! splits by topic before it crosses the ceiling).
//!
//! Name: provisional (milestone 900's lane, 2026-10-15 UTC).

use super::*;

/// The interrupt rows of [`SWISH_CHECK_SCRIPT`], in table order.
pub(super) const LINES: &[Line] = &[
    // **A supervised job, interrupted**, under DECISIONS §24 (interrupting the foreground
    // process), and these two are the only lines this gate presses `^C` for (see
    // [`SWISH_CHECK_INTERRUPTED`]). Neither was typed here
    // until 2026-09-25, and in that time the first supervised job of every boot failed with `could
    // not map the job frame`, because the shell mapped its job frame over its own terminal page.
    // The heeder is the cooperative tier: one `^C`, and the job reports it stopped. It goes first
    // because it is the first supervised job of the boot, which is the one that failed.
    line(
        0,
        "interrupt_heeder",
        &[
            "^C interrupts it.",
            "the job caught the interrupt and stopped cleanly after",
        ],
    ),
    // The forcible tier: the ignorer never looks at the flag, so the grace timeout escalates one
    // `^C` to a `DESTROY` of the region the shell built it from, which force-kills its live thread
    // (DECISIONS §16 (object revocation), as amended). A shell whose teardown was refused says so
    // instead.
    line(
        0,
        "interrupt_ignorer",
        &["^C again: tearing the job down.", "its memory reclaimed"],
    ),
];
