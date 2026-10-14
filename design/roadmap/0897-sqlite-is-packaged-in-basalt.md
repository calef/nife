---
status: PARTIAL
raised: 2026-10-10
promoted_from: sqlite-is-packaged-in-basalt
milestone_dependencies: 835, 810
decision_dependencies: 265
machine_requirements: none
specific_machine: none
needs_person: no
---
# 897. SQLite is packaged in basalt, and its tests run in basalt's package build

*(The title and slug are provisional.)*

**PARTIAL, 2026-10-14 (UTC).** basalt builds SQLite 3.50.4 and `speedtest1` from pinned sources
and publishes a package `sqlite` per architecture. nife builds neither. On all three architectures
under QEMU, `swish-check` types `jig install sqlite` and runs the installed `speedtest1` in
memory, and the kernel suite runs both `speedtest1` runs on the same bytes; every run must print
the same verification hash. Left, as for milestones 810, 906 and 907: the package's digest in the
recipe. "Built" below has the rest, and why the tests stay in nife.

calef, 2026-10-10 (UTC): *"We can't add testing of every program that nife will ever run into
this tree. We need packages working."*

The sibling of milestone 810 (`ripgrep` is packaged in basalt and installed with `jig`), for SQLite.
Milestone 835 (a C library, stage 1: files, clock and memory) runs SQLite's `speedtest1` unmodified
on all three architectures. Its tests live in nife today (`system_tests/src/user/c_program_tests.rs`)
and skip unless `helpers/build-speedtest1.sh` ran. That is a foreign program tested inside the
kernel's tree, which calef ruled is the wrong place.

## The work

- basalt builds SQLite 3.50.4 and `speedtest1` from their pinned sources against the C library of
  the nife commit basalt pins, with the flags `helpers/c-library-cflags.sh` gives.
- basalt's package build runs `speedtest1 --verify` in memory and on a file, on each architecture,
  and fails unless both print the hash nife's tests check today.
- ioping is packaged the same way by a sibling of its own. That is milestone 906 (ioping is
  packaged in basalt, and its test installs it with `jig`), under §297 (software reaches a nife system as a package `jig` installs from basalt's index, the
  kernel included).
- When basalt's build runs them, the C program tests leave nife, and so do the two helpers.

Reuse: milestone 810's packaging and its basalt build, the helpers' pinned fetches, and the
assertions of `c_program_tests.rs`, moved rather than rewritten.

## Built 2026-10-14 (UTC)

Lane `milestone/897-sqlite-is-packaged-in-basalt`, with nifeos/basalt#23.

- basalt's C recipe shape unpacks a zip: a source with `unpack = true` is extracted beside itself
  after its SHA-256 matches. Every member is checked first, and an absolute name, a `..` step, a
  drive letter, a path resolving outside the build directory or a symbolic link refuses it.
- The recipe is basalt's `recipes/sqlite.toml`: the 3.50.4 amalgamation zip and `speedtest1.c`
  under the SHA-256s the helper pinned, `LICENSE.md`, and the helper's two compile-time options.
  The package is `sqlite` (provisional) and carries one program, `speedtest1`. The aarch64 package
  measured 1,300,376 bytes. The first published Release is `rolling-2026-10-14-85e15dd`, and
  `xtask/basalt-pins.index` pins its three lines.
- The license is `blessing`, SPDX's identifier for the SQLite Blessing. SQLite is public domain,
  which has no SPDX identifier; Fedora's sqlite package uses the same one.
- **Where the tests run.** This block asked for `speedtest1 --verify` to run in basalt's package
  build. That is the second option of milestone 810's ruling, and calef chose the first on
  2026-10-13 (UTC): *"1 is preferable because that tests jig and basalt's delivery mechanism as
  well."* So the tests stay in nife and run basalt's bytes, as for 906 and 907. Both
  `c_program_tests` `speedtest1` runs keep every assertion, and CI's `rg-threads` rows run them
  with `NIFE_SPEEDTEST1_REQUIRED=1`. `swish-check` types `jig install sqlite` and the in-memory
  run, held to the same hash.
- `helpers/build-speedtest1.sh` and `target/speedtest1/` left the tree. `C_PROGRAMS` names only
  `c_library_contract`, which fetches nothing.

## BUGS

- The recipe records the sources' digests, not the package's, as for milestones 810, 906 and 907.
- The package carries no `sqlite3` shell. `shell.c` is in the same zip, unbuilt and untried on
  nife.
- At the prompt only the in-memory run is typed. The file run needs a writable directory, which an
  unvouched program cannot hold.

## Follow-on

- **Outstanding.** The package's digest in the recipe: needs a build reproducible across machines.
  Checked 2026-10-14 (UTC): `recipes/sqlite.toml` records the sources' digests only.
- **Recorded.** No `sqlite3` shell yet, in this block's `BUGS`.
- **Recorded.** The prompt's file run, in this block's `BUGS`, waits on a recipe that vouches for
  a writable grant (§195 (a reviewed recipe vouches for a package)).

## Index row

SQLite (and ioping) built and tested in basalt's package build against the pinned nife, as `rg` is
by milestone 810, so foreign programs' tests leave nife's tree.
