#!/usr/bin/env python3
"""A CI step that hangs leaves evidence before the job's `timeout-minutes` takes its log away.

    helpers/hang_evidence.py run -- <cmd...>   # every `script/ci-build` step in ci.yml
    helpers/hang_evidence.py lint               # script/lint: every such step is wrapped
    helpers/hang_evidence.py --selftest

Name: provisional, minted 2026-10-19 (UTC) by lane ci/hung-shard-leaves-evidence, as are the
`target/hang-evidence/` directory and the `hang-evidence-*` artifact names. A python helper under
`helpers/`, outside `script/names`' scope, so this paragraph is its provenance.

# WHY

On 2026-10-19 (#2413, run 39295240195) the shard `test-aarch64-riscv64` ran into its 25-minute
`timeout-minutes`. GitHub kept no log of that job, neither from the job log API (BlobNotFound) nor
in the run's log archive, and the steps that upload the time record never ran. The hang could not
be reproduced, so its cause could not be found. The stages the kernel suite's own watchdog does
not bound (`xtask/src/time_record.rs`: the build, and the host pass) end only at the job timeout,
and the job timeout is exactly the event that loses the evidence.

# THE RULE

- `run` reads the job's `timeout-minutes` from `ci.yml` (by `GITHUB_JOB`, through
  `helpers/job-budget.py`'s parser) and `JOB_STARTED_AT` from the job's first step, and gives the
  command until `RESERVE_SECONDS` before the job would be killed. The deadline is the job's, not
  the step's, so a cold QEMU build earlier in the job shortens it rather than being forgotten.
- The command's output passes through as it arrives and is kept whole in
  `$RUNNER_TEMP/hang-evidence-step.log`, so a partial log exists whatever GitHub keeps.
- At the deadline it writes `target/hang-evidence/`: `summary.md` (the job, the last leg banner, the
  last kernel test announced, libtest's long-running warnings, the output's tail), `ps.txt` (the
  whole process forest with wait channels), and per descendant process its command line, status,
  threads with their names and wait channels, kernel stack, open files and a `gdb` backtrace of
  every thread. libtest names a test's thread after the test, so
  the thread list names a hung host test. Then it kills the command's whole tree and exits 124, so
  the job's `if: always()` steps run and upload.
- Every job that runs one uploads `target/hang-evidence/` with
  `if: always() && hashFiles('target/hang-evidence/**') != ''`.
- Outside GitHub Actions (no `GITHUB_JOB`) it runs the command unbounded, as if it were not there.

`lint` is the rung-two mechanism: a `script/ci-build` step added to ci.yml without the wrapper, or a
job without the upload, fails `script/lint`.

# BUGS

- **The guest's own state is not captured.** QEMU runs without a monitor socket, so a wedged guest
  shows as its serial tail (in the step log) and QEMU's host threads, not as guest registers.
- **A runner that is itself wedged** (out of memory, lost) dumps nothing, because the dump runs on
  it. #2413's missing log might have been that; this cannot tell the two apart beforehand, but a
  hang that leaves no `hang-evidence` artifact now says the runner was the problem.
- **gdb is installed on demand** when the runner image lacks it, which spends up to
  `GDB_INSTALL_SECONDS` of the reserve. The summary says when it could not.
- `verify.yml`'s jobs are not covered: none of them runs `script/ci-build`.
"""

import collections
import importlib.util
import os
import re
import shutil
import signal
import subprocess
import sys
import threading
import time

WORKFLOW = ".github/workflows/ci.yml"
EVIDENCE_DIR = "target/hang-evidence"
# The dump, the kill and every later `if: always()` step (two artifact uploads, the time summary,
# the budget check) fit in this. Measured 2026-10-19 UTC on run 39326171835 (a hung host test with a
# paused QEMU child, five descendants): deadline to the job's end took 12 s, gdb's install included.
RESERVE_SECONDS = 240
DUMP_SECONDS = 150
GDB_INSTALL_SECONDS = 90
GDB_SECONDS = 20
TAIL_BYTES = 256 * 1024
WRAP = "helpers/hang_evidence.py run -- "
UPLOAD_IF = "if: always() && hashFiles('target/hang-evidence/**') != ''"


