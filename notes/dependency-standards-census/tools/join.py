"""Join the census sources into one row per crate version and count passes per nife standard.

usage: python3 -I join.py <scratch> <out.tsv>
"""
import csv, os, sys

S = sys.argv[1]
OUT = sys.argv[2]


def load(path, key=("crate", "version")):
    with open(os.path.join(S, path)) as f:
        return {tuple(r[k] for k in key): r for r in csv.DictReader(f, delimiter="\t")}


meas = load("measure.tsv")
reg = load("facts/registry.tsv")
prac = load("upstream/practices.tsv")
tests = load("tests.tsv")
miri = load("miri.tsv")
classes = {}
for line in open(os.path.join(S, "measure-list.tsv")):
    n, v, c = line.rstrip("\n").split("\t")
    classes[(n, v)] = c
deps = load("deps2.tsv")
classes[("vendor-redoxfs", "0.9.1")] = "shipped"
classes[("vendor-relibc", "pinned")] = "shipped"

UNSAFE_CEILING = 60  # notes/unsafe-obligations/ceilings.md: nife's own density ceiling


def yes(v):
    return v.startswith("yes")


rows = []
for key, m in meas.items():
    r = {"crate": key[0], "version": key[1], "class": classes[key]}
    d = deps.get(key)
    r["builds"] = d["builds"] if d else ("redoxfs_server" if key[0] == "vendor-redoxfs" else
                                          "c_library" if key[0] == "vendor-relibc" else "")
    for c in ("shipped_code_lines", "unsafe_blocks", "unsafe_per_10k", "safety_comment_pct",
              "unsafe_impl_send_sync", "static_mut", "unsafe_fn", "unsafe_fn_no_safety_doc",
              "forbid_or_deny_unsafe_code", "fns_over_100", "longest_fn", "files_over_2000",
              "files_over_2000_generated", "longest_file", "comment_blocks_over_40", "test_attrs",
              "kani_in_package", "build_rs", "rust_version", "readme_bugs_heading"):
        r[c] = m[c]
    t = tests.get(key, {})
    r["package_tests"] = (f"{t['build']}:{t['passed']}p/{t['failed']}f" if t else "not run")
    mi = miri.get(key)
    r["miri_sample"] = f"{mi['result']}:{mi['passed']}p/{mi['failed']}f/ub{mi['ub_reported']}" if mi else ""
    p = prac.get(key, {})
    for c in ("ci_runs_tests", "miri_in_ci", "kani_or_proofs", "fuzz_targets", "oss_fuzz",
              "mutation_testing_in_ci", "msrv_tested_in_ci", "clippy_in_ci", "semver_checks_in_ci",
              "contributors_count", "top_contributor_share", "last_commit_date", "archived", "audits"):
        r["up_" + c] = p.get(c, "")
    g = reg.get(key, {})
    for c in ("owners_count", "releases_last_24_months", "reverse_dependencies_count",
              "is_newest_semver_compatible", "rustsec_all_count", "rustsec_vuln_ids",
              "rustsec_info_ids", "rustsec_affecting_measured", "in_rust_std", "in_rustc_tree",
              "in_chromium", "in_android", "in_fuchsia", "vet_combined_builtin",
              "vet_trusted_publisher_covers", "vet_google_status"):
        r["reg_" + c] = g.get(c, "")
    rows.append(r)

cols = list(rows[0].keys())
with open(OUT, "w") as f:
    f.write("\t".join(cols) + "\n")
    for r in sorted(rows, key=lambda r: (r["class"], r["crate"], r["version"])):
        f.write("\t".join(str(r[c]).replace("\t", " ") for c in cols) + "\n")


def i(v):
    try:
        return int(v)
    except ValueError:
        return 0


def present(v):
    return bool(v) and not v.startswith("no")


