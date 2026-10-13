# The IPC fastpath: the appendices to design/roadmap/0188-ipc-fastpath.md

[Milestone 188 (the IPC fastpath)](../0188-ipc-fastpath.md) carries the decision it waits on, the
three findings, and what a fastpath may skip and must keep. It also carries the plan with each
phase's outcome, the after-phase-3 figures, where phase 4 stands after E3 on radon, the open
questions and the `BUGS`.
This directory carries the working its prose budget has no room for.

- [`scoping.md`](scoping.md): the starting numbers and their sources, and the tables behind the
  three findings.
- [`design-space.md`](design-space.md): what a fastpath may skip and must keep, the proofs and
  invariants, how two paths stay in agreement, and alternatives A to F priced.
- [`phases-1-to-3.md`](phases-1-to-3.md): pull request #732's re-measurement, the before and after
  on every architecture, the control run and the cost.
- [`phase-4-evidence.md`](phase-4-evidence.md): the case for and against phase 4 after phases 1 to
  3, and the E3 run on radon.

*Name: provisional. The directory follows §212 (a prose budget)'s siting default. calef names
directories.*

- `scoping` Name: provisional. For the parent's "The starting numbers" and its three findings. calef
  names files.
- `design-space` Name: provisional. For the parent's sections on what a fastpath skips, its proofs,
  agreement and the alternatives. calef names files.
- `phases-1-to-3` Name: provisional. For the parent's "What phases 1 to 3 measured". calef names
  files.
- `phase-4-evidence` Name: provisional. For the parent's "What this says about phase 4" and its E3
  section. calef names files.
