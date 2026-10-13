# Milestone 47 appendix: absolute paths and `bind`

An appendix to [milestone 47 (navigation and naming)](../0047-navigation-and-naming.md). The page is
complete without it. It holds how absolute paths resolve and how `bind` composes a namespace.

## Absolute paths: Plan 9's answer, not DOS's

`/` is the root of your own namespace. Plan 9 kept absolute paths and made `/` the root of *your*
namespace, assembled from what you were given, so two processes can both open `/lib/foo` and get
different files. The system that took namespaces furthest did not abolish paths, it made them
personal. That lines up with "every shell has its own root" in [the main
page](../0047-navigation-and-naming.md)'s earned divergences.

A path as *authority* (`open()` resolving against a namespace nobody granted you) is out
permanently. A path as a *name* (a string, and a name is not a capability) survives.

The resolver lives in the client, not the FS server. A server that walked multi-component paths
would put path-walking back into a server, against the discipline of §27 (the filesystem service)
that open-by-path exists only inside the server relative to one bound directory. In the client, the
server still only ever sees a single-component name relative to a capability presented to it, and
no server learns a name it did not already own. The namespace is another endowment, inspectable in
`caps`, which Unix cannot offer, since you cannot enumerate what your paths could reach.

How it works:

- In the shell, `grant_plan::nav::Path::from_root` and `swish::Nav::walk_from`. A grant records a
  *position* rather than a token (`grant_plan::designate` resolves once, at plan time), and
  `swish::open_at` re-walks that position from the root at run time.
- In the `std` PAL, `sys/fs/nife.rs`'s `count_names`, where a leading `/` joins `.` as a component
  that names the base rather than a place.
- `pwd` prints a position relative to your own root (`Cwd::render`, `/logs/2026`), and typing that
  token back resolves.

The properties that follow:

- It grants nothing. `/a/b` is `cd` to your root followed by two descents. The guest suite asks the
  same two probes with and without the slash from two shells rooted in two subtrees
  (`navscape::ABSOLUTE_REACHED_INNER` / `ABSOLUTE_REACHED_SECRET`). Each reaches exactly the file
  its own root holds. `/..` is refused exactly as `..` at the root is, because your root is the only
  root there is.
- The `std` half is smaller than Plan 9's. A nife process holds one directory capability, so the
  slash selects nothing. It lets `current_dir()` answer (`/`, and `Unsupported` for a process that
  holds no directory). `temp_dir()` and `current_dir()` name the same place, and a crate that builds
  a path from `current_dir().join(..)` gets a path that resolves. That is what a `std` program of
  milestone 64 (enough `std` to run somebody else's crate) needs, since it cannot be handed a root
  and told to `cd`.
- A `Dir` handle is its own root, a deliberate divergence from `openat`. POSIX makes an absolute
  path ignore the `dirfd`; here `Dir::open_file("/x")` resolves under that `Dir`. The Unix rule
  exists because `/` names one global thing and a `dirfd` is a shortcut into it, and neither is true
  here. It cannot widen anything, since a process holding a `Dir` holds the root it descended from.

The cost is that two processes, or two shells with binds, see different files at one path. That is
powerful and confusing, and Plan 9 users will attest to both halves.

## `bind` is not blocked on a mount table. It is blocked on a second grant

`bind` is how §50 (namespace composition, `bind`, not stored paths) composes a namespace. The shell
builtin is `bind <target> <name>`, in `mkdir`'s category: it mints no capability and spawns nothing.

- A bind entry is a value, not a capability. Everything downstream of the planner re-walks a
  *position* from the root, so a bind is a `nav::Cwd` under a name: no cspace slot, no handle to
  leak, no lifetime. It cannot name above the root by construction rather than by a check. `Cwd` has
  exactly two constructors, `root()` and a `descend` that refuses a bad component, and `ascend`
  returns false at depth zero. That is the ladder's first rung, which is why it has no proof
  harness: a Kani harness would restate the type.
- `crates/grant_plan::nav::Bindings` (provisional name) is a fixed table of up to four entries,
  each mapping a name to a `(Which, Cwd)` position. It composes with `nav::TwoRoots`'s two fixed
  labels rather than replacing them: a grant label always wins, so a bind can never shadow `a` or
  `b`.
- `Holdings::resolve` tries a grant label first (when the shell holds two), then the bind table,
  then the literal walk from the sole root. So a shell with nothing bound resolves byte for byte as
  without `bind`. `Holdings::bind` refuses a name that collides with a grant label, and the table
  refuses a name already bound, a full table and an unnameable name.
- `target` resolves through the same bind-aware `plan_path`/`walk` every verb shares, so binding a
  name under an existing bind composes. `..` past a bound name climbs the *real* tree the bind points
  at, rather than stopping at a boundary invented at the alias: it can misdirect, never grant more.
- `caps` prints a bound name's own row (`bind <name> -> <real position>`), beside the two-grant
  namespace section of milestone 154 (a process that holds two directory capabilities).
- A bind files a directory or file position under a name, which is all `HOME` ever was. Nothing
  consumes one specially: `cd` with no arguments does not read a bind the way Unix reads `$HOME`.

`kernel::user::shell_navigation_tests` proves it over the real wire on both ISAs. A bound name lists
the real directory it points at (`ls`). Three `cd ..`s from inside it reach the real parent, the real
root, and refuse there exactly where a direct walk to that depth would.

The mount table is the cheap half. The case that pays for the mechanism is a union of two grants:
`/photos` from one caretaker and `/backups` from another, in one process, with neither able to name
the other's parent. With one grant, every bind is an alias inside one tree.

That case is host-tested in `grant_plan` but not guest-provable. Nothing tells a live `swish`
process it holds a second directory capability (`_start`'s three `START` words are spoken for), and
both real init entry points pass `second_dir: None`. `fs_service::start_granted_dir` starts one
caretaker and hands one endpoint; a second needs a second caretaker, a second slot and a
spawn-protocol position. That is milestone 154's open gap, an endowment question
([environment.md](environment.md) explains why that category is expensive), and what the second
directory should be is a boot-time policy call for an architect.
