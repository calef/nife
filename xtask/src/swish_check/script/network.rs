//! **The network, from the booted system** (milestone 590): the preview, the echo client
//! twice, the witness that reaches nothing, and `caps reboot`.
//!
//! Cut from the table in `script.rs` verbatim by milestone 900 (the swish-check script
//! splits by topic before it crosses the ceiling).
//!
//! Name: provisional (milestone 900's lane, 2026-10-15 UTC).

use super::*;

/// The network rows of [`SWISH_CHECK_SCRIPT`], in table order.
pub(super) const LINES: &[Line] = &[
    // **The network, from the booted system** (milestone 590 (the booted system starts its network
    // stack)). Every network test
    // before these four started `net_stack` from the kernel's harness; here the progenitor built it
    // at boot from the NIC the kernel granted it, and a program a person typed reaches the runners'
    // echo peer through it. The preview first, because the row it prints is the whole of who may
    // reach the network from this prompt.
    line(
        0,
        "caps network_echo_client --mem 4",
        &[
            "cap 10 endpoint  network  WRITE",
            "a program without this row reaches no network at all",
        ],
    ),
    line(
        1,
        "network_echo_client --mem 4",
        &["echo peer 10.0.2.9:7777 answered: nife-net!"],
    ),
    // **Twice, and the second is the one that says the stack outlives its clients.** The first job
    // handed `net_stack` a page and exited; its region was reclaimed, which revokes that page out
    // of the stack's address space too. A stack that kept a stale window, or a socket number that
    // could not be opened again, answers the first run and fails this one.
    line(
        1,
        "network_echo_client --mem 4",
        &["echo peer 10.0.2.9:7777 answered: nife-net!"],
    ),
    // **The negative control.** A program that declares no network `CALL`s the network's slot
    // anyway; the kernel must refuse it for want of a capability. A spawn service that endowed the
    // stack to every child prints `REACHED` here instead.
    line(
        1,
        "unreachable_network_witness",
        &[
            "network: refused (no capability at slot 10)",
            "entropy: refused (no capability at slot 9)",
            "domain: refused (no capability at slot 7)",
            "reboot: refused (no capability at slot 13)",
            "slots held: 0\n",
        ],
    ),
    // **`caps reboot` names the authority** (milestone 805 (`reboot` at the prompt), exit criterion
    // 3), at the real prompt and without running it: typing `reboot` mid-script would end this
    // boot. [`reboot_phase`] types it, after the last scripted line.
    line(
        0,
        "caps reboot",
        &[
            "cap 13 reboot    WRITE. restart the machine",
            "cap 14 endpoint  sync   WRITE. the file server answers SYNC on it and refuses",
        ],
    ),
];
