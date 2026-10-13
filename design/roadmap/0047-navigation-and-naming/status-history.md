# Milestone 47 appendix: the status record, as it accumulated

An appendix to [milestone 47 (navigation and naming)](../0047-navigation-and-naming.md), moved out of
that page on 2026-10-13 (UTC) under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md).
The main page is complete without it. It holds the block's status paragraphs as each lane wrote
them, in the order they were written, so the dated record survives the page's shorter summary. Long
sentences were split as they moved; no figure, date, name or quotation was dropped.

The sections these paragraphs refer to now live in the sibling appendices:
[mkdir-and-rm.md](mkdir-and-rm.md), [links-and-bind.md](links-and-bind.md),
[touch-and-globbing.md](touch-and-globbing.md), [absolute-paths-and-bind.md](absolute-paths-and-bind.md),
[environment.md](environment.md), [environment-built.md](environment-built.md),
[path-and-completion.md](path-and-completion.md) and [shell-grammar.md](shell-grammar.md).

## The status token and the discharge

The token read `IN-PROGRESS` with no branch anywhere and nobody holding it, found
2026-08-17 by the status-accuracy sweep. Two claims in the "still to do" sentence below were also
false, and both are corrected there rather than here. The glob caretaker was built 2026-07-31, and
the `std` PAL's three namespace verbs were bound 2026-08-04.

Discharged 2026-08-18. The navigation half is built. The namespace half (absolute paths,
environment, `PATH`, and `bind`) had no forcing use case from the shell. This block's own sequencing
was to let milestone 64 (enough `std` to run somebody else's crate) measure first, so a real crate's
demands could size the remaining scope. That measurement has landed and it did its job. So the gate
it was waiting for is discharged rather than merely aged.

## Where it stood on 2026-10-03

Where this stands, 2026-10-03 (UTC): every item below is Done, refused or routed to a numbered
milestone, so the block is `BUILT`. Re-checked against the tree that day. The last open item, the
set grant at the prompt, was built after the 2026-09-26 check. calef ruled it 2b in §228 (how a set
of matched names reaches the progenitor) and then option A on 2026-09-27
(notes/a-set-grant-at-the-prompt.md). Milestone 205 (how a foreign program is told what to do)
built it. `spawnproto::NAMESET_BIT` is sent by `components/src/swish.rs` and read by
`crates/system_initializer`, which builds `components/src/fs_nameset_caretaker.rs`. The race the
note records is closed by milestone 599 (a frame per filesystem client channel).

Built 2026-09-26: `caps` prints the inert-configuration values a child will read. It reads them from
the boot shell's own read-only view of the same frame (DECISIONS §111 (inert configuration is a
validated page)'s preview; notes/env-config.md). Completion came once calef ruled §227 (how Tab
reaches the shell) as option D. The shell edits its own line over raw mode, and Tab finishes a
program or file name from its own authority (notes/shell-line-editing.md). `PATH` for installed
programs was ruled B2 in §229 (how a bare name at the prompt reaches an installed program): the
activation set, never an owner's vouch (`swish::bare`, notes/packages.md). The function-call syntax
fork was refused by §141 (application is grant) on 2026-09-03.

The secrets third moved out of this milestone. It is §41 (the endpoint is the broker)'s endpoint,
milestone 65 (a secrets service)'s service and §165 (where a stored secret comes from)'s open
question, and no program on nife needs one (Follow-on, on the main page).

## The pieces as they were built

Absolute paths were built 2026-08-18 (`milestone/47-namespace`). They were the first piece of the
namespace half and the one the other three lean on. `/` is the root of your own namespace, Plan 9's
answer, in the shell and in the `std` PAL together, so that one fork was answered once.