def _load_budget_module():
    here = os.path.dirname(os.path.abspath(__file__))
    spec = importlib.util.spec_from_file_location("job_budget", os.path.join(here, "job-budget.py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


jb = _load_budget_module()


def deadline_for(workflow_text, job, started):
    """Epoch seconds at which to give up on the step, or None when the job has no timeout."""
    body = jb.jobs_of(workflow_text).get(job)
    timeout = jb.timeout_of(body) if body is not None else None
    if timeout is None:
        return None
    return started + timeout * 60 - RESERVE_SECONDS


# The kernel suite announces a test as `test <path> ... ` with no newline until it finishes
# (xtask/src/time_record.rs, `stage_after`), so the last announcement without a verdict after it is
# the one running.
KERNEL_TEST = re.compile(r"test (\S+) \.\.\. ?(ok|FAILED|ignored)?")
# libtest prints a test only when it finishes, except for this warning, which is how a hung host
# test names itself.
LIBTEST_SLOW = re.compile(r"test \S+ has been running for over \d+ seconds")


def what_was_running(tail):
    """(last `==>` banner, last kernel test still running or None, libtest's long-running lines)."""
    banner, running, slow = None, None, []
    for line in tail.splitlines():
        if line.startswith("==>"):
            banner = line
        slow += LIBTEST_SLOW.findall(line)
        for m in KERNEL_TEST.finditer(line):
            running = None if m.group(2) else m.group(1)
    return banner, running, slow


def sh(args, timeout, sudo=False):
    """Output of a diagnostic command, or what went wrong. Never raises: a dump that crashes halfway
    leaves less than one that records its own failures."""
    if sudo:
        args = ["sudo", "-n"] + args
    try:
        r = subprocess.run(args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout)
        return r.stdout.decode(errors="replace")
    except (OSError, subprocess.SubprocessError) as e:
        return "(%s: %s)\n" % (" ".join(args), e)


def read(path, missing=None):
    try:
        with open(path, "rb") as f:
            return f.read().decode(errors="replace").replace("\0", " ")
    except OSError as e:
        return "(%s)" % e if missing is None else missing


def descendants(root):
    """`{pid: ppid}` for every live descendant of `root`, from `ps`, so a child that left the
    process group is still found."""
    out = sh(["ps", "-eo", "pid=,ppid="], 10)
    parent = {}
    for line in out.splitlines():
        parts = line.split()
        if len(parts) == 2 and parts[0].isdigit() and parts[1].isdigit():
            parent[int(parts[0])] = int(parts[1])
    found, frontier = {}, [root]
    while frontier:
        p = frontier.pop()
        for c, pp in parent.items():
            if pp == p and c not in found:
                found[c] = pp
                frontier.append(c)
    return found


def ensure_gdb():
    if shutil.which("gdb"):
        return "gdb present"
    out = sh(["apt-get", "install", "-y", "-qq", "--no-install-recommends", "gdb"],
             GDB_INSTALL_SECONDS, sudo=True)
    return "gdb installed on demand" if shutil.which("gdb") else "gdb unavailable:\n" + out[-2000:]


def dump_process(pid, outdir, gdb_ok):
    comm = read("/proc/%d/comm" % pid, "").strip() or "gone"
    d = os.path.join(outdir, "procs", "%d-%s" % (pid, re.sub(r"[^A-Za-z0-9_.-]", "_", comm)))
    os.makedirs(d, exist_ok=True)
    parts = {
        "cmdline": read("/proc/%d/cmdline" % pid),
        "status": read("/proc/%d/status" % pid),
        "wchan": read("/proc/%d/wchan" % pid),
        "stack": sh(["cat", "/proc/%d/stack" % pid], 5, sudo=True),
        "fds": sh(["ls", "-l", "/proc/%d/fd" % pid], 5, sudo=True),
    }
    threads = []
    for tid in sorted(os.listdir("/proc/%d/task" % pid) if os.path.isdir("/proc/%d/task" % pid) else []):
        t = "/proc/%d/task/%s" % (pid, tid)
        state = re.search(r"^State:\s*(.*)$", read(t + "/status"), re.M)
        threads.append("%s\t%s\t%s\t%s" % (tid, read(t + "/comm").strip(), read(t + "/wchan").strip(),
                                           state.group(1) if state else "?"))
    parts["threads"] = "tid\tname\twchan\tstate\n" + "\n".join(threads) + "\n"
    if gdb_ok:
        parts["backtrace"] = sh(["gdb", "-batch", "-nx", "-p", str(pid), "-ex", "info threads",
                                 "-ex", "thread apply all bt"], GDB_SECONDS, sudo=True)
    for name, text in parts.items():
        with open(os.path.join(d, name + ".txt"), "w") as f:
            f.write(text)
    return comm


def dump(root_pid, tail, log_path, job, cmd, started, outdir):
    """Write the evidence and return the one-line account for the annotation."""
    give_up = time.time() + DUMP_SECONDS
    os.makedirs(outdir, exist_ok=True)
    banner, running, slow = what_was_running(tail)
    with open(os.path.join(outdir, "ps.txt"), "w") as f:
        f.write(sh(["ps", "-eo", "pid,ppid,pgid,etimes,stat,wchan:32,pcpu,rss,args", "--forest"], 15))
    procs = descendants(root_pid)
    gdb = ensure_gdb() if procs else "no descendants"
    gdb_ok = shutil.which("gdb") is not None
    dumped = []
    for pid in sorted(procs):
        if time.time() > give_up:
            dumped.append("%d (skipped: dump out of time)" % pid)
            continue
        dumped.append("%d %s" % (pid, dump_process(pid, outdir, gdb_ok)))
    if log_path and os.path.exists(log_path):
        shutil.copy(log_path, os.path.join(outdir, "step.log"))
    elapsed = (time.time() - started) / 60.0
    account = "%s: the step had not exited after %.1f minutes of the job; killed %d s before " \
              "timeout-minutes. Last banner: %s. Kernel test running: %s." % (
                  job, elapsed, RESERVE_SECONDS, banner or "none", running or "none")
    with open(os.path.join(outdir, "summary.md"), "w") as f:
        f.write("# A hung step, %s UTC\n\n%s\n\n" % (time.strftime("%Y-%m-%d %H:%M:%S", time.gmtime()), account))
        f.write("- command: `%s`\n- %s\n" % (" ".join(cmd), gdb))
        f.write("- libtest's long-running tests:\n%s\n" % ("".join("  - `%s`\n" % s for s in slow) or "  - none\n"))
        f.write("- descendants dumped under `procs/`:\n%s\n" % "".join("  - %s\n" % d for d in dumped))
        f.write("## The last 200 lines of output\n\n```\n%s\n```\n" % "\n".join(tail.splitlines()[-200:]))
    return account


def kill_tree(proc):
    """TERM, then KILL, to the process group and to every descendant that left it."""
    for sig, wait in ((signal.SIGTERM, 10), (signal.SIGKILL, 5)):
        pids = list(descendants(proc.pid))
        try:
            os.killpg(proc.pid, sig)
        except OSError:
            pass
        for p in pids:
            try:
                os.kill(p, sig)
            except OSError:
                pass
        try:
            proc.wait(timeout=wait)
            if not descendants(proc.pid):
                return
        except subprocess.TimeoutExpired:
            pass


def run(cmd):
    job = os.environ.get("GITHUB_JOB")
    if not job:
        os.execvp(cmd[0], cmd)
    started_text = os.environ.get("JOB_STARTED_AT", "")
    if not started_text.isdigit():
        print("::error::JOB_STARTED_AT is %r; the job's first step did not record it" % started_text)
        return 1
    started = int(started_text)
    with open(WORKFLOW) as f:
        deadline = deadline_for(f.read(), job, started)
    if deadline is None:
        print("::error::%s job %s has no timeout-minutes to set a deadline from" % (WORKFLOW, job))
        return 1
    print("==> hang evidence: %s gives this step until %s UTC" % (
        job, time.strftime("%H:%M:%S", time.gmtime(deadline))), flush=True)

    log_path = os.path.join(os.environ.get("RUNNER_TEMP", "target"), "hang-evidence-step.log")
    os.makedirs(os.path.dirname(log_path), exist_ok=True)
    env = dict(os.environ, PYTHONUNBUFFERED="1")
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                            start_new_session=True, env=env)
    tail = collections.deque()
    tail_size = [0]
    lock = threading.Lock()

    def pump():
        out = sys.stdout.buffer
        with open(log_path, "ab") as log:
            while True:
                chunk = os.read(proc.stdout.fileno(), 65536)
                if not chunk:
                    return
                out.write(chunk)
                out.flush()
                log.write(chunk)
                log.flush()
                with lock:
                    tail.append(chunk)
                    tail_size[0] += len(chunk)
                    while tail_size[0] > TAIL_BYTES and len(tail) > 1:
                        tail_size[0] -= len(tail.popleft())

    reader = threading.Thread(target=pump)
    reader.start()
    try:
        code = proc.wait(timeout=max(0, deadline - time.time()))
        reader.join(timeout=30)
        return code
    except subprocess.TimeoutExpired:
        pass
    with lock:
        text = b"".join(tail).decode(errors="replace")
    account = dump(proc.pid, text, log_path, job, cmd, started, EVIDENCE_DIR)
    kill_tree(proc)
    print("\n==> hang evidence: %s" % account, flush=True)
    print("::error title=hung step::%s The evidence is the job's hang-evidence artifact." % account)
    return 124


def lint_workflow(path, text):
    problems = []
    for job, body in jb.jobs_of(text).items():
        steps = jb.steps_of(body) or []
        wrapped = False
        for step in steps:
            for line in step:
                code = line.strip()
                if code.startswith("#") or "script/ci-build" not in code:
                    continue
                if WRAP + "script/ci-build" not in code:
                    problems.append("%s job %s: `%s` is not run through `%s`" % (path, job, code, WRAP.strip()))
                wrapped = True
        if not wrapped:
            continue
        uploads = ["\n".join(s) for s in steps
                   if "actions/upload-artifact" in "\n".join(s) and EVIDENCE_DIR in "\n".join(s)]
        if not any(UPLOAD_IF in u for u in uploads):
            problems.append("%s job %s: no step uploads %s/ with `%s`" % (path, job, EVIDENCE_DIR, UPLOAD_IF))
    return problems


GOOD = """\
jobs:
  test-shard:
    timeout-minutes: 25
    steps:
      - run: echo "JOB_STARTED_AT=$(date +%s)" >> "$GITHUB_ENV"
      # script/ci-build in a comment is not a step
      - run: helpers/hang_evidence.py run -- script/ci-build ${{ matrix.check }}
      - name: Upload the hang evidence
        if: always() && hashFiles('target/hang-evidence/**') != ''
        uses: actions/upload-artifact@x
        with:
          path: target/hang-evidence/
  fmt-free:
    timeout-minutes: 5
    steps:
      - run: echo hi
"""


def selftest():
    ok = True

    def expect(label, got, want):
        nonlocal ok
        if got != want:
            ok = False
            print("FAIL %s: got %r want %r" % (label, got, want), file=sys.stderr)

    expect("deadline is the job's timeout less the reserve", deadline_for(GOOD, "test-shard", 1000),
           1000 + 25 * 60 - RESERVE_SECONDS)
    expect("no timeout, no deadline", deadline_for("jobs:\n  j:\n    steps:\n", "j", 0), None)
    expect("unknown job, no deadline", deadline_for(GOOD, "nope", 0), None)

    # The kernel suite's announcement with no verdict is the running test; one with a verdict is not.
    tail = ("==> kernel suite: aarch64\ntest a::one ... ok\ntest a::two ... "
            "test b::slow has been running for over 60 seconds\n")
    expect("running test and banner", what_was_running(tail),
           ("==> kernel suite: aarch64", "a::two", ["test b::slow has been running for over 60 seconds"]))
    expect("finished test is not running", what_was_running("test a::one ... ok\n")[1], None)

    expect("good fixture", lint_workflow("f", GOOD), [])
    bad = GOOD.replace("helpers/hang_evidence.py run -- script/ci-build", "script/ci-build")
    expect("unwrapped step", len(lint_workflow("f", bad)), 1)
    bad = GOOD.replace("        if: always() && hashFiles('target/hang-evidence/**') != ''\n", "")
    expect("upload without always()", len(lint_workflow("f", bad)), 1)

    print("hang_evidence selftest: %s" % ("ok" if ok else "FAILED"))
    return 0 if ok else 1


def main(argv):
    if argv[1:] == ["--selftest"]:
        return selftest()
    if len(argv) >= 4 and argv[1] == "run" and argv[2] == "--":
        return run(argv[3:])
    if argv[1:] == ["lint"]:
        with open(WORKFLOW) as f:
            problems = lint_workflow(WORKFLOW, f.read())
        for p in problems:
            print("hang_evidence: " + p, file=sys.stderr)
        return 1 if problems else 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    # `os._exit`, not `sys.exit`: a descendant that escaped the kill can still hold the output pipe,
    # and the reader thread blocked on it must not keep the step alive past its verdict.
    status = main(sys.argv)
    sys.stdout.flush()
    os._exit(status)
