# Milestone 118 appendix: the audit of 2026-08-18

An appendix to [milestone 118 (CLAUDE.md has a budget)](../0118-constitution-budget.md), moved out
of that block on 2026-10-12 (UTC) under [§212 (a prose
budget)](../../decisions/0212-a-prose-budget-for-every-document.md). The block is complete without
it; this is the audit's working, kept so a reader can check the block's summary against it.

## Which rules have a mechanism, and which are the budget

calef asked for the measurement this block had been describing: count the rules, count how many
have a mechanism behind them, and treat the rest as the budget. Done 2026-08-18, and it changes the
shape of the work rather than confirming it.

868 lines, 10,001 words, 60,570 bytes, 16 sections. 54 line-leading bolded claims, of which
roughly 33 are rules and the rest are argument, evidence or framing. Against that, `script/lint`
alone runs 32 named checks, and `roadmap`, `decisions`, `names`, `citations`, `audits`, `verify`
and the suite run more.

So the file is not mostly unmechanised prose. That is the finding, and it is not what "the file
has gotten huge" implies.

## Rules that already have a mechanism, where the prose could shrink

| the rule | what enforces it |
|---|---|
| names are calef's; crates and modules in scope | `script/names`, `--check` in `script/lint` |
| ~~`snake_case` for Rust things, hyphens for scripts~~ | wrong, corrected later: no such check exists |
| what two binaries share is a crate, not a `#[path]` module | lint counts `#[path]` consumers and fails at two |
| architecture code stays under `arch/` | lint's rule-1 check, but only partly: see the cut |
| delete a lane's branch at merge | `delete_branch_on_merge`, so the platform does it |
| every fence names its counterpart | lint |
| benchmarks are first-class; measure, do not argue | the icount tripwire, and `script/icount` since milestone 78 (the load-sensitive assertions) |
| `nifefs` caps archive names at 32 bytes | the format, enforced by the compiler |

Eight rules whose paragraphs can become a sentence and a pointer, because the mechanism is the
argument now.

Four of those eight did not survive verification. This table was assembled from `script/lint`'s
banner list, and a banner names a check's subject rather than its coverage. The second lane read
each gate's body and recorded the result in [the applicable cut](applicable-cut.md); two rows are
struck through or qualified above. Read that appendix, not this table, before deleting anything.

## Two rules whose prose has gone stale, which is the opposite failure

The citations paragraph is wrong. It says *"after any renumber, check citations by content, not by
running the gate"*, because `script/decisions --check` proves a `§N` resolves to *some* section and
never the right one. Milestone 97 (citations that name what they cite) built that gate.
`script/citations` opens by naming exactly that blind spot and closing it, and it caught a wrong
gloss on pull request #305 on 2026-08-18. So the constitution is instructing agents to do by hand
what a check now does, which is worse than a rule with no mechanism: it teaches distrust of a
working gate.

"Never squash-merge a branch" is prose against a permissive setting. `allow_squash_merge` was
`true` on this repository when this was written, so the platform was configured to allow the thing
the constitution forbids. (Since fixed: it is `false` as of #334, 2026-08-18, verified live by the
second lane.) Turning it off makes the rule unrepresentable, which is rung one for one API call. The
paragraph then becomes an explanation rather than a prohibition anybody can violate. The reason is
worth keeping either way. The lane of milestone 96 (one init) put the loader unification in its own
commit *ahead of* the migration, precisely so a boot failure could not be ambiguous between two
changes. A squash-merge destroys that.

## The budget: four rules that could have a mechanism and do not

These survive only because somebody remembers, and the first two both failed on 2026-08-18:

1. "The maintainer starts the two watchers at the beginning of every session." The drain died when
   the maintainer pruned the worktree it was running from. It then printed `No such file or
   directory` every 150 seconds for three hours while looking exactly like a quiet queue.
   `notes/merge-queue.md` already records that neither script reports its own death; this is that
   entry arriving.
2. "Prune a lane's worktree the moment its pull request merges." Nineteen worktrees and 44 GB had
   accumulated before anyone looked, on a machine that hit zero bytes free once already.
3. "Squash against the base commit you branched from, never `origin/main`." A recorded scar with
   no check: `origin/*` is not a fixed point in a worktree, and the failure stages other lanes'
   files as your own.
4. `needs-architect` enforcement. §88 (`needs-architect` as a required check) is `PROPOSED` and
   unbuilt, so the label is enforced by *one script choosing not to arm it*. Nothing stops a merge
   by another route, and nothing stops work merging if the label was never applied. On 2026-08-18
   a second session re-applied a label the maintainer had removed, which is the coordination half
   of the same gap.

Everything else unmechanised is judgment and should stay prose: the top-up rule, the handoff
rule, lane count against the collision surface, "correct yourself loudly", "push back when he is
wrong", "explain on request". A gate over any of those would be a gate about taste.

## The clause the tenet needs

calef, 2026-08-18: prefer gates over prose in this file. With one qualification the same day
earned: a gate is only better when it can be right about the tree, so record what it measured when
you built it. The branch-prefix check rejected legitimate work four times and was widened after
each one. `script/lint`'s own justification for a disabled check had rotted from "100% false
positives" to no longer true, and nobody noticed because a justification is not a measurement
anybody re-runs. A gate with its measurement beside it can be told stale from live; one without it
gets deleted by whoever it inconveniences.
