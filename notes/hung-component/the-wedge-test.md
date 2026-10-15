# The hung component: what the test shows, and why the wedge is a CALL

An appendix to [`notes/hung-component.md`](../hung-component.md). It records the four results of
`swapper`'s `ROLE_HUNG` case and why its wedge takes the shape it does.

## The four results

The case is demonstrated on both ISAs. `swapper`'s `ROLE_HUNG` runs the direct channel's system
against an incumbent that swallows one request and stops answering. Four results, every one an
assertion about program order and not one of them about elapsed time:

1. The domain does not report a hang. `SURVEY` reports every member `BLOCKED` and none `DEAD`,
   which is byte for byte what a healthy idle system reads as: `abi::survey::BLOCKED` is the state of
   a server parked in `RECEIVE_CAP`, and that is every healthy server between requests. The widest view
   a supervisor has cannot tell the difference, and no death message has arrived by then either.
2. The supervisor's whole vocabulary is refused. `Rendezvous::REAP` is asked about *every* member of
   the domain and answers `StillAlive` every time. It is side-effect free by construction:
   `reap_supervised` decides `StillAlive` before it looks up a region.
3. The service is restored with no new authority, per the four-step table in the parent, and the drain step is
   asserted absent so a run that quietly quiesced cannot pass.
4. The stranded caller is not restored by that, and is freed only by the component that stranded
   it, after the service is already back. The two recoveries are separable and the report ordering
   proves it.

## Why the wedge is a `CALL`

Worth its own heading because a reader will otherwise reach for the simpler shape and reintroduce a
race the test was written to avoid.

The wedged instance announces itself with `call(NOTE, NOTE_WEDGED, served)` rather than `send`, and
the operator serves that one message with `RECEIVE_CAP` and keeps the reply capability, never using
it. That is not a message-passing style choice; it is the hang. Three properties fall out:

- The blocked state is provable rather than raced. `sched::ipc_call` marks the caller `Blocked`
  inside the same critical section that wakes the receiver, so at the instant the operator's
  `RECEIVE_CAP` returns, the instance is already parked. A `send` followed by a `receive` would leave a
  window in which the instance was still `Ready`, and a survey inside that window would read a state
  the assertion forbids. The test would then be measuring a scheduler race.
- It is the commonest real hang. Blocked awaiting a reply from a peer that will not answer is
  what a deadlock between two servers looks like from either side.
- It gives the operator the only handle anything has on a wedged process, which is what makes the
  cooperative release in result 4 expressible at all.

The wedge fires on the identity of one request (`WEDGE_SEQ`), not after a count of them. A count
would depend on how far the conversation had got when the operator was ready, which is a race; a
request identity happens in the same place on both architectures under any scheduling.
