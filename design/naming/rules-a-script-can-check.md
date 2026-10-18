# Rules a script can check

*An appendix to [`design/naming.md`](../naming.md), which is the rule. It says which of that page's
rules a script can apply to a name, and how a lane works the list of names that break one. The file
and its stem are provisional names; naming is an architect's.*

calef, 2026-10-18 (UTC): "I really feel like ideally we would fix unratified names to comply with
our rules, but we would not ratify them until we've done the actual ratification and review yet."
So a name that breaks a rule is renamed to comply and stays unratified. Passing every check below
ratifies nothing, because most of the rules need a reader.

## The worklist

```
python3 helpers/name_checks.py                    # every unratified name that breaks a checked rule
python3 helpers/name_checks.py crates/e1000e      # one area
```

Each line gives the name, its reach and the checks it fails. The list is in the order to work it:

1. `local`: no other Cargo package reads it. Rename it in its package, one area per pull request.
2. `crosses into N`: N other packages read it. Rename by family, so a type and the names that follow
   it move together.
3. Crates, programs, scripts and documents: a newcomer meets these first. Rename by family.
4. `wire or format`: defined in `abi`, a `*_contract` crate, `network_time_protocol`,
   `supervision_protocol` or a format §307 (how long a name may be) lists. Another program parses
   these, so list them for calef and never rename them in a lane.

A renamed name takes `Name: provisional, minted <date> (UTC) by <lane>.` It is never `ratified`.
A hit that is a foreign identifier (DesignWare's `PA` for a PHY address, Intel's `IA32_*`) keeps its
owner's spelling, and the lane says so in its pull request rather than renaming it.

## Which rules a script can check

Another package reads a name only through the defining crate's path, never the bare word. A method
or associated constant is read when its type is, or when it is called as `.name(` in a file that
names the crate. `helpers/name_crossings.py`'s `Reach` is the measure, and
`script/names --unratified` ranks by the same one.

| Rule | Kind | Checked by |
|---|---|---|
| Rust's casing for the item's kind | mechanical | `name_checks.py` |
| Abbreviations a ruling spells out (§154 (the acronym test is whether the phrase is spoken)'s table, [spelled-out-rulings.md](spelled-out-rulings.md)) | mechanical | `name_checks.py`; `cap` also by `cap_abbreviation.py`, as a ratchet |
| American spelling | mechanical | `typos --locale en-us` and `house_style.py`'s list |
| No `-d`, no Unix service word | mechanical | `script/lint`'s naming block for programs; `name_checks.py` for a trailing `d` component |
| Lowercase, hyphenated file names | mechanical | `script/lint`'s naming check 6 |
| Questions are `is_`, `has_`, `can_` or a finite verb | partly: the shape is checked, the exemptions are judgment | `name_checks.py` lists a `bool` function outside the shapes |
| Crossing a package boundary | partly: read by path, not by type | `name_crossings.Reach` |
| The acronym test for an abbreviation nobody has ruled on | judgment | not checked |
| Nouns, not verbs; generic words; the standard word | judgment | not checked |
| A word whose kernel meaning differs (`map`, `frame`, `tick`) | judgment | not checked |
| A half implies two; a foreign identifier keeps its spelling | judgment | not checked |

## BUGS

- The question-shape check reads a first word ending in `s` as a finite verb, so a plural noun
  (`bits_set`) passes. An action that reports success (`insert`) is listed though the rule exempts
  it.
- The measure reads text, not types. A method called on a value whose type the reader never names,
  through a call that is not `.name(`, is missed, and so is a re-export under another name. A reader
  outside every Cargo package (a script that parses a constant out of a `.rs` file) is not counted.
- `wire or format` is a list of packages, so a wire name defined anywhere else is reported as
  `local` or `crosses`. A lane renaming one checks that no other program parses it.