TESTS = {
    "unsafe density <= 60 per 10k": lambda r: i(r["unsafe_per_10k"]) <= UNSAFE_CEILING,
    "SAFETY comment on every block (text check)": lambda r: i(r["unsafe_blocks"]) == 0 or r["safety_comment_pct"] == "100",
    "SAFETY comment on >= 90% of blocks": lambda r: i(r["unsafe_blocks"]) == 0 or i(r["safety_comment_pct"]) >= 90,
    "no unsafe at all": lambda r: i(r["unsafe_blocks"]) == 0 and i(r["unsafe_fn"]) == 0,
    "# Safety on every unsafe fn": lambda r: i(r["unsafe_fn_no_safety_doc"]) == 0,
    "no file over 2,000 lines": lambda r: i(r["files_over_2000"]) == 0,
    "no hand-written file over 2,000 lines": lambda r: i(r["files_over_2000"]) - i(r["files_over_2000_generated"]) == 0,
    "no function over 100 code lines": lambda r: i(r["fns_over_100"]) == 0,
    "no comment block over 40 lines": lambda r: i(r["comment_blocks_over_40"]) == 0,
    "package tests build and all pass": lambda r: r["package_tests"].startswith("ok") and "/0f" in r["package_tests"] and not r["package_tests"].startswith("ok:0p"),
    "upstream CI runs tests": lambda r: yes(r["up_ci_runs_tests"]),
    "upstream CI runs Miri": lambda r: yes(r["up_miri_in_ci"]),
    "upstream has fuzz targets or OSS-Fuzz": lambda r: (r["up_fuzz_targets"] not in ("", "none") and not r["up_fuzz_targets"].startswith("n/a")) or yes(r["up_oss_fuzz"]),
    "Kani or other formal proofs": lambda r: r["kani_in_package"] == "yes" or (r["up_kani_or_proofs"] and not r["up_kani_or_proofs"].startswith(("none", "n/a"))),
    "Kani proofs specifically": lambda r: r["kani_in_package"] == "yes",
    "upstream mutation testing in CI": lambda r: yes(r["up_mutation_testing_in_ci"]),
    "MSRV declared": lambda r: bool(r["rust_version"]),
    "MSRV tested in upstream CI": lambda r: yes(r["up_msrv_tested_in_ci"]),
    "upstream clippy in CI": lambda r: yes(r["up_clippy_in_ci"]),
    "upstream semver-checks in CI": lambda r: yes(r["up_semver_checks_in_ci"]),
    "README has a BUGS/known-issues heading": lambda r: r["readme_bugs_heading"] == "yes",
    "no advisory affects the measured version": lambda r: r["reg_rustsec_affecting_measured"] in ("", "none"),
    "no vulnerability advisory ever": lambda r: not r["reg_rustsec_vuln_ids"],
    "cargo-vet audit reaches version (union of 5 sets)": lambda r: present(r["reg_vet_combined_builtin"]) and r["reg_vet_combined_builtin"] != "none",
    "in rustc, Chromium, Android or Fuchsia": lambda r: any(present(r[k]) for k in ("reg_in_rustc_tree", "reg_in_chromium", "reg_in_android", "reg_in_fuchsia")),
    "two or more owners": lambda r: i(r["reg_owners_count"]) >= 2,
    "a release in the last 24 months": lambda r: i(r["reg_releases_last_24_months"]) > 0,
    "repository not archived": lambda r: r["up_archived"] != "yes",
}
groups = {
    "shipped": [r for r in rows if r["class"] == "shipped" and not r["crate"].startswith("vendor-")],
    "vendored": [r for r in rows if r["crate"].startswith("vendor-")],
    "candidates": [r for r in rows if r["class"].startswith("candidate:")],
}
print("standard\t" + "\t".join(f"{g} ({len(v)})" for g, v in groups.items()))
for name, fn in TESTS.items():
    print(name + "\t" + "\t".join(str(sum(1 for r in v if fn(r))) for v in groups.values()))
with open(os.path.join(S, "passfail.tsv"), "w") as f:
    f.write("crate\tversion\tclass\t" + "\t".join(TESTS) + "\n")
    for r in rows:
        f.write(f"{r['crate']}\t{r['version']}\t{r['class']}\t" + "\t".join("pass" if fn(r) else "FAIL" for fn in TESTS.values()) + "\n")
