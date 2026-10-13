# Milestone 47 appendix: absolute paths, `bind`, and what forced the namespace half

An appendix to [milestone 47 (navigation and naming)](../0047-navigation-and-naming.md), moved out of
that page on 2026-10-13 (UTC) under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md).
The main page is complete without it. It holds the block's sections on absolute paths, on `bind`,
and on milestone 64 (enough `std` to run somebody else's crate) as the forcing customer. Long
sentences were split as they moved; no figure, date, name or quotation was dropped.

## Absolute paths: Plan 9's answer, not DOS's

### Built 2026-08-18. The recommendation below was taken, and it cost less than it priced.

`/` is the root of your own namespace. That holds in the shell (`grant_plan::nav::Path::from_root`,
`swish::Nav::walk_from`) and in the `std` PAL together. In the PAL it is `sys/fs/nife.rs`'s
`count_names`, where a leading `/` joins `.` as a component that names the base rather than a place.
That is what this block asked for when it said the resolution fork should be one fork answered once.

The resolver is the client's, as recommended, and it turned out to already be there. A grant
records a *position* rather than a token (`grant_plan::designate` resolves once, at plan time).
`swish::open_at` re-walks that position from the root at run time. So rooting a token at
`Cwd::root()` instead of at the shell's position was the whole of the change on the planning side.
The server still sees a single component against a handle it was given, and §27 (the filesystem
service) is untouched.

Four things this section did not predict.

- The forcing case was `pwd`, not a program. `Cwd::render` has printed `/logs/2026` since the day
  it was written, because a position relative to your own root is the only honest rendering. Typing
  that token back was a refusal. A round trip that does not close is the promotion trigger of §71
  (a limitation is promoted when it stops being a fact and becomes a plan) met exactly, and it was
  sitting in the tree for eighteen days.
- It grants nothing, and that is measurable rather than arguable. `/a/b` is `cd` to your root
  followed by two descents. The guest suite asks the same two probes with and without the slash from
  two shells rooted in two subtrees (`navscape::ABSOLUTE_REACHED_INNER` / `ABSOLUTE_REACHED_SECRET`).
  Each reaches exactly the file its own root holds. `/..` is refused exactly as `..` at the root
  is, because your root is the only root there is.
- The `std` half is smaller than Plan 9's and should be described that way. A nife process holds
  one directory capability, so the slash selects nothing: there is nothing else to select. What
  it buys is that `current_dir()` can answer (`/`, and `Unsupported` for a process that holds no
  directory). It also buys that `temp_dir()` and `current_dir()` finally name the same place, and
  that a crate which builds a path from `current_dir().join(..)` gets a path that resolves.
- A `Dir` handle is its own root, which is a deliberate divergence from `openat`. POSIX makes an
  absolute path ignore the `dirfd`; here `Dir::open_file("/x")` resolves under that `Dir`. The Unix
  rule exists because `/` names one global thing and a `dirfd` is a shortcut into it. Neither half
  is true here. It cannot widen anything either way, since a process holding a `Dir` holds the root
  it descended from.

The honest cost this section priced is unpaid so far. It should be watched rather than declared
avoided: "two processes seeing different files at one path is powerful and confusing". With one
capability per process the confusion has nowhere to live yet. It arrives with `bind`.

Distinguish a path as *authority* (`open()` resolving against a namespace nobody granted you: out
permanently) from a path as a *name* (a string, and a name is not a capability). The syntax can
survive even though the semantics cannot.

Plan 9 kept absolute paths and made `/` the root of *your* namespace, assembled from what you were
given. So two processes can both open `/lib/foo` and get different files. That is the
counter-example to gratuitous divergence: the system that took namespaces furthest did not abolish
paths, it made them personal. It also lines up with "every shell has its own root" in [the main
page](../0047-navigation-and-naming.md)'s earned divergences, which is not a coincidence.

The real decision is where the resolver lives, and it changes the security story:

- *In the FS server*: it accepts multi-component paths and walks them. Workable, but it puts
  path-walking back into a server. That is against §27's discipline that open-by-path exists only
  inside the server relative to one bound directory.
- *In the client's runtime* (`user_rt`): a small table of prefix to directory capability, granted at
  spawn, resolved locally and privately. The server still only ever sees a single-component name
  relative to a capability presented to it, leaving §27 (the filesystem service) intact.

Recommendation: the client's runtime. It yields absolute-looking paths with no server learning a
name it did not already own. The namespace becomes another endowment, inspectable in `caps`, which
Unix cannot do, since you cannot enumerate what your paths could reach. The honest cost is that two
processes seeing different files at one path is powerful and confusing, and Plan 9 users will attest
to both halves.

The `caps` half of that recommendation is not built, and it is worth saying why it would be
empty. A namespace with one root has one row, which `caps` already prints as the directory grant. It
becomes a real surface the moment there is more than one entry, which is `bind`'s question below.

## `bind` is not blocked on a mount table. It is blocked on a second grant (found 2026-08-18)

### Built 2026-08-26 (`milestone/47-namespace-remainder`), against the real mechanism. See the Status block for what it does and does not reach.

The Status block named here is now in [status-history.md](status-history.md). The section below is
kept as written. The sketch it ends on ("`bind` then falls out as a name on a `Cwd` per entry, and
`caps` gains a namespace section with more than one row in it") is exactly what got built once
milestone 154 (a process that holds two directory capabilities) supplied the second grant, checked
rather than assumed. What changed in the checking: the sketch's "per entry" turned out to mean a
*table* of entries rather than one (`nav::Bindings`, up to four, provisional). A shell with one
grant still wants to file more than one shortcut. The table composes with `nav::TwoRoots`'s two
fixed labels rather than replacing them (a grant label always wins, so a bind can never shadow `a`
or `b`). `caps` now prints a bound name's own row (`bind <name> -> <real position>`). It sits beside
the two-grant namespace section 154 already built rather than folded into it, since a bind is
additive to whatever grant rows already print.

DECISIONS §50 (namespace composition, `bind`, not stored paths) chose namespace composition over
stored paths and priced the unbuilt half as "a mount table per process and resolution through it.
That is real work". Building absolute paths priced it again, from inside, and the mount table is the
cheap half.

- A bind entry is a value, not a capability. Everything downstream of the shell's planner already
  re-walks a *position* from the root (`swish::open_at`). So a bind is a `nav::Cwd` under a name: no
  cspace slot, no handle to leak, no lifetime. And it cannot name above the root by construction
  rather than by a check. `Cwd` has exactly two constructors, `root()` and a `descend` that refuses a
  bad component, and `ascend` returns false at depth zero. That is the ladder's first rung, and it is
  why this half warrants no proof harness: a Kani harness would restate the type.
- What is missing is something to bind. A shell holds one directory capability, so a namespace
  assembled from what it holds has exactly one member and every bind is an alias inside one tree.
  The interesting case, and the only one that pays for the mechanism, is a union of two grants:
  `/photos` from one caretaker and `/backups` from another, in one process, with neither able to
  name the other's parent.
- Nothing in this system grants a second directory capability to one process.
  `fs_service::start_granted_dir` starts one caretaker and hands one endpoint. A second means a
  second caretaker, a second slot, and a spawn-protocol position to say which is which. That is an
  endowment question, which is the category this milestone's own environment section
  ([environment.md](environment.md)) says is expensive, and it is where the work actually is.

Minted as milestone 154 on 2026-08-23. The deliverable is the wiring plus the negative control that
only a union can state: one process, two subtrees, `/a/x` and `/b/y` both resolving, `/a/../b`
refused, and neither caretaker able to see the other's tree. `bind` then falls out as a name on a
`Cwd` per entry, and `caps` gains a namespace section with more than one row in it. Until that
exists, building the mount table would be machinery whose one interesting case is missing.

## Milestone 64 is what forces the namespace half of this milestone

Recorded here as well as in 64, so neither is picked up without it.

What remains open in this milestone is the namespace machinery: ~~absolute paths~~ (built
2026-08-18), environment variables, `PATH`, and `bind`. The real blocker of `bind` is the section
above rather than the mount table §50 named.

None of it has a forcing use case from the shell. `swish` works with per-shell roots; `bind` is
a mechanism nobody currently has to have. That is why this milestone has sat IN-PROGRESS with its
navigation half done and its namespace half designed.

Milestone 64 supplies the missing demand. `std::fs::File::open` takes a path, and a `std`
program is not a shell: it cannot be handed a root and told to `cd`. A crate that writes
`Path::new("assets").join("x.png")` is a concrete request for per-process namespace resolution,
which is exactly what `bind` is for. The `PATH` conclusion in
[path-and-completion.md](path-and-completion.md), that a program namespace is an endowment, gets its
first real customer at the same moment.

The sequencing this implies runs the other way from the obvious: let 64 measure first. Its probe
crates will report what a real dependency actually needs. That evidence is what this milestone's
remaining scope should be sized against, rather than building the general namespace and hoping it
fits. `File::open`'s resolution is then one fork answered once, spanning both milestones, instead of
a PAL trick here and a design there.
