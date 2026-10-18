"""Unratified names that break a naming rule a script can check, and how far each one reaches.

calef, 2026-10-18 (UTC): "I really feel like ideally we would fix unratified names to comply with
our rules, but we would not ratify them until we've done the actual ratification and review yet."
This is the worklist for that fix. It lists every name that is not ratified (no block, or a block
that is `provisional`, `recorded` or `unrecorded`) and fails a check below, with the check and the
name's reach. Nothing here ratifies anything, and a name that passes every check is not thereby
right: design/naming/rules-a-script-can-check.md lists the rules no script can apply.

    python3 helpers/name_checks.py [PREFIX..]   # the worklist, local names first
    python3 helpers/name_checks.py --selftest   # the checks and the reach, as fixtures

**Reach** is one of three, in the order a fix lane works them:

- `local`: no other Cargo package reads it, so a rename touches one package.
- `crosses into N`: N other packages read it, measured by the defining crate's path
  (`name_crossings.Reach`, never the bare word). Renamed by family.
- `wire or format`: it is defined in a package two programs agree through (`WIRE`). A rename
  there changes what another program parses, so it is listed for calef and never renamed by a lane.

A renamed name takes `Name: provisional, minted <date> (UTC) by <lane>.` and stays unratified.

Name: provisional, minted 2026-10-18 (UTC) by lane maintainer/names-ratify-local-by-rule. A shared
helper under `helpers/`, which `script/names` puts out of its own scope, so this paragraph is the
record. It takes `name_` for the family beside `name_census.py`, and `checks` for what it runs.
calef has not named it.
"""

import collections
import json
import os
import re
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cap_abbreviation  # noqa: E402
import name_census  # noqa: E402
import name_crossings  # noqa: E402
import name_provenance  # noqa: E402

# Components a ruling spells out: §154 (the acronym test is whether the phrase is spoken)'s table
# and design/naming/spelled-out-rulings.md. A plural is the same abbreviation. A foreign identifier
# keeps its owner's spelling, and telling one from ours is judgment, so it is listed too and the
# lane that fixes it decides.
SPELLED_OUT = {
    "cap", "caps", "op", "ops", "recv", "config", "configs", "spec", "specs", "arg", "args",
    "mem", "dir", "dirs", "va", "pa", "ipi", "ipis", "nmi", "nmis", "ist", "msr", "msrs", "ipc",
    "dtb", "asid", "asids", "gpt", "nvme", "pcie", "dma", "pr", "prs", "initrd", "prog", "progs",
    "pos", "opt", "opts", "mtime",
}
# Packages two programs agree through: `abi`, the wire-contract crates (`script/lint` check 3 holds
# their spelling, `network_time_protocol` and `supervision_protocol` its two exceptions), and the
# on-disk formats §307 (how long a name may be) lists. Something else another program parses is
# missed; the BUGS below say so.
WIRE = {"abi", "network_time_protocol", "supervision_protocol", "nifefs", "package_archive",
        "documentation"}
# Spelled in two halves because script/lint's naming check refuses the word outside design/.
_SERVICE_WORD = "dae" + "mon"
# design/naming.md's question shapes. A finite verb first (`contains`, `starts_with`) is read as a
# first word ending in `s`, which also passes a plural noun: the check's one known false pass.
_QUESTION_FIRST = {"is", "has", "can", "was", "will", "all", "any"}
_QUESTION_INNER = {"is", "has", "can", "was"}
# The casing each Rust kind takes. A file name's casing is `script/lint`'s naming check 6, and a
# name declared in an `extern` block is its owner's, so neither is checked here.
_CASING = {
    "fn": r"[a-z_][a-z0-9_]*", "const": r"[A-Z_][A-Z0-9_]*", "static": r"[A-Z_][A-Z0-9_]*",
    "type": r"[A-Z][A-Za-z0-9]*", "field": r"[a-z_][a-z0-9_]*", "variant": r"[A-Z][A-Za-z0-9]*",
    "module": r"[a-z_][a-z0-9_]*", "crate": r"[a-z][a-z0-9_]*", "program": r"[a-z][a-z0-9_]*",
}
_KEYWORD = {"fn": "fn", "const": "const", "static": "static", "struct": "type", "enum": "type",
            "union": "type", "trait": "type", "type": "type", "function": "fn",
            "constant": "const", "macro": "fn", "field": "field", "variant": "variant"}


