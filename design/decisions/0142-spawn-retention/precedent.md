# §142 appendix: precedent in this tree, and who has already acted

An appendix to [§142 (what a spawner retains over a child after
`START`)](../0142-spawn-retention.md): what this tree's recorded decisions and milestones already
say about a handle on a child, and how reversible each answer is. Moved out of that page on
2026-10-12 (UTC) under [§212 (a prose budget)](../0212-a-prose-budget-for-every-document.md). The
main page is complete without it; this file exists to verify or challenge it.

## What this tree already does in the analogous case

Five records bear directly, and they do not all point the same way.

§26 (the fault endpoint) already decided the shape of the one handle that exists. Supervision is
granted at spawn only: the endowment goes in the reserved slot, `START` reads it, records
`Thread::fault_ep`, and clears the slot so the child cannot forge messages on its own death
channel. §26.2 is explicit that this is fixed for life, and that runtime reattach (it names
`Tcb::SET_FAULT_EP`) is deferred until something demands supervision handoff, "and it is a new
decision when it does". §26.5 makes the endpoint one per supervisor, shared by every child, with
the kernel stamping the identity per message.

§32 (a supervisor may collect a corpse without being able to build one) put a verb on that
endpoint and drew the line at the corpse. `REAP` is authorized by the kernel checking that the
named thread's `fault_ep` *is* the endpoint being invoked. Consequence 2 is the sentence this
decision has to be read against: "It authorizes collecting a corpse, not killing. The method refuses
a thread that is still alive."

§32.3 also answered the naming question once, narrowly, and said so. The tid is authorized
relative to the endpoint it arrived on, which is "the endpoint-only naming discipline applied
consistently: the name means something only to the holder of the capability it came through, and it
is not a global handle", with the standing instruction that a later operation needing to name a child
"is a fresh decision and should reach for the same shape first". This decision is that later
operation.

§40 (there is no reaper of last resort) makes ownership the answer instead of a handle. A child's
resources come from its supervisor's region, so destroying the region reclaims the child and
everything built from it, and there is no privileged collector. Its first recorded caveat is directly
about retention: "The chain holds only if children are built from the supervisor's own region," and
whether building a child from a delegated region is forbidden or merely discouraged "is not yet
decided, and it should be before someone writes a supervisor that does it."

§92 (a caretaker is supervised by the client it serves) is the adjacent lifetime decision, and it
chose supervision over membership. It also established a fact this decision needs: construction is
already splittable, because `build_child_space` returns a TCB and an address space and starting the
child is a separate step, so "build the client's space, build the caretaker beneath it, hand over the
endpoint, start the client" is expressible. Anything that has to be arranged before `START` can be.

Milestone 105 (the two forks milestone 22 named and left) has been holding half of this question
since 2026-08-04. Its fork two is `Tcb::NAME`, "a tid that becomes a handle", with three options
recorded and none chosen: a method, per-child fault endpoints (already refused by §26.5), or the
builder reporting the tid it created. Its framing is worth keeping: "Whether the kernel owes a
supervisor the ability to resolve the identity it already sends, or whether that is userspace
bookkeeping."

Milestone 126 (who else is running, and who is allowed to ask) set the looking-versus-acting line,
with `Rights::ENUMERATE` separating them. Any answer here has to say which side it falls on.

## How reversible this is, and who has already acted

By the *move fast on what can be undone* test, which is "who else has already acted on this":

- R0 and R4 are reversible. R4 is a field in one userspace crate. R0 is the status quo, and
  writing it down costs a paragraph that a later decision can supersede.
- **R2 and R3 are not.** Both change what an existing capability means or add a method number to
  `abi`, which is the tenet's "anything two programs agree on" category, and §139's standing note
  gives the worked example of a method number whose retirement was foreseeable.
- **R1 is the least reversible of all, and not because of code.** It would overturn §32's stated line
  and milestone 126's, and a decision that overturns two recorded decisions is read by everyone who
  meets either of them afterwards.

Who has acted already: 25 call sites, in the R0 direction, without anyone deciding. That is the
honest status of the question, and it is the argument for answering it now rather than after
milestone 133 has answered it sideways.
