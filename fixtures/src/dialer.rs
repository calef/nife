//! **A program that asks for the network and two resolver zones, and is refused them**
//! (milestone 892 (a std program at the prompt holds the network and a resolver)).
//!
//! The package is `dialer` (`packages/dialer.recipe.toml`). Its manifest note is version 2
//! (`capability_manifest_note`'s zoned layout): the network, the zone `nife.test`, and the root
//! zone. Installing it lists those asks; nothing records a grant of them yet, so the installed
//! program is refused the network at the prompt and never runs:
//!
//! ```text
//! $ cd downloads
//! $ jig install ./dialer.nifepkg
//!   dialer asks for:
//!     reaches the network (connects out)
//!     looks up names in nife.test
//!     LOOKS UP ANY NAME AT ALL (the root zone)
//!   none of these is granted: no owner's grant is recorded at install yet
//!   installed; generation 11 is live
//! $ dialer
//!   refused: its manifest asks for the network, and no owner's grant of the network is recorded for it among the installed programs
//! ```
//!
//! # BUGS
//!
//! - Its body is reached only if the refusal breaks; it then says so and holds nothing else.
//!
//! Name: provisional, milestone 892's lane, 2026-10-19 (UTC).

#![no_std]
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::{exit, send};

/// Slot 0: where the line goes.
const OUT: u64 = 0;

/// **What this program asks for**: `uptime`'s output and the network.
const MANIFEST: grant_plan::Manifest = grant_plan::Manifest {
    network: true,
    ..grant_plan::Program::Uptime.manifest()
};

capability_manifest_note::carry!(MANIFEST, zones = &["", "nife.test"]);

/// The entry point, reached only if the refusal at the prompt breaks; it then says so.
///
/// Name: provisional, minted 2026-10-19 (UTC) by milestone/892-installed-programs-hold-the-network-and-zones.
#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    say(b"dialer: RAN, though no grant of the network is recorded\n");
    send(OUT, byte_sink_contract::eof(), 0, 0);
    exit();
}

fn say(mut rest: &[u8]) {
    while !rest.is_empty() {
        let (w0, w1, w2, n) = byte_sink_contract::pack(rest);
        send(OUT, w0, w1, w2);
        rest = &rest[n..];
    }
}

user_mode_runtime::panic_handler!();
