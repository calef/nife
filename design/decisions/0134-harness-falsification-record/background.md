# §134 appendix: why now, the two failures, and the prior art

Evidence for [§134 (a harness carries a machine-replayable falsification
record)](../0134-harness-falsification-record.md): the question that forced it, the two failures it
separates, and the field it borrows from. Moved out of that page on 2026-10-12 (UTC) under [§212 (a
prose budget)](../0212-a-prose-budget-for-every-document.md). The main page is complete without it.

## Why now

`notes/verification.md` already states the rule, and states it well:

> Falsify a property before believing it. Break the code the harness guards and confirm the harness
> fails. Every milestone 35 property was falsified this way, and one falsification corrected a claim in
> the code (the load-bearing guard was not the one the comment pointed at). A harness that cannot be
> made to fail is not evidence.
>
> -- notes/verification.md

It is rung four, honored by whoever remembers, and milestone 191 measured what that produces:
145 harnesses, 23 `kani::cover!` sites across 4 of 24 harness crates, and no harness anywhere
recording what was done to falsify it. The reverse pass found the predictable result:
`capability::subset_is_reflexive` proves `a & !a == 0`, a tautology no plausible implementation error
breaks, and twelve of the 26 `paging` harnesses restate six properties once per ISA.

The forcing question is calef's, and it is about a claim rather than about code: if the project is
going to say proven, the evidence that the proofs are load-bearing cannot itself be a written
claim that they are load-bearing. That is one level of indirection away from evidence, and it is the
same shape as the nine misrecorded roadmap statuses and the fabricated block quote that survived
twelve days of gates.

## Two failures, and they are not the same problem

Conflating them produces a convention that half works.

Vacuity is mechanical. Assumptions contradict, the input set is empty, and Kani reports
`SUCCESSFUL`. `kani::cover!` is the check that catches it, which `notes/verification.md` already
says.

**Tautology is not.** A property true of every plausible implementation passes every cover check
ever written. Only falsification catches it: if nobody can describe a change that turns the harness
red, the harness is decorative.

## Prior art, and it is a field rather than an idea

Read rather than recalled, because this tree carried a fabricated block quote for twelve days.

The model-checking literature has done this since roughly 2004, under two names that belong
together. Vacuity detection and coverage metrics are the standard sanity checks, and the
survey work makes the unifying observation explicit: both work by *repeating the verification on a
mutated input*, with vacuity mutating the specification and coverage mutating the system.

What this section proposes has a name there: mutation coverage for model checking, where an
element of the system is considered covered by the specification if changing that element falsifies
the specification. That is "break the code, confirm the harness fails", stated formally, with two
decades behind it.

§39's protected class applies: a term a reader already knows from outside costs a newcomer
nothing, so this convention should be spelled in the field's vocabulary rather than in ours.

There is also a cheaper family, Inductive Validity Cores, which compute the minimal set of model
elements a proof actually needed and so give coverage without re-running anything. *(Since checked: Kani exposes none, as `script/falsifications` records.)*

Sources: [Coverage Metrics for Formal
Verification](https://link.springer.com/chapter/10.1007/978-3-540-39724-3_11), [Sanity Checks in
Formal Verification](https://www.cs.huji.ac.il/~ornak/publications/concur06b.pdf), [Mutation Coverage
Estimation for Model Checking](https://link.springer.com/chapter/10.1007/978-3-540-30476-0_29).
