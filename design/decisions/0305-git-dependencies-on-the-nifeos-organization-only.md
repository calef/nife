---
status: DECIDED
raised: 2026-10-17
decided: 2026-10-17
ratified_by: calef
---

# 305. Git dependencies are allowed on `nifeos/*` only, pinned to a landed commit

Raised by the soak for the crates nife publishes itself (milestones 813 to 821). calef agreed on
2026-10-17 (UTC), replying "Agrred" to the maintainer's proposal that each such crate first operates
as its own repository, consumed by nife from that repository, until evidence shows its API and its
cut are settled. A crate that has left this tree is a dependency of nife that no registry serves, so
`deny.toml`'s `unknown-git = "deny"` with an empty `allow-git` had to give way.

## The decision

A dependency of this tree may be a git dependency when all three hold:

1. The URL is `https://github.com/nifeos/<repository>`. No other host and no other organization.
2. It is pinned by `rev` to a full commit that has landed on that repository's main. Never a
   branch, never a tag, never a commit on an unmerged branch.
3. The repository is a component or a crate of this project, not a mirror or a fork of someone
   else's code.

`deny.toml` enforces the first: `[sources.allow-org]` admits the `nifeos` organization on GitHub
and `unknown-git = "deny"` refuses every other git source. cargo-deny does not check the second,
so the pull request that adds or bumps the pin cites the landed commit as `nifeos/<repo>@<sha>`
and review checks it.

This is the same rule basalt applies to its pins and the workspace's cross-repository rule applies
to a bump: land in the repository that owns the code first, then bump the pin in a separate pull
request. The dependency is recorded on the blocked side only (§201 (one roadmap until a citation has to cross, and the blocked side declares the dependency)).

## What this does not allow

- A git dependency on anyone else's repository, a fork included. A third-party crate comes from
  crates.io under §46 (thin primitives or whole subsystems; we write everything in between) or is vendored.
- A `path` dependency standing in for the move. The soak counts only if the crate leaves this
  tree; the exit criteria are in the roadmap blocks for milestones 813 to 821.

## Alternatives refused

- **Leave `allow-git` empty and publish first.** Publishing locks the name and every published
  version, and the cut is the part most likely to change. Irreversible, and the evidence the soak
  gathers does not exist yet.
- **Allow any git source with a `rev`.** The check exists to notice a dependency quietly repointed
  at a fork. An organization allow-list keeps that property; a blanket allow loses it.
- **A local registry or vendored copy of each crate.** Vendoring is for code that needs a patch
  (§46, thin primitives or whole subsystems). A first-party crate that is the unit under test needs its own history and its own CI,
  which a vendored copy hides.

## BUGS

- cargo-deny cannot check that a `rev` is an ancestor of the repository's main. The rule rests on
  the bump pull request citing the landed commit, which is rung four of the mechanisms ladder.
- Crates published later move from a git `rev` to a registry version; that is a separate pull
  request per crate and is not covered here.
