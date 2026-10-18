---
status: DECIDED
raised: 2026-10-18
decided: 2026-10-18
ratified_by: calef
---

# 309. A supervised job's `ThreadControlBlock` goes to the shell that asked for it

*Number provisional; the integrator mints it at merge.*

Raised by milestone 48 (job control)'s phase two, on nifeos/nife#2332. calef ruled option A on
2026-10-18 (UTC), and the same day ruled the two clauses below that A left open: no wiring bit
asks for the capability, and it travels without `GRANT`.

## The decision

When the progenitor builds a supervised job, it hands the job's `ThreadControlBlock` capability
to the shell that requested the job. The progenitor keeps no copy.

- [§142 (what a spawner retains over a child after `START`)](0142-spawn-retention.md)'s
  `Retention` field gains a third value, `Retention::Requester { reason }`. Its name is
  provisional. Under this value `start_child` neither deletes the TCB nor keeps it. It returns the
  TCB to the caller, which sends it on.
- On the wire, the capability is the one message that follows `SPAWN_OK` on the result rendezvous,
  and only for an interruptible request. No wiring bit asks for it, because a supervised request
  is the ask. The shell must receive it before it does anything else. Until it does, the
  progenitor is blocked in that `SEND`.
- The capability carries the rights `SUSPEND` and `RESUME` need and not `GRANT`, so the shell can
  pause its job and cannot pass that power on.
- Every other spawn path keeps what it declares today. An ordinary job at the prompt is built in
  the progenitor's pool with `Retention::Nothing`, and the shell holds nothing else for it.

## Why the shell, and not the progenitor

The shell already holds the region a supervised job was built from, and `DESTROY` on that region
ends the job. On a started thread a TCB admits only `SUSPEND` and `RESUME`: `CONFIGURE`,
`CAP_INSERT`, `START` and `SET_THREAD_POINTER` refuse anything but an embryo. So the shell gains
pause and nothing more. That is milestone 48's own premise, that a job is what the shell holds
capabilities for, made true for the one kind of job where the rest was already true.

The refused option, B, kept the TCB in the progenitor and added suspend and resume requests to
the spawn channel. That would put a second party between the shell and its own job, add a request
vocabulary to a wire two programs agree on, and leave the progenitor holding authority over threads
it built for someone else. At equal effort A still wins, and the efforts are about equal.

## What it leaves alone

- [§300 (the paused state)](0300-tcb-suspend-resume-the-paused-state.md)'s verbs are unchanged. The
  holder of the TCB is still the one who may pause the thread; this section decides who that holder
  is for a supervised job.
- §142's rule still holds. Retention is declared at the call site, and a spawn path cannot forget
  to decide. This adds one more answer the field may give.
- A stale capability is harmless. Reclaiming a region makes every capability delegated out of it
  stale (`notes/object-revocation.md`), so after `kill -9` or a reaped finish the shell's TCB slot
  names nothing.
