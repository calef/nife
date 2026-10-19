#!/usr/bin/env python3
"""Write and read the per-job time record from a gate that is not `cargo xtask`.

    NIFE_TIME_RECORD=f helpers/time_record.py run <leg> <image> <kind> <name> -- <command...>
    helpers/time_record.py lint            # script/lint: every ci.yml checkout job uploads one or is exempt
    helpers/time_record.py --selftest

Milestone 808 (every gate accounts for its time), under §254 (a gate prints what each item cost,
and a job near its budget warns rather than fails). The record is the tab-separated file milestone
807 (the kernel suite reports what each test cost) defined in `xtask/src/time_record.rs`: a header,
then `leg, image, kind, name, milliseconds` rows. CI uploads it as the job's `time-record-<job>`
artifact and `helpers/ci_job_times.py update` folds it into the daily record.

`cargo xtask` writes its own rows in Rust. This is the one writer for everything else, so the
format has two implementations, one per language, and the selftest holds them to the same header
by reading the Rust constant. `script/fuzz` times each target through `run`; `script/falsifications`
imports `append` for each replayed record.

# The rule for who owns the file

The kernel suite owns `target/time-record.tsv`, which `cargo xtask test` empties at its start.
Every other writer appends only to `NIFE_TIME_RECORD`, and writes nothing when it is unset, so a
person running `script/fuzz` at a laptop does not grow a file nobody uploads. `run` still runs the
command and returns its exit status either way.

# The lint (milestone 808's exit criterion 2)

Every `ci.yml` job that checks out the tree has an `actions/upload-artifact` step whose `name:` is
`time-record-<job>`, or `time-record-<job>-` and a matrix value, or a line in `EXEMPTIONS` with its
reason. The artifact name carries the job key because the artifacts API does not say which job
uploaded what, and the collector needs it. An exemption for a job that uploads, or for no job, is
refused, so the file cannot hold a stale reason.

# BUGS

- A tab or a newline in a name would split a row, so `append` turns each into a space. The name
  in the record is then not byte-for-byte the item's own.
- `run` times the whole command, build included. For `script/fuzz` that is cargo-fuzz's build
  of the target plus the fixed 60 s; the build is the part that varies.

Name: provisional, minted 2026-10-19 (UTC) by milestone 808's lane, as is `run`. A python helper
under `helpers/`, outside `script/names`' scope, so this paragraph is its provenance.
"""

import importlib.util
import os
import re
import subprocess
import sys
import tempfile
import threading
import time

HEADER = "leg\timage\tkind\tname\tmilliseconds"
WORKFLOW = ".github/workflows/ci.yml"
EXEMPTIONS = ".github/time-record-exemptions"
ARTIFACT = "time-record-"

# `script/falsifications` replays on worker threads, and two first appends must not both write the
# header.
_LOCK = threading.Lock()
RUST_SOURCE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "xtask", "src",
                           "time_record.rs")


def path():
    return os.environ.get("NIFE_TIME_RECORD") or None


def clean(field):
    return re.sub(r"[\t\r\n]", " ", field)


def append(rows, to=None):
    """Append `(leg, image, kind, name, milliseconds)` rows to `to`, or to `NIFE_TIME_RECORD`.
    Returns False when there is nowhere to write. Best effort: a record that cannot be written must
    not change a gate's verdict."""
    to = to or path()
    if not to:
        return False
    try:
        os.makedirs(os.path.dirname(os.path.abspath(to)), exist_ok=True)
        with _LOCK, open(to, "a") as f:
            fresh = f.tell() == 0
            if fresh:
                f.write(HEADER + "\n")
            for leg, image, kind, name, ms in rows:
                f.write("%s\t%s\t%s\t%s\t%d\n" % (clean(leg), clean(image), clean(kind), clean(name),
                                                    int(ms)))
    except OSError as e:
        print("time_record: cannot write %s: %s" % (to, e), file=sys.stderr)
        return False
    return True


def parse(text):
    """The rows of a record file as dicts. Raises ValueError on a wrong header or a bad row."""
    lines = text.splitlines()
    if not lines or lines[0] != HEADER:
        raise ValueError("the first line is not the time record's header")
    rows = []
    for n, line in enumerate(lines[1:], 2):
        if not line:
            continue
        f = line.split("\t")
        if len(f) != 5 or not f[4].isdigit():
            raise ValueError("row %d does not parse: %r" % (n, line))
        rows.append({"leg": f[0], "image": f[1], "kind": f[2], "name": f[3], "milliseconds": int(f[4])})
    return rows


def _job_budget():
    """helpers/job-budget.py, which owns the workflow's job and step parser. Loaded by path because
    its file name has a hyphen."""
    here = os.path.dirname(os.path.abspath(__file__))
    spec = importlib.util.spec_from_file_location("job_budget", os.path.join(here, "job-budget.py"))
    jb = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(jb)
    return jb


def parse_exemptions(text):
    """`{job: reason}` and a list of problems."""
    out, problems = {}, []
    for n, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split(None, 1)
        if len(parts) < 2:
            problems.append("%s:%d: %s has no reason" % (EXEMPTIONS, n, parts[0]))
            continue
        out[parts[0]] = parts[1]
    return out, problems


def uploads_record(job, steps):
    """Whether one of `steps` uploads this job's record under a name the collector can map back."""
    for s in steps:
        text = "\n".join(s)
        if "actions/upload-artifact" not in text:
            continue
        m = re.search(r"^\s+name:\s*(.+?)\s*$", text.split("with:", 1)[-1], re.M)
        if m and (m.group(1) == ARTIFACT + job or m.group(1).startswith(ARTIFACT + job + "-${{")):
            return True
    return False


