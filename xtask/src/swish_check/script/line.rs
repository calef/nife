//! **The row type and its constructors**: [`Line`], what every script here is a slice
//! of, `line` and `keys`, which build one, and `JOB_DID_NOT_RUN`, the sentences a refused
//! job prints.
//!
//! Cut from `script.rs` by milestone 900 (the swish-check script splits by topic before it
//! crosses the ceiling).
//!
//! Name: provisional (milestone 900's lane, 2026-10-15 UTC).

/// **One line this gate types, what it must answer, and how many jobs it runs.**
///
/// The job count sits beside the line because it is a fact about the line, and until 2026-09-26 it
/// was a hand-kept total in the success summary instead: every lane that added a line had to edit
/// that total and the script's declared length, so any two such lanes conflicted (five rebases in
/// one day: #1318, #1320, #1322, #1329 and #1330). Now the script is a slice, the total is summed
/// from the lines each leg actually typed, and adding a line touches the line.
///
/// `jobs` is how many children the progenitor builds from its bounded job pool for this line: one
/// per program stage that runs. **Zero** for a builtin (`echo`, `ls`, `apropos`, `package`), a
/// `caps` preview, a line refused at the prompt with nothing spawned, an image the activation set
/// refuses, and a supervised job (`interrupt_heeder` and `interrupt_ignorer`, built from the
/// shell's own untyped rather than the pool). A pipeline counts each program stage, so
/// `wc gate.txt | wc` is two; a `builtin | program` pipeline is one. `rm` with its caretaker is one
/// job whose region holds two processes. It is a required field with no default, so a line cannot
/// be added without saying.
///
/// **The count is checked, not only printed.** A line with `jobs > 0` whose answer carries the
/// shell's "could not spawn" or "faulted" sentence fails the gate even when its wanted phrases are
/// empty (`uuid > id.txt`), which is what makes "ran N jobs" in the summary a statement this gate
/// verified. What it cannot catch is a tag that is too low, since nothing at the prompt reports a
/// job that nobody counted; the host test `a_job_count_names_a_program` bounds a tag from above.
#[derive(Clone)]
pub(in crate::swish_check) struct Line {
    pub(crate) typed: &'static str,
    /// What the prompt shows once `typed` has been edited: the same text for a plain line, and
    /// the finished word for one with a Tab in it, or `^C` for one abandoned (see [`keys`]).
    pub(crate) echoed: &'static str,
    pub(crate) jobs: u8,
    pub(crate) answer: &'static [&'static str],
}

/// **What the shell prints when a job it asked for never answered**, either half:
/// `components/src/swish.rs` prints the first when the progenitor could not build the job, and the
/// second is `swish::FAULTED_SENTENCE`, for a job that trapped first. Spelled here rather than
/// imported because xtask does not depend on `swish`, and a phrase is enough for a transcript.
pub(in crate::swish_check) const JOB_DID_NOT_RUN: [&str; 2] = [
    "could not spawn",
    "that command faulted and was killed before it answered",
];

/// A [`Line`], positionally, so the script reads as the prompt does. The name is provisional.
pub(in crate::swish_check) const fn line(jobs: u8, typed: &'static str, answer: &'static [&'static str]) -> Line {
    Line {
        typed,
        echoed: typed,
        jobs,
        answer,
    }
}

/// **A line typed with editing keys in it** (milestone 47 (navigation and naming), DECISIONS §227
/// (how Tab reaches the shell)): `typed` goes to the UART byte for byte, and `echoed` is what the
/// prompt then shows. A line ending in `^C` (`\x03`) is sent with no Enter, and must answer
/// nothing, because the shell discarded it. The name is provisional.
pub(in crate::swish_check) const fn keys(
    jobs: u8,
    typed: &'static str,
    echoed: &'static str,
    answer: &'static [&'static str],
) -> Line {
    Line {
        typed,
        echoed,
        jobs,
        answer,
    }
}

/// Whether a scripted line is abandoned with `^C` rather than run.
pub(in crate::swish_check) fn interrupted_at_prompt(typed: &str) -> bool {
    typed.ends_with('\x03')
}

