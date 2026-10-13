# Milestone 47 appendix: `touch` and globbing

An appendix to [milestone 47 (navigation and naming)](../0047-navigation-and-naming.md), moved out of
that page on 2026-10-13 (UTC) under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md).
The main page is complete without it. It holds how both halves of `touch` were built, and how a glob
became a grant. Long sentences were split as they moved; no figure, date, name or quotation was
dropped.

## `touch`, both halves now built. See notes/touch.md.

### The create half was built 2026-08-22.

It splits the way `mv` and `rm` did, and the split held: creating an empty file if absent needed
nothing this milestone had not already built (`fs_proto::fs::CREATE`, milestone 31 (a capability
shell) phase 2). This section originally expected that half to reach for §49 (removal is a
directory operation)'s `DirSpec`, the same program grant `rm` takes. The reasoning was that it is "a
program granted the directory a name lives in". Building it found a cheaper answer. `touch` needs no
more than `CREATE` on the directory the shell already holds, which is `mkdir`'s right and not a new
grant. So it shipped as a builtin in `mkdir`'s category rather than a program in `rm`'s. `rm` needed
a program because `-r` is a destructive recursive walk that should not run with the shell's whole
endowment. `touch` recurses over nothing and destroys nothing, so the reason that moved `rm` out of
the builtin set does not apply here.

### The mtime half was built 2026-08-24, once the authority question was decided.

