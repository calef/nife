---
status: PROPOSED
raised: 2026-10-19
milestone_dependencies: none
decision_dependencies: 300
machine_requirements: none
specific_machine: none
needs_person: no
---
# Every supervisor dispatches on the event codes it knows

Raised by the 2026-10-19 documentation sweep
([decisions read forward](../../audit-reports/2026-10-19-decisions-read-forward.md), finding 12).

[§300 (the paused state, a TCB verb, silent)](../../decisions/0300-tcb-suspend-resume-the-paused-state.md)
keeps a stopped event out of §26's message and rests that on a rule: unknown event codes are never
acted on, so a later `EVENT_STOPPED` is additive rather than a migration across every supervisor.
The section says every consumer already dispatches on `EVENT_FAULT` and `EVENT_EXIT`. Two do not.
`components/src/job_undertaker.rs` collects the tid on any event it receives, and
`components/src/sub_server_supervisor.rs` reaps on any event and reads every code that is not
`EVENT_FAULT` as a clean exit. Either would reap a paused child as dead the day a third code exists.

Nothing is wrong today, because the kernel sends only the two codes. The work: enumerate every
receiver of the supervision endpoint (`receive_fault` and its peers), decide per consumer what an
unknown code does (ignore it and keep receiving is the reading §300 implies; a consumer that cannot
ignore one should say why), and add the check that keeps it true, ideally one shared dispatch in
`supervision_protocol` that a consumer cannot bypass, so the rule is a type rather than a review.
The correction to §300's sentence about the tree is the integrator's.

Reuse: none exists; the dispatch is on this tree's own §26 wire, and `supervision_protocol` is where the shared shape would live.
