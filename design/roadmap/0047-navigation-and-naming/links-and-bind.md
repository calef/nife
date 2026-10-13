# Milestone 47 appendix: links, `bind`, and where `rm` meets them

An appendix to [milestone 47 (navigation and naming)](../0047-navigation-and-naming.md), moved out of
that page on 2026-10-13 (UTC) under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md).
The main page is complete without it. It holds the `ln` analysis, the naming search that settled
symlinks as `bind`, and the edges where `rm` meets links. Long sentences were split as they moved; no
figure, date, name or quotation was dropped.

## `ln`: hard links make it not a tree, and symlinks stop being an escalation

Two verbs with very different stories. Neither is built. Hard links are declined (calef,
2026-08-23, DECISIONS §110 (hard links are declined, for want of a customer)). No customer needs
them, and `mv`/`RENAME` already covers the atomic-replace idiom they're usually reached for.
Offering them would cost an audit of every place subtree reasoning quietly assumes a tree rather than
a DAG, for a feature nobody's asked for. Symlinks' mechanism question is separately settled (§50
(namespace composition), below: `bind`, not stored paths).

Hard links are mechanically easy. RedoxFS keeps link counts, and §48 (navigation is the shell
rebinding what it holds)'s deferred-delete fix
already depends on them: "the last link goes" is exactly what made `rm` an unlink rather than a
revoke. A second name for one node is a short step from there.

The problem is structural, and it is ours rather than Unix's. §47 (a directory capability carries
six rights) justified `DESCEND` as a
separate right because otherwise "the shape of the tree would decide how much authority a grant
carried". Hard links make it not a tree. A file reachable from two directories sits in two
subtrees, so "this subtree" stops having a clean boundary: you granted a name, and the node is also
reachable through one you did not mention. That is not automatically wrong (the grant was the name),
but every piece of subtree reasoning written so far quietly assumes a DAG cannot happen, and that
assumption should be made explicit before it is falsified. Unix forbids hard links to *directories* to
prevent cycles; the argument is stronger here, where a cycle also defeats `rm -r`'s bottom-up
termination.

Symlinks are the interesting one, and the answer is a real result. A symlink stores a path,
resolved at open time, and this milestone already decided paths resolve in the client, against the
holder's own position, with `..` clamped at the root (§48). So: resolved against *whose* namespace?

Resolve against the holder's, and it follows that a symlink cannot escalate. It can only name
what the resolver could already reach. Unix's symlink attacks: the `/tmp` races, the confused-deputy
TOCTOU classics: work because resolution happens against a *global* namespace carrying the
*victim's* authority. There is no global namespace here and no borrowed authority, so a symlink can
misdirect but cannot grant. Same shape as the `PATH` result
([path-and-completion.md](path-and-completion.md)): the escalation vector closes because there is
nothing ambient to point into.

The cost is that one symlink means different things to different holders. That sounds alarming and is
exactly Plan 9's per-process namespace behavior, so it is a well-explored place to stand rather than
a novel one.

Hard links: decided, declined (§110 (hard links declined)). What remains, for symlinks: what a stored path containing
`..` means when the holder's root is shallower than the creator's. §48 clamps, so it should clamp
here too rather than erroring, but that is a decision, not yet made.

### ~~Open fork~~ SETTLED 2026-07-31: `bind`, not stored paths (DECISIONS §50)

calef chose namespace composition. The analysis below is kept because the naming search is the
evidence for the decision rather than a digression: twenty-eight-plus candidates, terminating without
a winner, which is what a construct that does not fit any familiar relationship looks like. `bind`
needed no search: Plan 9 and `mount --bind` already named it. See §50 for the decision, what it
costs, and the inert-stored-path escape hatch if milestone 55 (Time Machine) turns out to need
on-disk fidelity.

#### The analysis that settled it: was the mechanism right, and if so what is it called? (raised 2026-07-31)

