# Local names are ratified by rule

*An appendix to [`design/naming.md`](../naming.md), which is the rule. It says which of that page's
rules a script can apply, since only those can ratify a name by
[§310 (a name read only inside its package is ratified by rule)](../decisions/0310-a-name-read-only-inside-its-package-is-ratified-by-rule.md).*

## What a lane does

Apply the rule one area per pull request:

```
python3 helpers/name_local.py --list crates/e1000e      # who qualifies there, and who fails what
python3 helpers/name_local.py --apply crates/e1000e     # write the block onto each that qualifies
python3 helpers/module_block_ratchet.py --bank          # drop the rows those blocks cleared
```

The block is the item's last doc paragraph, `/// Name: local by rule, <date> (§310).`, dated the
day it was checked (UTC). A name that fails a check keeps no block and stays on calef's queue.

When `script/lint` fails a `local` name, another package now reads it or it fails a check. Rewrite
its block to `/// Name: provisional.` in the same change. That is the rule working, not a fault
to route around.

## Which rules are mechanical

Another package reads a name only through the defining crate's path, never by the bare word. A
method or associated constant is read when its type is, or when it is called as `.name(` in a file
that names the crate. `helpers/name_crossings.py`'s `Reach` is the measure, and
`script/names --unratified` ranks by the same one.

Each rule on this page, by whether a script can apply it:

| Rule | Kind | Checked for `local` by |
|---|---|---|
| Rust's casing for the item's kind | mechanical | `name_local.py` |
| Abbreviations a ruling spells out (§154's table, [spelled-out-rulings.md](spelled-out-rulings.md)) | mechanical | `name_local.py`; `cap` also by `cap_abbreviation.py` |
| American spelling | mechanical | `typos --locale en-us` and `house_style.py`'s list |
| No `-d`, no Unix service word | mechanical | `name_local.py`, as `script/lint`'s naming block does for programs |
| Not a refused name | mechanical | `script/names --refused` |
| Questions are `is_`, `has_`, `can_` or a finite verb | partly: the shape is checked, the exemptions are judgment | a `bool` function outside the shapes goes to calef |
| Crossing a package boundary | partly: read by path, not by type | `name_crossings.Reach` |
| The acronym test for an abbreviation nobody has ruled on | judgment | not checked |
| Nouns, not verbs; generic words; the standard word | judgment | not checked; modules and crates are outside §310 for this reason |
| A word whose kernel meaning differs (`map`, `frame`, `tick`) | judgment | not checked |
| A half implies two; a foreign identifier keeps its spelling | judgment | not checked |

A name that fails a mechanical check is not refused. It stays unratified and goes to calef.

## BUGS

- The question-shape check reads a first word ending in `s` as a finite verb, so a plural noun
  (`bits_set`) passes.
- The measure reads text, not types. §310's BUGS lists what it misses.
