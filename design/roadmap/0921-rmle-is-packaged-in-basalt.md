---
status: BUILT
raised: 2026-10-14
built: 2026-10-14
milestone_dependencies: 611, 809, 810, 906
decision_dependencies: 297, 195, 197, 250
machine_requirements: none
specific_machine: none
needs_person: no
---
# 921. rmle is packaged in basalt, and `jig` installs it

*(Minted 2026-10-14 (UTC) by lane `milestone/921-rmle-is-packaged-in-basalt`, under §297
(software reaches a nife system as a package `jig` installs from basalt's index, the kernel
included). The number is provisional until the merge queue lands it; the title and slug are
drafts.)*

calef approved on 2026-10-13 (UTC) a packaging order for first-party programs after the
benchmarks: rmle and the demos as the first first-party optional packages, then `core-tools`.
`packages/rmle.package.toml` makes rmle `optional`, as Debian ships each editor as its own
package. §297 item 2 says basalt builds and publishes first-party packages too.

## What exists, checked 2026-10-14 (UTC)

- `rmle` is the `[[bin]]` `rmle` of `components` (`components/src/rmle.rs`), a native program
  with no manifest note. Every image packs it, as it packs every `components` and `fixtures`
  `[[bin]]`.
- Only the kernel harness starts it: `kernel/src/user/rmle_service.rs` wires its raw terminal,
  its directory and its report, and `system_tests/src/user/rmle_tests.rs` drives it. No prompt
  line can start it.

## The work

1. basalt builds a first-party program: a recipe names a `[[bin]]` of the pinned nife's own
   workspace, built at that commit for each architecture's bare-metal target, with nothing
   fetched. The source archive names the nife commit as the whole of its source.
2. `recipes/rmle.toml` packs `bin/rmle`, `MIT OR Apache-2.0`, kind `optional`, published to
   basalt's index.
3. `script/swish-check` types `jig install rmle` against basalt's package on every architecture.
4. nife's tests keep building rmle from source, as milestone 909 (basalt composes the bootable
   image from its packages) item 5 keeps nife's own image build for nife's tests, so a change to
   rmle is tested before basalt pins it.

## Built 2026-10-14 (UTC)

Lane `milestone/921-rmle-is-packaged-in-basalt`, with nifeos/basalt#24.

- basalt's `helpers/packages.py` has a first-party shape: a `[nife]` table names the workspace
  package, and the build runs `cargo build --release --locked -p components --bin rmle` at the
  pinned commit for each bare-metal target, strips symbols, and packs with `cargo xtask package`
  (items 1 and 2). The aarch64 package measured 99,108 bytes. The first published Release is
  `rolling-2026-10-14-0f4fc9d`, and `xtask/basalt-pin` names a basalt commit whose recipe records its digests.
- `xtask/basalt-pin` names the basalt commit whose recipe records rmle's digests, and `script/swish-check` types `jig install
  rmle` (item 3). The line is not followed by `rmle`, which no prompt line can start.
- `rmle_tests` still builds rmle from source (item 4).

## Exit

On aarch64, riscv64 and x86_64: a nife at basalt's pinned commit runs `jig install rmle` against
basalt's index, and the installed package's digest is the index's.

Reuse: milestone 810 (`ripgrep` is packaged in basalt and installed with `jig`)'s build and
publishing, and milestone 906 (ioping is packaged in basalt)'s pins and swish-check lines.

## BUGS

- An installed `rmle` cannot be started. No prompt line wires a raw terminal for a program, so the
  package delivers bytes nobody runs until something gives rmle a launch path and a manifest note.
- Every image still packs rmle, because every image packs every `components` and `fixtures`
  `[[bin]]`. An `optional` package leaves the shipped image when milestone 909 composes the image
  from base packages.
- The package carries rmle at basalt's pinned nife commit, so it lags nife's `main` until the pin
  moves.

## Follow-on

- **Recorded.** rmle has no launch path from the prompt, in this block's `BUGS`.
- **Milestone 909.** Milestone 909 (basalt composes the bootable image from its packages) takes
  rmle out of the shipped image, since only base packages compose it.

## Index row

The editor reaches a customer as an optional package basalt builds from nife's own source, the
first first-party program to do so.