Environment variables' inert third was built 2026-08-23 (`milestone/47-env-config-page`), the
second piece of the namespace half. It is a read-only page carrying `TZ`, `LANG` and `TERM`, each
validated against a closed domain before it is ever written (DECISIONS §111 (inert)). See
[environment.md](environment.md) and notes/env-config.md. What is built is end to end for a std
program (`env_proto`, the kernel-side assembly and mapping, the `std` PAL's `sys/env::seed`). It is
proven by `std_exerciser` on both ISAs. What remains is a shell-facing customer to declare wanting it.
No `grant_plan::Manifest` field exists yet, the position `clock` was in before `date`. That customer
is what would also make the `caps` preview extension §111 asks for meaningful. The other two thirds
of what Unix calls "environment" stay where this milestone always put them. Names (`PATH`, `HOME`)
wait on `bind`, and secrets are answered elsewhere by an endpoint (§41).

That shell-facing customer was built 2026-08-26 (`milestone/47-remainder-round2`): `printenv`
and `grant_plan::Manifest::config`. They are wired into real init on both boards, not only the
kernel test harness that stood in for it above. See "The shell-facing customer was built 2026-08-26"
in [environment-built.md](environment-built.md).

`bind` was built 2026-08-26 (`milestone/47-namespace-remainder`). It closed the gap this block's own
"blocked on a second grant" section named: milestone 154 (a process that holds two directory
capabilities) supplied the second grant. This lane found the doc's own sketch ("a bind entry is a
value, not a capability... a `nav::Cwd` under a name") held up against the real, now-built mechanism
without change. `crates/grant_plan::nav::Bindings` (provisional name) is a small fixed table, up to
four entries. It maps a name to a `(Which, Cwd)` position this shell reached some other way.
`Holdings::resolve` tries a grant label first (when this shell holds two), then the bind table. For
the one-grant shell every real boot is today, it then does the same literal walk from the sole root
it always did. So `bind` is additive, and a shell with nothing bound resolves byte for byte as
before. `Holdings::bind` is the mutator. It refuses a name that collides with a grant label, in
addition to everything the table itself refuses (bound, full, unnameable).

It is wired into the real interactive `swish` as a builtin, `bind <target> <name>`, in `mkdir`'s
category (mints no capability, spawns nothing). `target` resolves through the same bind-aware
`plan_path`/`walk` every other verb now shares. So binding a name under an existing bind composes
rather than needing a special case. And `..` past a bound name climbs the *real* tree the bind
points at, rather than stopping at a boundary invented at the alias. That is the same "misdirect,
never grant more" property §50 (namespace) gives symlinks-as-bind. Proven over the real wire, both
ISAs, extending `kernel::user::shell_navigation_tests`' navigating witness. A bound name lists the
real directory it points at (`ls`). Three `cd ..`s from inside it reach the real parent, the real
root, and refuse there exactly where a direct walk to that depth would.

What this increment does not reach, honestly. `bind` in a *two-grant* shell (composing two
disjoint trees under more names than the two grant labels) is host-tested in `grant_plan` but not
guest-provable. Milestone 154's own gap is still open: nothing tells a real, live `swish` process it
holds a second directory capability at all (`_start`'s three `START` words are spoken for). Both
real init entry points still pass `second_dir: None`. That gap is 154's, not this lane's to close.
It is a spawn-protocol wire decision 154 itself declined to make "under this lane's time pressure," as its block puts it.
And what the second directory should even *be* is an architect's boot-time policy call. DECISIONS
§126 (moving `cwd`) made the harder judgment call next to it, the real single moving cwd, without
answering this one. This lane did not invent an answer to either.

`ln`'s symlink half is retired from "still to do" rather than built as a separate command.
DECISIONS §50 settled that symlinks-as-stored-paths are superseded by `bind`, not implemented beside
it. So `bind` landing *is* that half of `ln` landing, under Plan 9's name rather than Unix's. Hard
links stay declined (DECISIONS §110 (hard), want of a customer); see the `ln` section in
[links-and-bind.md](links-and-bind.md), kept as history, with this line added rather than rewritten.

Completion and `PATH` were investigated on 2026-08-26 and neither was built. Each turned out to
have a real fork underneath, and their own sections carry the findings and the options
([path-and-completion.md](path-and-completion.md)). They were cut from here on 2026-09-26, where they
were repeated word for word in substance.

## Corrections the status checks found

`RMDIR` and `rm -r` were also already built, found 2026-08-22 by the same kind of status check
that caught the `IN-PROGRESS` token above. The code (`components/src/rm.rs`, `fs_proto::fs::RMDIR`),
the decision (`DECISIONS §49`) and the concept note (notes/rm.md) all say `Built 2026-07-31`. But
this roadmap block never got the matching annotation. So the "rmdir and rm -r" section
([mkdir-and-rm.md](mkdir-and-rm.md)) read as still-undecided years after the design it describes
shipped. Recorded here rather than left for the next reader to rediscover in the git log. §49
(removal is a directory operation) is that decision.

`touch` is fully built. The create half landed 2026-08-22: `touch <name>` makes an empty file if
the name is not there and does nothing if it is. It uses `fs_proto::fs::CREATE` (already built for
milestone 31 (a capability shell) phase 2) through the same shell-builtin shape `mkdir` already has.
The mtime half landed 2026-08-24, once DECISIONS §112 (`touch`'s two behaviors need two rights)
settled the authority question. Bare `touch` bumps to now (`fs::SETMTIME`, needs only `dir::WRITE`).
`touch -t <RFC-3339-instant>` asserts an arbitrary one (`fs::SETMTIME_AT`, needs
`dir::WRITE | dir::SETTIME`, the seventh rung on the directory rights ladder). See
[touch-and-globbing.md](touch-and-globbing.md) and notes/touch.md.

`bind` was built 2026-08-26, and it is the mechanism environment's "names" third pointed at
("`PATH`... and `HOME`... wait on `bind`"). But it does not by itself close that third. `bind`
files a *directory or file position* under a name, which is the whole of what `HOME` ever was (a
directory capability wearing a string costume). Nothing consumes it specially yet: no `cd` with no
args reads a bind the way Unix reads `$HOME`. `PATH` is the harder half of "names" and stays open
for the deeper reason in [path-and-completion.md](path-and-completion.md). What remained is recorded
under "Where this stands" above. `ln`'s symlink half is retired rather than unbuilt. DECISIONS §50
(namespace composition) already settled that symlinks-as-stored-paths are superseded by `bind`, not
built beside it, so `bind` landing is that half landing. Hard links stay declined for want of a
customer (DECISIONS §110).

## Milestone 64's second pass

64's second pass (2026-08-18) reports that nothing in this milestone was ever waiting on 64. It
hands over the sized demand this block asked for: named customers at ranks 16, 18 and 27, plus
path-joining. It also hands over one 64 did not expect. A *seeded* environment has to arrive from
this milestone's endowment, because `std::env` on nife starts empty by construction and 16 direct
consumers want otherwise. `env::var` was rank 4 with no PAL at all. The shape of its absence is the
warning worth carrying here. `getenv` answered `None` harmlessly while the same fallback's `env()`
was `panic!`. So `std::env::vars()` aborted the process and compiled perfectly. The dangerous refusal
is the one that answers.

## The sequencing question

One sequencing question is open and deliberately not made a gate. Milestone 122 (`OPENDIR` reaches
the PAL) is `NOT-STARTED`, and some of the namespace half may want it. The 2026-08-17 status sweep
raised that and declined to rule, and this line does not rule either. Recording it as `NONE` with
the question named is the honest state. A lane can start, and the first thing it should establish is
whether its piece needs 122. Turning an unestablished dependency into a gate is how a milestone sits
blocked on nothing. That is what the twelve days behind this block's own corrected sentence cost.

## In brief, as it stood

In brief. A navigation model for a system with no global namespace. Keep the Unix command names
and behavior wherever they can work honestly; diverge only where the capability model forces it, and
say why each divergence is earned. The keystone is built: the directory capability and its rights
ladder, six rungs at the keystone and a seventh added by DECISIONS §112 for `touch -t` (DECISIONS
§47 (the directory capability's rights), notes/dir-capability.md). So are the five commands, on
both ISAs: `cd`, `pwd`, `ls`, `mkdir` and `rm` as shell builtins. `..` is clamped at your root by
popping the stack of capabilities the shell descended through, and `pwd` is relative to that root. A
name on a command line is resolved against the shell's position at the moment the grant is made, so
a child holds a capability to one file and cannot re-resolve anything. `rm` is `UNLINK`, added to
`fs_proto` here and separated from revocation in the contract's own words. Revocation is not offered,
because the FS server's handle table is per *server* and it cannot enumerate the clients holding
handles. The headline is proven with the real shell binary: two shells rooted in two subtrees, each
told nothing about which it holds, and neither can name the other's files (notes/shell-navigation.md).

The glob caretaker is built too, which this sentence claimed as outstanding for seventeen days. It
was built 2026-07-31 (merge `5e48826c`, branch `milestone/47-glob-wiring`, wiring commit `00f4e277`;
no pull request, it predates the workflow). It is proven end to end on both ISAs by
`what_a_shell_shows_is_what_a_set_grant_takes_away` (`kernel/src/user/glob_grant_tests.rs:29`, no
arch `cfg` at `kernel/src/user.rs:2259`). That test runs the real `swish` binary expanding one pattern
two ways. It runs the real `rm` binary behind a real `fs_nameset_caretaker` holding only `REMOVE`
over the two matched names. `rm` is then pointed at a name one directory entry outside the set and
gets `ENOENT`. Witnessed from the host by `xtask::redoxfs_glob_grant_took_exactly_the_match`
(`xtask/src/main.rs:4360`). See notes/glob-grant.md, which has said "Built 2026-07-31" the whole
time. See also "Built 2026-07-31: the matcher, then the grant" in
[touch-and-globbing.md](touch-and-globbing.md), which contradicted the sentence from inside this same
file.

Still to do: see "Where this stands" above, which replaced a longer list here that had drifted from
the sections below it. The `std` PAL's `rename`, `unlink` and `rmdir` were bound by milestone 64
(enough `std` to run somebody else's crate) on 2026-08-04 (pull request #113). They answer
`Unsupported` only when the process holds no filesystem capability at all.
