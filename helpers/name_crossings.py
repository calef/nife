"""Which unratified names cross a Cargo package boundary, and what a program is, for `script/names`.

**Why this file exists.** On 2026-10-17 (UTC) calef ruled that a name crossing a crate boundary is
reviewed before a name internal to one crate: a crossing name is read by someone who does not own
the crate, so a wrong one costs more. Internal names still need ratifying later; the order is a
priority, not an exemption. `script/names --unratified` ranks the items and modules tier by it.

**The measure is a stated heuristic, not the compiler.** A compiler-backed answer (`cargo doc`
JSON, rust-analyzer) means building eight workspaces, minutes per run, and `script/names` is run
for a single name. A name defined in package P crosses when all of these hold:

1. It is visible outside P: an item or field declared `pub` with no restriction, a module declared
   `pub mod`. A variant has no `pub` of its own and passes.
2. Another package Q lists P as a dependency (any table, any target, renames followed).
3. A file in Q holds both the identifier of P (`designware_ethernet`) and the name as whole words.
   For a free function, constant, type or module (declared at column 0) it must also reach the name
   by a path (`p::a::name`) or a `use p...;` listing it. A method, field, variant or associated
   constant has no path to show, so it keeps the two-words test.

The two words are needed in one file because the heuristic that counted every file naming
the word reported 171 crossings (156 items, 15 modules), and its false hits were files that reached
the name without the crate: kernel's `sched/rendezvous/tests` through `xtask`, which does not depend
on `kernel`. Requiring the dependency edge removes those. What it still gets wrong: a name that
only shares its spelling with a use in Q (`new`, `len`), a glob import that never mentions the crate
beside the name, and a re-export that moves the name to another path. The first over-counts and the
other two under-count.

**A program is a `[[bin]]`** (calef's tiering, 2026-10-17). A file in `components/src/` that another
program includes through `mod` or `#[path]` and that has no `[[bin]]` entry is a module of the
program that includes it, and ranks as one.

Name: provisional, minted 2026-10-17 (UTC) by the lane that ranked crossings. A shared helper under
`helpers/`, which `script/names` puts out of its own scope, so this paragraph is the record. It sits
beside `name_provenance.py` and `name_rules.py`; `boundaries` was considered and collides with
`packages.py`'s package boundaries, which are nife packages and not Cargo's. calef has not named it.
"""

import os
import re
import sys

_DEP_TABLES = ("dependencies", "dev-dependencies", "build-dependencies")


def bin_paths(manifest_text):
    """The `src/...` path of every `[[bin]]` a manifest declares.

    Read by line rather than with `tomllib`, which is 3.11 and later: `script/names <name>` runs
    this on every call and must work under the older `python3` a machine may put first on PATH.
    """
    out, in_bin = set(), False
    for line in manifest_text.split("\n"):
        s = line.strip()
        if s.startswith("["):
            in_bin = s == "[[bin]]"
        elif in_bin:
            m = re.match(r'path\s*=\s*"([^"]+)"', s)
            if m:
                out.add(m.group(1))
    return out


def is_program(rel_in_src, bins):
    """True when `src/<rel>` (a file, or `<dir>/main.rs`) is a declared binary."""
    return "src/" + rel_in_src in bins


def includer_of(name, sources):
    """The file stem that declares `name` through `mod` or `#[path]`, from `{stem: text}`, or None."""
    pattern = re.compile(rf'#\[path\s*=\s*"{re.escape(name)}\.rs"\]|^\s*(?:pub\s+)?mod\s+{re.escape(name)}\s*;',
                         re.M)
    for stem in sorted(sources):
        if stem != name and pattern.search(sources[stem]):
            return stem
    return None


def parse_manifest(text):
    """`(package name, lib identifier, set of dependency package names)`, name None for a workspace."""
    import tomllib    # here and not at the top: only the ranking needs it (3.11 and later)
    doc = tomllib.loads(text)
    name = doc.get("package", {}).get("name")
    ident = doc.get("lib", {}).get("name") or (name or "").replace("-", "_")
    tables = [doc]
    tables += list(doc.get("target", {}).values())
    deps = set()
    for table in tables:
        for key in _DEP_TABLES:
            for dep, spec in table.get(key, {}).items():
                deps.add(spec.get("package", dep) if isinstance(spec, dict) else dep)
    return name, ident, deps


def package_of(path, dirs):
    """The package name owning `path`: the longest directory in `dirs` that is a prefix of it."""
    best = None
    for d in dirs:
        if (d == "" or path.startswith(d + "/")) and (best is None or len(d) > len(best)):
            best = d
    return dirs[best] if best is not None else None


def declaration(lines, header):
    """The declaration line under the doc run that starts at 1-based `header`, or ''."""
    for line in lines[header - 1:]:
        s = line.strip()
        if s and not s.startswith(("//", "#[", "#![")):
            return line
    return ""


def is_pub(lines, header, what):
    """Whether the item whose doc run starts at 1-based 'header' is declared `pub` with no scope."""
    if what == "variant":
        return True
    return re.match(r"pub\b(?!\s*\()", declaration(lines, header).strip()) is not None


