"""Every name `script/names` tracks, out of any tree, for `script/names` and `script/metrics`.

**Why this file exists.** `script/names` reads the working tree and `script/metrics` reads a
`git ls-tree` at each week's commit, and until 2026-10-18 (UTC) each enumerated the named things
itself. `script/names` grew items, modules, directories, documents and the naming rules, and the
metrics copy still counted four kinds: 325 names for 2026W42 against 1,793, so the chart missed most
of the naming review. Now both pass a tree to `census()` here, and only the sourcing differs: a list
of paths and a function that reads one. `script/metrics` also fails when the current week's
`names_total` disagrees with `script/names`' total, so the sourcing cannot drift unseen either.

**What each caller supplies.** `paths` is every file in the tree, repository-relative. `read(path)`
returns the file's text, or None when the tree has no such file. Nothing here opens a file or runs
git, which is what lets a historical tree be counted without a checkout.

**`missing` is the names whose block is absent or malformed.** A surface that must carry a block
and does not is still a named thing, so `script/metrics` counts it toward `names_total` and toward
no status (the "No provenance block" band). On a tree that passes `script/names --check` it is zero.

Name: provisional, minted 2026-10-18 (UTC) by lane maintainer/metrics-names-series. A shared helper
under `helpers/`, which `script/names` puts out of its own scope, so this paragraph is the record.
It takes `name_` for the family beside `name_provenance.py`; `census` because it counts a whole
tree, where `name_provenance` parses one block. calef has not named it.
"""

import os
import re

import name_crossings
import name_provenance
import name_rules

# Where a program's file lives. `user/src/` is only in history: milestone 175 split it into the
# other two on 2026-09-13, and `script/metrics` counts each week from the tree it had. (Milestone 175 is the split of `user/`
# into `components/` for services and `fixtures/` for test and benchmark programs.)
PROGRAM_DIRS = ("components/src", "fixtures/src", "user/src")
_RECORD_STEM = re.compile(r"^\d")
_PACKAGE_NAME = re.compile(r'^\s*name\s*=\s*"([^"]+)"', re.M)

# Why a header-shaped line is not the block, phrased for whoever has to fix it. The tokens are
# `name_provenance`'s; the wording is the gate's.
STRAY_WHY = {
    name_provenance.STRAY_MARKUP:
        "markdown sits between the comment marker and `Name:`, which the parse does not skip",
    name_provenance.STRAY_INDENT:
        "it is indented past the single space the parse allows after the comment marker",
    name_provenance.STRAY_MARKER:
        "it uses a different comment marker from the one this surface's block is read at",
    name_provenance.STRAY_SECOND:
        "it is a second block below the one that was read, and nothing reads past the first",
}

# An exception to §75 (directories under `design/` and `notes/` carry provenance in their own
# README) (2026-10-06, UTC). Each entry under notes/skills/
# is an Agent Skills directory whose SKILL.md name the specification fixes, so the entry has no
# README of its own. calef ratified all six entry names in one ruling, which notes/skills/README.md
# records beside the directory's own block, and judged a README per entry the gate's fault ("SKILL.md
# holds the information"). So a directory DIRECTLY under notes/skills/ that holds a SKILL.md is
# covered by the parent's block. helpers/agents-index-check.py fails an entry with no row there.
SKILLS_DIR = "notes/skills"


def _surfaces(paths, read):
    """`(kind, name, path, prefix)` for every crate, program, module, script and package."""
    pathset = set(paths)
    crates, out, bins = set(), [], {}
    for path in paths:
        parts = path.split("/")
        if parts[0] == "crates" and len(parts) > 2:
            crates.add(parts[1])
        elif len(parts) == 2 and parts[0] == "script":
            out.append(("script", parts[1], path, "#"))
    for crate in sorted(crates):
        out.append(("crate", crate, f"crates/{crate}/src/lib.rs", "//!"))
    for directory in PROGRAM_DIRS:
        depth = directory.count("/") + 2
        found = {}
        for path in paths:
            if not path.startswith(directory + "/"):
                continue
            parts = path.split("/")
            if len(parts) == depth and path.endswith(".rs"):
                found.setdefault(parts[-1][:-3], path)
            elif len(parts) == depth + 1 and parts[-1] == "main.rs":
                found[parts[-2]] = path
        if not found:
            continue
        # A program is a `[[bin]]` (calef's tiering, 2026-10-17 UTC). A file another program
        # includes through `mod` or `#[path]` is a module of that program.
        manifest = os.path.dirname(directory) + "/Cargo.toml"
        if manifest not in bins:
            bins[manifest] = name_crossings.bin_paths(read(manifest) or "")
        for name in sorted(found):
            # A directory program's block is in `main.rs` even when a `<name>.rs` sits beside it.
            path = found[name]
            rel = path[len(directory) + 1:]
            kind = "program" if name_crossings.is_program(rel, bins[manifest]) else "module"
            out.append((kind, name, path, "//!"))
    for path in paths:
        parts = path.split("/")
        if (parts[-1] != "Cargo.toml" or len(parts) == 1 or parts[0] in ("crates", "vendor")
                or any(p.startswith(".") or p == "target" for p in parts[:-1])):
            continue
        hit = _PACKAGE_NAME.search(read(path) or "")
        if hit:
            out.append(("package", hit.group(1), path, "#"))
    return out, pathset


