# Milestone 47 appendix: `PATH` and completion

An appendix to [milestone 47 (navigation and naming)](../0047-navigation-and-naming.md), moved out of
that page on 2026-10-13 (UTC) under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md).
The main page is complete without it. It holds the block's design of `PATH`, the two passes that
priced it, and the record of completion as built. Long sentences were split as they moved, and the
option names in the two tables lost their bold; no figure, date, name or quotation was dropped.

## `PATH`: there is no search, because there is no ambient namespace to search

The absolute-paths section ([absolute-paths-and-bind.md](absolute-paths-and-bind.md)) takes Plan
9's answer for paths in general. `PATH` is that same question narrowed to programs, and Plan 9
answers it the same way. Plan 9 has no `PATH` variable at all. `/bin` is bound per-process,
union-mounted from whatever that process's namespace assembled, so what you can run is what is
bound. Taking the same answer here is consistency, not a new idea.

`PATH` is two bad things at once. It is a *search*, over a namespace you have *ambient access to*.
The search makes the order of a string into a security boundary: a writable directory ahead of a
system one, or `.` anywhere in it, and someone plants an `ls` that you then run. The ambient access
is why the order matters at all. `PATH` never controlled *access* (permissions did), only which of
your already-reachable options wins. The tell is that `which` exists as a whole command whose job is
answering "which one did I actually get?".

So the program namespace is the endowment, and a name binds to exactly one thing in it. The
property that follows is the same class as `rm -rf /` (in [mkdir-and-rm.md](mkdir-and-rm.md)):
`PATH` injection is structurally impossible rather than mitigated. There is no search order to
manipulate, no `.` to include by accident, and no writable directory that can precede a system one,
because there is no search.

The distinction that makes it work. A shell may extend its program namespace only with
capabilities it already holds, so extending is a naming convenience and never an authority increase.
Unix nominally has this property too and loses it in practice. Ambient authority means everyone can
read `/usr/bin`, so `PATH` order becomes the de facto security boundary. Here it cannot be, because
naming and access are separate things.

Four open questions, none decided:

- Unions and shadowing. A namespace unioned from several sources brings first-match-wins back,
  which is the ambiguity just removed. Plan 9 chose ordered union with explicit before/after on
  `bind`. The alternative is to refuse ambiguity (an error when two sources offer `ls`), which is
  more honest and probably more irritating.
- Enumeration. "What can I run?" is enumeration of the namespace. It is the same insight as globbing
  and completion ([touch-and-globbing.md](touch-and-globbing.md)) and bounded the same way:
  completion cannot offer a program no capability reaches.
- Compile-time set to runtime lookup. `Prog` is a closed enum with `from_name` today, and init
  already loads from the initrd by name, so half the mechanism exists. What is missing is enumeration
  and not being a fixed set.
