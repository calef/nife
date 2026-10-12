# Milestone 198 appendix: the gate line and the scoping of 2026-09-19

An appendix to [milestone 198 (a package manager)](../0198-package-manager.md), moved out of that
page on 2026-10-12 (UTC) under [§212 (a prose
budget)](../../decisions/0212-a-prose-budget-for-every-document.md). The main page is complete
without it. This is the history of the block's gate line, and the scoping lane's forks and first
slice as they stood before §157 (a trivial install is a web page, a USB drive, and packages over
the internet) superseded the slice.

## The history of the gate line

Ruled by calef on 2026-09-19 (16:32 UTC): `MILESTONE 23` is dropped from this line. It read
`DECISION, MILESTONE 23`, inherited through milestone 39 (repository structure for a
loosely-coupled OS), until then. That made a loop. This block waited on milestone 23 (a
capability-routed component OS with live replacement). Its one residual, state handoff, is declined
until a customer exists (§116 (live component state handoff is declined, for want of a
customer)).
And no second customer can be accepted until this block exists. The scoping lane checked
what a first slice would need from milestone 39's repository split. It found only an SDK gap (an
outside author needs this repository cloned to get the toolchain), which is a candidate milestone
of its own rather than a reason to wait on the split. The argument is in
[DECISIONS §156 (what the package manager waits
on)](../../decisions/0156-the-package-manager-waits-on-a-decision-not-milestone-23.md).
Milestone 39 keeps its own gate; this ruling does not touch it.

A second half of that fork was already ruled and this gate did not say so, found by milestone
435's first slice on the same day.
[§151 (the goal of the repository split is independent release and third-party
programs)](../../decisions/0151-repository-goal-is-independent-release.md) took the goal on
2026-09-15, and names this
milestone's own sentence in doing it. What §151 deliberately leaves open is the order: when the
split happens and against what preconditions. It lists a package format existing so `basalt` has
something to assemble as one of them, which is this milestone. The format, the activation shape and
the repository split are still not this block's and are deliberately not raised here.

## Scoped 2026-09-19

*History since 2026-09-19: the trivial-install row and the first slice below are superseded by
§157; the three fork proposals stand and are placed on rungs in the parent's rung table.*

A scoping lane (`milestone/198-package-manager-scoping`) built nothing and wrote the forks as
proposals, one decision each, for calef to rule on separately. The status did not move: nothing was
built, and the gate line was left as minted because changing it was the first proposal's ask.

| Fork | Proposal | Shape |
|---|---|---|
| The gate | [DECISIONS §156](../../decisions/0156-the-package-manager-waits-on-a-decision-not-milestone-23.md) | Decided 2026-09-19 by calef, as recommended: `Gate: DECISION` alone. The loop is real (198 waits on 23, whose residual waits on a customer, who waits on 198), and nothing in a first slice needs the split's timing. One real dependency was found, on a downloadable toolchain rather than on the split |
| Package format | [DECISIONS §197 (a package is one archive file)](../../decisions/0197-a-package-is-one-archive-file.md) | Decided 2026-09-20 by calef, after comparing apt, Homebrew, Alpine, Haiku and Nix: one archive file per package, the mainstream container. The reviewed recipe of §195 (a reviewed recipe vouches for a package) vouches for it |
| Activation | [installing-a-package-mutates-or-composes.md](../0507-installing-a-package-mutates-or-composes.md) | Decided 2026-09-23 by calef (§208 (installing a package is granting it)): A3, with rollback. As proposed, options with no winner: mutate, compose a union view, or only widen what may be spawned. The program namespace is sealed at boot, and the spawner gives the file service away, so nothing that builds processes can read an installed program today |
| Trust (found, not briefed) | [DECISIONS §195](../../decisions/0195-a-recipe-vouches-and-the-owner-may-overrule.md) | Decided 2026-09-19 by calef, after reading how apt, pkg, pacman, Nix, Fuchsia and Homebrew do it: Homebrew's shape (digests in reviewed recipes, per source) with the owner-vouches escape hatch every one of them keeps. The image's measured table becomes the first source |
| Trivial install | [DECISIONS §157](../../decisions/0157-a-trivial-install-is-a-web-page-a-usb-drive-and-packages.md) | Decided 2026-09-19 by calef, not as recommended: a web page, a download written to a USB drive and installed, then packages over the internet. The lane had recommended a QEMU run bundle first; that slice is superseded |

Superseded 2026-09-19 by §157 and by the parent's "Rescoped 2026-09-19".

## The superseded slice, and what survived it

The scoping lane's first slice had four items. Item 1, packages as host-side recipes, survives as
rung 3a's producer. Items 2 (image composition, milestone 150 (adding a program should not need
eight hand-maintained lists), PR #968) and 3 (the QEMU run bundle,
a reversible call per §157) are off the path. Item 4, removal, is part of 3a's exit criterion and of
the activation ruling.
