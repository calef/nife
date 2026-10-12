# What a tracker can and cannot carry: an appendix to milestone 533

[Milestone 533 (whether the roadmap should become issues, and what a citation means after the repository
split)](../0533-the-roadmap-after-the-split.md) keeps the question, the three layers, the options and
the recommendation. This appendix holds the proposal's checks of what an issue tracker can carry,
each answered by a command, and the things a tracker does better.

## The four questions from the brief, each answered by a command

Can a gate read it offline? No, and the cost is concrete. None of the four gates touches the
network today:

```
$ for f in script/lint script/names script/citations script/roadmap; do
>   grep -nE '^[^#]*\b(gh|curl|wget) ' "$f"; done
(no output)
```

`script/lint` mentions `gh` once, in a comment at line 1927. Moving any status a gate reads into a
tracker makes `script/lint` require a network and a token, on a laptop, in CI, and in every lane
worktree. That is a dependency in the sense of §46 (thin primitives or whole subsystems) and it is
in the gating path.

Does it version with the code? No, and this is the one that cannot be worked around. A record in
the tree is evaluated *at the revision you checked out*; a tracker reference is always evaluated at
present time. Demonstrated:

```
$ old=da2330141b76219ba91a9fcaac42719d689d9993   # 2026-08-01
$ git show $old:design/roadmap.md | grep '^### 39\.'
## Repository structure for a loosely-coupled OS, and the road to a distribution
$ git show $old:design/roadmap.md | grep -oE '^#{2,3} [0-9]+\.' | tail -1
## 57.
```

At that commit the tree knows what it meant by milestone 39 (repository structure for a
loosely-coupled OS) and knows that 443 does not exist. An
issue tracker cannot answer either question about that commit: `#39` resolves to whatever issue 39
is today, and nothing records what it was. A code comment citing `#39` is therefore a dangling
pointer into the future. That is the failure §76 (what catches a milestone status that is wrong in
both places) is a whole decision about, with the two records now unable to disagree because only one
of them has a past.

Does a citation survive a checkout of an old tag? Today, yes, because the target is in the same
checkout. There is exactly one tag in this repository (`decision-151-model-clarification`) and no
release tags, so this is currently theoretical; it stops being theoretical the moment `basalt`
releases anything.

What happens if the repository moves or the host changes? The tree has already run half this
experiment. Milestone 120 (the rename: the OS becomes `nife`) moved the repository from the `calef`
user to the `crickertech`
organization on 2026-08-15 and recorded what survived: *"ruleset, Actions and history survived; the
one casualty was the `TOOLCHAIN_BUMP_PAT` secret."* That was a GitHub *transfer*, which carries
issues. A split is not a transfer. It is `git filter-repo` or equivalent, which carries files
and history and carries no issues at all, into a repository whose issue numbering starts at 1. Any
citation that named an issue number would need rewriting, and §194 (sessions interleave rather than
serialize) records what rewriting citations
by number costs: *"a citation rewritten by number can be silently wrong and still pass every gate,
because the section it now names exists."*

## What a tracker is genuinely better at, and this is not a formality

- A stranger filing something without a clone. Principle 3 is about a newcomer succeeding
  without asking anyone; it has a filing side this tree has no answer for. A person who hits a bug
  in a released `basalt` image cannot open a pull request against a roadmap block.
- Cross-repository work items. After §151 there is no single tree that can hold "this needs a
  change in the kernel and in `basalt`". Today every work item is inside one tree by construction.
- Assignment and notification, which this project substitutes for with `helpers/merge-drain.sh`
  and a steward, both of which `notes/merge-queue.md` is honest do not report their own death.
- A backlog with an owner. `script/roadmap --proposed` shows 24 unnumbered proposals, all
  dated 2026-09-19. The pile is young, so there is no ageing evidence yet, and the block for
  milestone 247 (follow-on work named by a finished milestone goes nowhere) has its own
  reasoning says a due-date gate *"would be routed around by not writing proposals, which is
  worse."* A tracker is better at a backlog than a directory is. Whether it is better *enough* to
  pay for a second place where truth lives is the open part.
