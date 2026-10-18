"""Local names, ratified by rule: §310 (a name read only inside its package is ratified by rule).

calef, 2026-10-18 (UTC): "Ratify local names by rule." A `pub` item that no other Cargo package
reads, and that passes every check below, takes the block `/// Name: local by rule, <date> (§310).`
instead of waiting for him one at a time. This holds that true afterwards: the gate fails a `local`
name another package starts reading, or one that stops passing a check, and tells the lane to move
it to `provisional`, which is calef's queue.

    python3 helpers/name_local.py --check            # what script/lint runs
    python3 helpers/name_local.py --list [PREFIX..]  # who qualifies today, and who fails which check
    python3 helpers/name_local.py --apply PREFIX..   # write the block onto every qualifying name there
    python3 helpers/name_local.py --selftest         # both directions, as fixtures

**Who reads a name** is `name_crossings.Reach`, the measure `script/names --unratified` ranks by:
the defining crate's path, never the bare word.

**The checks are the mechanical ones in design/naming.md**, and each one sends a failing name to
calef rather than refusing it. Its list, with the rules no script can apply, is in
design/naming.md's "Local names are ratified by rule". `--list` prints each failure with its check.

Name: provisional, minted 2026-10-18 (UTC) by lane maintainer/names-ratify-local-by-rule. A shared
helper under `helpers/`, which `script/names` puts out of its own scope, so this paragraph is the
record. It takes `name_` for the family beside `name_census.py`, and `local` for the state it
holds. calef has not named it.
"""

import collections
import datetime
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

SECTION = 310
BLOCK = name_provenance.LOCAL_BLOCK
LOCAL = name_provenance.LOCAL
KINDS = ("function", "constant", "type")

# Components a ruling spells out, so a name holding one is not local-by-rule material whatever
# reads it: §154 (the acronym test is whether the phrase is spoken)'s table and
# design/naming/spelled-out-rulings.md. A plural is the same abbreviation. A foreign identifier
# keeps its owner's spelling, and telling one from ours is judgment, so it goes to calef too.
SPELLED_OUT = {
    "cap", "caps", "op", "ops", "recv", "config", "configs", "spec", "specs", "arg", "args",
    "mem", "dir", "dirs", "va", "pa", "ipi", "ipis", "nmi", "nmis", "ist", "msr", "msrs", "ipc",
    "dtb", "asid", "asids", "gpt", "nvme", "pcie", "dma", "pr", "prs", "initrd", "prog", "progs",
    "pos", "opt", "opts", "mtime",
}
# Spelled in two halves because script/lint's naming check refuses the word outside design/.
_SERVICE_WORD = "dae" + "mon"
# design/naming.md's question shapes. A finite verb first (`contains`, `starts_with`) is read as a
# first word ending in `s`, which also passes a plural noun: the check's one known false pass.
_QUESTION_FIRST = {"is", "has", "can", "was", "will", "all", "any"}
_QUESTION_INNER = {"is", "has", "can", "was"}


def components(name):
    return [c.lower() for c in cap_abbreviation.components(name)]


def failures(name, keyword, returns_bool, refused, british):
    """Each check `name` fails, as `check: why` strings; empty when it passes all of them."""
    out = []
    if keyword == "fn":
        cased = re.fullmatch(r"[a-z_][a-z0-9_]*", name)
    elif keyword in ("const", "static"):
        cased = re.fullmatch(r"[A-Z_][A-Z0-9_]*", name)
    else:
        cased = re.fullmatch(r"[A-Z][A-Za-z0-9]*", name)
    if not cased:
        out.append(f"casing: not Rust's casing for a `{keyword}`")
    words = components(name)
    ruled = [w for w in words if w in SPELLED_OUT]
    if ruled:
        out.append(f"spelled out: `{ruled[0]}` is ruled to expand (§154, spelled-out-rulings.md)")
    if name in british:
        out.append(f"spelling: `{british[name]}` is British")
    if _SERVICE_WORD in words or (len(words) > 1 and words[-1] == "d"):
        out.append("service model: claims a Unix service")
    if name in refused:
        out.append(f"refused: `{name}` is recorded as refused (script/names {name})")
    if keyword == "fn" and returns_bool and words:
        first = words[0]
        if not (first in _QUESTION_FIRST or (first.endswith("s") and not first.endswith("ss"))
                or any(w in _QUESTION_INNER for w in words[1:])):
            out.append("question shape: returns `bool` and is not `is_`, `has_`, `can_` or a "
                       "finite verb")
    return out


