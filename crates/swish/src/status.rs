//! **What a command did**: [`Status`], which is what `$?` reports and what `&&` reads, and
//! [`STATUS_WORD`], the word that reads it.
//!
//! Moved out of the crate root by milestone 884 (the shell logic crate is split along its seams).
//!
//! Name: provisional (milestone 884, 2026-10-12 UTC), for calef's naming pass.

/// **What a command did**, which is what `$?` reports and what `&&` reads (milestone 67,
/// notes/swish-language.md).
///
/// # The fork this milestone had to settle: a refusal is not an error
///
/// On Unix `127` (no such command) and a program's own `exit(1)` are the same kind of integer, and
/// `&&` cannot tell them apart because the shell has nothing better to say. Here the two are
/// genuinely different events and the shell knows which:
///
/// - [`Refused`](Status::Refused) is **the shell declining to run the line**, decided at the prompt
///   from what this shell *holds* and what a manifest says, with nothing spawned, nothing opened
///   and no authority moved. It is reproducible: the same line refuses again. "You hold no such
///   capability", `Refusal::TooManyNames`, a pattern that matched nothing, a name that is not a
///   name.
/// - [`Failed`](Status::Failed) is **something was attempted and did not work**: the filesystem
///   answered with an errno, the progenitor had no memory to spawn with, a job was interrupted.
///
/// Separating them is the answer to "what does a status mean when the thing that failed was a
/// refusal", and it is worth a number because the two answer different questions. "Did my command
/// fail?" and "was I able to ask?" are not the same question, and a shell that refuses constantly
/// and by design should be able to say which one happened.
///
/// # What it is *not*, stated because the gap is real
///
/// **No program in this system reports an exit status**, and this value does not pretend one did. A
/// spawned program answers with a *value* (`least_authority_demo 7` answers 49), with bytes, or through a job
/// frame, and none of those is a status. So `$?` is the shell's own reading of what happened to the
/// line, which today is all there is. A per-program status is DECISIONS §301 (exit status and output)'s: `SYS_EXIT`'s first
/// word, forwarded in the `EVENT_EXIT` message, built by milestone 172 (a capability-native subprocess primitive) and not yet here.
///
/// # And what it cannot carry
///
/// One small integer, which designates nothing. There is no capability in it, no name, no handle:
/// a `&&` chain hands the next segment a **bit**, and that segment is planned from scratch against
/// what the shell holds, exactly as if it had been typed alone. See [`sequence`](crate::sequence).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Status {
    /// The command ran and the shell has nothing to report. `$?` is `0`.
    #[default]
    Ran,
    /// Something was attempted and did not work. `$?` is `1`.
    Failed,
    /// The shell would not run it, so nothing did. `$?` is `2`.
    Refused,
}

impl Status {
    /// Whether `&&` should carry on. Only [`Ran`](Status::Ran) is a yes.
    pub fn is_ok(self) -> bool {
        matches!(self, Status::Ran)
    }

    /// The number `$?` shows.
    pub fn code(self) -> u64 {
        match self {
            Status::Ran => 0,
            Status::Failed => 1,
            Status::Refused => 2,
        }
    }

    /// Read a status back out of [`code`](Status::code). The shell keeps it in an atomic cell,
    /// because every printer in the program can reach it and none of them holds a `&mut` to
    /// anything shared.
    pub fn from_code(code: u64) -> Status {
        match code {
            0 => Status::Ran,
            1 => Status::Failed,
            _ => Status::Refused,
        }
    }

    /// The number as bytes, which is `'static` because there are three of them.
    ///
    /// That is not a micro-optimization: a substituted word has to be a slice with the line's
    /// lifetime, and a `'static` slice unifies with any of them. The shell has had a capped heap
    /// since 2026-09-26, so a value that needs a buffer could now have one; what keeps it one word
    /// is [`pieces`](crate::pieces), not the absence of an allocator.
    pub fn digits(self) -> &'static [u8] {
        match self {
            Status::Ran => b"0",
            Status::Failed => b"1",
            Status::Refused => b"2",
        }
    }
}

/// **The word that reads the last command's status.** Recognized where words are expanded, which
/// today is [`echo`](crate::echo).
///
/// It is spelled `$?` because that is the spelling every shell user already knows, and this project
/// does not respell a name a reader arrives with. It is **not** a variable: there is no variable
/// mechanism here at all (milestone 47 owns that, and studies it as "the same question wearing a
/// string costume"), so this is one word the expander knows, in the same category as a pattern.
pub const STATUS_WORD: &[u8] = b"$?";

#[cfg(test)]
mod tests;
