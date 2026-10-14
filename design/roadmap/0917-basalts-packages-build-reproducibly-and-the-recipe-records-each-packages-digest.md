---
status: NOT-STARTED
raised: 2026-10-13
milestone_dependencies: 810
decision_dependencies: 195, 197
machine_requirements: none
specific_machine: none
needs_person: no
---
# 917. basalt's packages build reproducibly, and the recipe records each package's digest

*Name: provisional. The number 917 is provisional until the merge queue lands it, and the title
and the slug are drafts. Branch `lane/reproducible-packages`.*

On 2026-10-13 (UTC) the assistant noted that no milestone owned this work. The only home was a
Follow-on line of milestone 810 (`ripgrep` is packaged in basalt and installed with `jig`): *"The
package's digest in the recipe: needs a build reproducible across machines, not measured."* It
proposed minting a milestone for it, and calef answered *"Yes"*.

## Why

basalt's index vouches for whatever bytes one CI run produced. Nobody can rebuild a package and
check that it matches. Two milestones lean on a package's digest, and a third defines the packages:

- Milestone 607 (nife is built and updated from packages) assembles the image from base packages
  by digest.
- Milestone 909 (basalt composes the bootable image from its packages) composes each image from
  those packages, by digest.
- Milestone 611 (every program and crate belongs to a package) draws the package boundaries those
  digests cover.

None of them says the digest is reproducible. §195 (a reviewed recipe vouches for a package) puts
a package's digest in a reviewed recipe. A reviewer can only vouch for a digest anyone can rebuild.
§197 (a package is one archive file, named and vouched for by its recipe) makes that digest the
digest of one file, so the archive's bytes must be stable.

## Where it starts

- basalt's `recipes/ripgrep.toml` pins the `ripgrep` crate and its patched `ignore` by SHA-256.
  Its header says the package digest is not recorded, because nobody has measured reproducibility.
- basalt's `helpers/packages.py` builds the package in its `packages` workflow. It uses the pinned
  nife's `cargo xtask foreign-note` for the manifest note and `cargo xtask package` for the archive.
- nife's archive writer is `package_archive::write_package` in `crates/package_archive`. The
  note writer is `xtask/src/foreign_note.rs`.
- Pull request #2128 (milestone 810) adds `cargo xtask basalt-fetch`. It pins each published
  package's digest in nife's own `xtask/basalt-pins.index`, a second copy of what basalt produced.

## The work

1. Measure first. Build `ripgrep` twice, on two different runners, with basalt's `packages`
   workflow. Diff the `.nifepkg` for each architecture and record which bytes differ. Likely causes,
   unverified until measured: build paths embedded in the binary, timestamps, archive member order,
   and the manifest note.
2. Fix each cause in the repository that owns the code. The archive writer and the note writer are
   nife's. The build flags are basalt's. Candidate fixes are `--remap-path-prefix`,
   `SOURCE_DATE_EPOCH`, and sorted members in `write_package`.
3. A gate in basalt. The package build fails unless a second build yields identical bytes. The
   recipe gains one package digest line per architecture.
4. nife's early-warning rows read each digest from basalt's recipe at the basalt commit nife names.
   `cargo xtask basalt-fetch` keeps no digest of its own, so no digest-bump chore remains.

## Exit

On aarch64, riscv64 and x86_64, two independent builds of `ripgrep` produce byte-identical
packages. basalt's gate enforces it. The recipe's digest matches the published package.

Reuse: Debian's and Arch's reproducible-builds practice, which `SOURCE_DATE_EPOCH` and
`--remap-path-prefix` come from. `diffoscope` was considered for item 1. A plain byte diff plus the
archive's member list may be enough, so it is a tool to reach for, not a dependency.

## BUGS

- A second build doubles basalt's package build time.
- Some nondeterminism may be in the Rust toolchain or the linker, not fixable here. If so, the
  gate cannot pass for that architecture until upstream fixes it, and the block records which.

## Follow-on

- **Recorded.** Milestones 909 and 607 trust digests this milestone makes reproducible, and neither
  lists 917 in `milestone_dependencies`. Adding it edits their fields, which is the maintainer's
  call, so it is left for review.

## Index row

A package's digest is only worth vouching for if anyone can rebuild it and get the same bytes.
This makes basalt's builds reproducible, gates on it, and puts the digest in the reviewed recipe.
