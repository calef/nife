//! **The routing of a typed line**: [`Route`] and [`route`], including the one ordering with a
//! bug behind it, the prefix words answered before the line is split on its operators.
//!
//! Moved out of the crate root by milestone 884 (the shell logic crate is split along its seams).
//!
//! Name: provisional (milestone 884, 2026-10-12 UTC), for calef's naming pass.

use super::*;

/// **What kind of thing the user typed**, decided before anything is invoked.
///
/// The variants are not three grammars. They are the three shapes the shell has to act on
/// differently, and the order they are decided in is load-bearing (see [`route`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Route<'a> {
    /// `caps`, with everything after it. The tail is a **whole command line**, operators included.
    Caps(&'a [u8]),
    /// `time`, with everything after it (milestone 86). The tail is a whole command line for
    /// [`Caps`](Route::Caps)'s reason, and the shell **re-dispatches** it: what gets timed is the
    /// line taking the path it would have taken untimed, so "what you time is what you run" is a
    /// property of the code rather than a claim about it.
    Time(&'a [u8]),
    /// `xargs`, with everything after it (milestone 109). The tail is a whole command line for
    /// [`Caps`](Route::Caps)'s reason, and the shell re-dispatches it **once per batch** of what its
    /// pattern matched. The name is provisional.
    Xargs(&'a [u8]),
    /// One command, with no operators on it: the path every line took before milestone 50.
    One(&'a [u8]),
    /// A line with `>`, `<` or `|` on it, already split into stages.
    Pipeline(Line<'a>),
    /// The line cannot be split at all, so there is nothing to run.
    Cannot(Refusal),
}

/// **Read one line and decide what it is.**
///
/// The two prefix words are asked about **before** the line is split, and that ordering is the whole
/// reason this is a function worth having. Their operand *is* a command line, so they have to see
/// the operators: `caps date | wc` must print two stages, and splitting first would hand it the word
/// `date` and silently lose the rest. `time date | wc` is the same shape with the same bug behind
/// it, one milestone later, and it costs one arm rather than a second mechanism.
///
/// A line with no operators takes exactly the path it took before they existed, which is deliberate.
/// The operators must not make an ordinary command more expensive or more complicated, because an
/// ordinary command is nearly every command.
///
/// ```
/// use swish::{Route, route};
///
/// // The tail keeps the pipe. `caps` will preview two stages, not one.
/// assert!(matches!(route(b"caps date | wc"), Route::Caps(b"date | wc")));
/// // And `time` keeps it for the same reason: it times the pipeline, not its first word.
/// assert!(matches!(route(b"time date | wc"), Route::Time(b"date | wc")));
/// // The third prefix word (milestone 109), which batches a whole line for the same reason.
/// assert!(matches!(route(b"xargs rm *.txt"), Route::Xargs(b"rm *.txt")));
/// assert!(matches!(route(b"least_authority_demo 7"), Route::One(b"least_authority_demo 7")));
/// match route(b"date | wc") {
///     Route::Pipeline(l) => assert_eq!(l.stages().len(), 2),
///     other => panic!("expected a pipeline, got {other:?}"),
/// }
/// ```
pub fn route(cmd: &[u8]) -> Route<'_> {
    match grant_plan::parse(cmd) {
        Command::Caps(tail) => return Route::Caps(tail),
        Command::Time(tail) => return Route::Time(tail),
        Command::Xargs(tail) => return Route::Xargs(tail),
        _ => {}
    }
    match line::split(cmd) {
        Ok(l) if l.is_plain() => Route::One(l.stages()[0]),
        Ok(l) => Route::Pipeline(l),
        Err(r) => Route::Cannot(r),
    }
}

#[cfg(test)]
mod tests;
