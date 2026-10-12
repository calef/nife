#!/usr/bin/env python3
"""List, and on request delete, the `target/` of every worktree whose pull request has merged.

    python3 helpers/prune_build_output.py            # dry run: say what would go, and how big it is
    python3 helpers/prune_build_output.py --remove   # delete exactly those
    python3 helpers/prune_build_output.py --selftest

PROVISIONAL NAME, minted 2026-10-12 (UTC) by maintainer/prune-merged-build-output; calef has not
ruled on it. Spelled like its sibling, helpers/prune_toolchains.py.

Why this exists. briefs/merge-and-cleanup.md step 3 says to remove a lane's worktree once its pull
request merges, because the branch deletion reclaims none of the 2 to 10 GB of `target/` behind it.
Nothing ran that step. On 2026-10-12 the data volume reached 100% with 32 merged lanes' worktrees
still holding their build output; every lane then running failed its builds and pushes with ENOSPC,
and deleting those 32 `target/` directories freed 140 GB. calef asked the same day for the cleanup
to run on its own once a pull request merges.

It deletes build output only. The worktree, its branch and any uncommitted work stay, because
`git worktree remove` is the step that can lose work and the brief keeps it a person's. Build output
is rebuilt by the next `cargo` in that worktree, so a wrong deletion costs one build.

What is kept, and so never touched:

  - the main checkout, and any worktree with no branch checked out;
  - a worktree whose branch has an open pull request, or none at all (a held branch, or a lane
    that has not pushed yet);
  - a `target/` written to in the last RECENT_MINUTES, because a build may be running there. A lane
    that reopens a merged branch is caught by this too.

It runs on this Mac from launchd, hourly, beside the other jobs; the plist is kept off-tree by the
maintainers. It cannot run in CI, which never sees this machine's worktrees.

BUGS
  - Merged means GitHub says so for that head branch, from the newest 1,000 merged pull requests.
    A lane older than that keeps its `target/` until someone runs the brief by hand.
  - The recent-write check reads the modification time of `target/` and its first level only, so a
    build that writes only deep inside an old directory can look idle. A deletion under a running
    cargo fails that build, which a rerun fixes.
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import time

RECENT_MINUTES = 60


def run(*args, **kw):
    return subprocess.run(args, capture_output=True, text=True, **kw)


def worktrees(repo):
    """(path, branch) for every worktree but the main one; branch is None when detached."""
    out = run("git", "-C", repo, "worktree", "list", "--porcelain").stdout
    found, path, branch = [], None, None
    for line in out.splitlines() + [""]:
        if line.startswith("worktree "):
            path, branch = line[len("worktree "):], None
        elif line.startswith("branch refs/heads/"):
            branch = line[len("branch refs/heads/"):]
        elif not line and path:
            found.append((path, branch))
            path = None
    return found[1:]


def pull_requests(repo):
    """(merged heads, open heads), or None when GitHub cannot be read."""
    heads = {}
    for state in ("merged", "open"):
        r = run("gh", "pr", "list", "--state", state, "--limit", "1000", "--json", "headRefName",
                cwd=repo)
        if r.returncode != 0:
            print(f"prune-build-output: cannot list {state} pull requests: {r.stderr.strip()}",
                  file=sys.stderr)
            return None
        heads[state] = {p["headRefName"] for p in json.loads(r.stdout)}
    return heads["merged"], heads["open"]


def last_write(target):
    newest = os.path.getmtime(target)
    with os.scandir(target) as entries:
        for e in entries:
            try:
                newest = max(newest, e.stat(follow_symlinks=False).st_mtime)
            except OSError:
                pass
    return newest


def size(path):
    r = run("du", "-sh", path)
    return r.stdout.split()[0] if r.returncode == 0 and r.stdout else "?"


def decide(trees, merged, opened, now, last_write=last_write):
    """[(path, verdict)], verdict 'remove' or a reason to keep."""
    out = []
    for path, branch in trees:
        target = os.path.join(path, "target")
        if not os.path.isdir(target):
            continue
        if branch is None:
            out.append((path, "keep: no branch checked out"))
        elif branch in opened:
            out.append((path, f"keep: {branch} has an open pull request"))
        elif branch not in merged:
            out.append((path, f"keep: {branch} has no merged pull request"))
        elif now - last_write(target) < RECENT_MINUTES * 60:
            out.append((path, f"keep: target/ written in the last {RECENT_MINUTES} minutes"))
        else:
            out.append((path, "remove"))
    return out


def selftest():
    root = tempfile.mkdtemp()
    try:
        trees = []
        for name in ("merged", "merged-busy", "open", "held", "detached", "no-target"):
            os.makedirs(os.path.join(root, name, "target") if name != "no-target"
                        else os.path.join(root, name))
            trees.append((os.path.join(root, name), None if name == "detached" else "b/" + name))
        now = time.time()
        writes = {os.path.join(root, "merged-busy", "target"): now - 60}
        got = dict(decide(trees, {"b/merged", "b/merged-busy", "b/open"}, {"b/open"}, now,
                          lambda t: writes.get(t, now - 86400)))
        want = {
            os.path.join(root, "merged"): "remove",
            os.path.join(root, "merged-busy"): f"keep: target/ written in the last {RECENT_MINUTES} minutes",
            os.path.join(root, "open"): "keep: b/open has an open pull request",
            os.path.join(root, "held"): "keep: b/held has no merged pull request",
            os.path.join(root, "detached"): "keep: no branch checked out",
        }
        if got != want:
            print(f"prune-build-output: selftest FAILED\n  got  {got}\n  want {want}", file=sys.stderr)
            return 1
        print("prune-build-output: selftest passed (5 cases)")
        return 0
    finally:
        shutil.rmtree(root)


def main(argv):
    if argv == ["--selftest"]:
        return selftest()
    if any(a != "--remove" for a in argv):
        print(__doc__.split("\n\n")[1], file=sys.stderr)
        return 2
    remove = "--remove" in argv
    repo = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    heads = pull_requests(repo)
    if heads is None:
        return 1
    removed = 0
    for path, verdict in decide(worktrees(repo), *heads, time.time()):
        target = os.path.join(path, "target")
        if verdict != "remove":
            print(f"prune-build-output: {verdict.upper().split(':')[0]} {path} ({verdict.split(': ', 1)[1]})")
        elif remove:
            shutil.rmtree(target, ignore_errors=True)
            removed += 1
            print(f"prune-build-output: REMOVED {target}")
        else:
            removed += 1
            print(f"prune-build-output: WOULD REMOVE {target} ({size(target)}); its pull request merged")
    if not removed:
        print("prune-build-output: nothing to remove")
    elif not remove:
        print("prune-build-output: dry run; pass --remove to delete these")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
