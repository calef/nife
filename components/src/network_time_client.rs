//! **The network time client** (§43 (reading the clock is a page), notes/ntp.md): turns
//! `network_time_protocol`'s wire format into a clock correction. It holds a network capability and
//! a capability to propose a time, and does not hold the clock. Its test server is
//! `fixtures/src/network_time_test_server.rs`, and `fixtures/src/unwritable_clock_witness.rs`
//! proves the missing slot.
//!
//! ```text
//!   entropy ──an endpoint──►┌──────────────┐──an endpoint──► net_stack ──► the network
//!   (8 random bytes,        │  this client │ (the socket contract, UDP 123)
//!    the nonce)             └──────┬───────┘
//!                                  │ an endpoint: PROPOSE
//!                          ┌───────▼────────┐
//!                          │ clock service  │──writes──► the clock page
//!                          └────────────────┘   (this process holds NO mapping of it,
//!                                                read-only or otherwise)
//! ```
//!
//! **A compromised NTP client can lie inside the service's bounds and do nothing else**: with no
//! clock page, setting the time is not an operation it can express, where Unix `ntpd` runs as root.
//! `an_ntp_client_holds_no_writable_clock_page` proves it: the witness, endowed by the same
//! function as this client (`spawn_with_client_endowment`), writes where a setter maps the page and
//! faults. One endowment function means no second capability list can drift.
//!
//! The 64-bit nonce comes from the entropy service, or the client sends nothing and reports
//! [`RPT_NO_ENTROPY`]; a guessable nonce would undo the one defence against an off-path attacker.
//! It is a one-shot synchroniser: up to [`ATTEMPTS`] requests, sleeping on a `Timer` between them,
//! then one proposal and exit. A long-running client is its own design question
//! (`design/roadmap/0674-the-last-yield-spinners-sleep-on-a-timer.md`). A kiss-o'-death is never
//! retried.
//!
//! The test server is also the network, holding `READ` on this client's socket endpoint, so the
//! client's real code runs against it. That proves the socket-contract glue, the 48-byte packet,
//! the nonce, that a rejected reply moves nothing, and that a sample becomes a proposal. It does not
//! prove smoltcp, UDP or the NIC (proven elsewhere), nor anything about a real internet server.
//!
//! Name: ratified 2026-09-14 (calef), replacing `ntp`. Refused keeping `ntp`: nothing types this
//! name, so a typed command's latitude does not reach it. `client` is NTP's own mode word.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use abi::rights;
use clock_protocol::propose;
use network_time_protocol::{Query, Reject, Timestamp};
// The socket contract, verbatim from the file `net_stack` compiles, so this client and the test
// server cannot drift from the real server's idea of the wire format.
// An NTP client speaks the UDP half of the contract and never the TCP half, so the rest of the
// file is dead here. Allowed rather than trimmed: the value of compiling the *same file* net_stack
// does is that the two cannot drift, and a per-consumer subset would throw that away.
#[allow(dead_code)]
use socket_protocol::*;
use user_mode_runtime::mapped_window::{MappedWindow, PAGE};
use user_mode_runtime::{
    call, call_receiving, cntfrq, exit, map_page_frame, now, retype_into_page_frame,
    retype_sleeper, send, send_capability, sleep_until, yield_now,
};

// =================================================================================================
// The slots, and the words this program reports.
//
// The report vocabulary is one numbering space across all three programs milestone 290 split this
// file into, and each declares only the words it sends. The numbers did not move in that split, so
// a boot log from before it still reads. `kernel/src/user/ntp_service.rs`'s `rpt` module is the
// whole vocabulary in one place, as the kernel side of a wiring always is here.
// =================================================================================================

/// Slot 0: the report endpoint (WRITE).
const REPORT: u64 = 0;
/// Slot 1: the socket contract's endpoint. **This client's whole network authority.** The client
/// holds `WRITE`, the test server `READ`.
const STACK: u64 = 1;
/// Slot 2: an untyped budget, to mint and map the one shared frame.
const MEMORY_REGION: u64 = 2;
/// Slot 3: the clock service's **propose** endpoint (WRITE). Not a page, and not a way to set
/// anything.
const PROPOSE: u64 = 3;
/// Slot 4: the entropy service's endpoint (WRITE): "you may obtain randomness", naming no device.
const ENTROPY: u64 = 4;