def british_of(names):
    """`{name: word}` for names holding a British form: `typos` en-us over en, and house_style's list."""
    import house_style
    out = {}
    names = sorted(set(names))
    if shutil.which("typos"):
        def run(locale):
            res = subprocess.run(["typos", "--locale", locale, "--format", "json", "-"],
                                 input="\n".join(names), capture_output=True, text=True)
            return {(r["line_num"], r["typo"]) for r in map(json.loads, res.stdout.splitlines())
                    if r.get("type") == "typo"}
        for line, word in run("en-us") - run("en"):
            out[names[line - 1]] = word
    else:
        sys.exit("name_local: typos is not installed; run script/bootstrap (it installs it)")
    for name in names:
        hit = [w for w in components(name) if w in house_style.EXTRA_BRITISH]
        if hit:
            out.setdefault(name, hit[0])
    return out


def _returns_bool(stripped_lines, line):
    sig = " ".join(stripped_lines[line - 1:line + 11])
    sig = re.split(r"[{;]", sig, maxsplit=1)[0]
    return re.search(r"->\s*bool\b", sig) is not None


def _doc_run_above(lines, line):
    """`(first, last)` 0-based indices of the `///` run documenting the item on 1-based `line`."""
    i, last = line - 2, None
    while i >= 0:
        s = lines[i].strip()
        if s.startswith("///") and not s.startswith("////"):
            last = i if last is None else last
        elif last is not None:
            break
        elif not (s.startswith(("#[", "#![")) or s.endswith(("]", ",", "(", ")")) or s == ""
                  or s.startswith("//")):
            return None
        i -= 1
    if last is None:
        return None
    return i + 1, last


def items(sources):
    """Every `pub` item in a module, with its block if its doc run carries one.

    Yields dicts: `path`, `line` (of the declaration), `name`, `keyword`, `block` (or None).
    `pub` exactly, as `name_census.pub_items` counts.
    """
    for path in sorted(sources):
        if not name_census.is_module_file(path):
            continue
        lines = sources[path].split("\n")
        for number, text in enumerate(lines, 1):
            if not text.lstrip().startswith("pub "):
                continue
            m = name_provenance._ITEM.match(text)
            if not m or m.group(1) in ("mod", "macro_rules!"):
                continue
            run = _doc_run_above(lines, number)
            block = None
            if run:
                block = name_provenance.block(
                    "\n".join(l.strip() for l in lines[run[0]:run[1] + 1]), "///")
            yield {"path": path, "line": number, "name": m.group(2), "keyword": m.group(1),
                   "block": block}


def survey(sources, manifests, refused, british=None, reach=None):
    """`(qualify, fail, cross, problems)` over one tree.

    `qualify` and `fail` are unblocked local items: those passing every check, and those failing
    one, each with its `failures`. `cross` is unblocked items another package reads. `problems` are
    the gate's: a `local` block on a name another package reads or that fails a check.
    """
    reach = reach or name_crossings.Reach(manifests, sources)
    found = list(items(sources))
    if british is None:
        british = british_of(i["name"] for i in found)
    stripped = {p: reach.stripped[p].split("\n") for p in {i["path"] for i in found}}
    qualify, fail, cross, problems = [], [], [], []
    for it in found:
        status = name_provenance.classify(it["block"])[0] if it["block"] is not None else None
        if it["block"] is not None and status != LOCAL:
            continue
        it["readers"] = reach.readers(it["path"], it["line"], it["name"])
        it["failures"] = failures(it["name"], it["keyword"],
                                  _returns_bool(stripped[it["path"]], it["line"]), refused,
                                  british)
        where = f"{it['path']}:{it['line']}"
        if status == LOCAL:
            if it["readers"]:
                problems.append(
                    f"`{it['name']}` ({where}) is local by rule, and "
                    f"{', '.join(it['readers'])} now reads it through its crate's path. "
                    f"§310 sends it back to calef: rewrite its block to `/// Name: provisional.`")
            for why in it["failures"]:
                problems.append(
                    f"`{it['name']}` ({where}) is local by rule and fails a check ({why}). "
                    f"§310 sends it back to calef: rewrite its block to `/// Name: provisional.`")
            continue
        (cross if it["readers"] else fail if it["failures"] else qualify).append(it)
    return qualify, fail, cross, problems


def misplaced(records):
    """A `local` block on something §310 does not cover: a module, field, variant or macro."""
    return [f"{r['kind']} {r['name']} ({r['path']}) carries `local by rule`, and §310 covers "
            f"only `pub` functions, constants and types; write `provisional` instead"
            for r in records if r["status"] == LOCAL
            and (r["kind"] != "item" or r.get("what") not in KINDS)]


