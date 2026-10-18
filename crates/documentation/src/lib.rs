//! **The documentation crate**: markdown in, styled terminal bytes out, and the index that says
//! what exists. It is the pure half of the documentation service: a streaming
//! renderer ([`Renderer`]), the byte layout of a search index ([`index`]), and the query that reads
//! one. No IO, syscall, endpoint or allocator is on a confined program's path;
//! `components/src/mdr.rs` is the program. The crate and program names differ on purpose: the
//! crate is a superset of what the program does.
//!
//! It renders ATX headings, wrapped paragraphs, fenced code, block quotes, nested bullet and
//! ordered lists, aligned tables, thematic breaks, and the inline set `**strong**`, `*emphasis*`,
//! `` `code` ``, `~~strike~~`, `[text](dest)` and `![alt](dest)`: every construct this
//! repository's own markdown uses, which is the corpus the viewer serves. Losing text is the one
//! failure a documentation service cannot have, so oversized tables degrade their layout instead
//! ([`TABLE_ROWS`], [`TABLE_COLS`]).
//!
//! # BUGS
//!
//! - A source line longer than [`LINE_MAX`] is truncated silently in the output; the repository's
//!   longest is 1928 bytes <!--count:longest-markdown-line-->. [`Renderer::is_truncated`] reports it.
//! - A lazy continuation inside a quoted fence keeps its quote marker and stays code, where
//!   `CommonMark` would end the fence. `a_fence_inside_a_block_quote_closes` holds the quoted
//!   fence itself, which only a unit test can see.
//! - Setext headings are not recognised: `---` alone is far more often a thematic break here.
//! - Reference links render as literal text, and HTML passes through as text.
//! - A table spilling past [`TABLE_ROWS`] loses header emphasis and alignment in its second chunk,
//!   since the delimiter row is not carried across the flush. No rows are lost.
//! - Width counts characters, not columns, so a wide CJK character counts as one.
//! - The mutation residue is listed in notes/mutation-testing.md. One is a hang: `j /= 1` in
//!   `take_table_row`'s trim loop never terminates; replacing the loop with a slice trim removes it
//!   (`notes/project-metrics/mutation-triage.csv`).
//!
//! Name: ratified 2026-09-13 (calef). Refused `manual`, `doc`, `documentation_system`, `markdown`.

#![no_std]

#[cfg(feature = "builder")]
extern crate alloc;

pub mod index;
mod render;

pub use render::{Attr, LINE_MAX, MAX_DEPTH, Renderer, Style, TABLE_COLS, TABLE_ROWS, TABLE_TEXT};

/// Somewhere rendered bytes go.
///
/// The same shape as [`line_editor::Sink`](../line_editor/trait.Sink.html), and deliberately not
/// that trait: this crate has no dependencies, and a renderer that needed the line discipline crate
/// to describe "bytes leave here" would have taken one for a four-line definition. A caller that
/// holds both can implement this over that in one line.
///
/// **An implementation may be called with an empty slice** and must tolerate it.
pub trait Sink {
    /// Take these bytes. There is no partial acceptance and no error: a sink that cannot take them
    /// drops them, which is the same choice `byte_sink_contract` makes for a destination that has gone.
    fn put(&mut self, bytes: &[u8]);
}
