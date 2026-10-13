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

**PARTIAL, 2026-10-12 (UTC).** basalt builds `ripgrep` from a recipe that pins its sources by
SHA-256, and a booted nife runs `jig install ripgrep` and `rg --version` against the repository it
built, on all three architectures under QEMU. Left: publishing at `basalt.nifeos.org`, which has no
host, and recording the package's digest in the recipe. "Built 2026-10-12" below has the rest; one
convention it needed is a question for calef there.

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

## What exists, checked 2026-10-07 (UTC)

In nife:

- `helpers/build-ripgrep.sh` downloads `ripgrep-14.1.1.crate` from `static.crates.io` with `curl`
  and checks no hash. It builds for three targets with `-Zbuild-std` against the `nife-dev` farm
  that `cargo xtask std-src` builds, into `target/ripgrep/<triple>/rg`.
- `xtask` packs that file into each architecture's initrd whenever it exists
  (`xtask/src/archive.rs:354`, `:525`, `:653`), so a base image can carry it by accident.
- Only `system_tests/src/user/ripgrep_tests.rs` runs it, by hand-placed capabilities.
- It carries no manifest note, so it is in no package and no recipe.

In `nifeos/basalt`, read through `gh api` on 2026-10-07: `README.md`, `pins.toml` with one entry
pinning `nife` by full commit id, `helpers/pins.py`, and two workflows. `gate.yml` runs the pinned
commit's own `script/ci-build test`. `pin-bump.yml` proposes a new pin daily. The README says
*"basalt has no toolchain of its own. It builds each component at the toolchain pin that component's
commit carries."* There is no recipe, no package build, no checksum check and no published index.

## The work

1. A recipe in `nifeos/basalt`, not in this tree, naming the crate (`ripgrep`), the version
   (`14.1.1`) and a SHA-256 of the `.crate` file. The build refuses a download that does not match.
   crates.io's index carries a `cksum` per version, which the recipe can be checked against
   (unverified). The recipe format is basalt's to choose; nife's `packages/*.recipe.toml` (§195 (a
   reviewed recipe vouches for a package)) is the shape to start from.
2. A build job in basalt. It checks out nife at the commit `pins.toml` names, runs `cargo xtask
   std-src` there for that commit's `nife-dev` farm, and builds `rg` for each target with the flags
   `helpers/build-ripgrep.sh` uses. One build for all three architectures, per §19 (architectural
   parity).
3. The manifest note, written at build time: `runner = elf`, `runtime = Std`,
   `arg = Words(ReadOnly)` and `output = Bytes`, in milestone 597 (a program carries its manifest in
   an ELF note)'s encoding, linked as an object (`-Clink-arg=note.o`, measured by #1319). Nothing
   writes a note for a foreign program yet, so the writer is part of this milestone. It belongs in a
   form basalt can call at the pinned commit, not a copy.
4. A package archive per architecture (§197 (a package is one archive file)), kind `optional`,
   vouched by the recipe's digest line (§195).
5. Publication to basalt's package index at `basalt.nifeos.org` (§250 (an image names its
   distribution's package index)). Basalt publishes nothing today, so the first publish job is part
   of this milestone, or of whichever milestone builds the index first.
6. Installation by `jig install ripgrep` (milestone 809 (the package client becomes a program)), to
   where swish finds an installed program (§219 (how the shell names an installed program to the
   spawner)). It is vouched through the activation set, so it gets its note's grants.
7. Stop packing `rg` into the base image by accident: `xtask` packs it only where a test asks, with
   a comment citing this milestone. That is not a restriction to tests. In the interim before
   milestone 809 works, an image carrying `rg` is a test image and says so; after it, customers get
   `rg` from `jig install`.

## Built 2026-10-12 (UTC)

- The recipe is `nifeos/basalt`'s `recipes/ripgrep.toml`: the crate at 14.1.1 and its SHA-256
  (crates.io's `cksum`), and `ignore` 0.4.23 at its SHA-256 with nife's
  `helpers/ripgrep-ignore-walk.patch`, read from the pinned commit and refused unless its SHA-256
  is the recipe's. A download that does not match is refused before anything is built (item 1).
- The build is basalt's `helpers/packages.py`: the pinned nife's `cargo xtask std-src` and
  `cargo xtask foreign-note`, `helpers/build-ripgrep.sh`'s flags with symbols stripped, and the
  pinned nife's `cargo xtask package` as the one producer (items 2 to 4). Packages measured
  2026-10-12: 2,950,424 bytes on aarch64, 2,782,560 on riscv64, 2,934,096 on x86_64, under
  `spawnproto::IMAGE_MAX_PAGES`' 4 MiB. Unstripped they were 5.1, 11.7 and 4.4 MB.
- The program is `rg`, in a package named `ripgrep`. The installer took the member named after
  the package as the program, so a package named `ripgrep` could only install `ripgrep`. It now
  falls back to the one member named `bin/<program>` (`package_archive::PROGRAM_PREFIX`), and
  basalt packs `bin/rg`. That is the question below.
- The repository basalt writes is the layout `jig` reads (`rolling/metadata/stand-in-index`,
  `rolling/targets/`), kept as a workflow artifact (item 5's layout; not its publication).
- The proof is basalt's `packages` workflow: it runs this tree's swish-check rows with
  `NIFE_BASALT_REPOSITORY` naming that repository, the test index host serves its lines beside the
  fixtures', and the second boot types `jig install ripgrep` then `rg --version`, which prints
  `ripgrep 14.1.1` (item 6). Without the variable, as in nife's own CI, the two lines are skipped
  and say why. Locally green on aarch64, riscv64 and x86_64. Fetching 2.8 MiB over software TLS
  under TCG took 21 s on aarch64, 29 s on riscv64 and 46 s on x86_64, so the two lines take `rg`'s 90 s bound.

Item 7 is unchanged: `xtask` still packs a built `rg` into the archive for milestone 121's tests.
An installed `rg` is unaffected, since a bare word reaches an image program only through
`grant_plan::Prog`.

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

Reuse: the build reuses `helpers/build-ripgrep.sh`'s flags and `cargo xtask std-src` unchanged,
and the package format is §197's. `cargo install` and `cargo-binstall` were considered for the fetch.
Neither builds against a custom target's patched `std`, and neither emits a §197 archive, so they
would replace only the download (unverified for `cargo-binstall`'s custom-target support).

## BUGS

- Nothing is published at `basalt.nifeos.org`; the check serves basalt's repository from the test
  index host. That is a host and DNS, calef's.
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

- **Outstanding.** Publishing at `basalt.nifeos.org`: checked 2026-10-12, `nifeos.org` answers no
  DNS query. A host, DNS and a certificate are calef's.
- **Outstanding.** The package's digest in the recipe: needs a build reproducible across machines,
  not measured. Checked 2026-10-12: the recipe records the sources' digests only.
- **Outstanding.** Several programs in one package, as calef ruled: the installer, §229's lookup and
  `jig remove` taking every `bin/` member. Two packages offering the same program name is a question
  for calef when that work starts, not before.
- **Recorded.** Item 7, `rg` packed into the archive for milestone 121's tests, is unchanged; this
  block's "Built 2026-10-12" says why it does not reach an installed `rg`.

## Index row

The first third-party program nife ships the way every later one will. A recipe in the
distribution names its source by checksum, basalt builds it against a pinned nife, a manifest note
says what it may hold, and `jig` installs it from an index. `ripgrep` already builds and runs on all three
architectures, so what this proves is the path a customer takes, not the port. It is optional, not base.
