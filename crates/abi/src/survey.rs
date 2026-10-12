/// The `next_cursor` that means the survey is complete. It is also the cursor to start one
/// with, which is not a collision: a caller passes it in once and stops when it comes back.
pub const DONE: u64 = 0;

/// On a run queue, waiting for a CPU.
pub const READY: u64 = 1;
/// On a CPU right now. On a multi-core machine this may include the surveyor's own thread, if
/// the surveyor is somehow its own supervisor's child.
pub const RUNNING: u64 = 2;
/// Blocked in an IPC rendezvous that has not happened.
pub const BLOCKED: u64 = 3;
/// A corpse (DECISIONS §26): it faulted or exited, its supervisor was told, and it persists
/// until [`rendezvous::REAP`](super::rendezvous::REAP) collects it. **This is the state `ps` exists to
/// make visible**, and
/// it is the one Unix cannot show you without a parent that happens to have called `wait`.
pub const DEAD: u64 = 4;

/// **Which per-thread record a [`SURVEY`](super::rendezvous::SURVEY) asks for.** The selector
/// calef ruled on 2026-09-21: a new per-thread fact is a new value here, never a new return
/// register.
///
/// The value rides in x1, the argument `SURVEY` never used, and lands in x2 of the answer. x0
/// (the cursor) and x1 (the tid) are the same for every record, so the walk a caller writes is
/// the same walk whichever record it wants.
///
/// Names provisional: calef names public items.
pub mod record {
    /// **What run state the thread is in**: one of this module's parent's state codes. The
    /// record `SURVEY` answered before selectors existed, and **0 so that it still does**.
    /// Every caller written against the old three-word contract passed 0 into the then-unused
    /// x1, so it selects this record by accident and keeps working on purpose.
    pub const STATE: u64 = 0;

    /// **Which core the thread was placed on when it started**: a cpu id, or [`NO_CPU`].
    ///
    /// The fact `sched::spawn_reporting_placement` has given the in-kernel job-mix supervisor
    /// since milestone 240 (the soak reports what happened and not where) and that had no userspace
    /// path, which is what kept that supervisor
    /// in the kernel.
    ///
    /// **Placement, not "where it is running now"**, and the difference is deliberate rather
    /// than an approximation nobody got around to sharpening. Tracking the current core means
    /// a store in the context switch, which is the hottest line of the hottest function: the
    /// kernel's `Thread::last_cpu` does exactly that and is behind a soak-build feature gate
    /// because shipping it unconditionally cost 5.7% of `ipc_fastpath`'s footprint on aarch64,
    /// over milestone 132 (the fast path's footprint, and a gate)'s 5% bound. Placement is written once, when the thread starts, so it
    /// costs the IPC path nothing. A thread that was stolen onto another core, or woken onto
    /// its waker's, still reports where it was placed. A reader wanting "now" does not have it
    /// and must not read this as it.
    ///
    /// **The id is a name, not an index**, and this is the sentence that keeps a reader out of
    /// the bug `cpu_set` exists to kill. The online set is not `0..n`: on the VisionFive 2 it
    /// is `{1, 2, 3}`, because slot 0 is an M-mode monitor core with no MMU, and treating the
    /// count as an index put `init` into a parked core's inbox for three boots. So key a
    /// per-core tally by the id this record returns and never iterate a range. A census built
    /// that way needs no online mask at all: the ids it observes are by construction a subset
    /// of the online set, which is exactly what a placement census is asking about. A reader
    /// that genuinely needs the machine's online set does **not** get it here, and has no path
    /// to it today; that gap is stated where this record is documented rather than left for
    /// somebody to paper over with a range.
    pub const PLACEMENT: u64 = 1;

    /// The [`PLACEMENT`] answer for a thread with no placement to report. `u64::MAX` rather
    /// than a plausible small number, so a reader that ignores it is wrong loudly rather than
    /// quietly tallying core 255.
    ///
    /// Every thread a survey can report has started, so this should not be reachable: a
    /// supervision endpoint is recorded at `START` (DECISIONS §26 (the fault endpoint: thread death becomes a
    /// message a supervisor holds)), so an embryo is not in its
    /// domain yet, and a corpse keeps the placement it was started with. It is defined anyway
    /// rather than left to a guess, which is the same posture `sched::last_cpus` takes with its
    /// own `u8::MAX`.
    pub const NO_CPU: u64 = u64::MAX;

    /// **How long the thread has been scheduled on a CPU, in milliseconds.** Scheduled
    /// on-CPU time, which is what Linux's `utime`/`stime` and Zircon's
    /// `zx_info_thread_stats_t::total_runtime` report, and what a reader who knows `top`
    /// expects `%CPU` to be derived from. DECISIONS §150 (how does a thread's CPU time reach userspace?) refused the two cheaper
    /// answers: wall-clock age reads identically for a thread that slept five minutes and one
    /// that ran five minutes, and userspace sampling over `SURVEY` misses every thread that
    /// runs between two samples.
    ///
    /// **Milliseconds rather than ticks, and the unit is the wire contract.** A tick count
    /// would oblige every reader to learn this kernel's `TICK_HZ` and would silently change
    /// meaning the day that constant moved; a millisecond means the same thing in every build
    /// and on every architecture. The granularity is nonetheless the tick, which is 10 ms on
    /// all three architectures today, so this number moves in steps rather than smoothly.
    ///
    /// **It is sampled at the timer tick, not accumulated at the context switch**, which is
    /// §150's sub-choice 1 and has a visible consequence rather than only an implementation
    /// one: a thread that runs entirely between two ticks is charged **nothing**, and a thread
    /// that happens to be on the CPU at every tick is charged for the whole of each one. That
    /// is `jiffies`-based accounting, the bargain Linux also takes, and it is why a reader must
    /// not treat two of these numbers that should have matched as a contradiction.
    ///
    /// **What a holder of [`rights::ENUMERATE`](crate::rights::ENUMERATE) learns, said plainly
    /// because it is more than the record before it.** A run state is one of five values and
    /// says only what a thread is doing at the instant of the call; this is a **continuous,
    /// monotonic** signal about a thread the viewer may not otherwise be able to name, and two
    /// reads of it measure how much work that thread did in between. A confined viewer holding
    /// a supervision endpoint can therefore watch the shape of another thread's workload
    /// without holding anything that lets it act on that thread. §150 weighed that and accepted
    /// it, on the ground that `ENUMERATE` is already "the right to learn what exists, as
    /// distinct from acting on it", and the leak is bounded by the domain: a survey never
    /// reaches outside the supervision domain the caller was endowed. §204 (how userspace asks where a thread runs) records that
    /// [`PLACEMENT`] is strictly less than this, which is the comparison that makes the
    /// magnitude concrete rather than adjectival.
    ///
    /// Name: provisional. calef names public items.
    pub const CPU_TIME: u64 = 2;

    /// **Whether this kernel answers a record.** The one enumeration of the selector space, so
    /// adding a record is an edit here and an arm in the kernel's walk rather than a hunt.
    ///
    /// The caller refuses an unknown record with [`crate::Error::BadMethod`] *before* walking
    /// the domain, which is why this is a predicate rather than a `match` folded into the
    /// extraction: a bad selector must be refused even when the domain is empty and there is no
    /// thread to extract anything from.
    #[must_use]
    pub const fn is_known(record: u64) -> bool {
        matches!(record, STATE | PLACEMENT | CPU_TIME)
    }
}

#[cfg(test)]
mod tests;
