# The ratchet: why, what it cannot catch, and what it costs

An appendix to [`notes/citations.md`](../citations.md), which states the conclusion.

## Why a ratchet rather than the sweep the main page refused

The main page's BUGS priced the sweep at 2,911 sites and refused it, on the ground that every one
has to be read. A pattern-applied gloss would make a wrong citation look verified, which is the
milestone-24 defect with better spelling. That argument is about a sweep and says nothing against a
rule on new text. The ratchet reads nothing retroactively and writes nothing mechanically.

What it buys is DECISIONS §151 (the goal of the repository split is independent release). A number
is one namespace over one tree, and it means nothing in a repository that does not hold the block. A
gloss is a string a reader or a `grep` resolves wherever the block ends up.

## What it cannot catch

- A citation that arrives in a merge commit. The diff is `base..HEAD`, and a merge brings in lines
  this branch never wrote. That is deliberate. The alternative fires on a lane for another lane's
  text.
- A grounded gloss that is still the wrong citation, which is the main page's standing BUGS entry
  and is not made smaller by the ratchet. It is made more *visible*. A wrong gloss is something a
  reader can see, and a bare number is not.
- A file that has already glossed the number keeps the escape forever, including for a later
  mention that means something else. That is the price of the per-file rule, and it is the right
  price. The reader has the name in front of them.
- A parenthetical that opens with the citation (`(milestone 298, notes/mdns.md)`) is not a gloss
  in this scheme and never was, because `CITE` looks for `milestone N (`. The ratchet asks such a
  site to be rephrased (`milestone 298 (retire the multicast DNS responder), notes/mdns.md`).
- Captured logs and the generated roadmap index are out of the corpus, with `vendor/`,
  `patches/` and lockfiles. Nobody wrote those lines as prose, and a fixture is the one thing a lane
  must not edit to satisfy a gate.
- It reads `HEAD`, not the working tree, because the line numbers in a `base..HEAD` diff are HEAD's
  and reporting them against an edited file would point at the wrong lines. So a fix has to be
  committed before this goes quiet. Since 2026-09-24 the tool enforces that. An uncommitted edit
  that adds or removes a citing line fails the run with "Commit first" instead of reporting on the
  old text (see [bugs.md](bugs.md)).

## What it costs, measured before it was turned on

Over the 42 pull request merges from #953 to #1002, replaying each lane's own diff against its merge
base, there was a median of 16 asks per pull request, each answered by one parenthetical. The
distribution is skewed rather than flat. 8 of the 42 would have been silent, and the one outlier is
#970 at 893, which landed 24 proposal files at once. That number is the cost of writing 24 new
documents that cite the roadmap heavily, which is exactly the case the gate is for.