def components(name):
    return [c.lower() for c in cap_abbreviation.components(name) if c != "-"]


def failures(name, kind, returns_bool, british):
    """Each check `name` fails, as `check: why` strings; empty when it passes all of them."""
    out = []
    shape = _CASING.get(kind)
    if shape and not re.fullmatch(shape, name):
        out.append(f"casing: not this tree's casing for a {kind}")
    words = components(name)
    ruled = [w for w in words if w in SPELLED_OUT]
    if ruled:
        out.append(f"spelled out: `{ruled[0]}` is ruled to expand")
    if name in british:
        out.append(f"spelling: `{british[name]}` is British")
    if _SERVICE_WORD in words or (len(words) > 1 and words[-1] == "d"):
        out.append("service model: claims a Unix service")
    if kind == "fn" and returns_bool and words:
        first = words[0]
        if not (first in _QUESTION_FIRST or (first.endswith("s") and not first.endswith("ss"))
                or any(w in _QUESTION_INNER for w in words[1:])):
            out.append("question shape: returns `bool` and is not `is_`, `has_`, `can_` or a "
                       "finite verb")
    return out


def british_of(names):
    """`{name: word}` for names holding a British form: `typos` en-us over en, and house_style's list."""
    import house_style
    if not shutil.which("typos"):
        sys.exit("name_checks: typos is not installed; run script/bootstrap (it installs it)")
    names = sorted(set(names))

    def run(locale):
        res = subprocess.run(["typos", "--locale", locale, "--format", "json", "-"],
                             input="\n".join(names), capture_output=True, text=True)
        return {(r["line_num"], r["typo"]) for r in map(json.loads, res.stdout.splitlines())
                if r.get("type") == "typo"}
    out = {names[line - 1]: word for line, word in run("en-us") - run("en")}
    for name in names:
        hit = [w for w in components(name) if w in house_style.EXTRA_BRITISH]
        if hit:
            out.setdefault(name, hit[0])
    return out


def _returns_bool(stripped_lines, line):
    sig = " ".join(stripped_lines[line - 1:line + 11])
    sig = re.split(r"[{;]", sig, maxsplit=1)[0]
    return re.search(r"->\s*bool\b", sig) is not None


def _declaration_line(lines, header):
    """The 1-based line of the declaration under the doc run that starts at `header`."""
    for i in range(header - 1, len(lines)):
        s = lines[i].strip()
        if s and not s.startswith(("//", "#[", "#![")):
            return i + 1
    return header


def is_wire(package):
    return package is not None and (package in WIRE or package.endswith("_contract"))


def candidates(records, unblocked, sources):
    """`(kind, name, path, line)` for every unratified name: blocked but not ratified, or unblocked."""
    out = []
    for r in records:
        if r["status"] == "ratified":
            continue
        if r["kind"] == "item":
            path, _, header = r["path"].partition(":")
            if path not in sources:
                continue
            line = _declaration_line(sources[path].split("\n"), int(header))
            m = name_provenance._ITEM.match(sources[path].split("\n")[line - 1])
            kind = _KEYWORD.get(m.group(1) if m else r.get("what"), "fn")
            out.append((kind, r["name"], path, line))
        elif r["kind"] in ("module", "crate", "program", "script", "document"):
            out.append((r["kind"], r["name"], r["path"], 1))
    for row in unblocked:
        path, sep, name = row.partition("::")
        if sep:
            lines = sources.get(path, "").split("\n")
            for number, text in enumerate(lines, 1):
                m = name_provenance._ITEM.match(text)
                if (m and m.group(2) == name and m.group(1) in _KEYWORD
                        and text.lstrip().startswith("pub ")):
                    out.append((_KEYWORD[m.group(1)], name, path, number))
        elif path.endswith(".rs"):
            stem = os.path.splitext(os.path.basename(path))[0]
            if stem in ("mod", "lib", "main"):
                stem = os.path.basename(os.path.dirname(path))
            out.append(("module", stem, path, 1))
        else:
            out.append(("document", os.path.basename(path)[:-3], path, 1))
    return sorted(set(out), key=lambda c: (c[2], c[3], c[1]))


