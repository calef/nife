//! **`timetable`: a cron whose every entry is a grant** (milestone 129).
//!
//! A Unix crontab line runs as a user and can do whatever that user can do; nothing in the line
//! bounds it. Here an entry is **a schedule plus a grant expression**, checked at registration by
//! [`grant_plan::plan`], the same function the shell checks a prompt line with. The result is an
//! [`Endowment`](grant_plan::Endowment): the complete authority the scheduled child will hold,
//! printable before anything fires. An entry cannot name what its program's manifest does not
//! declare, and cannot be backed by authority the scheduler was not given.
//!
//! # The four answers registration can give
//!
//! | answer | what it means | example entry |
//! |---|---|---|
//! | [`Admission::Fires`] | planned, backed, and armed | `every 5s least_authority_demo 7` |
//! | [`Admission::Refused`] | the program's own manifest refuses the line, exactly as at the prompt | `every 5s memory_grant_depleter` (it exists to spend a budget and none was named) |
//! | [`Admission::Unbacked`] | the line is legal and **this scheduler holds nothing to back it** | `every 5s date` (a clock is a capability, and the scheduler has none) |
//! | [`Error`] | the document itself does not parse, with the line number | `every fortnight least_authority_demo 7` |
//!
//! [`Admission::Unbacked`] is a fact about the scheduler's own endowment
//! (DECISIONS §43 (reading the clock is a page)), so it is fixed by granting the scheduler a clock
//! at the spawn site, not by editing the entry.
//!
//! # What the document looks like
//!
//! ```text
//! # a comment
//! every 200ms  least_authority_demo 7
//! at-boot      least_authority_demo 3
//! every month on last fri at 17:00  least_authority_demo 5
//! ```
//!
//! `every <interval>` and `at-boot` run on the monotonic counter and need nothing. A calendar line
//! ([`recurrence`]) names a time of day in UTC, is exactly one RFC 5545 RRULE, and needs the
//! timetable to hold the wall clock. See notes/scheduled-execution/calendar-grammar-g5.md.
//!
//! This crate performs no IO and makes no syscalls, so the four answers are host-tested. The
//! program that holds the budget, reads the counter and builds the children is
//! `components/src/timetable.rs`.
//!

//! Name: ratified 2026-09-13 (calef), for this crate, the program `components/src/timetable.rs`
//! and `components/timetable.conf`. A noun for a table of scheduled entries that avoids
//! `scheduler`, which here means `kernel/src/sched.rs`; for the same reason `schedule_store` became
//! `timetable_store`. Refused `cron`, which names the ambient-authority model this inverts.
//! Refused `almanac`, which promises astronomical calendar scheduling. Refused `metronome`, which
//! names the tick and not the grant. Refused `grant_schedule`, worse as a word typed at a prompt.

#![no_std]

pub mod contract;
pub mod recurrence;
pub mod registration;

mod admission;
mod document;
mod plan_text;
mod registry;
#[cfg(test)]
mod test_support;

pub use admission::{Admission, Held, SHIPPED_HELD, Unbacked, schedulable};
pub use document::{Document, Entry, Error, Schedule, parse};
pub use plan_text::{Audit, write_plan};
pub use registry::{Registry, Row, SPENT, UNARMED, UNSTAMPED, WallReading, next_after};

/// Nanoseconds in a second. Spelled here rather than taken from `clock_protocol`, because this crate
/// decides *when* rather than *what time it is*: it never touches the wall clock, and depending on
/// the wall-clock contract to name a unit would claim otherwise.
pub const NANOS_PER_SEC: u64 = 1_000_000_000;

/// The most entries one document may carry.
///
/// A ceiling rather than a limit anybody meets, and past it the document is
/// [`Error::TooManyEntries`] rather than quietly short of an entry somebody wrote down. Eight is
/// what a fixed array costs nothing to hold; the reason there is an array at all is that this crate
/// allocates nothing, so a document borrows its own bytes and the table is inline.
pub const MAX_ENTRIES: usize = 8;

// Machine-checked proofs (Kani). Behind `#[cfg(kani)]`, so an ordinary build or test never sees
// them; `script/verify` runs them.
#[cfg(kani)]
mod proofs;
