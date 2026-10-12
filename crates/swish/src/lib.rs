//! **The capability shell's logic, with the IO taken out** (milestone 70).
//!
//! `components/src/swish.rs` is the program: it holds a terminal endpoint, a spawn channel, a result
//! channel, a budget, and (in the wiring that has one) a directory capability. This crate is
//! everything that shell decides or renders with **no capability in hand**, so it compiles for the
//! host and its tests run in milliseconds instead of under QEMU.
//!
//! The split follows the line CLAUDE.md draws for `coremark`, `line_editor` and `compositor`: a
//! crate and a program share a name when the crate is that program's logic. Reading it the other
//! way is the useful direction. If a function here needed a capability, it would not be here.
//!
//! # What is in here
//!
//! - **The routing of a typed line.** [`route`] answers "what kind of thing did the user type",
//!   including the one ordering that has a bug behind it: the prefix words (`caps`, `time`) are
//!   answered *before* the line is split on its operators, or `caps date | wc` would lose everything
//!   after the pipe.
//! - **What a duration reads as.** [`write_duration`] and [`write_timing`] (milestone 86), plus
//!   [`write_untimed`] for the three reasons there is no duration to print. The arithmetic is here
//!   rather than in the program because it is arithmetic, and the program's half is two reads of a
//!   page it holds a capability to.
//! - **Pattern versus text.** [`is_pattern`] and [`expansion`], which decide whether a word is a
//!   designation to resolve or arbitrary text to print, and which word on a line gets expanded.
//! - **Every sentence the user reads.** [`write_say`], [`write_refusal`], [`write_outcome`],
//!   [`write_preview`], [`write_holdings`], [`write_help`], [`write_pwd`], [`write_num`].
//!
//! # How a renderer is written here
//!
//! Every rendering function takes `out: &mut dyn FnMut(&[u8])` rather than printing. The program
//! passes its terminal `print`; a test passes a collector; the guest witnesses (which have no
//! terminal at all) pass a buffer. That was already the shape `ls` and `echo` used inside the shell,
//! so nothing about the program's IO had to be restructured to lift the rest.
//!
//! Anything that needs a directory listing takes a second callback, `expand`, of the same shape:
//! the *matching* is pure and lives in `grant_plan::expand`, and only reading the directory needs a
//! capability. So [`echo`] and [`expansion`] are testable end to end against a fake directory.
//!
//! ```
//! use grant_plan::expand::NameSet;
//! use swish::{Say, echo};
//!
//! // A directory holding two files, standing in for the one the shell would have to ask a server
//! // for. Everything else on this path is the real thing.
//! let mut listing = |_pattern: &[u8]| -> Result<NameSet, Say> {
//!     let mut set = NameSet::empty();
//!     set.push(b"notes.txt", false);
//!     set.push(b"report.txt", false);
//!     Ok(set)
//! };
//!
//! let mut printed = Vec::new();
//! let said = echo(
//!     b"the set: *.txt",
//!     swish::Status::Ran,
//!     &mut listing,
//!     &mut |b| printed.extend_from_slice(b),
//! );
//!
//! assert!(matches!(said, Say::Nothing));
//! // The words with no magic in them came through byte for byte; the pattern became what it
//! // designates, which is exactly what `rm *.txt` would have been granted.
//! assert_eq!(printed, b"the set: notes.txt report.txt");
//! ```
//!
//! # BUGS
//!
//! Three of the shell's functions are **not** here and could not be lifted without restructuring
//! its IO, which milestone 70 was scoped not to do:
//!
//! - `builtin` and `dispatch_one`, because every arm is a capability invocation (`cd`, `ls` and
//!   `mkdir` are requests to the filesystem server) or a print. [`route`] lifts the decision they
//!   sit under; the arms themselves stay with the wiring.
//! - `run`, whose body is two calls into `grant_plan` (both already host-tested there) wrapped
//!   around a choice between two spawn paths. Lifting it would have moved the spawn decision away
//!   from the code that can act on it and bought no new coverage.
//! - `spawn`, `pipeline` and everything below them, which is capability movement and nothing else.
//!
//! Name: ratified 2026-08-01 (calef, milestone 63), replacing `shell`. Refused `shell` (a category
//! rather than a name: `bash`, `zsh`, `fish` and `rc` are identities), `capsh` (Linux's libcap
//! ships `capsh(1)`) and `sheesh` (it carries a 2020-21 timestamp where `bash` and `fish` are
//! era-neutral, and it is an interjection of exasperation, while this shell's most characteristic
//! behavior is refusing things by design). A swish is the shot that goes through the net touching
//! nothing, which is least authority in one word. The crate takes the program's name because the
//! crate is that program's logic (DECISIONS §63); it was lifted out on 2026-08-02 by milestone 70.

#![no_std]

pub mod bare;
pub mod complete;
pub mod jig_versions;
pub mod sequence;

mod apropos;
mod batching;
mod endowment;
mod number;
mod preview;
mod routing;
mod say;
mod sentences;
mod status;
#[cfg(test)]
mod test_support;
mod timing;
mod words;

#[cfg(test)]
extern crate std;

pub use apropos::{write_apropos, write_found};
pub use batching::{Sweep, write_batch, write_sweep};
use endowment::write_config_values;
pub use endowment::{write_census, write_holdings};
use environment_protocol::ConfigPage;
use filesystem_protocol::dir;
use grant_plan::expand::{Expansion, NameSet};
use grant_plan::line::{self, Line};
use grant_plan::nav::{self, Refused};
use grant_plan::{
    ArgSpec, Command, Endowment, Holdings, OutputSpec, Prog, Refusal, RunSpec, Streams, spawnproto,
};
pub use number::write_num;
pub use preview::{Vouched, write_capabilities, write_image_capabilities, write_preview};
pub use routing::{Route, route};
pub use say::{HEAP_MAX_BYTES, Say, write_say};
pub use sentences::{
    FAULTED_SENTENCE, NO_DISPLAY_SENTENCE, REFUSED_BY_MANIFEST_SENTENCE, UNVOUCHED_SENTENCE,
    USER_USAGE, VOUCH_USAGE, image_refusal_sentence, write_help, write_outcome, write_pwd,
    write_refusal, write_vouch,
};
pub use status::{STATUS_WORD, Status};
pub use timing::{Untimed, write_duration, write_timing, write_untimed};
pub use words::{Piece, designation, echo, expansion, is_pattern, pieces, write_set};