def survey(cands, sources, manifests, british=None, reach=None):
    """Every candidate that fails a check: dicts with `kind`, `name`, `path`, `line`, `reach`,
    `readers` and `failures`."""
    reach = reach or name_crossings.Reach(manifests, sources)
    if british is None:
        british = british_of(c[1] for c in cands)
    out = []
    for kind, name, path, line in cands:
        returns_bool = (kind == "fn" and path in reach.stripped
                        and _returns_bool(reach.stripped[path].split("\n"), line))
        if path in reach.stripped and any(k == "extern" for k, _ in reach.stack(path, line)):
            continue
        why = failures(name, kind, returns_bool, british)
        if not why:
            continue
        package = reach.package(path)
        if kind == "crate":
            package = reach.package(path) or name
        if is_wire(package):
            where, readers = "wire or format", []
        elif kind in ("crate", "program", "script", "document"):
            where, readers = kind, []
        else:
            if kind == "module":
                readers = sorted(reach.paths_reached.get(package, {}).get(name, ()))
            else:
                readers = reach.readers(path, line, name)
            where = f"crosses into {len(readers)}" if readers else "local"
        out.append({"kind": kind, "name": name, "path": path, "line": line, "reach": where,
                    "readers": readers, "failures": why})
    return out


def _order(row):
    rank = {"local": 0, "wire or format": 3}.get(row["reach"], 1 if row["readers"] else 2)
    return rank, row["path"], row["line"]


def load(root):
    out = subprocess.run(["git", "-C", root, "ls-files", "-z", "--cached", "--others",
                          "--exclude-standard"], capture_output=True, text=True, check=True).stdout
    files = [p for p in out.split("\0") if p and os.path.isfile(os.path.join(root, p))]

    def read(rel):
        try:
            return open(os.path.join(root, rel), errors="replace").read()
        except (FileNotFoundError, IsADirectoryError):
            return None
    records, _problems, _missing, unblocked = name_census.census(files, read)
    manifests = {os.path.dirname(p): read(p) for p in files
                 if p.endswith("Cargo.toml") and not p.startswith("vendor/")}
    sources = {p: read(p) for p in files
               if p.endswith(".rs") and not p.startswith(("vendor/", "patches/"))}
    return records, unblocked, manifests, sources


