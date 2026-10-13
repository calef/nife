# Milestone 47 appendix: links, `bind`, and where `rm` meets them

An appendix to [milestone 47 (navigation and naming)](../0047-navigation-and-naming.md). The page is
complete without it. It holds why nife has no `ln`, and what that means for `rm`.

## `ln`: hard links make it not a tree, and symlinks stop being an escalation

There is no `ln`. Hard links are declined by §110 (hard links are declined, for want of a customer),
and symlinks as stored paths are replaced by `bind` under §50 (namespace composition). The reasons
below are the ones that still hold.

Hard links would make the namespace not a tree. §47 (a directory capability carries six rights)
makes `DESCEND` a separate right because otherwise "the shape of the tree would decide how much
authority a grant carried". A file reachable from two directories sits in two subtrees, so "this
subtree" stops having a clean boundary. Subtree reasoning throughout the tree assumes a DAG cannot
happen. `mv`/`RENAME` already covers the atomic-replace idiom hard links are usually reached for. And
a hard-linked directory could make a directory its own descendant, which `rm -r`'s bottom-up walk
would never finish.

A stored path resolved against its holder's namespace could not escalate: it can only name what the
resolver could already reach. Unix's symlink attacks (the `/tmp` races, the confused-deputy TOCTOU
classics) work because resolution happens against a *global* namespace carrying the *victim's*
authority. nife has neither, the same reason `PATH` injection is impossible
([path-and-completion.md](path-and-completion.md)). `bind` keeps that property without a stored
path: a bound name is a position in the holder's own namespace, so it can misdirect but cannot
grant. One name may mean different things to different holders, which is Plan 9's per-process
namespace behavior. Plan 9 has no symlinks either; per-process namespaces made them unnecessary, and
`bind` is its name for the mechanism. §50 holds the decision, its cost, and the inert-stored-path
escape hatch if milestone 55 (Time Machine) ever needs on-disk fidelity.

A stored path would also have been badly named "symbolic link" under §39 (a component is named for
what it is): "symbolic" is defined against "hard", and "link" links nothing.

### Where `rm` meets them, which is where the sharp edges are

`rm` operates on a name in a directory (§48 (navigation is the shell rebinding what it holds)). A
`bind` is not a directory entry: it is a value in the shell's own table, `grant_plan::nav::Bindings`
(`crates/grant_plan/src/nav.rs`), so no `UNLINK` or `RMDIR` can remove one. A bound name in an
operand is resolved by the shell when it plans the grant (`designate` in
`crates/grant_plan/src/planning.rs`, through `Holdings::anchor`), and the grant carries only the
real position it lands on. `rm` never sees the bound name.

Unlinking a file that is still open removes the name while the data survives, which is §48's
unlink-versus-revoke distinction. RedoxFS's deferred delete (`on_open_node` / `on_close_node` and
the release list) is what makes the last reference, not the first, the one that frees.
