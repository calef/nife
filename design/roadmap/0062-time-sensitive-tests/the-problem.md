# The problem as raised: an appendix to milestone 62

An appendix to [milestone 62 (tests that assert on time)](../0062-time-sensitive-tests.md), kept
apart under §212 (a prose budget). It is the block's framing as raised on 2026-08-01, with the
watchdog section added on 2026-08-04, and the later correction to its count of spins.

## A correction to the count below

The count in this block is stale, and left below as written rather than edited into the prose,
because the argument it supports is history: `sched.rs` holds 6 of these spins now, not nineteen.
Tree-wide the shape matches 19 sites, but 9 are not test code at all, and none of the 10 in tests has
the flake-prone shape. They are all "let it settle, then prove nothing more happened", which is a
negative assertion a loaded host cannot fail in the failing direction.

## The problem

A population of tests assert on elapsed time or on a fixed number of yields. `sched.rs` alone
holds about nineteen `for _ in 0..N { yield_now() }` spins, and the shape is always the same: give
the scheduler N chances, then assert something happened. `threads_round_robin` gives twenty yields
and asserts every thread ran at least once. `ticks_arrive_at_the_configured_rate` and the riscv
timer-drift assertion compare guest ticks against elapsed counter time.

None of them is wrong about what it wants to prove. All of them fail when the host is busy, because
a yield is not a guarantee and the guest's clock is the host's clock.

## Why this is worth a milestone rather than tolerance

It makes a real regression invisible. On 2026-08-01 it cost the integrator three separate
diagnosis cycles, and two of those ended in the wrong conclusion before being re-run. The credentials
lane hit three flakes, the xattr lane two, the CPU-matrix lane two, and the integrator hit three more
in different tests each time. A suite that fails for reasons unrelated to the change trains everyone
to re-run rather than to read, which is the exact habit that lets a genuine failure through.

Milestone 59 multiplies it fivefold. The CPU matrix runs the same suite five times, so every
timing test now has five chances per run to be unlucky, on a shared CI runner nobody controls.

And the honest diagnostic we rely on is expensive. The current rule is "a green run under load is
conclusive, a red one is not, so re-run quiet." That works, and it costs a full suite run every time,
and it depends on a human remembering to apply it.

## What the fix probably is, per class

- The bounded spins are the easy majority. Waiting for a condition with a deadline is a different
  thing from taking N turns: the test wants "eventually", not "within twenty yields". An
  event-driven wait, or a bound expressed in guest ticks rather than host-scheduler turns, makes
  them insensitive to what else the machine is doing.
- The genuinely temporal tests cannot be made deterministic and should not pretend to be.
  `ticks_arrive_at_the_configured_rate` is *about* the clock. These want an explicit, stated
  tolerance and a recorded retry budget, so a flake is a documented cost rather than a surprise.
- A third class may want to move off the emulator entirely. Scheduling policy is pure logic and
  some of it could be host-tested against a simulated clock, which is where this project already puts
  logic it wants to check in milliseconds.

## The watchdog cannot tell "stuck" from "slow", and it is the same defect one level up

Added 2026-08-04 from `notes/semihosting.md#the-hang-watchdog-and-what-it-cannot-tell-you`,
which records the limitation and names the fix
nobody owns. The suite's own deadlock detector has exactly the problem this milestone is about.

The heartbeat is bumped once per test, at the test's start, in `Testable::run`, and never while
a test runs. So "no progress for 60 s" cannot distinguish a genuine deadlock from a test that is
simply *slower* than 60 s. Both look identical from outside: the heartbeat stops advancing because
no *new* test started.

This has already cost a diagnosis. Milestone 32's FS-server test tripped the watchdog as a false
deadlock. It was not stuck, it was starved: leaked spinning driver threads crammed onto core 0 slowed
the RedoxFS mount past 60 s. Raising the limit made it pass, which is exactly what a deadlock can
never do, and that is the tell the current instrument cannot produce on its own. The thread dump
reinforced the wrong read, because a starved thread and a deadlocked one both sit `Blocked`/`Ready`
with nothing obviously moving.

The fix, per the note. A per-test *progress* heartbeat: the test, or the IPC and scheduler paths
under it, bumps a counter as work happens, so a slow test keeps the watchdog fed and a wedged one
does not. The other half already exists and was built while chasing that false deadlock. The
enriched `sched::dump_threads` reports each thread's EL0 PC and the per-endpoint sender, receiver
and pending counts, so two dumps a few seconds apart show whether the pipeline is changing state
(starved but progressing) or frozen.

Until the heartbeat is per-progress, the operating rule is what the note says to do by hand: read a
watchdog trip as "stuck or slow", and confirm which by raising the limit before assuming a lost
wakeup. That is a human step in the loop for exactly the reason the rest of this milestone gives, a
red run whose meaning has to be argued about trains everyone to re-run rather than to read.

It belongs here rather than in a harness milestone because it is the same failure the bounded
spins have: a bound expressed in something other than the property under test. Nineteen tests take
N turns and hope; the watchdog takes 60 seconds and hopes. Whatever answers the first should answer
the second.
