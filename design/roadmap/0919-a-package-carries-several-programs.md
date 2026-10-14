---
status: NOT-STARTED
raised: 2026-10-13
milestone_dependencies: 810, 809
decision_dependencies: 197, 208, 219, 229
machine_requirements: none
specific_machine: none
needs_person: no
---
# 919. A package carries several programs, and `jig` installs, finds and removes each one

*Name: provisional. The number 919 is provisional until the merge queue lands it, and the title
and the slug are drafts. Branch `lane/multi-program-packages`.*

calef ruled on 2026-10-12 (UTC), recorded in milestone 810 (`ripgrep` is packaged in basalt and
installed with `jig`): *"A package can contain many programs and they may be different from the
package name."* On 2026-10-13 (UTC) he answered *"Yes"* to minting this milestone and to placing it
in the packaging order before `core-tools`. `core-tools` carries five programs and `process-tools`
eight, and neither can be installed as a package until this lands.

## What exists, checked 2026-10-13 (UTC)

- The archive picks one program. `package_archive::installable` calls `program_member`, which
  takes the member named after the package, or else the one member under `PROGRAM_PREFIX`
  (`"bin/"`). A second `bin/` member makes `program_member` return `None`, so the install is
  refused as `Refusal::NoProgram`. `package_archive::Installable` holds one `program`, one `bytes`
  and one `digest`.
- The installer writes one row. The progenitor's `install`, in
  `crates/system_initializer/src/activation.rs`, places one file with `place_program`, at
  `packages/<package>/<version>/<program>`. It builds one `activation_set::Entry` and commits one
  generation through `activation_set::with_entry`.
- `jig remove` names a program. `jig`'s `remove` sends the word to the progenitor's `remove`, which
  calls `activation_set::without_entry`. Milestone 614 (two installed versions of one program)
  ruling 5 made the program the verb's object. `jig remove ripgrep` is therefore refused today,
  because the package's one program is `rg`.
- The bare-name lookup is already per program. The shell's `swish::bare::resolve` asks
  `activation_set::lookup` for the word's default row, then runs
  `Path::of(package, version, program)`. Each row carries its own program name, so two rows from
  one package are two bare names with no change here.
- Grants attach per program. The progenitor's `vouched`, in
  `crates/system_initializer/src/image_request.rs`, hashes the bytes it is handed and asks
  `activation_set::lookup_digest` for a row (§219 (how the shell names an installed program to the
  spawner) option D). `endowed_image` then endows them from their own ELF note. Each `bin/` member
  carries its own note and its own digest, so each program gets its own grants. Nothing attaches
  grants to a package as a whole.
- Two packages may not offer one name. `with_entry` refuses a row whose program another package
  already holds, as `activation_set::Error::Taken`. That is §229 (how a bare name at the prompt
  reaches an installed program) B2, built for one program a package.
- `jig` stages a whole package in one 4 MiB run (`spawnproto::IMAGE_MAX_PAGES` pages), so every
  program in a package shares that limit.

## The work

1. `package_archive` yields every program member. The member named after the package stays a
   program; every `bin/<program>` member is one more. A package with none still installs nothing.
   A `bin/` name that is empty or holds a `/` refuses the whole package. Each member's bytes are
   checked against its table-of-contents digest before any is placed.
2. The progenitor's `install` places every program under `packages/<package>/<version>/` and
   writes one generation holding a row and a default pointer for each. A refusal of any one
   program, `ImageName` or `Taken`, refuses the whole package, and nothing is committed. That keeps
   §208 (installing a package is granting it, and the activation set is versioned): one install is
   one generation, and one rollback undoes it.
3. `jig remove` takes every row a package wrote. The proposed spelling is in the second question
   below.
4. §229's lookup needs no new rule. The lane proves it finds each program by its own name, and
   that a removed program's word reaches nothing.
5. Each program runs with its own note's grants. The lane adds a test that the two programs of one
   package are endowed differently when their notes differ.
6. A test package with two programs, neither named after the package, is built in nife's fixtures
   beside `packages/greeting.recipe.toml`. It is cataloged on aarch64, riscv64 and x86_64.
7. `script/swish-check` installs it with `jig install`, runs both programs by bare name, removes
   it with `jig remove`, and checks that both words are refused afterwards.

## Exit

On aarch64, riscv64 and x86_64, under QEMU: a test package whose two programs have names that
differ from the package's installs with `jig install`. Both run by bare name at the swish prompt.
`jig remove` then makes both unreachable by bare name.

## The questions for calef when this work starts

### Two packages offering the same program name

calef deferred this until the work begins. §229 B2 already refuses the second package's install
when it offers one program. What is open is whether that holds when a package offers several, and
what is refused. The options:

1. **Refuse the whole second package, naming the program and the package that holds it.** This is
   what Debian does when two packages ship one file and neither declares a diversion. It is §229
   B2 unchanged, applied to every program at once.
2. **Install the programs that do not conflict, and leave out the one that does.** The install
   then succeeds with a warning, and the package is installed partly.
3. **Require an explicit choice**, such as an owner's flag naming which package provides the word.
   Debian's alternatives work this way.

Letting the later package win is not offered: §229 B2 refused it, because the first package's
program would silently lose its name.

Recommendation: option 1. It needs no new rule, and every program a package installs is one its
author shipped. Option 2 leaves a package that is installed but missing a program its own scripts
may call. Option 3 is a priority under another name, which §229 refused as B3. The cost of option 1
is that two packages offering `ps` cannot both be installed. Reversible: no package offers a
conflicting name today.

### What `jig remove` names

Ruling 5 of milestone 614 made the program the object when a package's one program shared its
name. With several programs, removing them one at a time leaves a package half removed. Proposed:
a word naming an installed package removes all its rows. A word naming only a program keeps ruling
5's meaning. A word that is both is refused, naming both, as §229 refuses a word that is both an
image program and a live entry. Reversible: only `swish-check` and `ripgrep` remove anything today.

## Dependencies

- Milestone 810 holds the ruling and the `bin/` convention this widens.
- Milestone 809 (the package client becomes a program, `jig`) supplies `jig install` and
  `jig remove`.
- §197 (a package is one archive file) is the format, and it needs no change.
- §208 is why one install is one generation.
- §219 option D is why grants follow a program's own bytes.
- §229 is the bare-name lookup and the conflict refusal.

Reuse: `program_member`, `place_program`, `with_entry` and `without_entry`, each extended from one
program to several. The fixture reuses `greeting`'s recipe shape. Debian's file-conflict rule and
its alternatives were read for the first question; `dpkg` itself was not considered, since nothing
here unpacks into shared directories (§208).

## BUGS

- A generation is one 4,096-byte page (the progenitor's `PAGE_BYTES`). A row and its pointer take
  about 170 bytes a program, by arithmetic on `activation_set`'s `Writer::entry` and
  `Writer::pointer`, not measured. One page then holds about two
  dozen installed programs. `core-tools` and `process-tools` together use thirteen of them.
- All programs in a package share the 4 MiB staging limit.

## Follow-on

- **Milestone 909.** Milestone 909 (basalt composes the bootable image from its packages) builds
  `core-tools`, `process-tools` and the rest of the base as packages. It depends on this milestone,
  and its block names the edge.

## Index row

A package is the unit a distribution ships, and a program is the unit a person types. Most real
packages carry several programs under names unlike their own, so `jig` installs, finds and removes
each one, and each runs with its own grants.
