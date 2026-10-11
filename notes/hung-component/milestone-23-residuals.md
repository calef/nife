# The hung component: what it left of milestone 23

An appendix to [`notes/hung-component.md`](../hung-component.md). It records what the hung-component
lane found about the other two residuals of milestone 23 (a capability-routed component OS with live
replacement), which it did not take.

## What is left of milestone 23

One residual was left at the time the note was written. The lane changed the shape of neither of the
two that remained, but it did narrow what one of them costs. A later lane built the other
(notes/dependency-orchestration.md, 2026-08-23). The state-handoff paragraph below is kept as the
note originally wrote it, and the orchestration paragraph adds what that later lane found.

State handoff is unchanged and is still the crux. It is a serialise-old / absorb-new protocol over a
supervisor-brokered channel, which is a wire format and so calef's. What the lane adds is a reason
it is *harder* than the block says. A hung component cannot be asked to serialize anything. So a
state handoff protocol that only works when the outgoing instance cooperates recovers a planned swap
and not a failure, which is the case it is most wanted for. Erlang/OTP's `code_change` has the same
shape and the same hole.

Dependency-aware orchestration is now built (`component_plan::depends_on` and `dependents`,
notes/dependency-orchestration.md), and the finding the note predicted held exactly. The quiescence
protocol orchestration needs is telling a dependent to degrade before its dependency is swapped.
That is precisely the step a hang makes unavailable. So a dependency-aware supervisor still needs a
non-cooperative fallback for every edge in the graph, not just for the component it is replacing.

That fallback is not built, for the same reason a watchdog is not built. Both of its halves are
behind the two decisions the parent names: how a supervisor notices, and what it may do to a
component that never cooperates.
