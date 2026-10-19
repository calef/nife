---
status: BUILT
raised: 2026-10-12
built: 2026-10-17
milestone_dependencies: 835, 809, 810
decision_dependencies: 297, 195, 197, 250
machine_requirements: none
specific_machine: none
needs_person: no
---
# 906. ioping is packaged in basalt, and its test installs it with `jig`

*(The title and slug are provisional.)*

**BUILT, 2026-10-17 (UTC).** basalt builds ioping from a recipe that pins `ioping.c` by SHA-256
and publishes one package per architecture. nife builds no ioping. On all three architectures under
QEMU, every pull request's `swish-check` rows type `jig install ioping` against basalt's package,
then time five reads with the installed program. The kernel suite times reads in a granted
directory with the same bytes. The recipe records each architecture's package digest, and
basalt's build refuses a package that does not match it. "Built" below has the rest.

calef, 2026-10-12 (UTC): *"They should all be installed via jig through basalt. I missed that for
our benchmarking tools, so I created debt by prioritizing them first."* This is that debt for
ioping. It is the sibling of milestone 810 (`ripgrep` is packaged in basalt and installed with
`jig`). It is also the sibling milestone 897 (SQLite is packaged in basalt, and its tests run in
basalt's package build) named for ioping.

## What existed, checked 2026-10-12 (UTC)

- `helpers/build-ioping.sh` fetches ioping 1.3's one C file, `ioping.c` at the `v1.3` tag, pinned
  by SHA-256, compiles it with `helpers/c-library-cflags.sh`, and links it through
  `helpers/build-c-program.sh` into `target/ioping/<triple>/ioping` for the three targets.
- `xtask` packs that file into the archive when it is on disk (`xtask/src/farm/programs.rs`).
- `system_tests/src/user/c_program_tests.rs` runs it in
  `unmodified_ioping_times_reads_in_a_granted_directory`, and skips when the helper has not run.
  Nothing in CI runs the helper.
- Milestone 834 (ioping on nife and Linux, storage latency one request at a time) runs the same
  program on xenon, and from 2026-10-12 its exit needs the package this milestone builds.

## The work

1. A recipe in `nifeos/basalt` naming ioping 1.3, its source URL and the SHA-256 the helper pins.
   The build refuses a download that does not match.
2. basalt's package build turns the helper into a build job. At the nife commit `pins.toml`
   names, it compiles `ioping.c` with that commit's C library flags. It links the result the way
   `helpers/build-c-program.sh` does, once for each architecture (§19 (architectural parity is a
   tenet; the targets are aarch64, riscv64, and x86_64)).
3. The manifest note the helper writes today, written by basalt's build instead, through the note
   writer milestone 810 builds.
4. A package archive per architecture (§197 (a package is one archive file, named and vouched for
   by its recipe)), kind `optional`. The recipe vouches for it (§195 (a reviewed recipe vouches for
   a package, and the machine's owner may overrule it)). It is published to basalt's index
   (§250 (an image names its distribution's package index, at `basalt.nifeos.org`, and a package's
   bytes may live anywhere)).
5. The test moves behind `jig install ioping`: the run in `c_program_tests.rs` installs the package
   first and times reads with the installed program, or the test moves into basalt's package build
   as milestone 897 moves SQLite's. Which of the two is this milestone's first finding.
6. When the package is the only source, `helpers/build-ioping.sh` and the archive's packing of
   `target/ioping/` leave this tree.

## Built 2026-10-14 (UTC)

Lane `milestone/906-ioping-is-packaged-in-basalt`, with nifeos/basalt#17.

- The recipe is basalt's `recipes/ioping.toml`: `ioping.c` at `v1.3` under the SHA-256 the helper
  pinned, and the flags ioping's Makefile passes. A download that does not match is refused before
  anything is compiled (item 1).
- basalt's `helpers/packages.py` gained a C recipe shape. It compiles each `[[object]]` with the
  pinned nife's `helpers/c-library-cc.sh` and `helpers/c-library-cflags.sh`, links with its
  `helpers/build-c-program.sh` (which writes the manifest note), and strips symbols as ripgrep's
  build does (items 2 to 4). The aarch64 package measured 278,240 bytes. The shape takes per-object
  flags, so milestones 907 (hackbench) and 897 (SQLite) reuse it. basalt's CI compiles with clang
  19, since Ubuntu's default clang 18 refuses `-mabi=aapcs-soft`. The first published Release is
  `rolling-2026-10-14-c9f3c5f`, and `xtask/basalt-pin` names a basalt commit whose recipe records its digests.
- Item 5's finding: both, as milestone 810 did for `rg`. `cargo xtask basalt-fetch` takes ioping's
  lines from the recipe `xtask/basalt-pin` names, and the kernel archive carries the package's `bin/ioping`
  member, accepted by `package_index::accept`. `unmodified_ioping_times_reads_in_a_granted_directory`
  runs it with every assertion unchanged. CI's `rg-threads` rows run that test with
  `NIFE_IOPING_REQUIRED=1`, so a missing package fails rather than skips.
- `script/swish-check` types `jig install ioping`, `ioping -v`, and `ioping -c 5 -i 0 -s 4k` on
  the installed program's own file, from inside `packages`. Unvouched, ioping is granted that
  file read-only and the clock page. A directory would need a writable grant for ioping's working
  file, which its note does not ask for.
- `helpers/build-ioping.sh` and `target/ioping/` left the tree (item 6). `C_PROGRAMS` no longer
  names ioping.
- Documentation, for the later step that installs it: upstream ships a man page, `ioping.1`, at the
  same `v1.3` tag. The package does not carry it.

## Exit

On aarch64, riscv64 and x86_64: a nife at basalt's pinned commit runs `jig install ioping` against
basalt's index, the archive's digest matches the recipe, and the installed `ioping` times reads in
a granted directory as the helper-built one does today. A recipe whose checksum disagrees with the
download fails basalt's build.

Reuse: the helper's pinned fetch and flags, `helpers/build-c-program.sh`'s link, and milestone
810's recipe, build job and publishing, all moved rather than rewritten.

## The package's digest, checked 2026-10-19 (UTC)

- basalt's `recipes/ioping.toml` carries a `[package_digest]` table, one SHA-256 for each of
  aarch64, riscv64 and x86_64, recorded by milestone 917 (basalt's packages build reproducibly)
  in nifeos/basalt#29. basalt's pin-bump rewrites it with each new pin.
- basalt's `helpers/packages.py build` refuses a package whose bytes hash to anything else, and
  basalt's `packages` workflow builds twice, in separate checkouts, and fails unless the two
  repositories are byte-identical.
- At the basalt commit `xtask/basalt-pin` names, the recipe's three digests equal the assets of
  the Release that commit published. `cargo xtask basalt-fetch` accepts each package only under
  them, through `package_index::accept`, and `jig install ioping` checks the index line built from
  them.

## BUGS

- At the prompt ioping times reads of a file, not a directory: its unvouched note grants a named
  word read-only. Milestone 834's read pass fits that. Its write pass needs a writable grant, which
  unvouched bytes cannot hold, so 834 needs ioping's recipe to vouch for more (§195) before it
  writes.
- A nife change the pinned ioping cannot run on cannot pass CI on its own, as for `rg` (milestone
  810's `BUGS`).

## Follow-on

- **Done.** The package's digest in the recipe, by milestone 917 (basalt's packages build
  reproducibly, and the recipe records each package's digest).
- **Recorded.** The prompt's ioping holds a named file read-only, so a write pass is refused,
  beside the feature in this block's `BUGS`. Milestone 834's write pass needs §195's vouch.
- **Done.** Milestones 907 (hackbench is packaged in basalt) and 897 (SQLite is packaged in
  basalt) build on the same C recipe shape.
- **Recorded.** The man page `ioping.1` waits on `jig` installing documentation, noted in "Built"
  above for the milestone being minted for packaged documentation.

## Index row

ioping built by basalt from a pinned source and installed on nife by `jig`, so the storage-latency
benchmark milestone 834 runs is a package and not a helper-built binary.
