---
status: PARTIAL
raised: 2026-10-07
milestone_dependencies: 597, 809
decision_dependencies: 151, 195, 219, 250
machine_requirements: none
specific_machine: none
needs_person: no
---
# 810. `ripgrep` is packaged in basalt and installed with `jig`

**PARTIAL, 2026-10-13 (UTC).** basalt builds `ripgrep` from a recipe that pins its sources by
SHA-256 and publishes one package per architecture on a GitHub Release, indexed at
`basalt.nifeos.org`. nife builds no `rg`. Its gates fetch that package by a pinned digest, and a
booted nife runs `jig install ripgrep` and `rg --version` against it on all three architectures
under QEMU, on every pull request. Left: recording the package's digest in the recipe. The two
"Built" sections below have the rest.

*(From fork 4 of
[milestone 849 (a package declares what it needs at run time)](0849-a-package-declares-what-it-needs-at-run-time.md),
pull request #1797; the title and slug are drafts.)*

The purpose: a customer runs `rg` at the swish prompt after `jig install ripgrep`. `ripgrep` is a
customer-facing `optional` package that any customer installs from basalt's index. What is excluded
is baking it into the base image. calef, 2026-10-07 (UTC): *"I don't know why ripgrep would be
restricted to test images. We want ripgrep usable by a customer."*

calef ruled on 2026-10-07 (UTC) that packaging `ripgrep` and making it work are two milestones:
*"Should we have a milestone for packaging ripgrep and a milestone for a functional ripgrep, which
seems like what the ripgrep milestone should be."* then *"Yes"*. Then where: *"To be clear, we
should package ripgrep in basalt. That's what it is for."* And how it arrives: *"With a functional
jig, ripgrep should be installed via jig and not part of the base image. It isn't base."*

This is the packaging half. Milestone 121 (`ripgrep` on nife) is the functional half, and it
depends on this one only for running `rg` at the swish prompt.

## What exists, checked 2026-10-13 (UTC)

In nife:

- No build of `rg`. `cargo xtask basalt-fetch` fetches basalt's published package for each
  architecture by the digest `xtask/basalt-pins.index` pins.
- `xtask` packs the package's `rg` into each architecture's initrd only when it was fetched, so a
  base image never carries it.
- `system_tests/src/user/ripgrep_tests.rs` and `ripgrep_threads_tests.rs` run it by hand-placed
  capabilities. `script/swish-check` installs it with `jig` and runs it by name and by path.

In `nifeos/basalt`: `pins.toml` pins `nife` by full commit id, and three workflows. `gate.yml` runs
the pinned commit's own `script/ci-build test`. `pin-bump.yml` proposes a new pin daily.
`packages.yml` builds `recipes/ripgrep.toml` at the pin, proves `jig` installs it, and on `main`
publishes the packages to a Release and the index to Pages. The README says *"basalt has no
toolchain of its own. It builds each component at the toolchain pin that component's commit
carries."*

## The work

1. A recipe in `nifeos/basalt`, not in this tree, naming the crate (`ripgrep`), the version
   (`14.1.1`) and a SHA-256 of the `.crate` file. The build refuses a download that does not match.
   crates.io's index carries a `cksum` per version, which the recipe can be checked against
   (unverified). The recipe format is basalt's to choose; nife's `packages/*.recipe.toml` (§195 (a
   reviewed recipe vouches for a package)) is the shape to start from.
2. A build job in basalt. It checks out nife at the commit `pins.toml` names, runs `cargo xtask
   std-src` there for that commit's `nife-dev` farm, and builds `rg` for each target with the flags
   nife's in-tree `std` programs link with. One build for all three architectures, per §19
   (architectural parity).
3. The manifest note, written at build time: `runner = elf`, `runtime = Std`,
   `arg = Words(ReadOnly)` and `output = Bytes`, in milestone 597 (a program carries its manifest in
   an ELF note)'s encoding, linked as an object (`-Clink-arg=note.o`, measured by #1319). Nothing
   writes a note for a foreign program yet, so the writer is part of this milestone. It belongs in a
   form basalt can call at the pinned commit, not a copy.
4. A package archive per architecture (§197 (a package is one archive file)), kind `optional`,
   vouched by the recipe's digest line (§195).
5. Publication to basalt's package index at `basalt.nifeos.org` (§250 (an image names its
   distribution's package index)), by a publish job in basalt.
6. Installation by `jig install ripgrep` (milestone 809 (the package client becomes a program)), to
   where swish finds an installed program (§219 (how the shell names an installed program to the
   spawner)). It is vouched through the activation set, so it gets its note's grants.
7. Stop packing `rg` into the base image by accident: `xtask` packs it only where a test asks, with
   a comment citing this milestone. That is not a restriction to tests. In the interim before
   milestone 809 works, an image carrying `rg` is a test image and says so; after it, customers get
   `rg` from `jig install`.

## Built 2026-10-12 (UTC)

- The recipe is `nifeos/basalt`'s `recipes/ripgrep.toml`: the crate at 14.1.1 and its SHA-256
  (crates.io's `cksum`), and `ignore` 0.4.23 at its SHA-256 with basalt's
  `patches/ripgrep-ignore-walk.patch`, refused unless its SHA-256 is the recipe's. A download that does not match is refused before anything is built (item 1).
- The build is basalt's `helpers/packages.py`: the pinned nife's `cargo xtask std-src` and
  `cargo xtask foreign-note`, the shared link script's flags with symbols stripped, and the
  pinned nife's `cargo xtask package` as the one producer (items 2 to 4). Packages measured
  2026-10-12: 2,950,424 bytes on aarch64, 2,782,560 on riscv64, 2,934,096 on x86_64, under
  `spawnproto::IMAGE_MAX_PAGES`' 4 MiB. Unstripped they were 5.1, 11.7 and 4.4 MB.
- The program is `rg`, in a package named `ripgrep`. The installer took the member named after
  the package as the program, so a package named `ripgrep` could only install `ripgrep`. It now
  falls back to the one member named `bin/<program>` (`package_archive::PROGRAM_PREFIX`), and
  basalt packs `bin/rg`. That is the question below.
- The repository basalt writes is the layout `jig` reads (`rolling/metadata/stand-in-index`,
  `rolling/targets/`). basalt's `publish` job puts the packages on a Release and the index on Pages
  at `basalt.nifeos.org` (item 5). The first is `rolling-2026-10-13-ad0b146`.
- The proof is basalt's `packages` workflow: it runs this tree's swish-check rows with
  `NIFE_BASALT_REPOSITORY` naming that repository, the test index host serves its lines beside the
  fixtures', and the second boot types `jig install ripgrep` then `rg --version`, which prints
  `ripgrep 14.1.1` (item 6). Locally green on aarch64, riscv64 and x86_64. Fetching 2.8 MiB over software TLS
  under TCG took 21 s on aarch64, 29 s on riscv64 and 46 s on x86_64, so the two lines take `rg`'s 90 s bound.

## Built 2026-10-13 (UTC): nife's tests take basalt's package

Lane `lane/ripgrep-from-basalt`, with nifeos/basalt#15. DECISIONS §297 (software reaches a nife
system as a package `jig` installs from basalt) item 3 makes a `helpers/build-*.sh` scaffolding
until the program's package exists, and this one's does.

calef ruled on 2026-10-13 (UTC). Asked whether nife's ripgrep tests should (1) fetch basalt's
published `rg` package by digest and install it with `jig` in the test image, or (2) move to
basalt's packages workflow, he answered: *"1 is preferable because that tests jig and basalt's
delivery mechanism as well."*

- `cargo xtask basalt-fetch` (`xtask/src/basalt.rs`) reads `xtask/basalt-pins.index`, basalt's
  served index lines for `ripgrep`, one per architecture. It fetches each package from the Release
  its line lists, and accepts it only through `package_index::accept`, the installer's own
  decision, under that line's SHA-256.
- It writes them as a repository under `target/basalt/`. The test index host serves its lines
  beside the fixtures', and `script/swish-check`'s second boot types `jig install ripgrep` and
  `rg --version` against them in nife's own CI (item 6).
- The kernel harness's archive and the shell check's disk carry the package's `bin/rg` member,
  accepted the same way (item 7). The 138-line threaded walk, the needle lines and the walk of
  `crates/` run basalt's bytes, with their assertions unchanged.
- The host's reference `rg` is ripgrep's own 14.1.1 Release build for the host, pinned by SHA-256.
- `NIFE_BASALT_REPOSITORY` still wins, so basalt's `packages` workflow tests the repository it
  just built, and the fetch takes the programs from it.
- Without a network a local run says what it could not fetch, and the `rg` lines and tests skip
  with that reason. In CI a failed fetch fails the row.
- nife has no ripgrep build. The `ignore` patch is basalt's, byte for byte the same file.

**Who bumps the pinned digests.** Nobody has to: basalt never prunes a Release, so the pinned URLs
keep serving. A lane that wants a newer build, or whose change the pinned build cannot run on,
refreshes `xtask/basalt-pins.index` with the one-line `curl` in `xtask/src/basalt.rs`'s header and
lets CI judge it. A ripgrep or recipe bump in basalt names the nife pull request that takes it.

### The question for calef: how a package names a program that is not its own name

Debian's `ripgrep` installs `/usr/bin/rg`; nife's installer had no way to say that. Options:

1. **A `bin/<program>` member** (built, recommended): no format change, readable to anyone who has
   met `/usr/bin`, and it extends to a package with several programs later. The cost is a
   convention two producers must follow, written in `package_archive`.
2. **A header field naming the program**: explicit, but a format version bump for every reader.
3. **Name the package `rg`**: no change at all, and against "`jig install ripgrep`".

Reversible: only this tree and basalt's one recipe use it. If calef refuses (1), the recipe and the
installer's fallback change together and nothing else moves.

Ruled 2026-10-12 (UTC). calef: *"A package can contain many programs and they may be different from
the package name."* That takes (1) and widens it: a package carries any number of `bin/<program>`
members, and no program need share the package's name. What is built is the one-program case, so
`ripgrep`'s `bin/rg` already follows the ruling; a package with two `bin/` members is still
refused (see `BUGS`).

## Exit

On aarch64, riscv64 and x86_64: a nife at basalt's pinned commit runs `jig install ripgrep` against
basalt's index, the archive's digest matches the recipe, and `rg --version` at the swish prompt
prints ripgrep's version. A recipe whose checksum disagrees with the download fails basalt's build.
Searching with `rg pattern dir` is milestone 121's exit, not this one's.

Reuse: basalt's build runs the pinned nife's `cargo xtask std-src`, `foreign-note` and `package`
unchanged, and the package format is §197's. nife's fetch decides with `package_index::accept`, the
installer's own function, and fetches with `curl`. `cargo install` and `cargo-binstall` were considered for the fetch.
Neither builds against a custom target's patched `std`, and neither emits a §197 archive, so they
would replace only the download (unverified for `cargo-binstall`'s custom-target support).

## BUGS

- The guest never fetches from github.com in a gate. The test index host serves basalt's bytes, so
  `jig`'s listed location, its one redirect and the Sectigo root are exercised by no CI boot.
- Only `swish-check`'s `jig install ripgrep` takes the installer's path. The kernel harness and the
  by-path lines run the package's program member, accepted on the host.
- A nife change the pinned `rg` cannot run on cannot pass CI on its own. basalt builds against
  nife's merged `main`, so the `rg` rows stay red until basalt builds at a commit that has the change.
- The recipe records the source's digest, not the package's: that needs a reproducible build,
  unmeasured.

- A search may not fit a `std` program's heap. PR #1777 measured `rg --threads 1 --no-mmap` over
  this tree at a 3.0 MiB peak on macOS, against about 1 MiB of `std` heap. That is milestone 121's
  to solve, but `rg --version` passing here says nothing about it.
- A package with more than one `bin/` member is refused, though calef ruled that a package may carry
  many programs. The installer, the shell's bare-name lookup (§229 (how a bare name at the prompt
  reaches an installed program)) and `jig remove` each handle one program per package today.
