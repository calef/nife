---
status: BUILT
raised: 2026-07-31
built: 2026-10-03
---
# 47. Navigation and naming: `cd`, `pwd`, `ls`, `mkdir`, `rm`, paths, and environment

Where this stands, 2026-10-03 (UTC): every item is Done, refused or routed to a numbered
milestone, so the block is `BUILT`. It was re-checked against the tree that day. The last open
item was the set grant at the prompt. calef ruled it 2b in §228 (how a set of matched names reaches
the progenitor) and then option A on 2026-09-27, and milestone 205 (how a foreign program is told
what to do) built it.

This page was split on 2026-10-13 (UTC). calef ruled on 2026-09-25 (UTC) that this block is not
split, and granted it a marked exception to §212 (a prose budget) at 17,386 words. That ruling is
superseded. calef, 2026-10-13 (UTC): "Lets launch a lane to bring the two exceptions and 0139 under
the cap safely." The exception marker is gone. Each section's full text, with its reasoning, its
measurements and the history kept beside it, moved into appendices under
[`0047-navigation-and-naming/`](0047-navigation-and-naming/README.md). This page is complete
without them.

| Piece | Where it stands | Appendix |
|---|---|---|
| The status history: the stale `IN-PROGRESS` token, the 2026-08-18 discharge, and each piece as it landed | record | [status-history.md](0047-navigation-and-naming/status-history.md) |
| `mkdir`, `rm`, `RMDIR` and `rm -r` | built 2026-07-31 (§49 (removal is a directory operation)) | [mkdir-and-rm.md](0047-navigation-and-naming/mkdir-and-rm.md) |
| `ln`: hard links declined (§110 (hard links are declined)); symlinks superseded by `bind` (§50 (namespace composition)), with the naming search that settled it | decided | [links-and-bind.md](0047-navigation-and-naming/links-and-bind.md) |
| `touch`, both halves (§112 (`touch`'s two behaviors need two rights)), and globbing with its set grant | built 2026-08-22, 2026-08-24 and 2026-07-31 | [touch-and-globbing.md](0047-navigation-and-naming/touch-and-globbing.md) |
| Absolute paths, `bind`, and why milestone 64 (enough `std` to run somebody else's crate) forced the namespace half | built 2026-08-18 and 2026-08-26 | [absolute-paths-and-bind.md](0047-navigation-and-naming/absolute-paths-and-bind.md) |
| Environment variables: the three thirds, the measured encodings, and the page of §111 (inert configuration is a read-only page) | decided 2026-08-23 | [environment.md](0047-navigation-and-naming/environment.md) |
| The config page, `printenv` and real init wiring; the secrets third | built 2026-08-23 and 2026-08-26; secrets moved to milestone 65 (a secrets service) | [environment-built.md](0047-navigation-and-naming/environment-built.md) |
| `PATH` and completion | built 2026-09-26 as §229 (how a bare name at the prompt reaches an installed program) B2 and §227 (how Tab reaches the shell) option D | [path-and-completion.md](0047-navigation-and-naming/path-and-completion.md) |
| `file:` and `run` removed; the function-call syntax fork | built 2026-07-31; fork refused by §141 | [shell-grammar.md](0047-navigation-and-naming/shell-grammar.md) |

In brief. A navigation model for a system with no global namespace. Keep the Unix command names
and behavior wherever they can work honestly; diverge only where the capability model forces it, and
say why each divergence is earned. The keystone is built: the directory capability and its
rights ladder (DECISIONS §47 (the directory capability's rights), notes/dir-capability.md), six rungs
at the keystone and a seventh added by DECISIONS §112 for `touch -t`. So are the five commands, on
both ISAs. `cd`, `pwd`, `ls`, `mkdir` and `rm` work as shell builtins. `..` is clamped at your root
by popping the stack of capabilities the shell descended through, and `pwd` is relative to that
root. A name on a command line is resolved against the shell's position at the moment the grant is
made, so a child holds a capability to one file and cannot re-resolve anything. The headline is
proven with the real shell binary: two shells rooted in two subtrees, each told nothing about which
it holds, and neither can name the other's files (notes/shell-navigation.md).

Why it matters. calef's framing, and it is the governing constraint: *"I hate Windows/DOS
specifically because they went differently than virtually every other OS I've used."* Gratuitous
divergence taxes every user forever. So the bar is not "is this more capability-pure", it is "does
the model actually force this." Three divergences clear that bar; the rest of Unix's surface should
survive unchanged.

## The reframe: `cd` was never the problem

A working directory, in capability terms, is *a directory capability the shell holds, used as the
default base for resolving names*. Held by the shell that is entirely legitimate, the same as its
untyped budget. The badness in Unix is three specific things, none of which is `cd` itself:

1. Children inherit it silently, so every process gets a starting point nobody granted it.
2. Relative paths resolve implicitly, so a program's reach depends on invisible state.
3. `..` walks out, so the cwd bounds nothing.

Fix those three and the command is fine.

## `cd`, `pwd`, `ls` are shell builtins, not programs

The same category as `caps`, which already prints the shell's whole endowment: they spawn nothing,
need no grant, and confer no new authority, because the shell is reading and rebinding what it already
holds. This also retires a worry raised while designing `ls`: that a listing program would be
over-granted, holding the power to read everything it lists. It is not a program.

The cwd stops at the process boundary. `wc report.txt` resolves the name against the
shell's current directory *at the moment the grant is made*, and the child receives a capability to
that one file. The child has no cwd, inherits nothing, and cannot re-resolve anything. The convenience
is the shell's; the authority is explicit.

## The earned divergences: three, then two

- ~~No global absolute paths.~~ Retired 2026-08-18, and it was never a position: it was the
  honest state of a system that had no namespace to root a path in. See "Absolute paths: Plan 9's
  answer, not DOS's" in [absolute-paths-and-bind.md](0047-navigation-and-naming/absolute-paths-and-bind.md),
  which is now built. The `InvalidFilename` refusal it describes survives
  for the two cases that still name nothing, `..` and a Windows-shaped prefix.
- `..` stops at your root. You descend from what you hold and never ascend past it. This is
  chroot's shape arrived at from the other direction.
- `pwd` is relative to your root, because naming anything above it implies a namespace that does
  not exist.

What that buys, and Unix cannot: every shell has its own root. Two shells can hold different
subtrees and neither can name the other's files, not by policy but because no capability reaching them
exists.


## The finding that should drive the build order

`cd`, `mkdir`, and per-process namespaces each converge on the same missing primitive: a verb that
returns a directory capability rather than bytes. It would be the first place this contract hands
back authority instead of data, and it deserves the care `Endpoint::REAP` got (§32): what rights does
the child directory carry, can they ever exceed the parent's, and who may call it. Build that first;
the commands are the easy part once it exists.

Sequencing. After milestone 37 (RedoxFS's crash consistency), which owns the FS server's block path. Effort: 2 lanes
estimated, and the second estimate proved low: the namespace lane of 2026-08-18 spent itself on
absolute paths alone and left environment, `PATH` and `bind` untouched, the last of those blocked on
an endowment question rather than on effort (one for the descend/create verb and the builtins, one for namespaces), noting that
estimates for unbuilt work are guesses on a scale calibrated from history, not measurements.
## Follow-on

- **Done.** The shell's heap: fallible, capped at 32 KiB, and no substituted value re-split
  (notes/heap.md).
- **Done.** The sequencing question this block leaves open, that milestone 122 (`OPENDIR` reaches
  the PAL) is `NOT-STARTED` and some of the namespace half may want it, is stale: 122 is BUILT
  2026-08-18 (pull request #320), both options, proven on both ISAs by `std_exerciser`. Nothing
  here is waiting on it.
- **Done.** "The `caps` half of that recommendation is not built" was true when written and is not
  now. `crates/swish/src/lib.rs` prints a bound name's own row and writes it, with a test asserting
  `bind recent -> /logs/2026`. The block's own `bind` paragraph (now in
  [status-history.md](0047-navigation-and-naming/status-history.md)) already says so; this sentence
  never got the correction.
- **Done.** Tab completion, built 2026-09-26 under §227 option D: see "Completion: built
  2026-09-26" in [path-and-completion.md](0047-navigation-and-naming/path-and-completion.md) and
  notes/shell-line-editing.md.
- **Milestone 657.** Milestone 657 (a live history search at the prompt). fish's extras, recorded rather than built: suggestions from history
  (`design/roadmap/0679-the-prompt-suggests-from-history.md`), coloring the first word by
  whether it can run (`design/roadmap/0678-the-prompt-colours-what-it-can-name.md`), a live
  `^R` search (`design/roadmap/0657-a-live-history-search-at-the-prompt.md`), and argument
  completion from the manifest (`design/roadmap/0662-argument-completion-reads-the-manifest.md`).
- **Done.** `PATH`, built 2026-09-26 as §229 B2: a bare word resolves through the live
  activation set (`activation_set::lookup_name`, which skips owner vouches), a name both the image
  and a package have is refused naming both, and install refuses a name another package provides
  (`NameTaken`) or the image carries (`ImageName`, calef's ruling of 2026-09-27). An owner's vouch
  now sits beside a package's entry instead of replacing it. Proven by host tests and by
  `script/swish-check` (`greeting` and `noteless` by bare name, `package install
  downloads/uptime.nifepkg` refused, `unvouched` reaching nothing). The prompt's both-names refusal
  is host-tested only, since no gate can now make the pair (`swish::bare`'s BUGS).
- **Milestone 65.** Environment's secrets third, moved rather than built. A secret here is an
  endpoint (§41), the service that holds one is milestone 65's (BUILT), and where a stored secret comes from is §165 (PROPOSED). §220
  (signed builds) met the one candidate customer, a signing key, on 2026-09-26 and recommended
  building no signer on nife until a build made on nife must run elsewhere. There is nothing for a
  navigation milestone to build here. Checked 2026-09-26.
- **Done.** Built 2026-09-26: printing inert-config values in `caps`. The progenitor places the
  frame it endows children with in the boot shell, `READ` only, at `grant_plan::SHELL_CONFIG_SLOT` and maps
  it at `SHELL_CONFIG_VA` (both provisional), and `caps printenv` prints the three values from it.
  Host-tested in `crates/swish`, proven at a real prompt by `script/swish-check`. A shell choosing a
  *different* set per child is not built and has no customer; it and a `login` session's blind
  preview are in notes/env-config.md's BUGS.
- **Milestone 154.** `bind` in a two-grant shell stays host-tested only. Both real entry points
  still pass no second directory (`components/src/swish.rs`, `user/src/system_initializer.rs`), and
  `crates/system_initializer` calls the path unverified against a real boot.
- **Done.** The set grant at the prompt: ruled 2b in §228 and option A by calef on 2026-09-27
  (notes/a-set-grant-at-the-prompt.md), built by milestone 205. A pattern matching two or more
  names reaches the progenitor as one read-only frame (`spawnproto::NAMESET_BIT`) and the
  progenitor builds `fs_nameset_caretaker` over a copy of it. Verified 2026-10-03.
- **Refused.** The two symlink questions this block leaves open, what a stored `..` means to a
  shallower holder and the `rm -r link/` trailing slash, are moot:
  `design/decisions/0050-namespace-composition.md` chose composition over stored paths, so there is
  no stored path to clamp or to slash.
- **Decision.** Hard links were considered and declined, written up as
  `design/decisions/0110-hard-links-declined.md`: no customer, `RENAME` already covers the
  atomic-replace idiom, and a DAG would cost an audit of every subtree argument in the tree.
- **Refused.** The function-call shell syntax fork calef raised 2026-07-30: DECISIONS §141
  (application is grant), 2026-09-03, recorded the model and refused each notation with a measured
  reason. The line above said it lived nowhere else; it has since 2026-09-03.

## Index row

divergence from Unix must be earned, never stylistic. Keep the commands; change only what the
capability model actually forces, and get one missing primitive right