/// A sample was accepted and proposed. `w1` is the `clock_protocol::status` the service answered with,
/// `w2` the nanoseconds proposed.
pub const RPT_SYNCED: u64 = 1;
/// A reply arrived and `Query::accept` refused it. `w1` is a [`reject_code`], `w2` the number of
/// requests sent. **Nothing was proposed.**
pub const RPT_REJECTED: u64 = 2;
/// No reply after [`ATTEMPTS`] requests. `w1` is the number sent.
pub const RPT_NO_REPLY: u64 = 3;
/// **No unguessable bits, so no request.** `w1` is the entropy service's reply word (or the kernel
/// error from a `CALL` on an empty slot, which is what "no capability" looks like).
pub const RPT_NO_ENTROPY: u64 = 4;
/// The socket contract refused us. `w1` names the step, so a wiring mistake is not a mystery.
pub const RPT_NET_ERROR: u64 = 5;
/// The local wall clock is outside the range an NTP timestamp can hold, so no exchange is possible.
pub const RPT_BAD_LOCAL_TIME: u64 = 8;

/// How many requests one synchronisation makes before giving up. Three is a client's ordinary
/// behaviour on a lossy path, not a widened timeout; the DNS check in `socket_test_client` settled on the same
/// number for the same reason.
pub const ATTEMPTS: u32 = 3;

/// The gap between attempts. Milliseconds rather than NTP's 64 seconds, because this client is still
/// one-shot (see the module docs). Asleep on a timer since milestone 106, whose resolution is the
/// 10 ms scheduler tick, so the gap is in practice one tick.
const RETRY_GAP_NANOS: u64 = 2 * 1_000_000;

/// Where this client maps the shared socket frame in its own address space. Above the program's
/// segments; the same address `socket_test_client` uses, and each address space is its own. The
/// test server picks its own, and the two do not have to agree: what they share is the frame's
/// *layout*, which is `socket_protocol`'s.
const PAGE_FRAME_VA: u64 = address_space_map::pair_page(0x0000_0000_00A0_0000);

// SAFETY: this program's own `PageFrame::MAP` (in `attach_page_frame`, before the frame is touched)
// mapped one page read/write at PAGE_FRAME_VA before any of `WINDOW`'s accessors are called
// (milestone 139).
const WINDOW: MappedWindow = unsafe { MappedWindow::new(PAGE_FRAME_VA, PAGE) };

/// `a0` is the server's IPv4 packed big-endian, `a1` its UDP port.
#[unsafe(no_mangle)]
pub extern "C" fn _start(server_ip: u64, server_port: u64) -> ! {
    client(server_ip, server_port)
}

/// Report and stop. One-shot, so it **exits** rather than parking: a role left spinning on a run
/// queue starves later tests on the same core, which is the finding `socket_test_client`'s `done` records.
fn done(w0: u64, w1: u64, w2: u64) -> ! {
    send(REPORT, w0, w1, w2);
    exit();
}

fn client(server_ip: u64, server_port: u64) -> ! {
    // The wall clock, read the only way this process can read it: by asking the service. A reader
    // would hold the clock page and do two loads and an add, but the whole point of this process is
    // that it holds no mapping of that page, so `propose::STATE` is its channel. It costs one round
    // trip, and it is taken once: the ambient monotonic counter carries the time from here.
    let (_state, wall0) = call(PROPOSE, propose::req(propose::STATE), 0);
    let local = LocalClock {
        wall0,
        mono0: monotonic_nanos(),
    };

    // **The nonce before the network.** A client that cannot obtain unguessable bits has nothing to
    // send, so it finds that out before it exercises any network authority at all, rather than
    // reaching the same refusal one socket later.
    let mut nonce = match nonce_bits() {
        Ok(bits) => bits,
        Err(word) => done(RPT_NO_ENTROPY, word, 0),
    };

    // Our one socket, a capability the stack hands back (§255 (each socket is its own
    // capability)), and then our page attached to it.
    let (REP_OK, _, Some(socket)) = call_receiving(STACK, OPERATION_OPEN_UDP, 0) else {
        done(RPT_NET_ERROR, 2, 0);
    };
    attach_page_frame(socket);

    let mut sent = 0u32;
    let mut last_reject = 0u64;
    while sent < ATTEMPTS {
        // A fresh nonce every attempt after the first. Reusing one would let a reply to an earlier
        // request answer a later one, which is the stale-reply half of the origin check's job.
        if sent > 0 {
            nonce = match nonce_bits() {
                Ok(bits) => bits,
                Err(word) => done(RPT_NO_ENTROPY, word, 0),
            };
        }

        // T1 as late as it can honestly be taken. The bytes on the wire do not depend on it (a
        // request carries the nonce, not the send time), so everything that can be done first is,
        // and what remains between this reading and the datagram leaving is one IPC. What is left
        // over lands in the measured delay, which is the same path asymmetry NTP already lives with.
        write_dst(server_ip, server_port);
        let Some(t1) = stamp(local.now()) else {
            done(RPT_BAD_LOCAL_TIME, local.now(), 0);
        };
        let query = Query::with_nonce(t1, Timestamp::from_bits(nonce));
        write_payload(&query.request());
        sent += 1;
        if call(
            socket,
            OPERATION_SENDTO,
            network_time_protocol::PACKET_LEN as u64,
        )
        .0 != REP_OK
        {
            done(RPT_NET_ERROR, 3, sent as u64);
        }

        let (n, _) = call(socket, OPERATION_RECEIVE, 0);
        if n == REP_ERR || n == 0 {
            poll_gap();
            continue;
        }

        // T4, read as close to the arrival as this side can manage: the RECEIVE reply is the first
        // instruction after the bytes landed.
        let Some(t4) = stamp(local.now()) else {
            done(RPT_BAD_LOCAL_TIME, local.now(), 0);
        };
        let mut wire = [0u8; network_time_protocol::PACKET_LEN];
        let n = read_payload(n as usize, &mut wire);

        match query.accept(&wire[..n], t4) {
            Ok(sample) => {
                // The correction, applied to the local clock as it is now rather than as it was at
                // T4: the proposal is a claim about the present, and the service will compare it
                // against the present.
                let corrected = local.now() as i128 + sample.offset.nanos();
                let proposed = corrected.clamp(0, u64::MAX as i128) as u64;
                let (status, _wall_after) = call(PROPOSE, propose::req(propose::PROPOSE), proposed);
                let _ = call(socket, OPERATION_CLOSE, 0);
                done(RPT_SYNCED, status, proposed);
            }
            Err(reject) => {
                last_reject = reject_code(reject);
                // A kiss-o'-death is an instruction, and the instruction is to stop.
                if matches!(reject, Reject::KissOfDeath(_)) {
                    break;
                }
                poll_gap();
            }
        }
    }

    let _ = call(socket, OPERATION_CLOSE, 0);
    if last_reject != 0 {
        done(RPT_REJECTED, last_reject, sent as u64);
    }
    done(RPT_NO_REPLY, sent as u64, 0)
}