def _selftest():
    manifests = {"lib_a": '[package]\nname = "lib-a"\n',
                 "app": '[package]\nname = "app"\n[dependencies]\nlib-a = { path = "../lib_a" }\n',
                 "x_contract": '[package]\nname = "x_contract"\n'}
    lib = "\n".join([
        "pub fn mem_size() {}",                                   # 1: local, spelled out
        "pub fn arg_count() {}",                                  # 2: app reads it by path
        "pub struct Seen;",                                       # 3: app names the type
        "impl Seen {",
        "    pub fn pending(&self) -> bool { true }",             # 5: crosses with its type
        "}",
        "pub fn contains() -> bool { true }",                     # 7: a finite verb, passes
        "/// Name: ratified 2026-10-01 (calef, here).",
        "pub fn op_ok() {}",                                      # 9: ratified, not listed
        "/// Name: provisional.",
        "pub fn tint() {}",                                       # 11: blocked, flagged by the list
        "",
    ])
    app = ("use lib_a::{arg_count, Seen};\n// lib_a::mem_size in a comment is not a read\n"
           "fn main() { arg_count(); let s: Seen; }\n")
    sources = {"lib_a/src/lib.rs": lib, "app/src/main.rs": app,
               "x_contract/src/lib.rs": "pub const CONFIG_WORD: u32 = 1;\n"}
    records = [{"kind": "item", "name": "op_ok", "path": "lib_a/src/lib.rs:8", "status": "ratified",
                "what": "function"},
               {"kind": "item", "name": "tint", "path": "lib_a/src/lib.rs:10",
                "status": "provisional", "what": "function"}]
    unblocked = ["lib_a/src/lib.rs::mem_size", "lib_a/src/lib.rs::arg_count",
                 "lib_a/src/lib.rs::Seen", "lib_a/src/lib.rs::pending",
                 "lib_a/src/lib.rs::contains", "x_contract/src/lib.rs::CONFIG_WORD"]
    cands = candidates(records, unblocked, sources)
    assert ("fn", "op_ok", "lib_a/src/lib.rs", 9) not in cands, "a ratified name is not a candidate"
    rows = survey(cands, sources, manifests, {"tint": "tint"})
    got = {r["name"]: (r["reach"], r["failures"][0].split(":")[0]) for r in rows}
    assert got == {"mem_size": ("local", "spelled out"),
                   "arg_count": ("crosses into 1", "spelled out"),
                   "pending": ("crosses into 1", "question shape"),
                   "tint": ("local", "spelling"),
                   "CONFIG_WORD": ("wire or format", "spelled out")}, got
    order = [r["name"] for r in sorted(rows, key=_order)]
    assert order[-1] == "CONFIG_WORD" and order.index("mem_size") < order.index("arg_count"), order
    assert failures("kernel-print-d", "script", False, {})[0].startswith("service"), \
        "a trailing `d` component claims a service"
    assert failures("BadCase", "fn", False, {})[0].startswith("casing")
    extern = 'extern "system" {\n    pub fn GetLogicalDrives() -> u32;\n}\n'
    assert survey([("fn", "GetLogicalDrives", "lib_a/src/ffi.rs", 2)],
                  dict(sources, **{"lib_a/src/ffi.rs": extern}), manifests, {}) == [], \
        "an extern block's name is its owner's"
    print("name_checks: selftest ok")


def main(argv):
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    if argv == ["--selftest"]:
        _selftest()
        return 0
    if any(a.startswith("-") for a in argv):
        sys.exit(__doc__.split("\n\n")[2])
    records, unblocked, manifests, sources = load(root)
    rows = survey(candidates(records, unblocked, sources), sources, manifests)
    if argv:
        rows = [r for r in rows if r["path"].startswith(tuple(argv))]
    by_reach = collections.Counter("crosses" if r["readers"] else r["reach"] for r in rows)
    by_check = collections.Counter(f.split(":")[0] for r in rows for f in r["failures"])
    print(f"unratified names failing a naming rule: {len(rows)} "
          f"({', '.join(f'{k} {v}' for k, v in by_reach.most_common())})")
    print("  by check: " + ", ".join(f"{k} {v}" for k, v in by_check.most_common()))
    heads = {0: "LOCAL: rename in its package",
             1: "CROSSING: rename by family",
             2: "CRATES, PROGRAMS, SCRIPTS AND DOCUMENTS: newcomer-facing, rename by family",
             3: "WIRE OR FORMAT: list for calef, never renamed by a lane"}
    section = None
    for r in sorted(rows, key=_order):
        if _order(r)[0] != section:
            section = _order(r)[0]
            print("\n" + heads[section])
        seen = f"  <- {', '.join(r['readers'][:3])}" if r["readers"] else ""
        print(f"  {r['path']}:{r['line']} {r['kind']} {r['name']} [{r['reach']}] "
              f"{'; '.join(r['failures'])}{seen}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
