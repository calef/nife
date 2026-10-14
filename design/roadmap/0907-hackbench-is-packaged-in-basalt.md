---
status: PARTIAL
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

**PARTIAL, 2026-10-14 (UTC).** basalt builds hackbench from a recipe that pins `hackbench.c` by
SHA-256 and publishes one package per architecture. nife builds no hackbench. On all three
architectures under QEMU, `swish-check` types `jig install hackbench` against basalt's package
and runs the installed program's thread mode, and the kernel suite runs both of
`c_thread_tests`' hackbench runs on the same bytes. Left, as for milestones 810 and 906: the
package's digest in the recipe. "Built" below has the rest.

calef, 2026-10-12 (UTC): *"They should all be installed via jig through basalt. I missed that for
our benchmarking tools, so I created debt by prioritizing them first."* This is that debt for
hackbench, on the pattern of milestone 810 (`ripgrep` is packaged in basalt and installed with
`jig`) and milestone 897 (SQLite is packaged in basalt, and its tests run in basalt's package
build).

## What existed, checked 2026-10-12 (UTC)

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

## Built 2026-10-14 (UTC)

Lane `milestone/907-hackbench-is-packaged-in-basalt`, with nifeos/basalt#21.

- The recipe is basalt's `recipes/hackbench.toml`: `hackbench.c` and rt-tests' `COPYING` at
  `v2.11`, each pinned by SHA-256, the source's being the helper's. It is `GPL-2.0-or-later`, so
  basalt's source archive is the source offer. The flags are the helper's (item 1).
- The build is milestone 906 (ioping is packaged in basalt)'s C recipe shape, unchanged but for
  two things kernel.org needed. A source names the `file` it is saved as, since cgit puts the tag
  in a query (`?h=v2.11`). And the fetch sends a named User-Agent, since kernel.org answers
  Python's default with 403 (items 2 to 4). The aarch64 package measured 278,523 bytes.
- Item 5's finding: both, as for ioping. `cargo xtask basalt-fetch` takes the three hackbench
  lines in `xtask/basalt-pins.index`, and the archive carries the package's `bin/hackbench`.
  `unmodified_hackbench_threads_over_pipes` and `..._over_socket_pairs` run it with every
  assertion unchanged. CI's `rg-threads` rows boot the suite filtered to `unmodified_` with
  `NIFE_HACKBENCH_REQUIRED=1`, which covers ioping's test in the same boot.
- `script/swish-check` types `jig install hackbench` and `hackbench -T -p -g 1 -f 2 -l 10`: one
  group of two senders and two receivers, whose stacks fit an unvouched `std` program's heap.
- `helpers/build-hackbench.sh` and `target/hackbench/` left the tree (item 6).
- Documentation, for the later step that installs it: upstream ships a man page,
  `src/hackbench/hackbench.8`, at the same `v2.11` tag. The package does not carry it.

## Exit

On aarch64, riscv64 and x86_64: a nife at basalt's pinned commit runs `jig install hackbench`
against basalt's index, the archive's digest matches the recipe, and the installed `hackbench`
passes the run `c_thread_tests.rs` makes of the helper-built one. A recipe whose checksum disagrees
with the download fails basalt's build.

Reuse: the helper's pinned fetch and flags and milestone 810's recipe, build job and publishing,
moved rather than rewritten.

## BUGS

- The recipe records the source's digest, not the package's, as milestones 810's and 906's do.
- At the prompt the run is smaller than the kernel suite's: an unvouched `std` program gets 256
  pages of heap, and each of hackbench's threads asks for a 64 KiB stack. Milestone 827's silicon
  runs need its recipe to vouch for a larger heap (§195), or a smaller run per process.
- A nife change the pinned hackbench cannot run on cannot pass CI on its own, as for `rg`.

## Follow-on

- **Outstanding.** The package's digest in the recipe: needs a build reproducible across machines.
  Checked 2026-10-14 (UTC): `recipes/hackbench.toml` records the sources' digests only.
- **Recorded.** The prompt's heap limit on hackbench's threads, in this block's `BUGS`, for
  milestone 827.
- **Milestone 897.** Milestone 897 (SQLite is packaged in basalt) is next on the C recipe shape;
  its amalgamation is a zip, which the shape does not unpack yet.
- **Recorded.** The man page `hackbench.8` waits on `jig` installing documentation, noted in
  "Built" above.

## Index row

hackbench built by basalt from a pinned source and installed on nife by `jig`, so the scheduler
benchmark milestone 827 runs is a package and not a helper-built binary.
