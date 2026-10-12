# §101 appendix: why not the alternatives

The alternatives [§101 (notification objects: async multiplexing without
wait-any)](../0101-notification-objects.md) weighed. Moved out of that page on 2026-10-12 (UTC)
under [§212 (a prose budget)](../0212-a-prose-budget-for-every-document.md). The main page is
complete without this one, which exists to verify or challenge it. No ruled word changed.

### Wait-any on endpoints (the port-set model)

A `RECV` that waits on a *set* of endpoints. Mach's port sets, and the form notes/compositor.md
considered.

Rejected for the reason seL4 rejected it: the set is a new container object with its own lifetime
and revocation, and every endpoint in the set gains a second wait queue (the set's). The
notification object is simpler: one queue, one word, no container, and the binding mechanism gives
the multiplexing without the set's complexity.

### Non-blocking RECV (the poll model)

A `RECV` that returns immediately if no message is pending. seL4 has this (`seL4_NBRecv`,
`seL4_Poll`), and it is part of this design (`POLL` on a notification). But non-blocking RECV alone
does not solve the multiplexing problem - it turns "block" into "spin," and a spin loop burns a hart
(milestone 106's current answer, which the notes call "the honest interim a riscv-SMP lost wakeup
forced"). The notification object is what makes non-blocking RECV useful: poll the endpoint, poll
the notification, and if nothing is ready, `WAIT` on the notification (which blocks until something
is).

### Pull-based source (the single-endpoint model)

Restructure the sink contract so a stage `CALL`s for input and `SEND`s output on one endpoint, and
the shell's loop is one `RECV` that either replies with bytes or writes bytes out. notes/pipes.md
records this as a design fork and calef's call.

Rejected for the reason the notes give: it destroys the property that the read end and write end are
separate capabilities, so the program on the right of a `|` could write back up its own input. That
property is called "load-bearing" and the notification object preserves it: read and write stay
separate endpoints, and the multiplexing happens at the wait level, not the protocol level.

### Buffering stage

Measured and rejected (notes/pipes.md, 2026-08-03; full text [at
`256815e56`](https://github.com/nifeos/nife/blob/256815e5609cfb961ccb6526f8a0d3316e589402/design/decisions/0101-notification-objects.md#buffering-stage)).
A buffer doubles the per-message cost and buys decoupling, not bandwidth, and the notification
object solves the deadlock it was proposed for.
