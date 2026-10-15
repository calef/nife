//! **The crash witness** (milestone 37 (prove RedoxFS's crash consistency)): the writes the FS
//! server is killed in the middle of, and the read back through the server that mounted the crashed
//! disk.
//!
//! Cut out of `fs_test_client.rs` unchanged on 2026-10-12 (UTC) by lane
//! maintainer/files-over-1500, for §282 (when a file is split, the parent becomes an index). What
//! another role uses is `pub(crate)` here.
//!
//! Name: provisional (lane maintainer/files-over-1500, 2026-10-12 UTC), for the module and the
//! file.

use filesystem_fixture as fixture;
use user_mode_runtime::{exit, send};

use crate::channel::{REPORT, check, get_page, open, read, write};

/// **The crash driver** (milestone 37): write one payload and get an acknowledgement, then write the
/// next one into a server that has been told to die partway through it.
///
/// It reports after the acknowledged write, because that report is the thing the property depends
/// on: "payload A was acknowledged" is what makes losing it a failure rather than a permitted
/// outcome. Then it issues the second write and **never returns from it**, because the server dies
/// inside the request and blocking IPC has no "your server is gone" reply.
///
/// That permanent block is not an oversight, it is the open item DECISIONS §27 and notes/fs-server.md
/// already record: a client of a dead server waits forever, and §26's fault endpoint wired into a
/// supervision tree (milestone 23) is the mechanism that would turn it into a message. This test
/// exhibits it rather than working around it; the thread is `Blocked`, so it costs no CPU, and the
/// kernel test does not wait on this client for anything after the first report.
pub(crate) fn crash_driver() -> ! {
    use fixture::crash;
    let h = open(crash::NAME);

    // Write A and read it straight back. "The server accepted my write" and "my write landed" are
    // different claims, and the second is the one that has to be true before the kill means anything.
    write(h, 0, crash::A);
    let mut buf = [0u8; 128];
    let n = read(h, 0, crash::A.len());
    check(n == crash::A.len());
    get_page(n, &mut buf);
    check(&buf[..n] == crash::A);
    send(REPORT, fixture::SUCCESS, crash::SAW_A, 0);

    // And now the one the server dies inside. This call never returns.
    write(h, 0, crash::B);
    // Unreachable in the test's configuration; if the injector failed to fire, say so rather than
    // exiting quietly and leaving the kernel test to time out with no explanation.
    send(REPORT, fixture::SUCCESS, crash::SAW_B, 0);
    exit();
}

/// **The crash verifier** (milestone 37): a fresh client of a fresh FS server that mounted the disk
/// the killed one left behind. It reports which of the two payloads the file holds.
///
/// It classifies rather than asserts, for the reason milestone 31's attacker reports a bitmap: the
/// interesting failure is not "it did not read back", it is "it read back something that was never
/// written", and a client that panicked on a mismatch would tell the kernel test nothing about
/// which.
pub(crate) fn crash_verify() -> ! {
    use fixture::crash;
    let h = open(crash::NAME);
    let mut buf = [0u8; 256];
    let n = read(h, 0, crash::A.len().max(crash::B.len()) + 8);
    get_page(n, &mut buf);
    let saw = if n == crash::A.len() && &buf[..n] == crash::A {
        crash::SAW_A
    } else if n == crash::B.len() && &buf[..n] == crash::B {
        crash::SAW_B
    } else {
        crash::SAW_NEITHER
    };
    // The length rides home in the first word: a partial write shows up as a number rather than as
    // a bare verdict, which is the difference between "this failed" and "this is what it did".
    send(REPORT, n as u64, saw, 0);
    exit();
}
