# Milestone 47 appendix: `mkdir`, `rm`, `rmdir` and `rm -r`

An appendix to [milestone 47 (navigation and naming)](../0047-navigation-and-naming.md), moved out of
that page on 2026-10-13 (UTC) under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md).
The main page is complete without it. It holds the design of `mkdir` and `rm`, and the removal
decision that shipped on 2026-07-31. Long sentences were split as they moved; no figure, date, name
or quotation was dropped.

## `mkdir` and `rm`

`mkdir` is the same verb family as descending: it mints a directory node and hands back a
capability to it, exactly as `CREATE` already returns a file handle. `mkdir` is descend-with-creation,
and the two should be designed together rather than separately.

`rm` is where Unix conflated two operations. `rm` unlinks a name. The data survives while anyone
holds a descriptor, and the blocks survive after that. So it cannot promise what people mean when
they delete something sensitive (and `shred` only pretends to on a copy-on-write filesystem like
ours). Separate them:

- Unlink: remove a name from a directory; existing capability holders keep reading. Unix's
  semantics, and genuinely useful (atomic replace and the temp-file idiom both depend on it).
- Revoke: the object dies and *every* capability to it goes stale.

The second is not exotic here: §13 (capability revocation and untyped reclamation) revokes frames,
§16 (object revocation) revokes objects, and generational names (`crates/slots`) make a stale
capability fail safely rather than point somewhere wrong. One implementation caveat to design rather
than gloss: the FS server validates handles against its own table, so invalidating them is
mechanically easy, but that table is per-session and the server does not track all outstanding
sessions today.

The rights ladder becomes explicit: a directory capability needs separable enumerate, open
(read versus write), create and remove. A program handed a directory to write logs into should
not thereby be able to delete what is there. `FileSpec` already makes this split for files, where the
manifest declares direction and the human designates the file without typing a mode.

And one safety property falls out free. `rm -rf /` is bounded here by what your directory
capability reaches, structurally. A shell rooted at a subtree cannot recursively delete the system,
because no capability naming those files exists in it. Not a guard rail, not a confirmation prompt,
not a check that could be wrong: there is nothing to name.

## `rmdir` and `rm -r`: Unix already made the safe choice (decided and built 2026-07-31)

### Built 2026-07-31, and the annotation added 2026-08-22. See DECISIONS §49 and notes/rm.md.

The code, the decision and the concept note have all said "Built" since the day this section was
written; this roadmap block did not, and that gap outlived the design it describes by three weeks.
The section below is kept as written, because the reasoning is the reasoning that shipped; treat it
as history rather than as an open question. §49 (removal is a directory operation) is the decision.

`mkdir` shipped in §48 (navigation is the shell rebinding what it holds) with no way to remove what
it makes: `rm` answers `EISDIR` and there is no `RMDIR`. The lane declined to add one, on the
grounds that "a verb that removes whatever it finds is how one word takes a subtree away". That
objection is right about a *recursive* verb and does not apply to Unix's, which is the point.

`rmdir(2)` removes only an empty directory, and that is the whole safety property. The recursion
in `rm -r` lives in userspace, as a loop of individually safe single-step operations: walk, unlink
files, remove empty directories bottom-up. No single call in the contract can take a subtree away.

So: `RMDIR` requires `REMOVE` on the parent and refuses non-empty with `ENOTEMPTY`. It is explicitly
not revocation, for §48's reason: the handle table is per server, so handles cannot be invalidated
for clients the server cannot enumerate.

The recursion is bounded by construction, which Unix cannot say. `rm -r` needs `ENUMERATE` to see,
`DESCEND` to recurse and `REMOVE` to delete, *at every level*, so the walk stops exactly where the
capabilities stop. Unix bounds `rm -rf /` with a permission check per file, which is a check that can
be wrong and famously has been. This milestone's existing note stands: not a guard rail, not a
confirmation prompt, "there is nothing to name".

`rm` is a program, not a builtin, and that is Unix's shape rather than a divergence from it.
`cd`/`pwd`/`ls` are builtins here because the shell is rebinding what it already holds; `rm -r` is a
destructive loop, not a rebinding. A builtin would run with the shell's entire endowment, while a
program takes an explicit attenuated grant. So `caps rm -r logs/` prints the subtree at risk before
anything happens, and a bug in the recursion can only reach what it was handed. Same shape as
globbing ([touch-and-globbing.md](touch-and-globbing.md)): attenuate, then hand over.

`-f` stays, with Unix's semantics (calef, 2026-07-31). An earlier draft of this section argued it
should not exist, on the reasoning that with no prompting its only remaining meaning is suppressing
errors, which §42 (a filesystem declares what it offers) forbids. That was wrong about what `-f`
does. It means *ignore nonexistent files and do not prompt*: a permission failure on a file that
exists still reports. Its real value is idempotency: `rm -f maybe-there` succeeding is what makes a
script re-runnable, and "absence is the desired state" is not a lie about failure. The divergence did
not earn its keep.

Reporting is Unix's, and it is quieter than an earlier draft of this section claimed. Checked
against `rm(1)` rather than remembered: silence on success, `-v` exists precisely because the
default prints nothing ("be verbose when deleting files, showing them as they are removed"). Failure
is a diagnostic plus exit status: "exits 0 if all of the named files or file hierarchies were
removed… If an error occurs, rm exits with a value >0." So a partial `rm -r` says what it could not
do and exits non-zero, and says nothing about what it did. An earlier draft here said it should
"report what it removed", which is the `-v` behavior, not the default.

`-f` is also broader than that draft assumed: "attempt to remove the files without prompting for
confirmation, regardless of the file's permissions. If the file does not exist, do not display a
diagnostic message or modify the exit status." So it suppresses the missing-file diagnostic *and*
its effect on the exit status. The claim that a permission failure still reports under `-f` was wrong.

One thing to settle when building it. A `rm -r` interrupted halfway leaves a partial tree, and
there is no transaction spanning requests. Adding one would mean the server holding a transaction
open across receives. That conflicts with the serve-loop-runs-one-request-to-completion property
§47 (a directory capability carries six rights) relies on for concurrency atomicity. Partial, with
failures reported and a non-zero exit, is the answer, and it happens to be exactly what Unix already
does.

Worth noticing while copying Unix here: `rm(1)` says "it is an error to attempt to remove the
files `/`, `.` or `..`". That is a literal special-case guard for `/`, shipped in the utility,
precisely the "guard rail, a check that could be wrong" this milestone contrasts itself against. We
need no such case: a shell holding a subtree cannot name the root, so there is nothing to
special-case. And `rm` on a directory stays a refusal (`EISDIR`) rather than a silent escalation to
recursive removal. That is Unix's behavior, and worth keeping for the same reason `rmdir` is
empty-only.