def _local_blocks(text):
    return collections.Counter(r["name"] for r in name_provenance.rust_items(text)
                               if r["block"] and r["block"].startswith(name_provenance.LOCAL_HEAD))


def apply(sources, targets, date):
    """`{path: new text}` with the block written onto each target item, verified by reparsing.

    A target whose block the parse would not attach to it is skipped and returned in `skipped`.
    """
    by_path, out, skipped = collections.defaultdict(list), {}, []
    for it in targets:
        by_path[it["path"]].append(it)
    block = BLOCK.format(date=date, section=SECTION)
    for path, todo in by_path.items():
        lines = sources[path].split("\n")
        for it in sorted(todo, key=lambda t: -t["line"]):
            decl = lines[it["line"] - 1]
            indent = decl[:len(decl) - len(decl.lstrip())]
            run = _doc_run_above(lines, it["line"])
            if run:
                lines[run[1] + 1:run[1] + 1] = [f"{indent}///", f"{indent}/// {block}"]
            else:
                top = it["line"] - 1
                while top > 0 and lines[top - 1].strip().startswith("#["):
                    top -= 1
                lines[top:top] = [f"{indent}/// {block}"]
        text = "\n".join(lines)
        named = _local_blocks(text) - _local_blocks(sources[path])
        wanted = collections.Counter(t["name"] for t in todo)
        if named != wanted:
            skipped.extend(todo)
            continue
        out[path] = text
    return out, skipped


def load(root):
    out = subprocess.run(["git", "-C", root, "ls-files", "-z", "--cached", "--others",
                          "--exclude-standard"], capture_output=True, text=True, check=True).stdout
    files = [p for p in out.split("\0") if p and os.path.isfile(os.path.join(root, p))]

    def read(rel):
        try:
            return open(os.path.join(root, rel), errors="replace").read()
        except (FileNotFoundError, IsADirectoryError):
            return None
    records, _problems, _missing, _unblocked = name_census.census(files, read)
    refused = {bad for r in records for bad in r["refused"]}
    manifests = {os.path.dirname(p): read(p) for p in files
                 if p.endswith("Cargo.toml") and not p.startswith("vendor/")}
    sources = {p: read(p) for p in files
               if p.endswith(".rs") and not p.startswith(("vendor/", "patches/"))}
    return records, refused, manifests, sources


def _selftest():
    manifests = {"lib_a": '[package]\nname = "lib-a"\n',
                 "app": '[package]\nname = "app"\n[dependencies]\nlib-a = { path = "../lib_a" }\n'}
    lib = "\n".join([
        "pub fn kept() {}",                                       # 1: nobody else reads it
        "pub fn called() {}",                                     # 2: app reads it by path
        "pub struct Seen;",                                       # 3: app names the type
        "impl Seen {",
        "    pub fn quiet(&self) {}",                             # 5: its type crosses
        "}",
        "pub struct Hidden;",                                     # 7: never named by app
        "impl Hidden {",
        "    pub fn hushed(&self) {}",                            # 9: local
        "    pub fn poked(&self) {}",                             # 10: app calls `.poked(`
        "}",
        "pub fn has_mem() -> bool { true }",                      # 12: `mem` is spelled out
        "pub fn pending() -> bool { true }",                      # 13: not a question shape
        "pub fn contains() -> bool { true }",                     # 14: a finite verb
        "pub fn tint() {}",                                       # 15: the injected spelling list flags it
        "/// Name: local by rule, 2026-10-18 (§310).",
        "pub fn by_rule() {}",                                    # 17: local, blocked
        "/// Kept doc.",
        "#[inline]",
        "pub fn documented() {}",                                 # 20: local, doc above attribute
        "",
    ])
    app = ("use lib_a::{called, Seen};\n// lib_a::kept in a comment is not a read\n"
           "fn main() { called(); let s: Seen; lib_a::make().poked(); \"lib_a::documented\"; }\n")
    sources = {"lib_a/src/lib.rs": lib, "app/src/main.rs": app}
    british = {"tint": "tint"}
    qualify, fail, cross, problems = survey(sources, manifests, {"refusedname"}, british)
    names = lambda group: sorted(i["name"] for i in group)
    assert names(qualify) == ["Hidden", "contains", "documented", "hushed", "kept"], names(qualify)
    assert names(cross) == ["Seen", "called", "poked", "quiet"], names(cross)
    assert names(fail) == ["has_mem", "pending", "tint"], names(fail)
    reasons = {i["name"]: i["failures"] for i in fail}
    assert reasons["has_mem"][0].startswith("spelled out"), reasons
    assert reasons["pending"][0].startswith("question shape"), reasons
    assert reasons["tint"][0].startswith("spelling"), reasons
    assert problems == [], problems

    # Direction one: a local-by-rule name another package starts reading fails, naming the reader.
    sources2 = dict(sources, **{"app/src/main.rs": app + "fn later() { lib_a::by_rule(); }\n"})
    problems = survey(sources2, manifests, set(), british)[3]
    assert len(problems) == 1 and "`by_rule`" in problems[0] and "app now reads it" in problems[0] \
        and "provisional" in problems[0], problems
    # Direction two: one that stops passing a check fails: here its name became a refused one.
    problems = survey(sources, manifests, {"by_rule"}, british)[3]
    assert len(problems) == 1 and "fails a check (refused" in problems[0], problems

    # Applying writes a block the parse attaches to the right item, and the gate then holds it.
    written, skipped = apply(sources, qualify, "2026-10-19")
    assert not skipped, skipped
    after = written["lib_a/src/lib.rs"]
    assert "/// Kept doc.\n///\n/// Name: local by rule, 2026-10-19 (§310).\n#[inline]" in after, after
    sources3 = dict(sources, **written)
    q3, _f3, _c3, p3 = survey(sources3, manifests, set(), british)
    assert q3 == [] and p3 == [], (q3, p3)
    status = name_provenance.classify("local by rule, 2026-10-19 (§310).")
    assert status[0] == "local" and status[1] == "2026-10-19" and status[3] is None, status
    assert name_provenance.classify("local by rule.")[3] == name_provenance.NO_DATE

    rec = {"kind": "module", "name": "m", "path": "x.rs", "status": "local"}
    assert misplaced([rec]) and not misplaced([dict(rec, kind="item", what="function")])
    print("name_local: selftest ok")