- Does `$PATH` survive as a string? If the namespace is a capability, `echo $PATH` has no
  referent, and that is a divergence on one of the most-referenced variables in shell scripting. It
  looks earned (the variable's two real uses, inspect and modify, become `caps` and a grant). But the
  cost is real and should be named rather than glossed.

Two milestones this reaches into. Milestone 49 (users, login, and attribution) is what *hands* a
session its program namespace, so "who gets which capabilities at startup" includes which programs.
And milestone 39 (repository structure and the road to a distribution) inherits the sharper
consequence. Installing a program becomes granting it into a namespace, which is a materially
different packaging story. That is worth being on the record before anyone designs a package manager
around the assumption that installation means writing into a globally readable directory.

### PATH, sized rather than merely found (investigated further 2026-08-26, `milestone/47-remainder-round2`). **PROPOSED, not decided.**

The 2026-08-26 lane found the manifest-provenance blocker and correctly declined to invent an
answer. This lane went one step further, on the instruction to price rather than merely name a
fork. Is carrying a manifest for a runtime-discovered program a contained extension, or does it
force the real packaging system? Both, depending on which half. Storage and trust turn out to be
cheap, almost free. The spawn protocol does not, and neither does the scope question, so this is
still a fork, just a better-priced one.

What this lane found that the previous one had not looked at: `nifefs` needs no format change at
all, and the trust chain already covers an arbitrary name.

- A manifest can be an ordinary sibling archive entry (`printenv.manifest` next to `printenv`,
  say), not a new field on `nifefs`'s directory entry. `nifefs` is a flat name-to-bytes store
  (`crates/nifefs`'s own module doc). A manifest is just bytes under a name, exactly like an ELF or
  `program_measurements` itself already is. No format bump, no `MAGIC` version, no `DIR_BLOCKS`
  change.
- `measured_boot::PROGRAM_MEASUREMENTS` is already generic over arbitrary names. It is a
  build-time text table, one `name sha256` line per packed archive entry
  (`crates/measured_boot::manifest_entries`), and `crate::trust::require(name, bytes)` checks any
  name against it. A manifest file packed at build time would get a line in this table for free,
  with zero new kernel mechanism. The same `require` call that already vouches for a program's
  bytes before `Elf::parse` touches them would vouch for its manifest's bytes before anything parses
  those either. Rule 7's "a shared definition is a crate" is the only new thing this half needs. That
  is a small, dependency-free crate (provisional name `manifest_proto`, `environment_protocol`'s own
  shape) defining `Manifest`'s on-disk encoding. The build tool that writes the bytes and the shell
  that reads them then share one definition and cannot drift.

What does not get cheaper, and is the reason this is still a fork. `grant_plan::Prog::id()` is
a small integer, 0..`PROG_COUNT`. Init decodes it by indexing a `[Option<Elf>; PROG_COUNT]` array.
It built that array once at its own startup by walking every *compiled-in* `Prog` variant's name
through `program()`. A name absent from that fixed table has no id to send and no slot for init to
have prepared, whatever a shell learns from a manifest file. Reaching a runtime-discovered program
therefore needs `spawnproto`'s request word to carry a name instead of, or alongside, a small int.
Today that word holds the program id, integer argument and memory-grant page count (`spawnproto`'s
own header). With a name, init can look the ELF up by string, the way it already looks up every
compiled-in program (`nifefs::Fs::read`, already string-keyed; the previous lane's own finding).

Mechanically this is the same shape `DIR_BIT`'s "expect two more `SEND`s" already establishes for
data that does not fit one word. So a `NAME_BIT` (provisional) carrying a length-prefixed name over N
more `SEND`s is not a new *kind* of mechanism. But it is a new wire message two programs (the shell
and init) must agree on forever. `spawnproto`'s own header says so explicitly ("adding a field is a
change here, not to the syscall surface... [still] the irreversible category"). This milestone's
environment section ([environment.md](environment.md)) already priced the identical category of
change for the config page. That places it squarely in the "move fast on what can be undone"
tenet's expensive column. It is reversible in principle, but every future program and every future
shell reads are written against whatever shape is chosen.

The scope question, which is sharper than the wire-format one. Even with both of the above
built, a program still reaches a running system only by being packed into the initrd at build
time. `nifefs` has no live write path, and the archive is mapped read-only into init. So this
mechanism would decouple "add a program" from "edit `grant_plan::Prog`'s Rust source and recompile
every consumer of it". That is a real, meaningful simplification: authoring two files and re-packing
the archive, rather than a source change across `grant_plan`, `swish` and every match on `Prog`. But
it would not decouple "add a program" from "rebuild and re-flash the boot image". That is what
`PATH` usually evokes (drop a binary onto a live machine and run it). Building the name-carrying
wire change now, without deciding whether that partial win is worth having on its own, carries a
risk. It could be exactly what this section's own closing paragraph already warned against:
designing a package manager's shape by accident, one wire message at a time, before milestone 39
decides what "installing a program" means here.

Three options, costed rather than merely listed (the six-questions discipline: what else was
considered, what this tree already does, what each costs, how reversible):

| Option | What it buys | What it costs | Reversibility |
|---|---|---|---|
| Do nothing further (status quo) | Nothing changes; `Prog` stays closed | Zero | N/A |
| Build the manifest-as-data half only (a `manifest_proto` crate, a build-time convention for pairing a program with its manifest file, `require`-checked the same as any program) with no spawn-protocol change, so nothing can actually be spawned from it yet | A `manifest_proto` crate other tooling (`caps`, a future installer) can already read and validate against, landed cheaply and reversibly, and it de-risks the harder half by proving the encoding before the wire change is designed | Builds a crate nothing consumes yet, which this tree's own convention (a lane ships nothing speculative) argues against unless a near-term consumer exists | Fully reversible: an unused crate costs nothing to delete |
| Build both halves (the wire change too) | Real decoupling of "add a program" from "recompile grant_plan" | A new wire message two programs must agree on forever, the expensive, hard-to-undo category this project reserves for calef, and it commits to a scope answer (build-time-only PATH) before milestone 39 has decided whether that is the right shape at all | The wire format, once shipped and depended on, is exactly the "who else has already acted on this" case the tenet asks about: every future program written against it inherits the shape |

Recommendation: neither yet. This project has its own limit on when a fork earns a lane
("recommend on reversible forks; give options only on irreversible ones"). This is the irreversible
kind, so options rather than a push toward one. If forced to rank, the middle option is the
defensible next step if and only if a near-term consumer is already planned. Otherwise it is exactly
the speculative crate this tree's own convention refuses. The full wire change should wait for
milestone 39 to at least sketch what "installing a program" means. Building the spawn protocol
first risks answering that question by accident, one opcode at a time. That is the failure this
section's closing paragraph already named before this lane existed.

What this does not decide. Whether `PATH` is worth building at all before it has a forcing
customer (unchanged: none exists today). And whether the manifest encoding, if built, belongs in
`grant_plan` itself or a new crate (a naming question, calef's per this tree's own rule, not sized
further here).

### The manifest question was answered elsewhere (checked 2026-09-26). **PROPOSED: how a bare name reaches an installed program.**

The table above is overtaken. §208 (installing a package is granting it, and the activation set is
versioned) made the activation set the record of what is installed, digest and manifest together.
§219 (how the shell names an installed program to the spawner) ruled option D: the shell reads a
program's bytes and sends them as frames, and the progenitor finds the manifest by digest. §197 (a
package is one archive file) puts the manifest in an ELF note (ruled 2026-09-26; the build is in
flight as PR #1338). No name crosses the wire, so the `NAME_BIT` priced above is not needed. The
"build-time only" scope objection is gone too: `package install` changes what runs without a
reflash.

What is left is this block's own first open question, unions and shadowing. Today an installed
program runs only by path (`packages/uptime/0.1.0/uptime`, `components/src/swish.rs`, `run`: a
token with a `/` is a file's bytes). Its bare name is refused as "no such program"
(notes/packages/fetching.md). Three options:

| | A bare name that is not in the image | Cost | Reversibility |
|---|---|---|---|
| Paths only (status quo) | Refused | Zero. `bind` already shortens a path | Nothing to undo |
| The live activation set, ambiguity refused | Resolves to the live generation's entry of that name. An installed name equal to an image program's is refused at `package install` | One lookup in a table the shell already reads for `caps <path>`. No search order exists, because each name has one entry | Cheap now; once scripts name installed programs bare, removing it breaks them |
| A bound directory (Plan 9's `/bin`) | Tried as `<bound name>/<name>` after the image | A convention for which bind is searched, and with two sources, an order | An order is the thing this section says `PATH` gets wrong |

Leaning toward the activation set, because it is "the program namespace is the endowment" with no
search at all: installing is what puts a name in it (§208). It is a name the shell and every script
will depend on, so it is an architect's.

## Completion: built 2026-09-26 as §227 option D

Two lanes on 2026-08-26 found that Tab is dropped by the terminal's line discipline. They priced a
round trip on the terminal wire (a reply flag and a resume opcode) to reach the shell mid-line.
That pricing is now DECISIONS §227 (how Tab reaches the shell), whose option D needed no wire at
all: the raw mode of milestone 169 (`kilo`, the smallest real text editor) already existed. calef
ruled D on 2026-09-26.

So the shell turns raw mode on at the prompt and runs `line_editor::LineDisc` itself. Tab is
`Event::Tab`, and `swish::complete` finishes the word from what the shell can name. In command
position that is a builtin or an image program. Otherwise it is an entry of the directory the word
leads into, listed with the same `ENUMERATE` `echo *` needs. `^C` at the prompt is a byte the
shell's editor turns into a discarded line. A supervised job still takes `^C` through the terminal's
count (§24 (interrupting the foreground process)), so the shell leaves raw mode just before it
spawns one. Proven by host tests in `crates/line_editor` and `crates/swish`, and at a real prompt
on all three architectures by `script/swish-check`, which types a Tab twice and a `^C` once. The
binary cost and the per-keystroke cost are measured in notes/shell-line-editing.md, with its BUGS.
fish's extras are proposals (the Follow-on on [the main page](../0047-navigation-and-naming.md)).
