//! **The system log, as logic**: milestone 613 (a system log service: the in-memory half), the
//! service §242 (a system log) ruled.
//!
//! One service that programs append to and readers read from. This crate is all of it except the
//! syscalls, which `components/src/system_log.rs` wraps around [`Log`]:
//!
//! 1. **Stamping.** A message arrives with the badge the kernel read off the writer's capability.
//!    The service looks the badge up in what the spawner registered ([`registry`]) and fills `seq`,
//!    `time`, `program`, `user` and `severity` itself. The writer supplies text and, optionally, a
//!    level; it never supplies its own name, user or time.
//! 2. **Lines.** A byte-sink writer sends sixteen bytes at a time. [`assembler`] joins one writer's
//!    chunks into lines, at most [`record::TEXT_MAX`] bytes, so two writers interleaving chunk by
//!    chunk still produce whole lines each.
//! 3. **F3 to JSONL.** A finished line becomes an F3 [`record::Header`] plus text, and [`json`]
//!    renders that as one JSON object per line, which is what is stored and what a reader gets
//!    (§242 Question 3 as ruled: F3 is the transport, JSONL the stored and read form).
//! 4. **The ring** ([`ring`]): 64 KiB, oldest out, read by sequence-number cursor. A reader whose
//!    cursor fell out of the ring gets one dropped-count line for the gap. A per-user read filters
//!    the same ring by the stamped user; there is no per-user store.
//!
//! # EXAMPLES
//!
//! Two writers, interleaved chunk by chunk, and a system reader:
//!
//! ```
//! use system_log::{Log, Handled};
//! use system_log_protocol::{control, read};
//! use byte_sink_protocol::pack;
//!
//! let mut log = Log::new();
//! for (badge, program) in [(1u32, &b"alpha"[..]), (2, &b"beta"[..])] {
//!     for m in control::name_messages(badge, control::PROGRAM, program).into_iter().flatten() {
//!         log.handle(0, m.0, m.1, m.2, 0);
//!     }
//! }
//! log.handle(0, control::word(control::OPERATION_READER, control::SCOPE_SYSTEM, 0, 9), 0, 0, 0);
//!
//! let (a, b) = (b"first half, then", b"one\n");
//! let (w0, w1, w2, _) = pack(a);
//! log.handle(1, w0, w1, w2, 10);
//! let (w0, w1, w2, _) = pack(b"hello from beta\n");
//! log.handle(2, w0, w1, w2, 20);
//! let (w0, w1, w2, _) = pack(b);
//! log.handle(1, w0, w1, w2, 30);
//!
//! let mut window = [0u8; read::WINDOW_BYTES];
//! let asked = log.handle(9, read::request(), 0, 0, 40);
//! assert_eq!(asked, Handled::Read { window: 0, cursor: 0 });
//! log.fill(9, 0, &mut window);
//! let (next, len, status) = read::header(&window);
//! let text = core::str::from_utf8(&window[read::DATA..read::DATA + len]).unwrap();
//! assert_eq!(status, read::status::OK);
//! assert_eq!(next, 2);
//! let mut lines = text.lines();
//! let beta = lines.next().unwrap();
//! let alpha = lines.next().unwrap();
//! assert!(beta.starts_with(r#"{"seq":0,"time":20,"source":2,"program":"beta","user":null,"#));
//! assert!(alpha.ends_with(r#""severity":"info","msg":"first half, thenone"}"#));
//! ```
//!
//! # BUGS
//!
//! - **A dropped-count line counts every user's records.** A per-user reader whose cursor fell out
//!   of the ring is told how many records were lost, and that number includes other users'
//!   records it would never have been shown. It leaks how busy the machine was, not what anybody
//!   wrote. Fixing it means keeping a per-user count of what was evicted, which is per-user state
//!   §242 ruled the ring should not need.
//! - **A line is stamped when it completes, not when its first byte arrived.** `time` is the
//!   moment the newline (or the cut, or the end of stream) reached the service. For a writer that
//!   prints a line in one go the difference is the sixteen-byte chunking and nothing more.
//! - **Eight writers can be mid-line at once.** A ninth writer's first chunk cuts the line of the
//!   writer that wrote least recently, which then appears as two records (the first flagged
//!   `"cut":true`). Lines are not lost; long-quiet partial lines are split.
//! - **No writer has a quota, and a writer picks its own severity.** The ring (`ring::BYTES`,
//!   64 KiB) evicts oldest-first across every writer, so one badge writing fast enough pushes every
//!   other writer's records out before a reader drains them, and readers see only the dropped
//!   count. Severity is the writer's framing bits, so any writer can mark a line `emerg`. Both are
//!   what `syslog(3)` allows a client too; what the badge guarantees is `program` and `user`, which
//!   a writer cannot choose (the registry is written from badge 0 only, and the kernel refuses to
//!   re-badge). Recorded by the 2026-10-03 security audit; a per-writer share of the ring is the
//!   per-user state §242 ruled the ring should not need, and no second domain writes here yet.
//! - **The intake endpoint must receive nothing the kernel writes.** [`Log::handle`] reads badge
//!   `0` as the spawner and any other badge as a writer, and the kernel's own deliveries carry
//!   whatever the mailbox's fourth word holds: `0` for an interrupt signal and a bound
//!   notification, so either reads as a control word from the spawner, and the fault address for
//!   a death message under §26 (the fault endpoint: thread death becomes a message a supervisor
//!   holds), which reads as bytes from a writer whose badge is that address. So the
//!   intake is never a thread's fault endpoint, never bound to a notification and never an
//!   interrupt's rendezvous. The spawner today (the kernel test) does none of these; milestone 342
//!   (the kernel and the `console` server drive one UART from two address spaces) is the first
//!   to wire the service in and inherits the rule (the 2026-10-03 security audit's follow-up).
//!
//! Name: ratified 2026-10-06 (calef, "`system_log` ratified.", in conversation), the noun §242 uses
//! for the service, shared with the program in `components/src/system_log.rs`. `starlog` was
//! considered and deferred, not refused: it fits the command people type to read the log, which
//! does not exist yet.

#![no_std]

pub mod assembler;
pub mod json;
pub mod registry;
pub mod ring;

use assembler::{Assembler, Line};
use byte_sink_protocol::{INLINE_MAX, Msg, unpack};
use registry::{Name, Registry};
use ring::Ring;
pub use system_log_protocol::record;
use system_log_protocol::record::Header;
use system_log_protocol::{control, read, severity};

#[cfg(test)]
mod tests;

mod log;
mod store;

pub use log::{Handled, Log};
use store::*;
