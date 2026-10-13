//! **The system log, as logic** (§242 (a system log)): one service that programs append to and
//! readers read from. This crate is all of it except the syscalls, which
//! `components/src/system_log.rs` wraps around [`Log`]:
//!
//! 1. **Stamping.** The service looks up the badge the kernel read off the writer's capability in
//!    what the spawner registered ([`registry`]) and fills `seq`, `time`, `program`, `user` and
//!    `severity` itself; a writer supplies text and optionally a level, never its name or time.
//! 2. **Lines.** [`assembler`] joins one writer's sixteen-byte chunks into lines of at most
//!    [`record::TEXT_MAX`] bytes, so interleaved writers still produce whole lines.
//! 3. **F3 to JSONL.** A line becomes an F3 [`record::Header`] plus text, rendered by [`json`] as
//!    one JSON object per line, the stored and read form (§242 Question 3).
//! 4. **The ring** ([`ring`]): 64 KiB, oldest out, read by sequence-number cursor. A reader whose
//!    cursor fell out gets one dropped-count line; a per-user read filters the same ring.
//!
//! # BUGS
//!
//! - A dropped-count line counts every user's records, leaking how busy the machine was; a
//!   per-user count is per-user state §242 ruled the ring should not need.
//! - A line is stamped when it completes, not when its first byte arrived.
//! - Eight writers can be mid-line at once; a ninth cuts the least recent writer's line into two
//!   records, the first flagged `"cut":true`.
//! - No writer has a quota, so one fast badge can evict every other writer's records, and a writer
//!   picks its own severity, as `syslog(3)` allows. The badge guarantees `program` and `user`.
//! - **The intake endpoint must receive nothing the kernel writes.** [`Log::handle`] reads badge 0
//!   as the spawner and any other badge as a writer, and a kernel delivery carries badge 0 (an
//!   interrupt or a bound notification) or a fault address (§26 (the fault endpoint)). So the
//!   intake is never a fault endpoint, never bound to a notification, never an interrupt's
//!   rendezvous; whoever wires the service in inherits that rule.
//!
//! Name: ratified 2026-10-06 (calef): §242's noun, shared with the program. `starlog` was deferred,
//! not refused, for a reader command that does not exist yet.

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
