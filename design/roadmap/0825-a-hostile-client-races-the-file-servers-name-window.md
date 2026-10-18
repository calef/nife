---
status: BUILT
raised: 2026-10-07
built: 2026-10-15
milestone_dependencies: 800, 812
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 825. A hostile client races the file server's name window

*(The title and slug are provisional.)*

## In brief

A client names its files in a window it shares with `redoxfs_server`. Since milestone 812 (threads
in one address space) a confined client can run a second thread that rewrites that window. The claim
this milestone holds (calef's "Refused names" ruling, 2026-10-15 UTC): the server acts on no name
its own check refuses, even if the window changes after the check. The refused names are `check_component`'s:
`..`, `.`, a name with `/` or `\`, the attribute store's directory, and invalid UTF-8.

The server copies each path name out of the window once, before the check. The check and the resolve
that follows both read that copy, so a rewrite of the page reaches past neither.

## Why

calef ruled the refusal log's order on 2026-10-07 (UTC), option (i): this was the next probe. The
gap sat at the head of `redoxfs_server/src/dispatch.rs`. A name was checked in the client's window
and then resolved from it. A rewrite between the check and the resolve could turn a checked name
into a refused one, or put invalid UTF-8 under a `&str`. Claims 19 and 24 of
`notes/confinement-claims.md` rest on it. `name_resolver` copies a name out before judging it; the
file server now does too.

**A swapped name cannot leave the client's subtree, as a fact about RedoxFS.** The engine stores no
`..` entries, and its lookup is literal within one directory (`vendor/redoxfs`'s `find_node`). The
attribute store's directory sits only in the image root. So a swap reaches a refused name planted
inside the grant, or invalid UTF-8 under a `&str`. It does not reach a name outside the subtree.
That is why the held claim is the refused-name one, narrower than the escape the probe set out to
boot, and why the fix is defensive.

## The fix, in force

Every verb that resolves a name takes a `NameCopy` (`redoxfs_server/src/names.rs`), not a `&str`.
Its only constructor, `NameCopy::copy_in`, copies the bytes into the value's own buffer of at most
one block and validates UTF-8 on the copy. Its text is reachable only through accessors that run
`check_component` on the copy. So the arms in `redoxfs_server/src/dispatch.rs` must copy each name
out of the window, and each verb must check it before resolving it.

The verbs still check where they always did: after `dir_at` for the path verbs, first for the rest.
That keeps every verb's error order unchanged, which is why the check is not in the constructor.
`RENAME` copies both operands. `walk_prefix` takes a `NameCopy`. `subtree_scope::walk` still takes
bytes, because a shared crate cannot name this server's type. That exception is marked at its one
call, whose bytes are the checked copy's.

`get_xattr_request` already copied its attribute name. The other attribute-name arms (`SETXATTR`,
`REMOVEXATTR`) take bytes in a separate namespace checked by `xattr::is_valid_name`. They are not
path names, so they are not part of this claim.

## The test

Two records hold the claim. The first is the compiler. `NameCopy`'s `compile_fail` doctest shows that
a window-borrowed `&str` cannot reach a verb (E0308). Dropping the copy from an arm fails the same
way, and reading a name's text without the check fails E0624 (both measured).

The second is `redoxfs_server/src/tests/name_window.rs`: host unit tests over `Server::handle`, one
per name-taking arm. Each drives the arm with every class of refused name and a legal control. The
mutating verbs also assert the directory is unchanged. They catch an arm whose check is wrong, which
the type cannot see.

Neither fires the live race. The attack is a second thread rewriting the page between the check and
the use, a data race that safe single-threaded host code cannot stage. The six tests pass against
the pre-fix dispatch too (measured). The type is what closes the race: the resolve reads the
server's copy, which no thread of the client can write.

## Done when

The server checks and uses only its own copy of every path name, and no arm acts on a name
`check_component` refuses. Met. The copy and the check are enforced by type, so dropping either does
not compile, and `name_window.rs` holds each arm's refusals on the host dispatch the running server
runs. Against RedoxFS no swap reaches outside the subtree, so the record is a held claim, not an
escape.

## Reuse

Milestone 800 (a non-Anthropic model attacks the confinement claim)'s attack shape, the file-server
fixtures in `redoxfs_server/src/tests`, the host dispatch the fuzz lane split out
(`redoxfs_server::dispatch`), and `name_resolver`'s copy-out. The stand-in reuses the crate's
existing `Server::handle` host tests and `DiskMemory` fixtures rather than booting a client.

## Follow-on

- **Recorded.** The live race (a second thread rewriting the window between the resolver's
  `check_component` and its `find_node`) is not reproduced by the host stand-in; it needs the
  running second thread. The copy closes it regardless, and `redoxfs_server/src/dispatch.rs`'s module
  header states the mechanism now in force.
- **Done.** `design/fatal-risks/7-the-confinement-claim.md` gained a dated 2026-10-15 paragraph (the
  milestone 275 (a gate that diffs the fatal-risks README against the roadmap it cites) required it) recording that 825 closed the refusal
  the fifth pass homed to it; the verdict stays calef's.
- **Milestone 927.** Rows 19 and 24 of `notes/confinement-claims.md` and the pass-4 refusal log still name
  the window TOCTOU as the open near miss; they want the new test cited and the result recorded. That
  edit touches the confinement records a lane does not own, so it is left to the confinement owner as milestone 927 (cite milestone 825's test in the confinement records):
  `design/roadmap/0927-cite-825-in-the-confinement-claims.md`.

## Index row

The fourth confinement pass's refused probe, closed defensively. A client's second thread could race
its own name window against `redoxfs_server`'s check-then-use. The server now copies each name out
first, so it acts on no name its check refuses.
