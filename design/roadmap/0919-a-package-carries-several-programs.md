---
status: BUILT
raised: 2026-10-13
built: 2026-10-14
milestone_dependencies: 810, 809
decision_dependencies: 197, 208, 219, 229
machine_requirements: none
specific_machine: none
needs_person: no
---
# 919. A package carries several programs, and `jig` installs, finds and removes each one

*Name: provisional. The title and the slug are drafts. Minted on `lane/multi-program-packages`;
built on `lane/multi-program-build`.*

calef ruled on 2026-10-12 (UTC), recorded in milestone 810 (`ripgrep` is packaged in basalt and
installed with `jig`): *"A package can contain many programs and they may be different from the
package name."* On 2026-10-13 (UTC) he answered *"Yes"* to minting this milestone and to placing it
in the packaging order before `core-tools`. `core-tools` carries five programs and `process-tools`
eight, and neither can be installed as a package until this lands.

## Built

Built 2026-10-14 (UTC) on `lane/multi-program-build`, against both rulings below.
`package_archive::installable` yields every program member, checked before any is placed. The
progenitor places each and writes one generation through `activation_set::with_entries`. A held
name refuses the whole package, and the reply names the program (word 2, a member index) and the
holder (two more messages, `spawnproto::holder_words`). `jig remove` is
`activation_set::without_package`. The fixtures are `duet` (`alto` and `tenor`, whose notes differ)
and `solo` (`alto` again). `script/swish-check` passes on aarch64, riscv64 and x86_64 under QEMU:
`duet` installs, both programs run by bare name with different grants, `solo` is refused naming
`alto` and `duet`, `jig remove duet` makes both words reach nothing, and `solo` then installs.
[notes/packages/several-programs.md](../../notes/packages/several-programs.md) has the transcript.

## The work

1. `package_archive` yields every program member. The member named after the package stays a
   program; every `bin/<program>` member is one more. A package with none still installs nothing.
   A `bin/` name that is empty or holds a `/` refuses the whole package. Each member's bytes are
   checked against its table-of-contents digest before any is placed.
2. The progenitor's `install` places every program under `packages/<package>/<version>/` and
   writes one generation holding a row and a default pointer for each. A refusal of any one
   program, `ImageName` or `Taken`, refuses the whole package, and nothing is committed. That keeps
   §208 (installing a package is granting it, and the installed programs are versioned): one install is
   one generation, and one rollback undoes it.
3. `jig remove` takes every row a package wrote (ruling 2 below).
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

## Rulings

calef ruled on both of this block's questions on 2026-10-14 (UTC), recorded on #2152.

1. **Two packages offering one program name**: *"If two packages offer the same program name,
   refuse the second package. Need to remove the first one before installing the second one."*
   The second package is refused whole, and the refusal names the program and the package that
   holds it, so the owner knows what to remove. This is §229 (how a bare name at the prompt
   reaches an installed program) B2, applied to every program at once.
2. **What `jig remove` names**: *"jig remove should work on packages, not programs."* `jig remove
   <package>` takes every row the package installed and its programs' pointers;
   `<package>@<version>` takes one version. A program's name is not the verb's object, which is
   milestone 614 (two installed versions of one program, each runnable)'s ruling 5 as it now reads.

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
