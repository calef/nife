---
status: NOT-STARTED
raised: 2026-10-07
milestone_dependencies: 800, 812
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 825. A hostile client races the file server's name window

*(The title and slug are provisional.)*

## In brief

Boot the attack that milestone 800 (a non-Anthropic model attacks the confinement claim)'s fourth
outsider pass declined, then close the gap. A client rewrites a name in its own window between
`redoxfs_server`'s check and its use. Can a name that passed `check_component` be swapped for one
that would not? If so, it reaches outside the client's subtree.

## Why

calef ruled the refusal log's order on 2026-10-07 (UTC), option (i): this is the next probe. It is
the likeliest live escape the outsider passes have left, and the one a later pass is most likely to
find, which would reset criterion (c)'s count. The gap is recorded at the head of
`redoxfs_server/src/dispatch.rs`: "A name is checked in the client's own window and then used from
it". Claims 19 and 24 of `notes/confinement-claims.md` rest on it. `name_resolver` copies a name out
before judging it. The file server does not.

The fix is this milestone's (calef, 2026-10-14 UTC), and it does not wait on the race being won.
Every name-taking arm builds a `&str` over the shared page, so a second writer can put invalid
UTF-8 under it, which is undefined behavior whether or not the swapped name escapes.

## Who can write the window

- The client's own second thread. Milestone 812 (`std::thread::spawn` runs real threads in one
  address space) is built, so a confined job can race its own window. This is the attack.
- Window 0's long-lived clients (the shell, `login`, the identity provisioner, the progenitor). They
  share the page but are kept apart by blocking, per `redoxfs_server`'s own BUGS, so none writes
  while another's request is in flight in the shipped boot.
- A window's previous holder. Closed: milestone 685 (a job is finished when its memory is back)
  never hands out a window whose last holder is unreaped. The dispatch BUGS still names this route
  and should drop it.

## The work

1. A hostile client, a role of an existing test binary as milestone 800's `socket_squatter` is. It
   holds a directory capability for a subtree on a disk fixture. One thread issues name-taking
   requests (`OPEN`, `CREATE`, `OPENDIR`, `MKDIR`) with a legal name; the other rewrites the window
   to `..`, a name with `/`, the attribute store's directory, and invalid UTF-8.
2. A test that goes red if any request reaches a name outside the subtree. Boot it on every ISA
   that runs the file server, and commit it red before any fix. If it stays green after a real try,
   record how many races ran and what bounds the window, and go on to the fix anyway.
3. The fix the dispatch BUGS names: copy the name out of the window into a bounded buffer, then
   check and resolve only the copy. One copy of at most a name's length per request.
4. A replayable falsification: restoring the borrow turns the test red where step 2 got it red. If
   the race was never won, a host test over `Server::handle` with a window that changes between
   check and use stands in, since the host dispatch is the running server's.

## Done when

The race has been booted, and the server checks and uses only its own copy of every name. Claims
19 and 24 cite the test and its falsification, and the pass notes' refusal log records the result:
an escape with a red test, or a held claim with the race counts.

Reuse: milestone 800's attack shape, the file-server fixtures already in `system_tests`, the host
dispatch the fuzz lane split out (`redoxfs_server::dispatch`), and `name_resolver`'s copy-out.

## Index row

The fourth confinement pass's refused probe, booted and closed. A client's second thread races its
own name window against `redoxfs_server`'s check-then-use, and the server then checks a copy.
