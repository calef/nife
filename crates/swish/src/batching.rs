//! **Batching at the bound** (milestone 109 (`xargs`: batching a grant too large to hand over)):
//! [`Sweep`], [`write_sweep`] and [`write_batch`].
//!
//! Moved out of the crate root by milestone 884 (the shell logic crate is split along its seams).
//!
//! Name: provisional (milestone 884, 2026-10-12 UTC), for calef's naming pass.

use super::*;

/// **The account of a batched line**: how many batches ran, how many names they designated, and
/// whether anything stopped the sweep.
///
/// It exists because a batched line is **one command with a partial effect**, which is a thing no
/// unbatched line here can be, and the only honest way to report one is to say where the boundary
/// fell. A sweep that stopped is a *prefix* of the match, which a person can hold in their head and
/// resume from; a sweep that carried on past a failure would be an arbitrary subset, and the user
/// could not tell which names it was.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Sweep {
    /// Batches that ran.
    pub batches: u64,
    /// Names those batches designated, which is the authority that actually moved.
    pub names: u64,
    /// The batch that stopped the sweep, counting from one, or `None` for a sweep that finished.
    pub stopped: Option<u64>,
}

impl Sweep {
    /// Record a batch that ran, carrying `names` names.
    pub fn ran(&mut self, names: u64) {
        self.batches += 1;
        self.names += names;
    }

    /// Record that the batch **after** the ones that ran did not, and stop.
    pub fn stop(&mut self) {
        self.stopped = Some(self.batches + 1);
    }
}

/// Write the sweep's account.
///
/// **The failing case says what was not attempted.** Unix's `xargs` carries on after a failed
/// invocation; here the shell printed each batch's set before it ran, so it names the boundary,
/// and batch four never runs after batch three failed.
///
/// ```
/// use swish::{Sweep, write_sweep};
///
/// let render = |s: &Sweep| {
///     let mut v = Vec::new();
///     write_sweep(s, &mut |b| v.extend_from_slice(b));
///     String::from_utf8(v).unwrap()
/// };
///
/// let mut done = Sweep::default();
/// done.ran(8);
/// done.ran(3);
/// assert_eq!(render(&done), "  11 names, in 2 batches\n");
///
/// // One batch is not a sweep worth reporting: the line behaved like any other line.
/// let mut one = Sweep::default();
/// one.ran(3);
/// assert_eq!(render(&one), "");
///
/// // And a stop names the boundary in both directions.
/// let mut stopped = Sweep::default();
/// stopped.ran(8);
/// stopped.stop();
/// assert_eq!(
///     render(&stopped),
///     "  batch 2 did not run: 8 names were handed over in 1 batch, and nothing after \
///      them was attempted\n",
/// );
///
/// // A sweep whose FIRST batch did not run says nothing: the refusal above it is the story.
/// let mut none = Sweep::default();
/// none.stop();
/// assert_eq!(render(&none), "");
/// ```
pub fn write_sweep(s: &Sweep, out: &mut dyn FnMut(&[u8])) {
    let batches = |n: u64, out: &mut dyn FnMut(&[u8])| {
        write_num(n, out);
        out(if n == 1 { b" batch" } else { b" batches" });
    };
    if s.batches == 0 {
        return;
    }
    match s.stopped {
        Some(at) => {
            out(b"  batch ");
            write_num(at, out);
            out(b" did not run: ");
            write_num(s.names, out);
            out(b" names were handed over in ");
            batches(s.batches, out);
            out(b", and nothing after them was attempted\n");
        }
        // **One batch prints nothing**, deliberately: `xargs` over a match that fits is the line the
        // user would have typed without it, and a footer under it would be noise claiming an event.
        None if s.batches == 1 => {}
        None => {
            out(b"  ");
            write_num(s.names, out);
            out(b" names, in ");
            batches(s.batches, out);
            out(b"\n");
        }
    }
}

/// Write one batch's set, before that batch runs.
///
/// **This is the per-batch form of the property the whole globbing lane exists for**: the expansion
/// you see is the grant. Under batching the set the user is shown has to be the set *this
/// invocation* is handed, not the union of the sweep, because the union is precisely the thing
/// nobody can hold. So it is printed once per batch, from the same [`write_set`] `echo` and the
/// grant preview use.
pub fn write_batch(index: u64, set: &NameSet, out: &mut dyn FnMut(&[u8])) {
    out(b"  batch ");
    write_num(index, out);
    out(b": ");
    write_set(set, out);
    out(b"\n");
}

#[cfg(test)]
mod tests;