Updating the modification time of a name that is already there was not built for two days. The
open decision this section used to name (is "set to now" the write right already held, or a
separate authority) is exactly what stopped it, not effort. The create half took an afternoon once
the decision to split it was made, and the mtime half took about as long once DECISIONS §112
(`touch`'s two behaviors need two rights) settled the question. The reason it was expressible at
all: the `std` PAL records that "the server keeps an mtime but the contract does not carry one".
RedoxFS tracks it; `fs_proto` did not expose it. It does now: `fs::GETMTIME`, `fs::SETMTIME`,
`fs::SETMTIME_AT` (all three provisional names).

The justification for the old refusal had gone stale. `notes/std.md` refused file times partly
because "there is no wall clock to interpret it against anyway": true when written, and false since
milestone 51 (wall-clock time, the `date` command) landed the clock (§43 (reading the clock is a
page), RTC drivers on both ISAs, `date`). Same shape as §43's own untestability note, which milestone
47's `date` work disproved: a scope note outlives the condition that justified it.

The authority question was decided (calef, 2026-08-23, DECISIONS §112 (`touch`'s two rights)): no,
they are not the same right. `touch` does two different things to a timestamp: set it to *now*, and
`touch -t` set it to *whatever you say*. The second is the ability to lie about history, which
matters for anything reasoning from mtime, backups included. That is §43's asymmetry again (reading
harmless, setting an authority), one level down. Two independent precedents converge on the same
split. POSIX's own `utime()` requires only write permission to set the current time but ownership to
set an arbitrary one. And §43 itself already separates reading the clock (broadly grantable) from
setting it (a distinct, more tightly held authority). Plain `WRITE` covers "now". A new, separate
right, not folded into `WRITE`, covers "arbitrary". It is the same separable-rights-ladder pattern
this milestone already uses for `enumerate`/`open`/`create`/`remove`, now a seven-rung ladder, with
`dir::SETTIME` (provisional) the seventh: DECISIONS §47 (a directory capability carries six rights)
extended by §112.

Built to that spec, exactly. Three verbs rather than one with a flag, because
`filesystem_protocol::verb::TABLE` encodes one fixed rights requirement per opcode and the two halves
need different ones. `GETMTIME`/`SETMTIME` need `dir::READ`/`dir::WRITE` respectively, resolved
directly under a directory handle like `UNLINK` (neither opens what it acts on). `SETMTIME_AT` needs
`dir::WRITE | dir::SETTIME`, with the caller's asserted seconds riding in the second word, for
`TRUNCATE`'s reason for using `w1` over the length field. `touch -t` takes an RFC 3339 instant
(`date`'s own output format), not Unix's compact `[[CC]YY]MMDDhhmm[.ss]`. That is an earned
divergence: this tree already had an RFC 3339 parser (`calendar`) and no reason to build a second
date grammar for one flag; see notes/touch.md's `BUGS`.

Proven over the real wire, not only in `fs_server`'s host tests. Extending
`filesystem_protocol::verb::TABLE` past `STATFS` also closed a latent gap, which the extension itself
required fixing to stay contiguous. `SYNC` (milestone 55 (Time Machine)) had never been given a row,
so every caretaker refused it with `EINVAL` and a program confined to a subtree could never `SYNC`
through it. `kernel::user::shell_navigation_tests` now proves the two-right split against a real,
narrower-than-`dir::ALL` grant (`TOUCH_NOW_NEEDS_ONLY_WRITE` / `TOUCH_AT_REFUSED_WITHOUT_SETTIME`)
and the round trip against a real command line (`TOUCH_MTIME_ADVANCED` /
`TOUCH_AT_ROUND_TRIPPED`), on both ISAs.

## Globbing, which decides how every multi-file operation grants

### Built 2026-07-31: the matcher, then the grant. See notes/glob.md and notes/glob-grant.md.

The decided answer is implemented rather than revisited: `rm *.txt` grants a directory capability
attenuated to a name set, served by `components/src/fs_nameset_caretaker.rs`. Four things this section
did not predict, and one it did:

- It predicted the shape of the change to `grant_plan`, and that is exactly what happened.
  `plan_against` fills its slots by index now, and takes an `Expansion` keyed to that index,
  because the endowment is the set rather than the pattern. `DirGrant.name` became `DirGrant.names`,
  which is the finding in the type system: a literal operand is the set of one.
- The caretaker is a third one, not a generalization of `fs_file_caretaker` or a mode on
  `fs_subtree_caretaker`. `fs_file_caretaker` serves the *file* protocol, so teaching it a set
  would be writing a directory caretaker; and `fs_subtree_caretaker`'s design property is that it
  performs no checks at all, which a name filter (on seven name-taking verbs) would end. The
  grants also have different shapes: a name rides in registers, a set needs a frame.
- An empty match is a refusal, zsh's answer. The obvious argument for bash's pass-through was
  checked and is wrong. Nothing here refuses `*` in a component, so passing the pattern through
  builds a grant whose namespace is a name nobody has. That grant acquires a referent the moment
  somebody creates that file.
- `ARG_MAX` landed at eight names, set by a stack overflow rather than by reasoning. Sixteen was
  the number the argument produced; the shell ran off the bottom of its stack planning one grant,
  twice. Exceeding the bound is a loud refusal at the prompt, never a truncation.
- Qualifiers and `**` stayed out, for notes/glob.md's reasons, which are authority questions and
  not scheduling ones. `xargs` was not built when this was written, so the answer at the bound was a
  refusal. Milestone 109 (`xargs`: batching a grant too large to hand over) built it on 2026-08-04,
  as a shell prefix word rather than a program. The refusal is still what happens for `xargs
  <program>`, because the shell cannot yet ask init to mint a per-batch caretaker. That missing
  delegation chain is this milestone's, and 109's block says so.

Tests: `grant_plan` and `fs_proto` host suites; `kernel::user::glob_grant_tests` on both ISAs (a real
shell expanding one pattern two ways, then `rm` as its own attacker behind a real
`fs_nameset_caretaker`); and `xtask::redoxfs_glob_grant_took_exactly_the_match` reading the image
from outside the guest.

zsh's glob engine is the best thing in the shell (`**/*.rs`, and qualifiers: `*(.)` for regular
files, `*(om[1])` for newest, `*(Lm+1)` for over a megabyte). The mechanism is unremarkable here
because a glob is an enumeration, and the rights ladder already separates `enumerate` out
([mkdir-and-rm.md](mkdir-and-rm.md)). The fork is not how to match. It is what a match grants.

`rm *.txt` with five hundred hits, four candidate answers:

| Answer | Verdict |
|---|---|
| Grant 500 file capabilities | Honest, and it exhausts capability slots |
| Grant the directory plus a name list | Cheap, and it over-grants catastrophically: `rm` could touch anything in that directory, which is the thing this whole model refuses |
| Make `rm` a builtin so the shell deletes and nothing is granted | Dodges the question, and costs `rm` as a program |
| A directory capability attenuated to a name set | The principled one |

The last is a smaller change than it looks, and that is the finding. `fs_file_caretaker` today
serves "a namespace of exactly one name"; globbing generalizes it to a set of names. Same
caretaker, same `fs_proto` protocol above and below, wider namespace. Nothing new in the kernel,
and the attenuation stays checkable from outside the confined program exactly as it is today.

The property worth demonstrating: the expansion you see is the grant. `echo *.txt` prints
literally the authority that `rm *.txt` would transfer, because the matched set *is* the namespace
the caretaker will serve. Unix cannot make that claim, since `rm`'s authority never came from the
command line at all; the glob merely told it which of its existing powers to use.

Who expands. The shell, before planning the grant, which is also what Unix does, so there is no
divergence to earn. The structural consequence is that `grant_plan::plan` must see the expanded set rather
than the pattern, since the endowment is the set.

Two costs to design rather than gloss.

- Qualifiers are not free. `*(.)` and `*(om[1])` need type, mtime and size *per candidate*, so one
  `enumerate` becomes N `FSTAT` calls, and they need a read right beyond enumerate. Decide whether
  qualifiers are in scope at all before building the matcher around them.
- `ARG_MAX` becomes a capability limit rather than a buffer limit. Unix's "argument list too long"
  is why `xargs` exists; here the ceiling is that you cannot hand a child a hundred thousand
  capabilities. The same failure with a more honest cause, and it wants the same answer (batching),
  so `xargs` earns its place for a better reason than Unix had.

Completion shares this mechanism and should be designed with it, not after it: tab completion is
also an enumeration, so the completion menu is a rendering of your authority and cannot offer a path
no capability reaches.
