//! **`installed_programs`**: what is installed, as a table that is never edited, only succeeded (§208
//! (installing a package is granting it, and the installed programs are versioned)). The progenitor reads
//! the live generation on every image spawn and looks the executable's digest up ([`lookup_digest`]);
//! the shell reads it to resolve a bare word and preview `caps`. Both take the location from here
//! (rule 7): a directory [`DIRECTORY`] holding [`CURRENT`] and one file per generation
//! ([`generation_name`]).
//!
//! **A generation is a text file that is never rewritten.** Install and remove write the next
//! generation ([`with_entries`], [`with_entry`], [`without_package`]), and a one-line `current`
//! names the live one ([`parse_current`], [`format_current`]), so a rollback rewrites one line, as
//! a Nix profile does. A generation holds two kinds of line:
//!
//! ```text
//! <digest> <program> <version> <package> <manager>   a row: what is installed, and who installed it
//! default <program> <digest>                         the pointer: what the bare word runs
//! grant <digest> network                             the owner granted those bytes the network
//! grant <digest> zone <zone>                         ... and names in one resolver zone
//! ```

//! **A `grant` line is the owner's grant** (calef, 2026-10-19 (UTC), on #2405: "a"), what milestone
//! 911 calls the grant record: keyed on the executable's digest, one line per grant, sorted
//! bytewise and each once, after the pointers. A zone is spelled as the resolver spells it
//! (`domain_name_resolution_contract::is_canonical_zone`), and the root zone, any name, is `.`
//! (provisional spelling). A grant whose digest names no row makes the table unreadable, as a
//! pointer to nothing does. A narrowing writes the next generation without the line; a removal
//! takes a removed row's grants with it.
//!
//! A `<digest>` is `sha256:<64 hex>`, as `measured_boot::digest_text` writes it. **Rows key on the
//! digest**, the packager's attestation; `program`, `version` and `package` are labels. A rebuild
//! claiming a live version is a second, visible row, and two versions coexist. No architecture
//! column (the digest is of target-specific bytes) and no clock (the generation index orders).
//! The default pointer lives in the generation file, so a rollback restores it too, and every
//! install moves it. An owner's vouch ([`OWNER`]) has version [`NO_VERSION`], claims no name
//! (§229 (how a bare name at the prompt reaches an installed program)) and never writes a pointer.
//! Any malformed line makes the whole table [`Error::Malformed`]: a half-parsed table would vouch
//! for whatever survived.
//!
//! # BUGS
//!
//! - Nothing collects old generations; a retention rule is owed before frequent installs.
//! - A table is read whole into the caller's buffer, about 140 bytes a row.
//! - A generation is read and written in one page by the progenitor, and a row with its pointer
//!   takes about 170 bytes, so one generation holds about two dozen installed programs.
//! - Nothing makes the `current` write atomic; a torn write on RedoxFS was not checked.
//!
//! Name: ratified 2026-10-18 (calef, "`installed_programs`"). Each row is a program, keyed by its
//! executable's digest, that may run, and vouching goes through the same installer as install and
//! remove, so "installed" covers every row.

#![no_std]

pub use measured_boot::Digest;

mod generation;
mod grants;
pub use generation::*;
pub use grants::{Grant, Granted, ROOT_ZONE, grants, network_granted};
