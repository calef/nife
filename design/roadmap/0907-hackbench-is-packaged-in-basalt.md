---
status: NOT-STARTED
raised: 2026-10-12
milestone_dependencies: 836, 809, 810
decision_dependencies: 297, 195, 197, 250
machine_requirements: none
specific_machine: none
needs_person: no
---
# 907. hackbench is packaged in basalt, and its test installs it with `jig`

*(Minted 2026-10-12 (UTC) by lane `maintainer/foreign-programs-via-jig`, under §297 (software
reaches a nife system as a package `jig` installs from basalt's index, the kernel included). The
number is provisional until the merge queue lands it; 905 was held by #2022 and 906 by this lane's
ioping block when this was written. The title and slug are drafts.)*

calef, 2026-10-12 (UTC): *"They should all be installed via jig through basalt. I missed that for
our benchmarking tools, so I created debt by prioritizing them first."* This is that debt for
hackbench, on the pattern of milestone 810 (`ripgrep` is packaged in basalt and installed with
`jig`) and milestone 897 (SQLite is packaged in basalt, and its tests run in basalt's package
build).

## What exists, checked 2026-10-12 (UTC)

Nothing on `main` yet. hackbench's helper arrives with #2009, milestone 836 (a C library, stage 2:
threads), which was open and unmerged when this was written. On that branch:

- `helpers/build-hackbench.sh` fetches rt-tests 2.11's `src/hackbench/hackbench.c` from the `v2.11`
  tag on kernel.org, pinned by SHA-256, and compiles it unmodified for the three targets into
  `target/hackbench/<triple>/hackbench`, with rt-tests' own flags and one warning disabled.
- `system_tests/src/user/c_thread_tests.rs` runs it, and skips when the helper has not run.

Milestone 827 (hackbench on nife and Linux, as a scheduler benchmark and a multicore soak) runs the
same program on silicon, and from 2026-10-12 its exit needs the package this milestone builds.

## The work

1. A recipe in `nifeos/basalt` naming rt-tests 2.11's `hackbench.c`, its source URL and the
   SHA-256 the helper pins. The build refuses a download that does not match.
2. basalt's package build turns the helper into a build job at the nife commit `pins.toml`
   names. It links against that commit's C library with threads, once for each architecture
   (§19 (architectural parity is a tenet; the targets are aarch64, riscv64, and x86_64)).
3. The manifest note, written by basalt's build through the writer milestone 810 builds.
4. A package archive per architecture (§197 (a package is one archive file, named and vouched for
   by its recipe)), kind `optional`. The recipe vouches for it (§195 (a reviewed recipe vouches for
   a package, and the machine's owner may overrule it)). It is published to basalt's index
   (§250 (an image names its distribution's package index, at `basalt.nifeos.org`, and a package's
   bytes may live anywhere)).
5. The test moves behind `jig install hackbench`: `c_thread_tests.rs` installs the package before
   it runs the program, or the test moves into basalt's package build as milestone 897 moves
   SQLite's.
6. When the package is the only source, `helpers/build-hackbench.sh` and the archive's packing of
   `target/hackbench/` leave this tree.

## Exit

On aarch64, riscv64 and x86_64: a nife at basalt's pinned commit runs `jig install hackbench`
against basalt's index, the archive's digest matches the recipe, and the installed `hackbench`
passes the run `c_thread_tests.rs` makes of the helper-built one. A recipe whose checksum disagrees
with the download fails basalt's build.

Reuse: the helper's pinned fetch and flags and milestone 810's recipe, build job and publishing,
moved rather than rewritten.

## BUGS

- Everything under *What exists* is on #2009's branch, not `main`. If #2009 changes the helper or
  the test before it merges, this block's description of them is stale.
- basalt has no package build yet. This waits on milestone 810 for the first one.

## Index row

hackbench built by basalt from a pinned source and installed on nife by `jig`, so the scheduler
benchmark milestone 827 runs is a package and not a helper-built binary.