- basalt's gate is one job for three architectures (its README's BUGS), so a red riscv64 build can
  hide behind a red aarch64 one.

## Follow-on

- **Done.** Publishing at `basalt.nifeos.org`, checked 2026-10-13: the index serves, and the
  Release `rolling-2026-10-13-ad0b146` holds the three packages.
- **Outstanding.** A route for a nife change the pinned `rg` cannot run on: basalt building at an
  unmerged nife commit, published where nife's CI can fetch it by digest. Not needed by any change
  so far.
- **Milestone 917.** Milestone 917 (basalt's packages build reproducibly, and the recipe records each
  package's digest).
- **Outstanding.** Several programs in one package, as calef ruled: the installer, §229's lookup and
  `jig remove` taking every `bin/` member. Two packages offering the same program name is a question
  for calef when that work starts, not before.
- **Done.** Item 7: nife packs no `rg` it built, only the fetched package's, and only when a
  gate fetched it ("Built 2026-10-13").

## Index row

The first third-party program nife ships the way every later one will. A recipe in the
distribution names its source by checksum, basalt builds it against a pinned nife, a manifest note
says what it may hold, and `jig` installs it from an index. `ripgrep` already builds and runs on all three
architectures, so what this proves is the path a customer takes, not the port. It is optional, not base.
