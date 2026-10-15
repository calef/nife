---
status: PROPOSED
raised: 2026-10-15
---

# 301. How a spawned child's exit status and output reach the program that spawned it

Raised 2026-10-15 by milestone 652's lane (twelve `DECISION` gates cite only ruled sections). The
fork belongs to milestone 172 (a capability-native subprocess primitive), whose gate has named it
since the block was written: [§219 (how the shell names an installed program to the
spawner)](0219-naming-an-installed-program-to-the-spawner.md) decided how a caller *starts* a child,
[§264 (fork is declined, and spawn is supported)](0264-fork-is-declined-and-spawn-is-supported.md)
declined `fork` and
made spawn the supported path, and what neither settles is the other half of `cargo`'s loop, "run
this, block until it is done, give me its output". §264 already says the supported shape is "name a
program, say what it is handed, get a handle to wait on"; the handle to wait on is the part that
does not exist. *(Section number provisional until the merge queue lands it.)*

## What is being decided

Three things, in the order a caller meets them:

1. How a caller blocks until a specific child is done, given that
   [§26 (the fault endpoint)](0026-fault-endpoint.md) delivers death as a message
   on a supervision endpoint the spawner chose.
2. Where the exit status comes from, given that §26's message carries
   `(event code, tid, fault pc, fault address, reserved)` and no status, because no program can
   state one today.
3. Whether the wait is one child's, as `cargo` wants, or a subscription, as the long-lived service
   supervisor §26 was built for wants.

## What this tree already does in the analogous case

`crates/supervision_protocol`'s `build_child` constructs the child and `start_child` runs it; the
crate holds the report and construction protocol words, and its own doc points the supervisor who
wants a corpse at [§32 (a supervisor may collect a corpse)](0032-reap-without-build.md)'s `REAP`.
Death arrives as §26's five words on the fault endpoint,
distinguished as fault or exit by the event code. Nothing in the kernel or the crates carries an
exit status: the event code says *that* a thread exited, never *how*.

The composition the block names is real. [§101 (notification objects)](0101-notification-objects.md) support `POLL`, a non-blocking
wait, and can be bound to a TCB so a signal ends a blocked receive;
[§12 (call/reply IPC)](0012-call-reply-ipc.md)'s `CALL` mints a one-shot
reply capability; §26's death message is never lost. So "park until this child dies" is reachable
today by receiving on the child's supervision endpoint. What that receive returns is the gap: a tid
and a fault site, where `cargo` wants an exit code.

Prior art, recalled and marked as such: Fuchsia reaches the same place without `fork` by making
process creation return a process handle and delivering the return code through that handle's
signal set; Unix's `waitpid` is the same idea with a pid as the handle. The tree cites Fuchsia's
`fdio` shape approvingly in [§10 (process model)](0010-capability-microkernel.md)'s own text, which
is the precedent this proposal works inside.

## The options

| | shape | what it costs |
|---|---|---|
| **A** | Composition, no kernel change. The child's runtime writes one status word to a frame the parent owns before exiting; the parent receives §26's death message, reads the word, then reaps. The convention lives in a crate, so the two sides cannot drift (rule 7). | The status is a convention, so a child that does not cooperate reports nothing. In this tree every child runs our runtime or our C library, both of which can cooperate, but the guarantee is by coverage, not by construction. |
| **B** | `Tcb::WAIT`, a blocking method on the TCB, returning an exit status the kernel records at exit. | The kernel has to take the status in, which is new syscall surface, and §26's message stays the supervisor's channel while the status travels on another one: two ways to learn about one death. |
| **C** | A job object: spawn returns one, `Job::WAIT` collects status and the granted output ends, `Job::DESTROY` reaps. | The most machinery: a new object kind, retype and slot costs, and a second lifetime beside the region and the endpoint, exactly the bookkeeping [§142 (what a spawner retains over a child)](0142-spawn-retention.md)'s decision just finished paying down for the TCB. |

Recommendation: A. The test from the decisions skill: would we still choose it if B and C cost the
same? Yes, because A keeps one death channel (§26's) and makes the status what it honestly is, a
fact the child states, rather than a second fact the kernel overhears. B's kernel-recorded status
is worth revisiting only for a child that cannot cooperate, and no such child exists in this tree:
foreign binaries come in through the C library, which is ours to change.

The crate is the deliverable milestone 172 describes: spawn through `build_child`, endow one
status frame, receive the death, return the code and the output the grants routed. If building it
meets a wall the options above do not name, that wall is the fork coming back, with numbers.

## Whether the premise is true

Checked 2026-10-15. `crates/supervision_protocol` has no wait-and-collect shape, and its death
protocol words carry no status. `kernel/src/sched` records no exit code anywhere: the §26 message
is the whole record. The block's own sizing note stands: the composition and a narrow method may
turn out to be the same mechanism used differently, which is exactly what building A will show.

## How reversible it is

A is a userspace crate and a convention in one struct, replaceable until a second consumer relies
on it. B and C are surface, which is the category this tree decides once and keeps. So the order
of trying is fixed by reversibility alone: A first, and only a measured failure of A spends B or
C's deliberation.

## What is blocked until this is answered

Milestone 172 itself, and behind it 173 (`cargo` and `rustc` self-hosting), whose build model needs
this loop per compilation unit. The `posix_spawn` shim §264 promises foreign programs waits on the
same answer, since its `spawn` has to return something a caller can wait on.
