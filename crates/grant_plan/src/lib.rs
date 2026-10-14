//! **`grant_plan`: the grant-expression logic of the capability shell** (milestone 31, phase 1).
//!
//! This crate is the pure logic behind the shell's one idea: on a nife command line,
//! *designation is authorization* (Mark Miller's principle). Naming a resource in a command is
//! how you grant it; a program that names nothing gets nothing beyond the report channel every
//! spawn carries. There is no ambient authority to fall back on, so the failure when a program
//! needs something the command did not grant is legible ("you hold no such capability"), not a
//! Unix-flavored EPERM.
//!
//! It performs no IO and makes no syscalls. It turns a line of bytes into a decision: a
//! [`Command`], and for a program invocation an [`Endowment`] (exactly what to grant the child) or a
//! typed [`Refusal`] the shell prints at the prompt. That split is DECISIONS §7 applied, the same
//! shape as `line_editor`: the parsing and the manifest checking are host-tested in milliseconds, and
//! only the wiring (the shell and the progenitor that carry the caps) needs QEMU. See
//! notes/grant-expression.md and notes/program-manifest.md.
//!
//! # The grammar, after milestone 47 took two words out of it
//!
//! ```text
//! <prog> [-abc] [--mem N] [token ...]
//! caps [command]
//! time <command>
//! help
//! echo <text>
//! ```
//!
//! `caps` and `time` are **prefix words**: their operand is a whole command line, operators
//! included, which is why neither is a program (a program takes tokens) and why both keep the tail
//! unsplit for the shell to act on.
//!
//! Short options arrived with `rm` (milestone 47's rmdir lane), and they are checked against the
//! program's [`Manifest`] at the prompt rather than passed along unread, because one of them can
//! **widen a grant**: `rm -r logs` hands over the subtree that `rm logs` does not.
//!
//! There is no `run` verb and no `file:` designator, and both removals are milestone 47's
//! (roadmap, "`file:` and `run` are not earned"). A bare program name spawns it, because a shell
//! where builtins are bare words and programs need a prefix makes the user learn which class a
//! command is in before they can type it. A bare token in a file position designates the file,
//! because `file:` marked the half of the grant that was already visible (*which* file) and said
//! nothing about the half that matters (read or write), which lives in the [`Manifest`] by design.
//!
//! # The three moving parts
//!
//! - [`parse`] tokenizes a command line into a [`Command`]. A first word that is not a builtin is a
//!   program invocation, and everything after it is the `--mem N` budget grant plus **unclassified
//!   positional tokens**: the parser deliberately does not decide which token is the argument and
//!   which is the file, because that is the manifest's job.
//! - A [`Manifest`] is a program's declared endowment: does it take an argument, does it require
//!   (or forbid) a memory grant, may it be granted a file and in which direction, does it report
//!   back. [`Prog::manifest`] is the static table.
//! - [`plan`] checks a parsed invocation against the named program's manifest, **placing the
//!   positional tokens into the slots the manifest declares**, and yields an [`Endowment`] or a
//!   [`Refusal`]. A mismatch is caught here, at the prompt, before anything is spawned, which is
//!   milestone 23's component contract in embryo.
//!
//! # The wire half
//!
//! [`spawnproto`] is the word layout for the shell-to-progenitor spawn protocol, the capability-shell
//! analogue of `line_editor::proto`. It is a userspace protocol (DECISIONS §21's shape): the kernel
//! routes the words and never reads them.
//!
//! Name: ratified 2026-08-01 (calef, milestone 63), replacing `capsh`. Refused `capsh` (Linux's
//! libcap ships `capsh(1)`, a capability shell wrapper, so a reader arriving from Linux would
//! assume ours is that tool); `designation`, `designate` and `designator` together (the user
//! designates by typing a name, and this crate's work starts after that, so all three put it in a
//! role it does not hold); and the grant synonyms `endow`, `award`, `confer`, `bestow`, `allot` and
//! `furnish` (grant is already this tree's word and a synonym is a decoder ring, and `endow` is
//! additionally taken by `supervision_protocol::ChildEndowment`). Deliberately not named for `swish`: seven
//! things use it, so naming it for one consumer would repeat `dwarden`'s defect.

#![no_std]
// milestone 68's ratchet is workspace-wide (§107); this crate opts out until its 22-item
// worklist (notes/doc-coverage.md) is burned down.
#![allow(missing_docs)]

pub mod expand;
pub mod image;
pub mod job_page_frame;
pub mod job_windows;
pub mod line;
pub mod nav;
pub mod spawnproto;
pub mod verbs;
pub mod word;

mod command;
mod escalation;
mod manifest;
mod parsing;
mod planning;
mod program;
mod refusal;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

pub use command::{
    Command, DirGrant, Endowment, FileGrant, Holdings, MAX_FLAGS, SecondDir, TouchArgs, rmopt,
};
pub use escalation::{Action, COOP_GRACE_TICKS, Escalation};
pub use image::{
    IMAGE_ROW, ImageRefusal, NO_NOTE_MANIFEST, image_can_carry, image_hears_words, image_manifest,
    image_request_fits,
};
pub use manifest::{
    ArgSpec, CATALOG_SLOT, DIAGNOSTICS_SLOT, DOMAIN_SLOT, DirSpec, ENTROPY_SLOT, FileSpec, Flags,
    INDEX_SOURCES_SLOT, INSTALLER_REPLY_SLOT, INSTALLER_SLOT, IndexStore, InputSpec, MACHINE_SLOT,
    MAX_DECLARED_FLAGS, Manifest, MemSpec, NETWORK_SLOT, NO_FLAGS, OutputSpec, REBOOT_SLOT,
    Runtime, SHARE_SLOT, SHELL_CONFIG_SLOT, SHELL_CONFIG_VA, STD_REGION_PAGES, SYNC_SLOT,
    UNVOUCHED_MANIFEST, UNVOUCHED_STD_MANIFEST, WordGrant, image_region_pages,
    named_std_region_pages,
};
pub use parsing::{
    BUILTINS, MAX_POSITIONALS, RunSpec, UserVerb, argv, check_words, each_word, parse, parse_run,
    tokenize, user_verb, vouch_verb, vouched_name,
};
pub(crate) use planning::nav_refusal;
pub use planning::{
    Streams, check_chain, plan, plan_against, plan_against_with, plan_stage, redirect_target,
};
pub use program::{PROG_COUNT, Prog};
pub use refusal::{CapabilityKind, Refusal};
pub use verbs::{Grants, MAX_VERB, VerbGrant, narrow_by_verb, verb_of, verb_of_page};

mod lexing;

use lexing::*;
pub use lexing::{MAX_FILE_NAME, file_name_fits, parse_u64, trim};

// ---- small byte helpers (no_std, no alloc) ----
