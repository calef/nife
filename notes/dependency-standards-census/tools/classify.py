"""Classify nife's third-party packages by replaying each image build's own cargo flags in `cargo tree`.

usage: python3 -I classify.py <nife-root> <out.tsv>

Each BUILD row below copies the flags of the command that produces something an image carries
(xtask/src/archive.rs build_programs, xtask/src/disk.rs redoxfs_server_build, xtask/src/uefi.rs,
xtask/src/farm/exerciser.rs build_jig, helpers/build-c-program.sh). A crate is
  shipped     if it is linked into a base/optional/sdk build,
  test-image  if only a test build (fixtures, system_tests, the exercisers) links it,
  build-time  if it is reached only through a proc-macro or a build script of those builds,
  host        if it appears in a lockfile and in none of the above (xtask, host tools, fuzz,
              dev-dependencies).
"""
import json, os, re, subprocess, sys

ROOT = os.path.abspath(sys.argv[1])
OUT = sys.argv[2]
CARGO = os.path.expanduser("~/.cargo/bin/cargo")
NONE = ["aarch64-unknown-none-softfloat", "riscv64imac-unknown-none-elf", "x86_64-unknown-none"]
NIFE = [os.path.join(ROOT, f"targets/{a}-unknown-nife.json") for a in ("aarch64", "riscv64", "x86_64")]
UEFI = ["aarch64-unknown-uefi", "x86_64-unknown-uefi"]
TLS_CONFIG = ["--config", os.path.join(ROOT, "cryptography_exerciser/.cargo/config.toml")]
JSON = ["-Zjson-target-spec"]

# (class, label, workspace dir, extra args, targets)
BUILDS = [
    ("shipped", "kernel", ".", ["-p", "kernel"], NONE),
    ("shipped", "components", ".", ["-p", "components"], NONE),
    ("shipped", "uefi_loader", ".", ["-p", "uefi_loader", "--features", "uefi"], UEFI),
    ("shipped", "redoxfs_server", "redoxfs_server", ["--no-default-features", "--features", "el0"], NONE),
    ("shipped", "jig", "jig", TLS_CONFIG + JSON, NIFE),
    ("shipped", "c_library", "c_library", JSON, NIFE),
    ("test-image", "fixtures", ".", ["-p", "fixtures"], NONE),
    ("test-image", "components+attackers", ".", ["-p", "components", "--features", "components/confinement_attackers"], NONE),
    ("test-image", "system_tests", ".", ["-p", "system_tests"], NONE),
    ("test-image", "std_exerciser", "std_exerciser", JSON, NIFE),
    ("test-image", "cryptography_exerciser", "cryptography_exerciser", JSON, NIFE),
    ("test-image", "pinned_tls_exerciser", "pinned_tls_exerciser", JSON, NIFE),
]
LOCK_WORKSPACES = [".", "jig", "c_library", "cryptography_provider", "pinned_tls_client", "std_exerciser",
                   "cryptography_exerciser", "pinned_tls_exerciser", "redoxfs_server", "tools/redoxfs_host",
                   "fuzz", "notes/packages/container-format-measurements/fuzz"]
RANK = {"shipped": 0, "test-image": 1, "build-time": 2, "host": 3}
LINE = re.compile(r"^(\d+)(\S+) v(\S+)(?: \((.*?)\))?(?: \(\*\))?$")

result = {}  # (name, version) -> {class, builds:set, path}
proc_macros = set()


def note(name, ver, cls, label, path):
    r = result.setdefault((name, ver), {"class": cls, "builds": set(), "path": path})
    if RANK[cls] < RANK[r["class"]]:
        r["class"], r["path"] = cls, path
    if RANK[cls] <= 2:
        r["builds"].add(f"{label}{'' if cls != 'build-time' else '(build)'}")


for ws in LOCK_WORKSPACES:
    meta = json.loads(subprocess.run([CARGO, "metadata", "--format-version", "1", "--locked", "--offline"],
                                     capture_output=True, text=True, cwd=os.path.join(ROOT, ws), check=True).stdout)
    for p in meta["packages"]:
        if any("proc-macro" in t["kind"] for t in p["targets"]):
            proc_macros.add((p["name"], p["version"]))
        if p["source"] is not None:
            note(p["name"], p["version"], "host", ws, (ws,))

failures = []
for cls, label, ws, extra, targets in BUILDS:
    for t in targets:
        cmd = [CARGO, "tree", "--locked", "--offline", "-e", "normal", "--prefix", "depth",
               "-f", "{p}", "--target", t] + extra
        proc = subprocess.run(cmd, capture_output=True, text=True, cwd=os.path.join(ROOT, ws))
        if proc.returncode != 0:
            failures.append((label, t, proc.stderr.strip().splitlines()[-1:]))
            continue
        # cargo tree prints [build-dependencies] as a separate section only in non-depth mode; in
        # depth mode a build edge is not marked, so ask again with -e build only to learn which
        # direct edges are build edges.
        stack = []  # (depth, name, ver, is_build_time)
        for raw in proc.stdout.splitlines():
            m = LINE.match(raw.strip())
            if not m:
                continue
            depth, name, ver, src = int(m.group(1)), m.group(2), m.group(3), m.group(4)
            while stack and stack[-1][0] >= depth:
                stack.pop()
            parent_bt = stack[-1][3] if stack else False
            bt = parent_bt or (name, ver) in proc_macros
            stack.append((depth, name, ver, bt))
            if src and not src.startswith("registry") and "crates.io" not in src and "/" in src:
                continue  # a path dependency: nife's own crate or vendored source
            path = tuple(s[1] for s in stack)
            note(name, ver, "build-time" if bt else cls, label, (label,) + path)

# Build-script dependencies and their own graphs: whatever -e normal,build adds over -e normal.
for cls, label, ws, extra, targets in BUILDS:
    for t in targets:
        cmd = [CARGO, "tree", "--locked", "--offline", "-e", "normal,build", "--prefix", "depth", "-f", "{p}",
               "--target", t] + extra
        proc = subprocess.run(cmd, capture_output=True, text=True, cwd=os.path.join(ROOT, ws))
        for raw in proc.stdout.splitlines():
            m = LINE.match(raw.strip())
            if not m or (m.group(4) and "/" in m.group(4)):
                continue
            key = (m.group(2), m.group(3))
            if key in result and result[key]["class"] == "host":
                result[key]["class"] = "build-time"
                result[key]["path"] = (label, "(build graph)", m.group(2))
            if key in result and RANK[result[key]["class"]] >= 2:
                result[key]["builds"].add(f"{label}(build)")

with open(OUT, "w") as fh:
    fh.write("crate\tversion\tclass\tproc_macro\tbuilds\texample_path\n")
    for (n, v), r in sorted(result.items(), key=lambda kv: (RANK[kv[1]["class"]], kv[0])):
        fh.write(f"{n}\t{v}\t{r['class']}\t{'yes' if (n, v) in proc_macros else ''}\t"
                 f"{','.join(sorted(r['builds']))}\t{' > '.join(r['path'])}\n")
from collections import Counter
print(Counter(r["class"] for r in result.values()))
print("failures:", failures)
