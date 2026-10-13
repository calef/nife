//! **`grant_plan`: the grant-expression logic of the capability shell** (milestone 31, phase 1).
//!
//! On a nife command line *designation is authorization* (Mark Miller's principle): naming a
//! resource in a command is how you grant it, and a program that names nothing gets nothing beyond
//! the report channel every spawn carries. The failure when a program needs something the command
//! did not grant is legible ("you hold no such capability"), not a Unix-flavored EPERM.
//!
//! It performs no IO and makes no syscalls. It turns a line of bytes into a [`Command`], and for a
//! program invocation an [`Endowment`] (exactly what to grant the child) or a typed [`Refusal`] the
//! shell prints at the prompt, host-tested (DECISIONS §7 (testing: QEMU harness + host-testable
//! crates)). See notes/grant-expression.md and notes/program-manifest.md.
//!
//! # The grammar
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
//! included, kept unsplit for the shell to act on. Short options are checked against the program's
//! [`Manifest`] at the prompt, because one of them can **widen a grant**: `rm -r logs` hands over
//! the subtree that `rm logs` does not. A bare program name spawns it, and a bare token in a file
//! position designates the file; read or write lives in the [`Manifest`].
//!

//! # The moving parts
//!
//! - [`parse`] tokenizes a command line into a [`Command`]. A first word that is not a builtin is a
//!   program invocation, and everything after it is the `--mem N` budget grant plus **unclassified
//!   positional tokens**: which token is the argument and which the file is the manifest's job.
//! - A [`Manifest`] is a program's declared endowment: does it take an argument, does it require
//!   (or forbid) a memory grant, may it be granted a file and in which direction, does it report
//!   back. [`Prog::manifest`] is the static table.
//! - [`plan`] checks a parsed invocation against the named program's manifest, **placing the
//!   positional tokens into the slots the manifest declares**, and yields an [`Endowment`] or a
//!   [`Refusal`], at the prompt, before anything is spawned.
//! - [`spawnproto`] is the word layout for the shell-to-progenitor spawn protocol, a userspace
//!   protocol (DECISIONS §21 (the terminal is a userspace component)): the kernel routes the words and
//!   never reads them.
//!
//! Name: ratified 2026-08-01 (calef, milestone 63), replacing `capsh`. Refused `capsh` (Linux's
//! `capsh(1)` is a different tool). Refused `designation`, `designate` and `designator` (the user
//! designates; this crate's work starts after that). Refused `endow`, `award`, `confer`, `bestow`,
//! `allot` and `furnish` (grant is already this tree's word, and `endow` is taken by
//! `supervision_protocol::ChildEndowment`). Not named for `swish`: seven things use it.

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

/// The longest file name a per-file grant can carry. Duplicated from `filesystem_protocol::grant::MAX_NAME`
/// rather than imported, because `grant_plan` is the shell's parser and must not depend on the filesystem
/// contract to check a command line; the pair is pinned by a test in each crate so a change to one
/// without the other fails on the host in milliseconds.
pub const MAX_FILE_NAME: usize = 16;

/// Whether a designated name can travel as a grant at all. A name is a single component, the same
/// rule the FS server enforces (DECISIONS §27): a path is not something this shell can express,
/// because there is no namespace here to walk.
pub fn file_name_fits(name: &[u8]) -> bool {
    !name.is_empty()
        && name.len() <= MAX_FILE_NAME
        && name != b"."
        && name != b".."
        && !name.contains(&b'/')
        && !name.contains(&b'\\')
}

// ---- small byte helpers (no_std, no alloc) ----

/// Trim ASCII whitespace from both ends.
pub fn trim(s: &[u8]) -> &[u8] {
    let mut a = 0;
    let mut b = s.len();
    while a < b && s[a].is_ascii_whitespace() {
        a += 1;
    }
    while b > a && s[b - 1].is_ascii_whitespace() {
        b -= 1;
    }
    &s[a..b]
}

/// Split off the first whitespace-delimited word; return `(word, rest)` where `rest` keeps its
/// internal spacing (so `echo` can print it verbatim). The word keeps its quotes if it had any;
/// [`parse`] takes them off, because whether the word is a builtin is a question about what it
/// says rather than about how it was spelled.
fn split_first_word(s: &[u8]) -> (&[u8], &[u8]) {
    let mut i = word::span(s, 0, &|b| b.is_ascii_whitespace());
    let word = &s[..i];
    while i < s.len() && s[i].is_ascii_whitespace() {
        i += 1;
    }
    (word, &s[i..])
}

/// Parse a base-10 `u64`. `None` for empty or any non-digit byte, so `--mem twelve` is a missing
/// grant rather than a silent zero.
pub fn parse_u64(s: &[u8]) -> Option<u64> {
    if s.is_empty() {
        return None;
    }
    let mut v: u64 = 0;
    for &b in s {
        if !b.is_ascii_digit() {
            return None;
        }
        v = v.checked_mul(10)?.checked_add((b - b'0') as u64)?;
    }
    Some(v)
}
