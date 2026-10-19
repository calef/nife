---
status: PROPOSED
raised: 2026-10-19
milestone_dependencies: 849, 809
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# Versions belong to contracts and runners, and an install is refused when a need goes unmet

Raised 2026-10-19 (UTC) by lane `proposal/849-fork-5-versioning`, which builds nothing. calef asked
how version dependencies between components are modeled and enforced, and how two dependent
packages update together. The answer is that it is unbuilt: fork 5 of milestone 849 (a package
declares what it needs at run time) is the one fork there still open. calef added on 2026-10-07
(UTC) that runner versions need covering too: a script may need Python 3.12 and not 3.8.

This file splits fork 5 into seven questions, one decision each. A "no" to any of them leaves fork
5 held, and milestone 809 (the package client becomes a program) goes on installing without a
version check, as it does today.

Reuse: `package_index::version` (dpkg's ordering, already in the tree), milestone 614 (two installed versions of one program, each runnable)'s side-by-side
versions, `installed_programs::with_entries`, and `capability_manifest_note`. No outside resolver
fits, because none of them reads a capability manifest.

## What the tree says, checked

- Nothing of milestone 849 is built. The `provides` table fork 2 ruled is in no package file and in
  no gate: `helpers/packages.py` has no `provides` key. `Manifest::contracts()` does not exist.
- `depends = [...]` takes package names only and is a link lint. `evaluate` in `helpers/packages.py` allows a
  cross-package link when the target is an interface or in `depends`; nothing else reads it.
- The `.nifepkg` header (`package_archive`'s module documentation) carries a name, a version and an
  architecture, and no dependency field. A package's version "is compared as bytes" there.
- Package versions are ordered elsewhere. `package_index::version::newest`
  (in `crates/package_index`) is dpkg's `verrevcmp`, and `jig list -u` uses it.
  `installed_programs::Entry::version` calls a version "a label, never an identity".
- The manifest note checks its own format version at decode
  (`capability_manifest_note::VERSION`, refused as `UnknownVersion`). That is the note's encoding,
  not any contract's.
- §235 (the OS is built and updated from packages)'s ABI revision field is ruled into the note and not built. No crate names it.
- `component_plan::Requirements::contract` is "not matched against anything: what makes a build
  substitutable is that it speaks the protocol, and a string it carries cannot prove that", in its
  own documentation. No contract has a version handshake on the wire.
- `component_plan::dependents` does not order a swap. It names the direct dependents to warn before
  one contract is swapped, and §231 (a swap's warning to a dependent is advisory) makes that warning advisory.
- One install is one package. `jig` takes one name per verb (`main` in `jig/src/main.rs`), and the
  progenitor's activation stages one package and commits one generation
  (`install` in `crates/system_initializer/src/activation.rs`).
- `with_entries` is not where that limit lives. Each `Entry` already carries its own `package`, and
  the function writes whatever rows it is handed into one generation
  (`installed_programs::with_entries`).
- Two versions of one program install side by side, and a project's version set selects among them
  (milestone 614; `crates/swish/src/jig_versions.rs`).
- Only `line_editor` swaps live (§308 (the installer asks the terminal to swap), milestone 694 (the installer asks the terminal to swap)). Every other member of an install takes effect
  at its next spawn or the next boot. Service providers are base programs built by
  `system_initializer`, and base is updated through the boot slot (§235), a full copy per slot.

Three things follow. A set install would change the activation and `jig`, not `with_entries`.
Nothing in the tree orders two swaps. And no provider swaps live, so swap order is not a question
yet (see "Stated, not asked").

## The questions

### 1. What carries a version

- A, recommended: the contract. `filesystem` is at level 3. A package never names another package's
  version. This is the direction fork 5 already records.
- B: the package, Debian's way: `needs redoxfs >= 0.4`, ordered by the `verrevcmp` already in the
  tree.

B fails on two facts in the tree. A contract can have two providers (redoxfs today, another
filesystem later), and they need one number to agree on. And a package version is the upstream's
claim, which `installed_programs` already declines to treat as an identity.

### 2. Which side of a contract carries the range

- A, recommended: the provider declares the levels it serves (`filesystem 2..3`), and a consumer
  declares the one level it was built against (`filesystem 3`). This is Fuchsia's shape: a component
  names one target API level, and each platform release supports several, Component Manager
  refusing a component whose level is retired.
- B: the provider declares one level, and the consumer a range. This is OSGi's shape (an exporter
  states one version, an importer a range such as `[4.2,5)`) and Debian's (a versioned `Provides`
  takes only `=`, a `Depends` takes `>=`).

A wins because the side that keeps compatibility code is the side that knows what it serves. nife
writes both ends of every contract. Under B a consumer guesses a future upper bound, and a wrong
guess refuses an install that would have worked. Retiring a level under A is one edit, at the
provider.

### 3. Whether a level is written by hand or by the build

- A, recommended: the build writes it. Each contract crate exports its current level as a constant.
  A program's note records the level of each contract crate it linked, as Fuchsia stamps a
  package's ABI revision. A provider's top level is derived the same way. Only its floor, the oldest
  level it still serves, is written by hand, in its manifest.
- B: every level is written by hand, in the package's `provides` table and the program's manifest.

A follows fork 1's ruling that needs are derived and cannot drift. A consumer cannot claim level 3
while compiled against level 2. A script or a foreign binary has no contract crate, so its block or
note states the level by hand, as fork A's block already states its needs.

The floor stays hand-written because nothing in a binary shows how much old protocol it still
answers. That is a recorded exception to rung 1, and a test per retained level is what keeps it
honest.

### 4. How a runner is versioned

- A: as a contract, with a provider range: `python` provides `3.8..3.12`.
- B, recommended: a runner keeps its upstream's version, and a script states the range it runs on.
  The interpreter's package provides one version (`python 3.12`). The block says
  `runner = python 3.10..3.11`, written as one bare word so fork A's grammar is unchanged.
  Ordering is `package_index::version`.

A is false for Python. 3.12 removed `distutils` (PEP 632), `asynchat`, `asyncore` and `imp`, so
Python 3.12 does not serve every 3.8 script. nife does not write Python, so no provider here can
honestly promise old levels. B is the field's own convention: PEP 723 gives a script's block a
`requires-python` field, "the Python version(s) with which the script is compatible", which is a
range on the consumer.

So contracts and runners differ in which side carries the range, for one reason: who owns the
compatibility. Milestone 614 already installs two versions side by side, so Python 3.11 and 3.12
coexist. Each script runs on the newest installed runner in its range, and removing a runner some
script still needs is refused.

### 5. Who decides at install

- A: `jig`, from the index, before download. That is the sketch.
- B, recommended: the progenitor decides when it commits a generation, from the notes of the
  programs in it. `jig` runs the same function earlier, from the index copy, to refuse before
  downloading and to order a plan. The function lives in one crate both link (rule 7).

A puts the decision in the client, and §270 (a package manager holds an installer endpoint, not the spawn endpoint) gave the client the right to ask, not to grant. Under B
the decision sits with the installer that already checks every package against its catalog. The
progenitor already reads the members it places, so the levels come from the notes, and the package
header needs no new field. The check covers the whole generation, so remove and rollback run it
too. The boot-slot update of §235 must run it over the installed table before it commits a slot,
or a new base can strand an installed consumer.

### 6. Whether anything checks after install

- A: nothing. The generation was checked when it was committed.
- B, recommended: the progenitor also checks at spawn. When it endows a program with a contract, it
  compares the note's level with the provider it built, and refuses with the contract and both
  levels named.
- C: B, plus a version exchange on the wire of every contract.

B exists for the one path that skips an install check: the boot falling back to the other slot
after a failed update, which §235 keeps so a bad base cannot brick a machine. Without B, an unmet
level shows up as protocol garbage at the first call. C is refused for now. A provider's word on
the wire proves no more than its note does, which is `component_plan`'s own argument against
matching `contract`. It also adds an opcode to every contract, and that wire format is the
irreversible kind. A live provider swap, when one exists, is where C would be reopened.

### 7. How two coupled packages update

- A: a set install. `jig install a b` stages both, and the progenitor commits one generation with
  one `current` flip and one rollback. The activation would carry several staged packages, past
  today's one run of `spawnproto::IMAGE_MAX_PAGES`.
- B, recommended: one generation per package, provider first, and a rule that makes that order
  always work. A provider's new release must still serve the level its last release served, as
  Fuchsia keeps a level runnable through its sunset phase. The gate refuses a release that drops a
  level with no release between. `jig install a b` stays one command, and `jig` orders the two
  installs by level.

Under B every intermediate generation passes question 5's check, so rolling back to any of them is
consistent by construction. Runners never couple at all under question 4, because the old and new
interpreter install side by side. A loses because it is the one path that admits a break with no
overlap, and that break is what forces quiescing both ends and swapping both at once. Debian needed
exactly that relaxation, unpacking a mutually exact-versioned pair together. B makes that case
unrepresentable instead of handling it.

## Prior art, and what each gave

| System | Read | Taken | Refused |
|---|---|---|---|
| Fuchsia API levels | [api_levels](https://fuchsia.dev/fuchsia-src/concepts/versioning/api_levels): one target level per component; supported, sunset and retired phases; ABI revision stamped at packaging; Component Manager refuses a retired level at start | the level on the contract, the provider's range, the build-time stamp, the sunset overlap | nothing material |
| Debian | [policy ch. 7](https://www.debian.org/doc/debian-policy/ch-relationships.html): section 7.5: versioned `Provides` takes only `=`; section 7.3: `Breaks`; section 7.2: unpack together for mutual exact versions | `verrevcmp`, already in the tree, for runner versions | the range on the consumer for contracts; unpack-together |
| OSGi | [core 8, module layer](https://docs.osgi.org/specification/osgi.core/8.0.0/framework.module.html): importer range, exporter single version, resolved before any code runs | resolution before run, which question 5 is | the importer's range, for question 2's reason |
| Nix | [`nix-env --upgrade`](https://nix.dev/manual/nix/2.24/command-ref/nix-env/upgrade): an upgrade of several packages is one new generation | generations, which §208 (installing a package is granting it) already has | the set as the only unit; question 7 keeps one package per generation |
| PEP 723 | [script metadata](https://peps.python.org/pep-0723/): `requires-python` | the consumer's range for a runner | none |

## Stated, not asked

- Live swap order. Today no provider swaps live. Under question 7's rule, the provider would go
  first and serve both levels while consumers follow, and `component_plan::dependents` names whom
  to warn. The no-overlap case of quiescing both ends is not built, and question 7 is what keeps it
  unneeded.
- A level is an unsigned integer per contract, starting at 1 and only rising. The word "level" and
  the constant's name are provisional.

## What it unblocks

Milestone 849's remaining build gains a level constant in each contract crate and a level per need
in the note. That takes the descriptor to version 2, alongside §235's ABI revision. It also gains
the shared check crate and the runner range in fork A's block. Milestone 809 gains the check at commit and
`jig install` of several names, ordered.

## BUGS

- The floor of a provider's range is hand-written (question 3), so it can claim a level whose code
  was deleted. Only a test per retained level catches that.
- A contract still names a kind of service, not an instance (849's own BUGS), so two providers at
  different levels leave the progenitor choosing. The spawn check refuses rather than picks.
- OSGi's specification was read only in its first 100,000 characters, which cover sections 3.2 to
  3.7.
