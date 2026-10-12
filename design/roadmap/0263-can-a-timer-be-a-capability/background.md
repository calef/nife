# Milestone 263's background: why a spike, and the decision it sits under

Moved whole on 2026-10-12 (UTC) from
[`design/roadmap/0263-can-a-timer-be-a-capability.md`](../0263-can-a-timer-be-a-capability.md) under
§212 (a prose budget). The page holds the answer, the corrected tables and the prices; this holds why
the question was priced as a spike, and the decision it sits under. One long sentence was split in
moving.

## Why a spike, and the precedent is milestone 106's own

Milestone 106 was priced this way, and it worked: the pricing lane built a prototype (a
`deadline` word on `Thread`, a cached earliest, an expiry walk in `on_tick`, a blocked-thread
census), measured it, and threw it away on purpose, because *"shipping it would have settled the
fork by accident."* Those numbers retired the cost objection that had stood since 2026-08-04.

This is not proposal-shaped procrastination, which AGENTS.md warns is an available way to avoid
deciding. The direction is decided. What is unpriced is a prerequisite underneath it, and nobody can
currently say what it costs, which is the condition that earns a lane.

## The decision this sits under

calef, 2026-09-05, on milestone 106's fork: serve the timed wait from a userspace timer service
signaling a notification, rather than from a new kernel blocking primitive. The reasoning is in
[§101 (notification objects)](../../decisions/0101-notification-objects.md), which anticipated it:

> A notification object lets a *userspace timer process* wake a thread at a deadline, which is how
> seL4 does it, but it requires that timer process to exist and to hold a clock capability.

All four of milestone 106's named consumers are userspace (`thread::sleep`, `Endpoint::RECV`'s
callers, the shell's `^C` poll, and `net_stack`). §101's carve-out for a kernel timed wait names
kernel needs (a watchdog, a scheduling deadline, an in-kernel retransmit) of which the tree has no
instance. Both of `sched.rs`'s no-timeout complaints are about userspace callers being hung.