def _includer(kind, path, paths, read):
    """For a program-directory file that is not a `[[bin]]`, the program that includes it."""
    if kind != "module":
        return None
    directory = os.path.dirname(path)
    name = os.path.splitext(os.path.basename(path))[0]
    sources = {os.path.basename(p)[:-3]: read(p) or "" for p in paths
               if os.path.dirname(p) == directory and p.endswith(".rs")}
    return name_crossings.includer_of(name, sources)


def _stray_problems(kind, name, path, prefix, text):
    """One problem per header-shaped line that is not the block (milestone 283 (one provenance
    block per file, in the spelling the parse reads)), and per refusal
    the parse cannot reach (2026-09-18), because either would leave a ruling invisible."""
    out = []
    for number, line, why in name_provenance.strays(text, prefix):
        if prefix == "//!" and line.lstrip().startswith("///"):
            continue    # an item's own block, judged with the `item` kind
        out.append(f"{kind} {name}: {path}:{number} reads as a provenance header and is not the "
                   f"block that was read, because {STRAY_WHY[why]}\n      {line.strip()}")
    for number, line in name_provenance.refusals_outside(text, prefix):
        out.append(f"{kind} {name}: {path}:{number} records a refusal the parse cannot read: it is "
                   f"outside the `Name:` block, so `script/names --refused` will never show it. Move "
                   f"it into the block, or move the block to the end of the comment.\n      "
                   f"{line.strip()}")
    return out


def _rust_records(paths, read, surface_files, problems):
    out = []
    for rel in paths:
        if not rel.endswith(".rs") or rel.startswith(("vendor/", "patches/")):
            continue
        text = read(rel)
        if text is None or ("Name:" not in text and "Provisional name" not in text
                            and "Name provisional" not in text):
            continue
        for it in name_provenance.rust_items(text):
            for number, line, why in it["strays"]:
                problems.append(f"item in {rel}:{number} reads as a provenance header and is not "
                                f"the block that was read, because {STRAY_WHY[why]}\n      {line}")
            for number in it["ad_hoc"]:
                problems.append(f"item in {rel}:{number} marks its name provisional in prose, which "
                                f"the worklist cannot see; write `/// Name: provisional, <who, "
                                f"when>.` instead")
            if it["block"] is None:
                continue
            if it["name"] is None:
                problems.append(f"item in {rel}:{it['line']}: a `Name:` block on a doc comment "
                                f"that documents nothing the parse can name")
                continue
            out.append(("item", it["name"], f"{rel}:{it['line']}", it["block"], it["what"]))
        if rel in surface_files:
            continue
        # A module's own `//!` block. Strays are read over `//!` lines only: a `///` header belongs
        # to an item, and a `//` note beside code (xtask records subcommand rulings so) is prose.
        own = "\n".join(line for line in text.split("\n") if not line.lstrip().startswith("///"))
        for _number, line, why in name_provenance.strays(own, "//!"):
            if not line.lstrip().startswith("//!"):
                continue
            problems.append(f"module in {rel}: a line reads as a provenance header and is not the "
                            f"block that was read, because {STRAY_WHY[why]}\n      {line.strip()}")
        found = name_provenance.block(text, "//!")
        if found is not None:
            stem = os.path.splitext(os.path.basename(rel))[0]
            if stem in ("mod", "lib", "main"):
                stem = os.path.basename(os.path.dirname(rel))
            out.append(("module", stem, rel, found, "module"))
    return out


