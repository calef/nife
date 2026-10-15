---
status: PROPOSED
raised: 2026-10-15
---

# 300. `Tcb::SUSPEND` and `Tcb::RESUME`: the paused state, designed once with the fault endpoint

Raised 2026-10-15 by milestone 652's lane (twelve `DECISION` gates cite only ruled sections), which
wrote the open fork into the place an open decision lives. The fork is
[§24 (interrupting the foreground)](0024-interrupting-the-foreground.md)'s own deferral, older
than the block that waits on it: milestone 48 (job control: `jobs`, `wait`, `kill`, `fg`, `bg`, and
a stopped state) has gated on it since it was recorded, and §24's trigger list in
`design/open-design-ideas.md` names real job control as trigger 2, which the shell's job-control
work has fired. *(Section number provisional until the merge queue lands it.)*

## What is being decided

Whether the kernel gains a way to stop a thread and let it run again, and what the surface looks
like if it does. Three sub-questions, in the order they decide each other:

1. What may suspend a thread: the `ThreadControlBlock` capability, the supervision endpoint, or
   something narrower.
2. What a suspend is: a silent park, or an event the supervisor receives, on the fault endpoint's
   shape.
3. What resume preserves: the registers, so the thread continues, or nothing, which would make
   suspend a spelled-out kill.

## What this tree already does in the analogous case

§24 built the interrupt design this extends, and it is two tiers: the cooperative tier is a message
on an endpoint the child was spawned holding, and the forcible tier is
[§16 (object revocation)](0016-object-revocation.md). Pause is
deliberately absent, deferred "for a consumer that does not exist yet". One now exists: the shell
wants `Ctrl-Z`, `bg` and a stopped state, and a stopped job that reads the foreground terminal has
to stop rather than be killed or warned.

[§26 (the fault endpoint)](0026-fault-endpoint.md) is the second half of the analogy, and the block's own words are the design request: the fault
endpoint is "the kernel turns a thread's state into a message a supervisor holds". Death already
delivers `(event code, tid, fault pc, fault address, reserved)` to the supervision endpoint. A
paused state is one more thing only the kernel witnesses.

The kernel's TCB surface today is `CONFIGURE`, `CAP_INSERT`, `START` and `SET_THREAD_POINTER`, all
refusing anything but an embryo. Nothing can park a started thread and let it continue, and §26's
message cannot express "stopped" either: it has an event code for fault and exit and no paused
value.

Prior art, read rather than recalled: `notes/blocked-thread-teardown/prior-art.md` records seL4's
answer. `seL4_TCB_Suspend` puts the thread in `ThreadState_Inactive`, the authority is the TCB
capability and nothing narrower, and a later `seL4_TCB_Resume` restarts it. Unix's `SIGSTOP` is the
other teacher, and §24 already refused its lesson: signals are ambient authority delivered by pid,
which is the model this kernel does not have. A stopped state here has to be a capability question.

## The options

| | shape | what it costs |
|---|---|---|
| **A** | `Tcb::SUSPEND`/`Tcb::RESUME`, authority the TCB capability, no message. State is observable only by whoever holds the TCB. | The smallest surface, and seL4's exact shape. A supervisor holding only the supervision endpoint (the §26 shape, and §32's reaper) cannot see or cause the pause. |
| **B** | The same verbs, and suspend delivers a stopped event on the supervision endpoint, the fault endpoint's own channel, so one vocabulary covers "the kernel turned a thread's state into a message". | Two consumers for one endpoint, and a rule for what a supervisor may do about a stopped event. The message is cheap; the policy is the cost. |
| **C** | Endpoint-only: suspend as a message on the supervision endpoint that the kernel acts on, no TCB method. | New endpoint semantics on the wire two programs already agree on, and it inverts §26's direction, where the kernel is the sender. A supervisor that holds no endpoint, as `build_child`'s callers sometimes do not, cannot pause at all. |
| **D** | Refuse until a debugger exists; build job control's phase one, which needs no kernel surface. | Leaves trigger 2 fired and unanswered, and the shell simulates `Ctrl-Z` by killing, which is the old behavior wearing a new key. |

Recommendation: B, with A's authority answer. Suspend and resume ride the TCB capability, because
every spawner in the tree holds the TCB it built through `CAP_INSERT` and START, and the retention
field of [§142 (what a spawner retains over a child)](0142-spawn-retention.md) exists exactly to
say when a spawner keeps it past START. The stopped event rides
the supervision endpoint, because "the kernel is the only witness, so it passes the news along" is
§26's sentence and this is one more kind of news. Resume preserves the registers, or the verb pair
is a kill with extra steps; seL4's `Resume` restarts, and this proposal asks for the weaker,
honest thing: continue where it stopped.

This is a syscall-surface fork, so the recommendation is a recommendation, not a decision. The
effort test: B costs more than A (one message at suspend, one more event code), and would still be
the choice, because the supervisor that most wants to know a job stopped is the one holding the
endpoint, not the TCB.

## Whether the premise is true

Checked 2026-10-15. `kernel/src/sched` has no paused or suspended thread state; the states are the
runnable and blocked set, `Finished` and `Dead`. The SUSPEND tracker in
`design/open-design-ideas.md` still carries the deferral with its three triggers, and job control
is the fired one. Nothing in the tree has grown a way to stop a thread resumably since §24
deferred it.

## How reversible it is

The verbs are cheap to add and cheap to rename before a consumer ships. The expensive half is the
same one §160 (what a subshell copies) names: what the state means to programs that observe it. A
stopped event on the supervision endpoint is wire, and wire is the category to decide once. That is
why the question is here rather than in a lane.

## What is blocked until this is answered

Milestone 48's phase two: the stopped state, `Ctrl-Z`, `bg` on a stopped job, and zsh's `suspend`.
Phase one (`jobs`, `wait`, `kill`, `&`, `fg` on a running job) needs no kernel surface and stays
startable. The other two triggers in §24's list, the userspace pager and a debugger, inherit the
answer without waiting for it: the block asks that the pair be designed for all three rather than
for job control alone.