Not decided. Two questions, in this order, because the second keeps answering the first.

Mechanism first. Plan 9 has no symlinks: it has `bind`. Per-process namespaces made them
unnecessary: you do not need a stored path that resolves oddly per holder when you can compose the
holder's namespace directly. This milestone already took Plan 9's answer for absolute paths and for
`PATH`; taking Unix's here, renamed, would be the inconsistent choice. Settle whether we want
namespace composition instead before settling a noun.

Then the name, because "symbolic link" fails §39 (a component is named for what it is) on both
halves. "Symbolic" is defined *against* "hard". So if hard links are declined, the adjective
contrasts with something that does not exist.
"Link" is worse: it links nothing. The by-name-ness is the entire content: there is no object
identity, and two holders may resolve the same entry to different files or to nothing.

The criterion, which rules out most candidates at once. A name here must not imply object
identity, must not imply a connection, and must not collide with "reference": in a
capability system a reference is unforgeable and holder-independent, the exact inverse of this. That
disposes of `link`, `reference`, `shortcut` and `pointer`.

Worked, and rejected with reasons rather than by taste:

| Candidate | Why not |
|---|---|
| `alias` | Semantically closer than `link`: a shell alias is stored text, expanded at use, meaning what the current environment makes it mean, with no identity claim. But taken twice: zsh's `alias` (which this milestone tracks, so we would collide with ourselves), and macOS "aliases", which store a file ID and survive the target moving: they track the object, the inverse of ours. Borrowing a Mac term for its opposite is a poor trade on a project whose first real user is a Mac |
| `costume`, `disguise` | Both imply an underlying thing being dressed or concealed, reinstating exactly the object identity the word must avoid. `disguise` also claims intent to mislead, naming into existence a danger this design removes: a stored name here cannot escalate, because it resolves only within what the holder already reaches |
| `projection`, `shadow` | Honest about viewpoint-dependence without implying concealment, and still metaphors. This project names descriptively (`net_stack`, `compositor`, `line_editor`), which is §39's doing; `link` got away with a false claim partly *because* it was a metaphor |
| `mirror` family (`erised`, `matsuyama`) | The best framing anyone found, and the only family to pass all three tests: a mirror shows something viewer-dependent, implies no object identity, implies no connection, and does not collide with "reference". It fails on the word rather than the idea. In computing a mirror is an identical replica at another location: "same content, elsewhere", which is the identity claim we are trying to avoid. The literary instances add their own wrong axis: Erised shows what you desire (ours shows what your namespace resolves to, often nothing), and the Matsuyama tale is about a mistake (the deception axis where `disguise` failed). Both also need a decoder ring, and `design/naming.md` sets the bar at names that parse without prior exposure |
| `fsalias` | Fixes the zsh collision, and prefixes are in-style here (`fs_file_caretaker`, `fs_subtree_caretaker`, `c_confiner`). But "filesystem alias" is exactly what Finder calls a macOS alias (the object-tracking one), so the prefix picks the *wrong* one of the word's two meanings. And prefixing to fix a collision is a smell: it answers *which* alias, where the objection was that alias claims another name for the same thing |

The descriptive candidate, if the mechanism survives: a third entry kind beside file and
directory: a `path`. A directory entry names a file, a directory, or a path; it stores a path and
the holder resolves it, which is the whole description. It also reads correctly when it fails: *"that
entry is a path that does not resolve"* is what happened, where *"that link is broken"* implies
something was once connected. The verb becomes writing a path into a directory rather than "linking",
which retires the `ln -s` shape and its trailing-slash footgun with it.

A further seven produced no new failure modes (`speculum`, `glass`, `scryer`, `mimic`, `imitate`,
`parallel`, `echo`), which is what an exhausted search looks like. They re-derive the four already
listed. `speculum` and `glass` and `scryer` are the mirror family with added baggage: a medical
instrument, a *material* that only means mirror with "looking" in front, and a word naming the
person looking rather than the thing looked into, plus divination. `mimic` and `imitate` reinstate
an original being imitated, which is where `costume` and `disguise` died, and `imitate` is a verb
besides. `echo` collides with a shell builtin we already have, exactly as `alias` collides with
zsh's. And `parallel` means concurrency, in a system with four cores and per-CPU run queues.