/// **Eight unguessable bytes, or an error word that says why not.**
///
/// `entropy_protocol::delivered` is what makes the two failures distinguishable with no probe: a byte
/// count is `0..=8`, and every error a `CALL` can return is one of the kernel's small negatives,
/// which read as enormous `u64`s. So `Err(0)` is "the service has no entropy" and `Err(huge)` is
/// "there is no entropy service in slot 4". Both stop the client; only the report tells them apart.
fn nonce_bits() -> Result<u64, u64> {
    let (r0, r1) = call(ENTROPY, entropy_protocol::req(entropy_protocol::GET, 8), 0);
    let Some(n) = entropy_protocol::delivered(r0) else {
        return Err(r0);
    };
    if n < 8 {
        return Err(r0);
    }
    let mut bytes = [0u8; 8];
    entropy_protocol::take(n, r1, &mut bytes);
    Ok(u64::from_le_bytes(bytes))
}

/// The local wall clock, anchored once against the monotonic counter.
///
/// `wall0` is what the clock service said at `mono0`; everything after that is counter arithmetic,
/// which is ambient and costs one instruction. When the service says the clock is `UNKNOWN` it
/// answers 0, and the arithmetic still works out: T1 and T4 are then measured from 1970, the offset
/// comes out as the whole distance to the server's clock, and the proposal lands on the server's
/// time. That is the bootstrap case, and it is the case a machine with no RTC is in.
struct LocalClock {
    wall0: u64,
    mono0: u64,
}

impl LocalClock {
    fn now(&self) -> u64 {
        self.wall0
            .saturating_add(monotonic_nanos().saturating_sub(self.mono0))
    }
}

/// Monotonic nanoseconds since boot, from the ambient counter.
///
/// Whole seconds and a remainder, not `ticks * 1e9`: at 62.5 MHz the naive product overflows a
/// `u64` about five minutes into a boot, and the clock built on it jumps backwards by six centuries.
/// The clock service's copy of this arithmetic carries the same note.
fn monotonic_nanos() -> u64 {
    let freq = cntfrq();
    let ticks = now();
    let secs = ticks / freq;
    let rem = ticks % freq;
    secs * clock_protocol::NANOS_PER_SEC + rem * clock_protocol::NANOS_PER_SEC / freq
}

/// Unix nanoseconds to an NTP timestamp. `None` outside the crate's representable window, which the
/// clock service's 2100 ceiling keeps us well inside; it is checked rather than assumed because the
/// alternative to checking is a timestamp 136 years out.
fn stamp(unix_nanos: u64) -> Option<Timestamp> {
    Timestamp::from_unix(
        unix_nanos / clock_protocol::NANOS_PER_SEC,
        (unix_nanos % clock_protocol::NANOS_PER_SEC) as u32,
    )
}