def reached(text, ident, name, strict):
    """Whether `text` reaches `name` of crate `ident`.

    Strict is a path (`ident::a::name`) or a `use ident...;` that lists the name: what a free
    function, constant, type or module needs to be reachable from outside. The loose form is the
    two words in one file, for what a path cannot show: a method called on a value, a field read,
    a variant matched after its enum was imported.
    """
    if not strict:
        return True
    if re.search(rf"\b{re.escape(ident)}(?:::\w+)*::{re.escape(name)}\b", text):
        return True
    return any(re.search(rf"\b{re.escape(name)}\b", m.group(0))
               for m in re.finditer(rf"\buse\s+{re.escape(ident)}\b[^;]*;", text))


def crossings(cands, manifests, sources):
    """`{candidate key: (defining package, sorted referencing packages)}` for each candidate that crosses.

    `cands` is `[(key, name, file, what, header_line)]` with `what` an item kind or `module`.
    `manifests` is `{directory: manifest text}`; `sources` is `{path: text}` of every Rust file.
    """
    dirs, info = {}, {}
    for d, text in manifests.items():
        name, ident, deps = parse_manifest(text)
        if name:
            dirs[d], info[name] = name, (ident, deps)
    words = {p: set(re.findall(r"[A-Za-z_][A-Za-z0-9_]*", t)) for p, t in sources.items()}
    by_package = {}
    for path in sources:
        by_package.setdefault(package_of(path, dirs), []).append(path)
    out = {}
    for key, name, file, what, header in cands:
        owner = package_of(file, dirs)
        if owner is None or owner not in info:
            continue
        if what == "module":
            visible = any(re.search(rf"\bpub\s+mod\s+{re.escape(name)}\b", sources[p])
                          for p in by_package.get(owner, []))
        else:
            visible = is_pub(sources[file].split("\n"), header, what)
        if not visible:
            continue
        ident = info[owner][0]
        users = sorted(q for q, (_i, deps) in info.items() if owner in deps and q != owner)
        decl = declaration(sources[file].split("\n"), header)
        strict = (what in ("function", "constant", "type", "macro", "module")
                  and decl == decl.lstrip())
        hit = sorted({q for q in users for p in by_package.get(q, [])
                      if ident in words[p] and name in words[p]
                      and reached(sources[p], ident, name, strict)})
        if hit:
            out[key] = (owner, hit)
    return out


def _selftest():
    lib = '[package]\nname = "lib-a"\n'
    app = '[package]\nname = "app"\n[dependencies]\nlib-a = { path = "../lib_a" }\n'
    other = '[package]\nname = "other"\n'
    manifests = {"lib_a": lib, "app": app, "other": other}
    sources = {
        "lib_a/src/lib.rs": "/// Name: provisional.\npub fn exported() {}\n\n/// Name: provisional.\n"
                            "pub(crate) fn hidden() {}\n\n/// Name: provisional.\nfn private() {}\n\n"
                            "/// Name: provisional.\npub fn unused() {}\n\npub mod open;\nmod closed;\n",
        "lib_a/src/open.rs": "", "lib_a/src/closed.rs": "",
        "app/src/main.rs": "use lib_a::exported;\nfn main() { exported(); lib_a::hidden(); }\n",
        "other/src/lib.rs": "fn x() { exported(); unused(); }\n",
    }
    cands = [("exported", "exported", "lib_a/src/lib.rs", "function", 1),
             ("hidden", "hidden", "lib_a/src/lib.rs", "function", 4),
             ("private", "private", "lib_a/src/lib.rs", "function", 7),
             ("unused", "unused", "lib_a/src/lib.rs", "function", 10),
             ("open", "open", "lib_a/src/open.rs", "module", 1)]
    got = crossings(cands, manifests, sources)
    assert got == {"exported": ("lib-a", ["app"])}, got       # `other` names it without depending on lib-a
    sources["app/src/main.rs"] += "use lib_a::open;\n"
    assert crossings(cands, manifests, sources)["open"] == ("lib-a", ["app"])
    assert "private" not in got and "hidden" not in got, "a non-pub item cannot cross"

    manifests["app2"] = app.replace('"app"', '"app2"')
    sources["app2/src/main.rs"] = "// lib_a\nfn exported() {}\nfn main() { x.hidden(); }\n"
    assert crossings(cands, manifests, sources)["exported"] == ("lib-a", ["app"]), \
        "the crate's name and the item's name in one file are not a reference"

    manifest = '[[bin]]\nname = "net_stack"\npath = "src/net_stack.rs"\n'
    bins = bin_paths(manifest)
    assert is_program("net_stack.rs", bins)
    assert not is_program("socket_table.rs", bins), "a #[path]-included file is not a program"
    host = {"net_stack": '#[path = "socket_table.rs"]\nmod socket_table;\n', "socket_table": ""}
    assert includer_of("socket_table", host) == "net_stack"
    assert includer_of("net_stack", host) is None
    print("name_crossings: selftest ok")


if __name__ == "__main__":
    if sys.argv[1:] == ["--selftest"]:
        _selftest()
    else:
        sys.exit("usage: name_crossings.py --selftest")
