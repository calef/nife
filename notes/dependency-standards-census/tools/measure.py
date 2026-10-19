"""Hold one crate source tree to nife's own code measures, using nife's own helpers unchanged.

usage: python3 -I measure.py <nife-root> <out.tsv> <name>=<version>=<dir> ...

Run with -I and with this file outside every measured tree, so nothing in a downloaded crate can be
imported. Every measured tree is only read.

Scopes, chosen to match nife's own:
  shipped source  every .rs except tests/, benches/, examples/, fuzz/ and build.rs: the unsafe census
                  scope (helpers/rust_source.py HOST_ONLY leaves out code that does not run on nife).
  all source      every .rs in the package: the scope of §283 (a function is held to 100 code
                  lines by a lint ratchet) and of §266 (a Rust source file stays under 2,000
                  lines, held by a ratchet that only falls) is every Rust file, tests included.
                  §267 (a comment states the constraint as it is now) sets the comment-block
                  ceiling counted here.
"""
import os, re, sys

NIFE = os.path.abspath(sys.argv[1])
sys.path.insert(0, os.path.join(NIFE, "helpers"))
import rust_source as rs  # noqa: E402  nife's unsafe census and code-line counter
import function_length as fl  # noqa: E402  nife's §283 counter (clippy's too_many_lines, by text)
import comment_block_ratchet as cb  # noqa: E402  nife's §267 counter

OUT = sys.argv[2]
NON_SHIPPED_DIRS = ("tests/", "benches/", "examples/", "fuzz/")

# script/lint's `unsafe fn contracts` check, copied: an `unsafe fn` declaration and a trait-impl header.
DECL = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:default\s+)?(?:const\s+)?'
                  r'unsafe\s+(?:extern\s+"[^"]*"\s+)?fn\s+(\w+)')
IMPL_FOR = re.compile(r'^\s*(?:unsafe\s+)?impl\b.*\bfor\b')
GENERATED = re.compile(r'@generated|automatically generated|auto-generated|do not edit', re.I)
FORBID = re.compile(r'#!\[\s*(forbid|deny)\s*\(\s*[^)]*\bunsafe_code\b')
TEST_ATTR = re.compile(r'#\[\s*(?:\w+::)*test\b')


def read_tree(root):
    files = {}
    for dp, dns, fns in os.walk(root):
        dns[:] = [d for d in dns if d not in ("target", ".git")]
        for f in fns:
            if f.endswith(".rs"):
                p = os.path.join(dp, f)
                with open(p, encoding="utf-8", errors="replace") as fh:
                    files[os.path.relpath(p, root)] = fh.read()
    return files


def shipped(path):
    return not path.startswith(NON_SHIPPED_DIRS) and path != "build.rs" and "/tests/" not in path


def safety_comment_coverage(text, code):
    """Unsafe blocks with a SAFETY comment just above or on the same line.

    A text approximation of clippy's `undocumented_unsafe_blocks`, which nife enforces on its own
    code and which cannot run on a dependency without compiling it as a workspace member. It looks
    on the block's own line before `unsafe`, then walks up over comment, attribute and
    statement-continuation lines (up to eight) for a comment containing `SAFETY` in any case.
    Validated against nife's own tree, where clippy holds the rule at zero misses.
    """
    lines = text.split("\n")
    total = found = 0
    for m in rs.UNSAFE_BLOCK.finditer(code):
        if rs.UNSAFE_EXTERN.match(code, m.start()):
            continue
        total += 1
        ln = code.count("\n", 0, m.start())
        own = lines[ln]
        if re.search(r"safety", own[: own.find("unsafe")] if "unsafe" in own else own, re.I) or \
                re.search(r"/[/*].*safety", own, re.I):
            found += 1
            continue
        j, steps = ln - 1, 0
        while j >= 0 and steps < 8:
            s = lines[j].strip()
            if re.search(r"safety", s, re.I) and ("//" in s or s.startswith(("/*", "*"))):
                found += 1
                break
            if s.startswith(("//", "#[", "/*", "*")) or s == "" and steps == 0:
                j -= 1; steps += 1
                continue
            # a statement that opens on an earlier line: `let x =` then `unsafe {` on the next
            if s.endswith(("=", "(", ",", "=>", "{", "|")) or s.startswith((".", "&&", "||")):
                j -= 1; steps += 1
                continue
            break
        # else: no comment found
    return total, found


def unsafe_fn_contracts(files):
    total = in_trait = missing = 0
    for path, text in files.items():
        lines = text.split("\n")
        depth, trait_at = 0, None
        for i, line in enumerate(lines):
            if trait_at is None and IMPL_FOR.match(line):
                trait_at = depth
            m = DECL.match(line)
            if m:
                total += 1
                if trait_at is not None:
                    in_trait += 1
                else:
                    j, doc = i - 1, []
                    while j >= 0:
                        s = lines[j].strip()
                        if s.startswith(("///", "//!")):
                            doc.append(s); j -= 1
                        elif s.startswith(("#[", "#!", "//")):
                            j -= 1
                        else:
                            break
                    if not any(re.search(r"#\s*Safety", d, re.I) for d in doc):
                        missing += 1
            depth += line.count("{") - line.count("}")
            if trait_at is not None and depth <= trait_at:
                trait_at = None
    return total, in_trait, missing