Two later candidates are worth their own line. `harmonic` clears all three tests: the stored path
as fundamental, the holder as resonator. It fails on the direction of causation, a failure mode
none of the others had. A harmonic is *determined by* its fundamental, whereas our resolution is
determined by the namespace, not by the stored name. The metaphor points the causal arrow
backwards. (`harmony` is simply the wrong axis: it means concord, where ours may resolve to nothing.)

`reflection` is the best of the mirror family, better than `mirror` itself. A reflection is
explicitly *not the thing*, where a computing mirror implies an identical replica. And its causation
is right, since what you see depends on the mirror *and* where you stand. It fails on a harder collision:
in programming, reflection is runtime introspection of types, which is precise, universal, and in
our own domain.

And that is the pattern behind the whole family. `mirror` → replica, `reflection` → introspection,
`echo` → a shell builtin we ship, `parallel` → concurrency. Physical-optics vocabulary has been
comprehensively borrowed by computing for unrelated meanings, so the one metaphor that actually fits
this construct is the one whose every word is already spent. That is not bad luck; it is why a flat,
non-metaphorical name is the likely answer if the mechanism survives at all.

That the naming is this hard is itself evidence. Seventeen candidates: the first eight failed for a
*different* reason each, and the rest found almost none. The one that passed every test failed on
the word being occupied by its own inverse. The
construct is the only thing in this design whose meaning depends on who is looking, and the vocabulary
has no slot for that. Plan 9 hit the same wall from the same premises and answered with a different
mechanism rather than a better noun, which is why this fork is mechanism first.

### Where `rm` meets them, which is where the sharp edges are

`rm` on a symlink removes the link, never the target. `rm(1)` says so outright: "the rm utility
removes symbolic links, not the files referenced by the links", and it is right for the reason §48
already established: `rm` operates on a name in a directory, and a symlink is a name.

`rm -r` must not descend through a symlink, and our reason differs from Unix's in a way worth
recording. Unix declines because following would escape: a symlink to `/` inside a directory
would turn `rm -r` into `rm -rf /`. Here it could not escape: a symlink resolves in the holder's
namespace, and `rm`'s namespace is the granted subtree with `..` clamped at its root (§48), so a
symlink cannot name anything outside the grant. We keep the behavior and lose the reason. The
behavior still earns its place: following would delete a different set of names than the grant
named, and "surprising but bounded" is still surprising.

`rm` on a hard-linked file removes one name and the data survives. That is not a special case. It
is exactly §48's unlink-versus-revoke distinction, and the mechanism is already built: RedoxFS's
deferred delete (`on_open_node` / `on_close_node` and the release list) is what makes the last link,
not the first, the one that frees.

The sharp one: `rm -r subtree/` where a file inside is also linked from outside. The subtree goes
away and the data does not, because the outside name still holds it. That is correct: you removed the
names you were granted, and you were never granted the other one, but it means "I deleted the
subtree" and "that content is gone" stop being the same statement. For a backup target (milestone
55) that distinction is worth stating rather than discovering.

And the cycle, which is a termination argument rather than a taste one. `rm -r` works bottom-up,
so a hard link making a directory its own descendant does not merely confuse it: it does not
terminate. Unix forbids hard-linked directories for this reason among others; here the same
prohibition is load-bearing for a verb we have already shipped the recursion for.

One footgun inherited if symlinks land: `rm -r link` versus `rm -r link/`. The trailing slash
changes whether the target's contents are in scope, which is a real source of accidents in Unix.
Decide it explicitly rather than letting the path parser decide by accident.