def lint_workflow(text, exemptions):
    jb = _job_budget()
    problems, jobs = [], jb.jobs_of(text)
    for job, body in jobs.items():
        steps = jb.steps_of(body) or []
        if not any("actions/checkout" in "\n".join(s) for s in steps):
            continue
        up = uploads_record(job, steps)
        if up and job in exemptions:
            problems.append("%s: %s uploads its record and is also exempt; drop the line" % (EXEMPTIONS, job))
        elif not up and job not in exemptions:
            problems.append("%s job %s: upload `%s%s` from its time record, or give it a line in %s "
                            "with the reason it has none" % (WORKFLOW, job, ARTIFACT, job, EXEMPTIONS))
    problems += ["%s: %s names no job in %s" % (EXEMPTIONS, j, WORKFLOW) for j in exemptions if j not in jobs]
    return problems


def lint(root="."):
    try:
        exemptions, problems = parse_exemptions(open(os.path.join(root, EXEMPTIONS)).read())
    except FileNotFoundError:
        exemptions, problems = {}, []
    return problems + lint_workflow(open(os.path.join(root, WORKFLOW)).read(), exemptions)


def run(leg, image, kind, name, argv):
    began = time.monotonic()
    code = subprocess.call(argv)
    append([(leg, image, kind, name, (time.monotonic() - began) * 1000)])
    return code


def selftest():
    ok = True

    def expect(label, got, want):
        nonlocal ok
        if got != want:
            ok = False
            print("FAIL %s: got %r want %r" % (label, got, want), file=sys.stderr)

    # The two writers agree on the header, or the collector reads one of them as garbage.
    rust = open(RUST_SOURCE).read()
    m = re.search(r'pub\(crate\) const HEADER: &str = "([^"]*)";', rust)
    expect("the Rust header is found", bool(m), True)
    if m:
        expect("the Rust and Python headers agree", m.group(1).replace("\\t", "\t"), HEADER)

    with tempfile.TemporaryDirectory() as d:
        f = os.path.join(d, "r.tsv")
        append([("host", "fuzz", "target", "a\tb", 1500.7)], to=f)
        append([("host", "fuzz", "target", "c", 2)], to=f)
        text = open(f).read()
        expect("one header for two appends", text.count(HEADER), 1)
        rows = parse(text)
        expect("a tab in a name becomes a space", rows[0]["name"], "a b")
        expect("milliseconds are whole", rows[0]["milliseconds"], 1500)
        expect("round trip", [r["name"] for r in rows], ["a b", "c"])
        try:
            parse("leg\n")
            expect("a wrong header is refused", False, True)
        except ValueError:
            pass
        try:
            parse(HEADER + "\na\tb\tc\td\n")
            expect("a short row is refused", False, True)
        except ValueError:
            pass

        # `run` returns the command's own status and records it either way.
        env = dict(os.environ, NIFE_TIME_RECORD=os.path.join(d, "run.tsv"))
        code = subprocess.call([sys.executable, os.path.abspath(__file__), "run", "host", "fuzz",
                                "target", "t", "--", "sh", "-c", "exit 3"], env=env)
        expect("run keeps the exit status", code, 3)
        expect("run recorded the failing command",
               [r["name"] for r in parse(open(env["NIFE_TIME_RECORD"]).read())], ["t"])
        env = dict(os.environ)
        env.pop("NIFE_TIME_RECORD", None)
        code = subprocess.call([sys.executable, os.path.abspath(__file__), "run", "host", "fuzz",
                                "target", "t", "--", "true"], env=env, cwd=d)
        expect("run without NIFE_TIME_RECORD still runs", code, 0)
        expect("and writes nothing", sorted(os.listdir(d)), ["r.tsv", "run.tsv"])

    # The lint: an uploading job passes, a silent one fails, an exemption covers it, and a stale
    # exemption is refused.
    good = """\
jobs:
  quick:
    runs-on: x
    steps:
      - uses: actions/checkout@v5
      - run: make
      - name: Upload the time record
        uses: actions/upload-artifact@v7
        with:
          name: time-record-quick-${{ matrix.leg }}
          path: target/time-record.tsv
  plain:
    runs-on: x
    steps:
      - uses: actions/checkout@v5
      - run: make
  nocheckout:
    runs-on: x
    steps:
      - run: gh api x
"""
    expect("an uploading job and an exempt one pass", lint_workflow(good, {"plain": "one command"}), [])
    expect("a job with neither is named", len(lint_workflow(good, {})), 1)
    expect("an exempt job that uploads is refused", len(lint_workflow(good, {"plain": "x", "quick": "x"})), 1)
    expect("an exemption for no job is refused", len(lint_workflow(good, {"plain": "x", "gone": "x"})), 1)
    expect("another job's artifact name does not count",
           len(lint_workflow(good.replace("time-record-quick-", "time-record-other-"), {"plain": "x"})), 1)
    expect("an exemption needs a reason", len(parse_exemptions("fmt\n# c\nlint why\n")[1]), 1)

    print("time_record selftest: %s" % ("ok" if ok else "FAILED"))
    return 0 if ok else 1


def main(argv):
    if argv[1:] == ["--selftest"]:
        return selftest()
    if argv[1:] == ["lint"]:
        problems = lint()
        for p in problems:
            print("time_record: " + p, file=sys.stderr)
        return 1 if problems else 0
    if len(argv) >= 8 and argv[1] == "run" and argv[6] == "--":
        return run(argv[2], argv[3], argv[4], argv[5], argv[7:])
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
