# The hung component: what counts as hung, argued

An appendix to [`notes/hung-component.md`](../hung-component.md), whose question 2 states the
answer: not elapsed time but unserved work, with the bound held by the supervisor. This is the
argument for it, owner by owner, and what is missing to build it.

Take the three candidate owners in turn, because the manifest answer is genuinely attractive and it
is wrong.

The manifest's, and no. `component_plan::Requirements` is where a deadline looks like it belongs:
one declaration, shipped with the contract, read by every supervisor. It fails on the tree's own
evidence. A contract cannot know the machine. The same `OPERATION_PUT` under QEMU TCG, under HVF,
and on a VisionFive 2 differ by orders of magnitude (notes/cpu-models.md, the matrix of milestone 59
(the CPU-model matrix)). A wall-clock number compiled into a `*_proto` crate would be a shipped
version of exactly the load-sensitive assertion milestones 62 and 78 exist to remove. It also fails
the manifest's own test: notes/component-manifest.md's line is that a manifest declares authority,
and a deadline is not authority, it is a service-level claim about a machine the declaration has
never met. Fuchsia does not put timeouts in a `.cm` either.

The client's, and not alone. The client is the party harmed, which is a real argument for its
deadline being the one that matters. But a client cannot act on it, so a client deadline is only
ever a report. Worse, a client on the default rung of the latency ladder is *blocked* by the time it
would like to give up: there is no timed `CALL`, so it has no moment at which to notice.

The supervisor's, denominated in progress. This is milestone 62 (tests)'s argument, one level out,
and the fact that this tree has already made it about its own test harness is why it is a
recommendation rather than a preference. 62's block:

> The heartbeat is bumped once per test, at the test's start, and never while a test runs. So "no
> progress for 60 s" cannot distinguish a genuine deadlock from a test that is simply *slower* than
> 60 s. [...] The fix: a per-test *progress* heartbeat [...] so a slow test keeps the watchdog fed
> and a wedged one does not.

And its closing line, which is the general form: *"a bound expressed in something other than the
property under test."* A duration is that. A monotone count of work completed is not.

Concretely, and it needs nothing that does not exist: a component publishes a monotone progress
counter in a page its supervisor owns. §41 (the endpoint is the broker, and a device is revoked by taking it back (milestone 23 (a capability-routed component OS with live replacement))) already has that page. `hot_swap_contract::LOG_VA` is a frame
the operator retyped from its own budget and mapped read/write into every instance, and every
instance stamps it as it serves. Reading it costs the watcher zero syscalls and the component one
store. A verdict is then: *work is owed (a request went in and no counter moved) and the counter has
not moved across k observations.* No clock is consulted, and the number k is the supervisor's, set
by somebody who knows the machine it is running on.

Two honest costs, and the first is the one to design against:

- The counter's granularity is the false-positive rate. A component that bumps once per request
  looks hung throughout a single legitimately long request. One that bumps inside its work loop does
  not. This is 62's own finding and it does not go away by moving up a level; it becomes a property of
  the contract, which is at least a property somebody writes down.
- Separating k observations still needs the watcher to wait, and there is no timed wait. See the
  next paragraph, because this is the fork.

What is genuinely missing, precisely. Measuring a duration needs nothing: `user_mode_runtime::now()`
is a plain register read (`CNTVCT_EL0` / `rdtime`), ambient by design, so any process can time
anything. Waiting on one is what does not exist. There is no timed wait anywhere in the kernel; the
syscall surface is `EXIT`, `YIELD`, `INVOKE`, `CAP_DELETE`. So a watchdog today must yield-spin
between observations, which burns a core and makes its own timing load-sensitive, exactly as
`net_stack`'s retransmit window does (`wait_for_nic`, and milestone 106 (wait) records the price).
That is milestone 106's fork, and 106 says in its own words not to settle it by accident. This lane
did not.

A citation to fix while passing through, because it is the exact failure `script/decisions
--check` cannot catch: milestone 106's block attributes the fork's three candidate shapes to
"DECISIONS §51 (sink)" twice, and §51 is *the sink protocol*. The three shapes are in milestone 51
(wall)'s roadmap block (`design/roadmap/0051-wall-clock-time.md`, its rejected-alternatives list).
§N and milestone N are colliding schemes, the gate proves only that a cited §N resolves to *some*
section, and a well-formed wrong citation is invisible to it. Corrected in 106's block by this lane.
