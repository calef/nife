---
status: DECIDED
raised: 2026-10-18
decided: 2026-10-18
ratified_by: calef
---

# 310. A name read only inside its package is ratified by rule

*Number provisional; the integrator mints it at merge.*

calef, 2026-10-18 (UTC), on a census of about 9,800 names of which he can rule on a few hundred
per session: "Ratify local names by rule." His queue then holds only names that cross a package
boundary or fail a check.

## The rule

1. What it covers. A `pub` item (a function, method, constant, static, type or trait) that
   carries no `Name:` block, is read by no other Cargo package, and passes every check in rule 3.
   Crates, programs, packages, scripts, modules, documents and directories are outside it, since
   each meets a judgment rule no script can apply (nouns over verbs, the `-d` vocabulary, where a
   document goes). A name that already has a block keeps it: a `provisional` block's author has
   asked for a ruling, and the rule does not answer for calef.
2. What "read by another package" means. Package Q reads an item of package P only when Q
   depends on P and a file of Q reaches the item through P's crate path:
   - `p::a::name`, or a `use p::...;` that lists `name`;
   - `m::name` where `m` came from a `use p::...;`;
   - any word of a file that holds a glob import from `p`.

   A method or associated constant is read when its type is, or when a file of Q that names `p`
   calls it (`.name(`, `::name(`, `::NAME`). An item named by `$crate::` in an exported macro, or
   in a file another package includes with `#[path]`, is read. A bare word in another package is
   never a read: matched that way, every crate's `Error` reached 38 packages.
3. The checks. Each is one a script can make, and each sends a failing name to calef rather
   than refusing it:
   - Rust's casing for the item's kind.
   - No component that a ruling spells out: §154 (the acronym test)'s table and
     [spelled-out-rulings.md](../naming/spelled-out-rulings.md) (`cap`, `op`, `recv`, `config`,
     `spec`, `arg`, `mem`, `dir`, `va`, `pa` and the rest).
   - American spelling, by `typos --locale en-us` and `helpers/house_style.py`'s list.
   - No word claiming the Unix service model.
   - Not a name recorded as refused (`script/names --refused`).
   - A function returning `bool` has one of design/naming.md's question shapes: `is_`, `has_` or
     `can_` first, a finite verb first (`contains`), or `<subject>_is_`, `_has_` or `_can_`.
4. The form. The lane that applies the rule writes, as the item's last doc paragraph:

   ```
   /// Name: local by rule, <YYYY-MM-DD> (§310).
   ```

   The date is the day the name was checked, in UTC. `local` is a fifth state beside
   `ratified`, `recorded`, `unrecorded` and `provisional`. It is counted apart from all four in
   `script/names` and in `script/metrics`' names chart, and it is not on the worklist.
5. What re-queues a name. `script/lint` fails when a `local` name is read by another package
   or fails a check, and says which. The change that caused it rewrites the block to
   `Name: provisional.`, which puts the name on calef's queue at its rank. Narrowing a `local`
   item to `pub(crate)` keeps its block.

## What it accepts

Most of design/naming.md is judgment, and the rule does not check it. An abbreviation nobody has
ruled on (`buf`, `ctx`, `idx`) passes, as do a verb naming a thing and a word used against its
kernel meaning (`map`, `frame`, `tick`). For a local name that cost is bounded. Its readers are
the package's own, and renaming it later touches one package.

## Why a block, and not a status computed from the tree

The class rules in [convention-rulings.md](../naming/convention-rulings.md) write no block, and
`script/names` cites the rule. Here the block earns its place for two reasons.
`helpers/module_block_ratchet.py` holds every `pub` item to carrying a block, against a baseline
that only shrinks, and a `local` block drains that list where a computed status would leave it.
And when a reader appears, a computed status would send the name back to the queue silently; the
block makes that a failure someone reads.

## What else was considered

- Writing `ratified` with a citation of this section. Refused: a lane never writes `ratified`,
  and the count of names calef ruled on would absorb names nobody read.
- The two-words measure `script/names --unratified` used for ranking. Refused for this rule: a
  method called `new` or `len` in any dependent file looked read. The ranking uses rule 2's measure
  too, so the queue's order and this rule agree.
- Including modules. Refused: the noun rule applies to a module's name and no script can apply it.

## Measured

On 2026-10-18 (UTC), before visibility is narrowed, by `helpers/name_local.py --list`:
7,083 `pub` items carry no block. 3,549 are read by no other package; 3,306 of those pass every
check and 243 fail one, 169 of them on a spelled-out abbreviation. Narrowing turns many of the 3,548 into `pub(crate)`, which leaves the
census, so the rule is applied only after that lands.

## BUGS

- The measure reads text, not types. A method called on a value whose type the reader never names,
  through a call that is not `.name(`, is missed. So is a re-export under another name.
- A reader outside every Cargo package (a script that parses a constant out of a `.rs` file) is
  not counted.
