//! The capability shell, at EL0: designation is authorization (notes/grant-expression.md). A Unix
//! child inherits your whole authority; here the command line is a **grant expression**, so naming
//! a resource grants it, a program that names nothing gets only the report channel every spawn
//! carries, and a refusal reads "you hold no such capability". `grant_plan` parses and checks the
//! line on the host; this file turns a checked grant into delegated capabilities.
//!
//! It grants only from what it holds: `prog --mem N` splits N pages from its own budget (slot 3),
//! and a bare name in a file position resolves only if the progenitor granted it a directory
//! ([`DIR_TERMINAL`]); see [`holdings`] and notes/pipes.md. `caps` prints its whole endowment and
//! `caps <command>` previews exactly what a command would get, including the one authority no
//! token designates, the clock a manifest like `date`'s declares. This shell holds its own clock
//! `READ` without `GRANT`, so `time <command>` can measure and cannot hand a clock on
//! (notes/time-command.md). No `run` verb and no `file:` prefix: a bare name spawns, and the
//! manifest declares a file token's direction.
//!
//! - slot 0: the terminal rendezvous (CALL: `OPERATION_WRITE` / `OPERATION_READLINE`).
//! - slot 1: a spawn rendezvous (direct the progenitor to start a program; `grant_plan::spawnproto`).
//! - slot 2: a result rendezvous (receive a spawned program's answer).
//! - slot 3: an untyped budget, the memory it grants with `--mem`.
//!
//! Pages shared with the terminal: `OUT_VA` (text and prompts out) and `LINE_VA` (lines in). The
//! directory ([`DIR_TERMINAL`], slot 4) and the clock page ([`CLOCK_SLOT`], from `x2`) depend on
//! the boot's wiring, so their numbers are not constants.
//!
//! # BUGS
//!
//! - A child holding the result endpoint can send `SPAWN_FAILED`, `SPAWN_NO_DISPLAY` or a wrong
//!   exit status about itself, though never about another job. Every `RECEIVE` returns the sender's
//!   badge in `x3`, so a badged copy per sender would close it at no new authority.
//! - A job that faults reaches the prompt as [`swish::FAULTED_SENTENCE`] with `$?` 1, through
//!   `job_undertaker`'s `JOB_FAULTED`; a job that hangs without faulting still hangs the prompt,
//!   and only an interruptible spawn can be ended with `^C`.
//! - A fault reported while watching a screen-narrowed tail arrives one command late
//!   (`job_undertaker`'s `BUGS`).
//!
//! Name: ratified 2026-08-01 (calef), replacing `shell`. Refused `shell` (a category, where `bash`
//! and `fish` are identities), `capsh` (Linux's libcap ships one) and `sheesh` (dated, and an
//! interjection). A swish goes through the net touching nothing: least authority in one word.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]
// `Vec::push_within_capacity`: the only way this shell adds to a `Vec`, because it cannot allocate
// behind the caller's back (see [`heap`]).
#![feature(vec_push_within_capacity)]

use filesystem_contract::{dirent, fs};
use grant_plan::expand::{Expander, NameSet, Resume};
use grant_plan::line::{self, Line, Source};
use grant_plan::nav::{self, Cwd, Refused, Step};
use grant_plan::{
    Action, Command, DirGrant as GrantDir, Endowment, Escalation, Refusal, RunSpec, Streams,
    TouchArgs, job_page_frame, spawnproto,
};
use line_editor::proto;
use swish::{Route, Say, Status, Untimed, sequence};
use user_mode_runtime::mapped_window::MappedWindow;
use user_mode_runtime::{
    call, cap_delete, cntfrq, destroy_memory_region, exit, monotonic_nanos, now, reap, receive,
    receive_fault, retype_into_kernel_object, retype_sleeper, send, sleep_until,
    split_memory_region, yield_now,
};

// **The shell is a directory of modules** (milestone 844 (the shell program is split along its
// seams), §273 (the shell program becomes a directory of modules)), and this file is their index
// (§282 (when a file is split, the parent becomes an index)). What four or more of the modules
// use has children of its own: `wiring`, `terminal`, `directory` and `status`. The globs below
// are how a module names what a sibling lends it with `pub(super)`.
mod capabilities;
mod jobs;
mod navigation;
mod pipeline;
mod prompt;
mod running;
mod witness;

use capabilities::*;
use jobs::*;
use navigation::*;
use pipeline::*;
use prompt::*;
use witness::*;

mod commands;
mod directory;
mod entry;
mod status;
mod terminal;
mod wiring;

use commands::*;
use directory::*;
use running::*;
use status::*;
use terminal::*;
use wiring::*;

/// **Every allocation this shell makes, and every one of them can fail.** calef's first condition:
/// the boot shell is the owner's root console, so running out of heap is a refusal with a sentence
/// ([`Say::HeapFull`]) and never a panic. Nothing outside this module names `alloc`, and nothing in
/// it calls an allocating method that panics on failure: capacity is reserved with
/// `try_reserve_exact`, and elements go in with `push_within_capacity`, which cannot allocate.
/// The compiler holds the first half, and `script/lint` holds that the declaration stays in
/// `heap.rs`.
mod heap;

use heap::heap_init;

user_mode_runtime::panic_handler!();