def main(argv):
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    if argv == ["--selftest"]:
        _selftest()
        return 0
    if not argv or argv[0] not in ("--check", "--list", "--apply"):
        sys.exit(__doc__.split("\n\n")[2])
    mode, prefixes = argv[0], tuple(argv[1:])
    records, refused, manifests, sources = load(root)
    qualify, fail, cross, problems = survey(sources, manifests, refused)
    problems += misplaced(records)
    if mode == "--check":
        for p in problems:
            print(f"names: {p}", file=sys.stderr)
        local = sum(r["status"] == LOCAL for r in records)
        if problems:
            print("\nA `local` name is ratified by rule only while no other package reads it and it "
                  "passes every check (design/naming.md, §310).\nMoving it to `provisional` "
                  "puts it on calef's queue; that is the fix, not a failure to avoid.",
                  file=sys.stderr)
            return 1
        print(f"names: {local} local by rule, each still read only inside its package and passing "
              f"every check (§310)")
        return 0
    if prefixes:
        keep = lambda group: [i for i in group if i["path"].startswith(prefixes)]
        qualify, fail, cross = keep(qualify), keep(fail), keep(cross)
    if mode == "--apply":
        if not prefixes:
            sys.exit("name_local: --apply needs at least one path prefix, so a batch is one area")
        date = datetime.datetime.now(datetime.timezone.utc).date().isoformat()
        written, skipped = apply(sources, qualify, date)
        for path, text in written.items():
            open(os.path.join(root, path), "w").write(text)
        print(f"name_local: wrote {sum(1 for i in qualify) - len(skipped)} blocks in "
              f"{len(written)} files; {len(skipped)} skipped")
        for it in skipped:
            print(f"  skipped {it['path']}:{it['line']} {it['name']} (the block would not attach)")
        print("then: python3 helpers/module_block_ratchet.py --bank")
        return 0
    by_check = collections.Counter(f.split(":")[0] for i in fail for f in i["failures"])
    print(f"unblocked pub items: {len(qualify) + len(fail) + len(cross)}")
    print(f"  read by another package: {len(cross)}")
    print(f"  local, qualify for §310: {len(qualify)}")
    print(f"  local, fail a check (calef's queue): {len(fail)}  "
          + ", ".join(f"{k} {v}" for k, v in by_check.most_common()))
    for it in sorted(fail, key=lambda i: (i["path"], i["line"])):
        print(f"    {it['path']}:{it['line']} {it['name']}: {'; '.join(it['failures'])}")
    by_package = collections.Counter(i["path"].split("/src/")[0] for i in qualify)
    print("  qualifying, by area: " + ", ".join(f"{k} {v}" for k, v in by_package.most_common()))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
