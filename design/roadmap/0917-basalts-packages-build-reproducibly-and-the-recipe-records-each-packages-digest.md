---
status: BUILT
raised: 2026-10-13
built: 2026-10-17
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

**BUILT, 2026-10-17 (UTC).** basalt's packages build reproducibly on aarch64, riscv64 and x86_64.
Every basalt recipe records one SHA-256 per architecture in its `[package_digest]` table, and
`helpers/packages.py build` refuses a package that hashes to anything else. basalt's `packages`
workflow builds twice, on two runners with different checkout, temporary, cargo and rustup paths,
and its `reproduce` job (`the two builds are byte-identical`) fails unless the two repositories
match. basalt's pin-bump workflow runs `--update-digests` and commits the changed digests in the
bump pull request. `cargo xtask basalt-fetch` reads each digest from the recipe at the basalt commit
`xtask/basalt-pin` names and keeps none of its own.

## What was measured

Two builds at the same paths on separate runners are byte-identical. A build at another checkout
path is not. The causes, both in basalt's build and neither in nife's `write_package` or
`foreign-note` (no timestamps or member order differed):

1. Absolute paths in panic locations: the nife checkout, `$CARGO_HOME/registry`, the unpacked
   `ignore` crate, and the toolchain's `core`.
2. Cargo hashes a path dependency outside its workspace root into crate ids by absolute path (the
   patched `ignore`; nife's `c_library`, a workspace of its own with dependencies in `../crates`).
   The ids set function order, so the layout moved.

basalt fixes them with a fixed build path (`/tmp/basalt-build`), cargo `trim-paths`, relative link
inputs and the patched `ignore` inside the crate directory (nifeos/basalt#29). A path-independent
`c_library` build in nife was declined as invasive.

## Exit

On aarch64, riscv64 and x86_64, two independent builds of `ripgrep` produce byte-identical
packages. basalt's gate enforces it. The recipe's digest matches the published package.

Reuse: Debian's and Arch's reproducible-builds practice, which `SOURCE_DATE_EPOCH` and
`--remap-path-prefix` come from. `diffoscope` was considered for item 1. A plain byte diff plus the
archive's member list may be enough, so it is a tool to reach for, not a dependency.

## BUGS

- A second build doubles basalt's package build time.
- The build is reproducible only at the fixed path `/tmp/basalt-build`. On macOS `/tmp` is a link,
  so a package rebuilt there does not match one built on Linux.
- The `reproduce` job is not a required check on basalt's `main` until calef adds it to the ruleset.
- That the digests the pin-bump bot writes reproduce is first proven by `packages` on a real bump.

## Follow-on

- **Recorded.** Milestones 909 and 607 trust digests this milestone makes reproducible, and neither
  lists 917 in `milestone_dependencies`. Adding it edits their fields, which is the maintainer's
  call, so it is left for review.

## Index row

A package's digest is only worth vouching for if anyone can rebuild it and get the same bytes.
This makes basalt's builds reproducible, gates on it, and puts the digest in the reviewed recipe.
