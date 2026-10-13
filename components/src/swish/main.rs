//! The capability shell, at EL0: designation is authorization (milestone 31, phase 1).
//!
//! Milestone 10 gave this program a command line; milestone 31 gives that command line a meaning it
//! does not have on Unix. A Unix child inherits your whole authority and then `open()`s whatever its
//! uid allows. Here the command line is a **grant expression**: naming a resource in a command is
//! how you grant it, and a program that names nothing gets nothing beyond the report channel every
//! spawn carries. There is no ambient authority to fall back on, so when a program needs something
//! the command did not grant, the refusal reads "you hold no such capability", not a Unix-flavored
//! EPERM. The parsing and the manifest checking are the host-tested `grant_plan` crate; this file is the
//! wiring that turns a checked grant into real, delegated capabilities.
//!
//! What the shell can grant, it grants from what it *holds*. The headline is `prog --mem N`, which
//! endows a program N pages of untyped **split from the shell's own budget** (slot 3) and delegated
//! to it. A bare name in a file position is the second, and whether it can be backed is a statement
//! about this shell's capability table rather than about the calendar: a shell whose the progenitor granted it a
//! directory (slot [`DIR_TERMINAL`]) resolves the name, and one granted none says "you hold no such
//! capability". Milestone 50 made the first case real at the interactive prompt; see [`holdings`],
//! notes/pipes.md and notes/grant-expression.md. `caps` prints the shell's whole endowment, and
//! `caps <command>` previews exactly what that command would grant, making DECISIONS §14's "reading
//! one literal tells you a process's whole authority" interactively true.
//!
//! **The clock is the one authority in a preview that no token designates** (milestone 51's wiring).
//! `date` declares it in its manifest and *The progenitor* endows it, read-only; `caps date` prints it anyway,
//! because a preview that showed only what the line designates would be off by exactly one
//! capability.
//!
//! **Since milestone 86 this shell holds a clock of its own**, at the slot the progenitor names in `x2`, and it
//! is `READ` **without** `GRANT`. That one bit is the whole design of `time <command>`: the shell can
//! read the wall clock to measure a command, and it cannot hand a clock to anything it spawns, so the
//! set of processes that can read the time is still decided by the manifests the progenitor reads (DECISIONS
//! §43). Timing is something an observer does with its own authority; the thing being timed needs
//! nothing and is not told. See notes/time-command.md.
//!
//! # The grammar lost two words in milestone 47
//!
//! There is no `run` verb: a bare program name spawns it, so builtins and programs are typed the
//! same way and nobody has to know which class a command is in. And there is no `file:` prefix: a
//! bare token in a file position designates the file, because the direction (the half that matters)
//! was always the manifest's to declare and the prefix only marked the half already on the screen.
//! `caps` is the visibility surface that remains, which is an argument for keeping it good.
//!
//! # The shell's world
//!
//! It holds, by convention (the progenitor granted them in this order):
//!
//! - slot 0: the terminal rendezvous (CALL: `OPERATION_WRITE` / `OPERATION_READLINE`).
//! - slot 1: a spawn rendezvous (direct the progenitor to start a program; `grant_plan::spawnproto`).
//! - slot 2: a result rendezvous (receive a spawned program's answer).
//! - slot 3: an untyped budget, the memory it grants with `--mem`.
//!
//! and two pages shared with the terminal: `OUT_VA` (we write text and prompts) and `LINE_VA`
//! (completed lines arrive). No role selector; the syscall runtime comes from `user_mode_runtime`.
//!
//! Two more slots are **wiring-dependent** and their numbers are therefore not constants here: the
//! directory ([`DIR_TERMINAL`], slot 4, when this boot has a filesystem) and the clock page, whose
//! slot arrives in `x2` because it sits after the filesystem pair and a boot with no disk has one
//! fewer capability under it. See [`CLOCK_SLOT`].
//!
//! # BUGS
//!
//! **A program's answer word on the result endpoint is the program's own claim.** A child whose
//! slot 0 was not redirected holds `result_ep` with `WRITE` (the spawn service's default in
//! `crates/system_initializer`), the same endpoint the spawn service's `SPAWN_FAILED`, the
//! `job_undertaker`'s `JOB_FAULTED` and this shell's `RESULT` reads share. The reads here take
//! three words and test `w0`, so a child can send `SPAWN_FAILED` (or, since milestone 715,
//! `SPAWN_NO_DISPLAY`) or a wrong exit status about itself; it cannot speak for another job,
//! because the wait is one job at a time. Since milestone 613 (a system log service) every
//! `RECEIVE` returns the sender's badge in `x3`, so a badged copy per sender would let this shell
//! tell the spawn service from a child at no new authority. Recorded by the 2026-10-03 security
//! audit, which met it beside its scope; a lie about one's own exit status is the lowest-value
//! thing a confined program can forge here.
//!
//! **A spawned command that faults no longer hangs the prompt, and here is what it costs**
//! (milestone 235, design/roadmap/0235-a-faulted-job-should-reach-the-prompt.md). This shell waits
//! on the job's result endpoint and a thread the kernel killed never sends on it, so until
//! `grant_plan::spawnproto::JOB_FAULTED` existed the wait had nothing to wake it. `job_undertaker`
//! now sends that word after collecting the corpse, this shell prints
//! [`swish::FAULTED_SENTENCE`] and `$?` reads 1. Measured rather than reasoned about, both before
//! and after: `components/src/least_authority_demo.rs` was patched to trap on one argument and `script/swish-check` run
//! against it, which failed with "the prompt never came back to take `least_authority_demo 7`" and then passed
//! through the same line.
//!
//! **A job that hangs without faulting still hangs the prompt**, which is the same symptom and a
//! different problem: a live thread blocked in a `RECEIVE` nobody will answer is not dead, nothing
//! reports it, and this shell cannot tell it from a slow one. Milestone 235's own block names this
//! as out of its scope and nothing here closes it. The forcible tier a person can reach already
//! exists for a job that was spawned interruptible (`^C` twice, DECISIONS §24); an ordinary command
//! is not one.
//!
//! **A fault reported while this shell is watching a screen-narrowed tail arrives one command
//! late.** `job_undertaker`'s own `BUGS` carries the mechanism and why closing it needs the syscall
//! surface.
//!
//! Name: ratified 2026-08-01 (calef, milestone 63), replacing `shell`. Refused `shell` (a category
//! rather than a name: `bash`, `zsh`, `fish` and `rc` are identities), `capsh` (Linux's libcap
//! ships `capsh(1)`, a capability shell wrapper, so a reader from Linux would assume ours is that
//! tool) and `sheesh` (it carries a 2020-21 timestamp where `bash` and `fish` are era-neutral, and
//! it is an interjection of exasperation, while refusing things is this shell's most characteristic
//! behavior by design). A swish is the shot that goes through the net touching nothing, which is
//! least authority in one word.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]
// `Vec::push_within_capacity`: the only way this shell adds to a `Vec`, because it cannot allocate
// behind the caller's back (see [`heap`]).
#![feature(vec_push_within_capacity)]

use filesystem_protocol::{dirent, fs};
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
