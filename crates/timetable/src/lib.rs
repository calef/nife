//! **`timetable`: a cron whose every entry is a grant** (milestone 129).
//!
//! Unix cron is ambient authority with a timestamp on it. A crontab line runs as a user and can do
//! whatever that user can do, so the crontab *is* the attack surface: the line `0 3 * * * rm -rf
//! /var/tmp/*` is dangerous not because of what it says but because of what the account behind it
//! holds. Nothing in the line bounds it, and nothing can, because on Unix a process's authority is
//! a property of who it runs as rather than of what it was handed.
//!
//! This crate inverts that. An entry is **a schedule plus a grant expression**, and the grant
//! expression is checked at registration by [`grant_plan::plan`], which is the same function the
//! shell checks a prompt line with. What comes out is an [`Endowment`](grant_plan::Endowment): the complete authority the
//! scheduled child will hold, computable and printable **before anything ever fires**. An entry
//! cannot name what its program's manifest does not declare, and it cannot be backed by authority
//! the scheduler itself was not given.
//!
//! # The four answers registration can give
//!
//! This is the demonstration, and it is the negative controls rather than the happy path. Unix cron
//! has one answer to every line (it runs, and whether it should have is not a question anything
//! asks); [`Registry::register`] has four, and three of them are refusals:
//!
//! | answer | what it means | example entry |
//! |---|---|---|
//! | [`Admission::Fires`] | planned, backed, and armed | `every 5s least_authority_demo 7` |
//! | [`Admission::Refused`] | the program's own manifest refuses the line, exactly as at the prompt | `every 5s memory_grant_depleter` (it exists to spend a budget and none was named) |
//! | [`Admission::Unbacked`] | the line is legal and **this scheduler holds nothing to back it** | `every 5s date` (a clock is a capability, and the scheduler has none) |
//! | [`Error`] | the document itself does not parse, with the line number | `every fortnight least_authority_demo 7` |
//!
//! **[`Admission::Unbacked`] is the one worth reading twice.** `date` at a Unix cron line always
//! works, because reading the clock there is ambient. Here the wall clock is a capability
//! (DECISIONS §43), the scheduler was granted none, and so a scheduled `date` is refused *at
//! registration, in writing*, rather than firing five hundred times and printing 1970. The refusal
//! is a fact about the scheduler's own endowment, which is why it cannot be fixed by editing the
//! entry: it is fixed by granting the scheduler a clock, which is a decision somebody makes on
//! purpose at the spawn site.
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
//! Three shapes. `every <interval>` and `at-boot` run on the monotonic counter and need nothing.
//! A calendar line (G5, ruled by calef on 2026-09-26; [`recurrence`]) names a time of day in UTC,
//! is exactly one RFC 5545 RRULE, and needs the timetable to hold the wall clock. See
//! notes/scheduled-execution/calendar-grammar-g5.md for the grammar and what it refuses.
//!
//! # What this crate is not
//!
//! It performs no IO and makes no syscalls, which is CLAUDE.md rule 7 and the reason the four
//! answers above are host-tested in milliseconds rather than under QEMU. The program that holds the
//! budget, reads the counter and builds the children is `components/src/timetable.rs`; everything here is
//! the decision it makes, lifted out so it can be tested and Kani-reached.
//!
//! Name: ratified 2026-09-13 (calef, working the unratified worklist), for this crate, the program
//! `components/src/timetable.rs` and `components/timetable.conf` together. Milestone 129's block
//! declined to propose a name and said the eventual one was calef's; this was the placeholder, and
//! it is the placeholder that won.
//!
//! `timetable` is a noun naming the thing this crate holds, a table of scheduled entries, and it
//! **avoids `scheduler`, which in this tree already means `kernel/src/sched.rs`** and would make two
//! unrelated things share a word.
//!
//! **That avoidance turned out to be the load-bearing half**, and it decided a second name the same
//! day. `crates/schedule_store` had taken the adjacent inflection, and calef asked the question a
//! reader scanning `crates/` would ask: *"What kind of schedule? Is this the thread scheduler?"* It
//! is not, and one inflection is not enough distance, so that crate becomes `timetable_store`. A
//! word this tree stepped around once is a word it should keep stepping around.
//!
//! Refused `cron`, which milestone 129's block calls "a placeholder in the oldest tradition" and
//! which names the model this milestone inverts: cron runs arbitrary commands as ambient authority
//! made periodic, where here every entry is a grant checked at parse time by `grant_plan`. Refused
//! `almanac`, a book of astronomical dates, promising calendar scheduling this does not do, and
//! `metronome`, which names the tick and says nothing about the grant, the half that matters.
//! Refused `grant_schedule`, which puts the inversion in the name and is worse as a word a person
//! types at a prompt. Refused `cron` (the roadmap block calls it "a placeholder in the oldest tradition", and
//! the whole point of the milestone is that this is *not* cron's model), `almanac` (a book of
//! astronomical dates, which promises calendar scheduling this deliberately does not do), and
//! `metronome` (names the tick and says nothing about the grant, which is the half that matters).

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