def cargo_field(root, key):
    try:
        text = open(os.path.join(root, "Cargo.toml"), encoding="utf-8").read()
    except OSError:
        return ""
    m = re.search(rf'^{key}\s*=\s*"([^"]*)"', text, re.M)
    return m.group(1) if m else ""


def has_dir(root, name):
    return os.path.isdir(os.path.join(root, name))


def measure(name, version, root):
    files = read_tree(root)
    ship = {p: t for p, t in files.items() if shipped(p)}
    census = rs.unsafe_census(sorted(ship.items()))
    blocks = census["outside_arch"]
    code_lines = census["code_lines"]
    total_blocks = found = 0
    for p, t in ship.items():
        a, b = safety_comment_coverage(t, rs.strip_non_code(t))
        total_blocks += a; found += b
    ufn, ufn_trait, ufn_missing = unsafe_fn_contracts(ship)
    fns = fl.measure({p: t for p, t in files.items()})
    over_fn = [(p, n, c) for (p, n), (s, c, _l) in fns.items() if c > fl.THRESHOLD]
    over_fn_ship = [x for x in over_fn if shipped(x[0])]
    longest_fn = max((c for (_p, _n), (_s, c, _l) in fns.items()), default=0)
    phys = {p: t.count("\n") for p, t in files.items()}
    over_file = [p for p, n in phys.items() if n > 2000]
    generated_over = [p for p in over_file if GENERATED.search(files[p][:4000])]
    librs = files.get("src/lib.rs", "") or files.get("src/main.rs", "")
    forbid = FORBID.search(rs.strip_non_code(librs).replace(" ", " ")) or FORBID.search(librs)
    tests = sum(len(TEST_ATTR.findall(t)) for t in files.values())
    kani = any("kani::" in t or "#[kani" in t for t in files.values())
    miri_aware = any(re.search(r"cfg\s*\(\s*(not\s*\(\s*)?miri|cfg_attr\s*\(\s*miri", t) for t in files.values())
    comment_over = sum(len(cb.blocks_over(t)) for t in files.values())
    bugs = ""
    for f in os.listdir(root):
        if f.lower().startswith("readme"):
            txt = open(os.path.join(root, f), encoding="utf-8", errors="replace").read()
            if re.search(r"^#+\s*(BUGS|Known (issues|bugs|limitations)|Limitations|Caveats)\b", txt, re.M | re.I):
                bugs = "yes"
    return {
        "crate": name, "version": version,
        "rs_files": len(files), "shipped_code_lines": code_lines,
        "all_code_lines": sum(rs.non_blank(rs.strip_non_code(t)) for t in files.values()),
        "unsafe_blocks": blocks,
        "unsafe_per_10k": (10000 * blocks // code_lines) if code_lines else 0,
        "unsafe_impl_send_sync": census["thread_safety"], "unsafe_impl_trait": census["trait_claims"],
        "static_mut": census["static_mut"],
        "safety_comment_blocks": found,
        "safety_comment_pct": (100 * found // total_blocks) if total_blocks else "",
        "unsafe_fn": ufn, "unsafe_fn_trait_impl": ufn_trait, "unsafe_fn_no_safety_doc": ufn_missing,
        "forbid_or_deny_unsafe_code": "yes" if forbid else "",
        "fns_over_100": len(over_fn), "fns_over_100_shipped": len(over_fn_ship), "longest_fn": longest_fn,
        "files_over_2000": len(over_file), "files_over_2000_generated": len(generated_over),
        "longest_file": max(phys.values(), default=0),
        "test_attrs": tests, "tests_dir": "yes" if has_dir(root, "tests") else "",
        "fuzz_dir_in_package": "yes" if has_dir(root, "fuzz") else "",
        "kani_in_package": "yes" if kani else "", "miri_cfg_in_package": "yes" if miri_aware else "",
        "build_rs": "yes" if os.path.exists(os.path.join(root, "build.rs")) else "",
        "comment_blocks_over_40": comment_over, "readme_bugs_heading": bugs,
        "rust_version": cargo_field(root, "rust-version"), "edition": cargo_field(root, "edition"),
    }


rows = []
for arg in sys.argv[3:]:
    name, version, root = arg.split("=", 2)
    rows.append(measure(name, version, root))
cols = list(rows[0].keys())
with open(OUT, "w") as fh:
    fh.write("\t".join(cols) + "\n")
    for r in rows:
        fh.write("\t".join(str(r[c]) for c in cols) + "\n")