/// The poll interval, asleep on a timer (milestone 106 (a wait that ends on either the interrupt or
/// the deadline)). The pair is retyped from this client's own untyped on the first gap; if the
/// untyped cannot pay for it, the gap falls back to the yield-spin it was until 2026-09-26.
fn poll_gap() {
    use core::sync::atomic::AtomicU64;
    use core::sync::atomic::Ordering::Relaxed;
    static TIMER: AtomicU64 = AtomicU64::new(0); // 0: not made yet; u64::MAX: refused
    static NOTIFICATION: AtomicU64 = AtomicU64::new(0);
    if TIMER.load(Relaxed) == 0 {
        // Up to the entropy slot, whose emptiness is how this client learns it has no entropy.
        let (t, n) = retype_sleeper(MEMORY_REGION, ENTROPY + 1).unwrap_or((u64::MAX, 0));
        NOTIFICATION.store(n, Relaxed);
        TIMER.store(t, Relaxed);
    }
    let timer = TIMER.load(Relaxed);
    if timer != u64::MAX {
        let ticks = abi::timer::counter_ticks_for(
            RETRY_GAP_NANOS / 1_000_000_000,
            (RETRY_GAP_NANOS % 1_000_000_000) as u32,
            cntfrq(),
        );
        if sleep_until(
            timer,
            NOTIFICATION.load(Relaxed),
            now().saturating_add(ticks),
        ) >= 0
        {
            return;
        }
    }
    let deadline = monotonic_nanos() + RETRY_GAP_NANOS;
    while monotonic_nanos() < deadline {
        yield_now();
    }
}

/// A stable small integer for each `Reject`, so a test (and one day a log) can name which check
/// refused a packet. The enum is the security story of unauthenticated NTP; collapsing it to a bool
/// at the first opportunity would throw that away.
fn reject_code(r: Reject) -> u64 {
    match r {
        Reject::Length(_) => 1,
        Reject::Version(_) => 2,
        Reject::Mode(_) => 3,
        Reject::KissOfDeath(_) => 4,
        Reject::Stratum(_) => 5,
        Reject::Unsynchronised => 6,
        Reject::OriginMismatch => 7,
        Reject::TransmitZero => 8,
        Reject::ServerOrderReversed => 9,
        Reject::ClientOrderReversed => 10,
        Reject::NegativeDelay => 11,
        Reject::RootDistance => 12,
    }
}

/// Mint one frame from our own budget, map it, and delegate it to `socket`. Exactly what
/// `socket_test_client` does, because it is exactly the same contract.
fn attach_page_frame(socket: u64) {
    // RETYPE answers with the new frame capability's slot, or a negative error.
    let frame = retype_into_page_frame(MEMORY_REGION);
    if frame < 0 {
        done(RPT_NET_ERROR, 0, 0);
    }
    let frame = frame as u64;
    // Map it writable; the page tables come from our untyped.
    if !map_page_frame(frame, PAGE_FRAME_VA, true, MEMORY_REGION) {
        done(RPT_NET_ERROR, 1, 0);
    }
    // Delegate it, narrowed to read/write, with the ATTACH request.
    if send_capability(
        socket,
        frame,
        rights::READ | rights::WRITE,
        OPERATION_ATTACH_PAGE_FRAME,
    ) < 0
    {
        done(RPT_NET_ERROR, 1, 0);
    }
}

// =================================================================================================
// The shared frame. Absolute-VA volatile access through `WINDOW` (milestone 139), the same
// abstraction socket_test_client, network_time_test_server, keyboard_driver, entropy and virtio_net_transport share.
// `va` is always `PAGE_FRAME_VA + <an offset constant>`, so subtracting PAGE_FRAME_VA recovers the offset
// `WINDOW` bounds-checks against.
// =================================================================================================

fn r8(va: u64) -> u8 {
    WINDOW.r8(va - PAGE_FRAME_VA)
}
fn w8(va: u64, v: u8) {
    WINDOW.w8(va - PAGE_FRAME_VA, v);
}
fn w16le(va: u64, v: u16) {
    WINDOW.w16(va - PAGE_FRAME_VA, v);
}

/// Write the destination header: the address and port the *wiring* named, not one this program
/// chose. Which server to ask is an endowment, the same way which network to use is.
fn write_dst(ip: u64, port: u64) {
    for i in 0..4 {
        w8(PAGE_FRAME_VA + OFF_DST_IP + i, (ip >> (24 - 8 * i)) as u8);
    }
    w16le(PAGE_FRAME_VA + OFF_DST_PORT, port as u16);
}

fn write_payload(bytes: &[u8]) {
    for (i, &b) in bytes.iter().enumerate() {
        w8(PAGE_FRAME_VA + OFF_PAYLOAD + i as u64, b);
    }
}

fn read_payload(n: usize, out: &mut [u8]) -> usize {
    let n = n.min(out.len());
    for (i, b) in out[..n].iter_mut().enumerate() {
        *b = r8(PAGE_FRAME_VA + OFF_PAYLOAD + i as u64);
    }
    n
}

user_mode_runtime::panic_handler!();
