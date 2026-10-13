---
status: NOT-STARTED
raised: 2026-10-13
milestone_dependencies: 909, 810, 121, 809, 801
decision_dependencies: 297, 151, 247
machine_requirements: none
specific_machine: none
needs_person: yes
---
# 916. Integration tests live in basalt: nife keeps kernel contracts, basalt tests the distribution

*Name: provisional. The number 916 is provisional until the merge queue lands it, and the title
and the slug are drafts. Branch `lane/integration-tests-home`.*

calef asked on 2026-10-13 (UTC): *"This does suggest to me that there are integration tests that
will eventually need to find a home somewhere. The ripgrep test isn't a kernel test. Where do you
think those tests should run eventually?"* The split below was proposed in reply, and he answered
*"Yes"*.

A test belongs to the repository whose promise it checks. A kernel guarantee is nife's, so nife
proves it with a program nife writes. "A stranger's program installs and runs" is a promise about
the distribution, so basalt proves it, on the image it ships.

## The ruled shape

1. Kernel contracts stay in nife. Each is driven by a small program nife writes itself. Two
   examples: a directory is enumerated through a capability, and a spawned thread is counted by the
   kernel.
2. Distribution integration runs in basalt, on every pin-bump pull request, and a failing run
   blocks the bump. The first case is `jig install ripgrep`, then `rg` walking a tree with four
   threads. SQLite, git and the benchmarks of §262 (nife is measured with the field's standard
   benchmarks, run unmodified, against Linux) follow.
3. A few early-warning rows stay in nife and run on each pull request. They fetch basalt's
   published package by digest and install it with `jig`. This is the shape milestone 810
   (`ripgrep` is packaged in basalt and installed with `jig`) builds for `ripgrep`. calef chose it
   because it exercises the delivery path too: *"1 is preferable because that tests jig and
   basalt's delivery mechanism as well."*
4. Fatal risk 1's evidence then sits in two places. Kernel contracts are proven in nife, and "a
   stranger's program runs" is proven in basalt. This milestone names that relation and does not
   edit [risk 1](../fatal-risks/1-only-software-written-for-nife.md); its verdict is the
   architect's under §216 (fatal-risk facts are correctable, and verdicts are the architect's).

## Where it starts

- `system_tests/src/user/ripgrep_tests.rs` runs unmodified `rg` over the priced fixture tree. Its
  transcript must be exactly the fixture's 138 match lines.
- `system_tests/src/user/ripgrep_threads_tests.rs` runs `rg` as a process at four cores. The
  kernel's count of threads the process started must be the main thread plus one worker per
  thread `rg` asked for.
- Both prove kernel facts through a foreign program. That is the part this milestone moves.
- basalt's `gate` workflow builds nife at its pin and runs `script/ci-build test`. It is one job
  covering aarch64, riscv64 and x86_64, and it tests no package of basalt's own.
- basalt's `main` has no branch protection and no ruleset, checked 2026-10-13 (UTC). A red gate
  therefore blocks nothing today.

## The work

1. nife writes its own replacement for each kernel fact `rg` proves now. One program enumerates a
   directory through a capability and checks the walk against the fixture. Another spawns threads
   and checks the kernel's count. `ripgrep_tests` and `ripgrep_threads_tests` then leave nife's
   system test, and nife's guarantees no longer rest on a foreign program.
2. basalt gains a distribution test stage. It boots the image milestone 909 (basalt composes the
   bootable image from its packages) composes, runs `jig install ripgrep`, and checks `rg`'s
   four-thread walk.
3. That stage runs on every pin-bump pull request. Making it blocking needs a required status
   check on basalt's `main`. That is a branch-protection change in `nifeos/basalt`, an admin act,
   and it is calef's.
4. nife keeps the early-warning rows of item 3 above, as milestone 810 builds them, and adds no
   others without a reason recorded beside the row.
5. Later packages add their own stages as they land: milestone 897 (SQLite is packaged in basalt),
   git, and the §262 benchmarks.

## Done when

- nife's system test proves directory enumeration and thread counting with programs in nife's
  tree, on all three architectures, and runs no `rg` outside the early-warning rows.
- A basalt pin-bump pull request runs `jig install ripgrep` and the four-thread walk on all three
  architectures, and a failing run blocks its merge.

## Dependencies

- Milestone 909 composes the image basalt's stage boots.
- Milestone 810 builds the package and the early-warning rows.
- Milestone 121 (`ripgrep` on nife: enumeration as a capability, and what the walk costs) holds
  the kernel facts that move to nife-owned programs.
- Milestone 809 (the package client becomes a program, `jig`, with the verbs an index needs)
  supplies `jig install`.
- Milestone 801 (packages over the internet: a host name, HTTPS and the distribution's index) lets
  a booted image fetch from basalt's index.
- §297 (software reaches a nife system as a package `jig` installs from basalt's index, the kernel
  included) is why distribution tests run on basalt's packages.
- §151 (the goal of the repository split is independent release and third-party programs) is why
  each repository proves its own promise.
- §247 (the split begins with basalt holding nife, and `procps` moves first) makes basalt the
  repository that pins nife.

Reuse: basalt's existing `gate` workflow and `script/ci-build test`, extended with a stage rather
than replaced. The early-warning rows are milestone 810's. nife's thread-count assertion reuses
`sched::process_threads_started`, which `ripgrep_threads_tests` already reads.

## BUGS

- A nife change that breaks `rg` is caught at the next pin bump, after it merged in nife, unless
  an early-warning row catches it first. The fix then lands in nife as a follow-up rather than
  blocking the change.
- basalt's gate grows heavier. Each tested package boots three architectures, and basalt's gate is
  one job for all three today. A failing leg can hide the legs after it, as basalt's README
  records, and each new package lengthens that one job.
- Until calef makes the stage a required check, a red run is advisory. This block cannot make it
  blocking from inside either repository.

## Index row

Each repository proves its own promise: nife proves kernel contracts with programs it writes, and
basalt proves that a stranger's program installs and runs on the image it ships, blocking any pin
bump that breaks it.