def _document_records(markdown, pathset, read, problems):
    """Directories and documents under design/, notes/ and briefs/. Returns `(records, missing)`."""
    out, missing = [], 0
    for d in sorted({os.path.dirname(p) for p in markdown} - {"design", "notes", "briefs"}):
        if _RECORD_STEM.match(os.path.basename(d)):
            continue
        if os.path.dirname(d) == "design/roadmap/proposals":
            continue    # a proposal's appendix: its slug is a draft title, skipped like a block's
        if os.path.dirname(d) == SKILLS_DIR and d + "/SKILL.md" in pathset:
            continue    # covered by notes/skills/README.md's block; see SKILLS_DIR
        readme = d + "/README.md"
        if readme not in pathset:
            problems.append(f"directory {d}/: no README.md, and §75 puts a directory's `Name:` "
                            f"block there")
            missing += 1
            continue
        own, stems, loose = name_provenance.markdown_blocks(read(readme) or "")
        for number, para in loose:
            problems.append(f"directory {d}/: {readme}:{number} reads as a provenance header the "
                            f"parse does not take (bold, a second directory block, or a prose "
                            f"`Provisional name`)\n      {para[:120]}")
        if own is None:
            problems.append(f"directory {d}/: {readme} carries no paragraph opening `Name:`")
            missing += 1
            continue
        out.append(("directory", d + "/", readme, own, "directory"))
        if d == "design/roadmap" or d.startswith("design/roadmap/"):
            continue
        names = sorted(os.path.basename(p)[:-3] for p in markdown
                       if os.path.dirname(p) == d and os.path.basename(p) != "README.md")
        for stem in names:
            if not _RECORD_STEM.match(stem):
                out.append(("document", stem, f"{d}/{stem}.md", stems.get(stem, own), "document"))
        for stem in sorted(set(stems) - set(names)):
            problems.append(f"directory {d}/: {readme} records `{stem}`, which is not a file here")
    return out, missing


def census(paths, read, british=None):
    """`(records, problems, missing)` for one tree. See this module's docstring for the inputs.

    A record is a dict with `kind`, `name`, `path`, `status`, `date`, `refused`, `text`, and `what`
    or `owner`. Names a rule on `name_rules.RULES_DOC` covers come back ratified, citing the rule.
    `british` is the word list the appendix rule's spelling check reads, `house_style`'s by default.
    """
    if british is None:
        try:
            import house_style
            british = house_style.EXTRA_BRITISH
        except ImportError:
            british = []
    paths = sorted(paths)
    problems, records, missing = [], [], 0
    surfaces, pathset = _surfaces(paths, read)
    surface_files = {path for kind, _n, path, _p in surfaces
                     if kind in ("crate", "program", "module")}
    markdown = [p for p in paths if p.endswith(".md") and p.startswith(("design/", "notes/",
                                                                         "briefs/"))]
    docs, missing = _document_records(markdown, pathset, read, problems)
    for kind, name, path, text, what in _rust_records(paths, read, surface_files, problems) + docs:
        status, date, refused, problem = name_provenance.classify(text)
        if problem is not None:
            problems.append(f"{what} {name} ({path}): the block must start `ratified <date>`, "
                            f"`recorded (where)`, `provisional` or `unrecorded`, with a date on "
                            f"`ratified` and a citation on `recorded` (found {text[:48]!r})")
            missing += 1
            continue
        records.append({"kind": kind, "name": name, "path": path, "status": status, "date": date,
                        "refused": refused, "text": text, "what": what})

    for kind, name, path, prefix in surfaces:
        source = read(path)
        if source is None:
            problems.append(f"{path} does not exist")
            missing += 1
            continue
        text = name_provenance.block(source, prefix)
        # Reported whether or not the block parsed: a file with no readable block is exactly the
        # file whose unreadable one is worth pointing at.
        problems.extend(_stray_problems(kind, name, path, prefix, source))
        if text is None:
            problems.append(f"{kind} {name}: no `Name:` block in {path}")
            missing += 1
            continue
        status, date, refused, problem = name_provenance.classify(text)
        if problem is not None:
            # The `recorded` example cites milestone 46 (rename the components for what they are,
            # and write down the naming rules), a real citation of the form.
            problems.append({
                name_provenance.NO_STATUS:
                    f"{kind} {name}: the block must start `ratified <date>`, `recorded (where)`, "
                    f"`provisional` or `unrecorded` (found {text[:48]!r})",
                name_provenance.NO_DATE:
                    f"{kind} {name}: `ratified` must carry a date (ratified 2026-08-04), "
                    f"because a ratification nobody can place is prose",
                name_provenance.NO_CITATION:
                    f"{kind} {name}: `recorded` must cite where (recorded (milestone 46)), "
                    f"because the claim IS that the reasoning is somewhere else",
            }[problem])
            missing += 1
            continue
        records.append({"kind": kind, "name": name, "path": path, "status": status, "date": date,
                        "refused": refused, "text": text,
                        "owner": _includer(kind, path, paths, read)})

    # **Names a rule covers are ratified, and cite the rule** (calef, 2026-10-17 UTC). Applied after
    # the blocks are read, so a covered name's own block still prints beside the rule.
    rules_text = read(name_rules.RULES_DOC)
    if rules_text is None:
        problems.append(f"{name_rules.RULES_DOC} does not exist, and the names it ratifies by rule "
                        f"are counted from it")
    else:
        _pulled, bad = name_rules.apply(records, name_rules.parse_rules(rules_text), markdown,
                                        lambda rel: read(rel) or "", british)
        problems.extend(bad)
    return records, problems, missing
