#!/usr/bin/env python3
"""List, and on request delete, the `target/` of every worktree whose work has landed.

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

It deletes build output only, and needs less to be sure: build output is rebuilt by the next
`cargo` in that worktree, so a wrong deletion costs one build. The worktree itself goes by
helpers/prune_worktrees.py, which also requires it clean and its HEAD on origin.

What is kept, and so never touched:

  - the main checkout, and any worktree with no branch checked out;
  - a worktree whose branch has an open pull request, or none at all (a held branch, or a lane
    that has not pushed yet);
  - a `target/` written to in the last RECENT_MINUTES, because a build may be running there. A lane
    that reopens a merged branch is caught by this too.

A worker is judged by its lane. A lane that fans out cuts worker worktrees on `wip/<slug>-<x>`
branches (`wip/long-function-paydown-a` for `maintainer/long-function-paydown`), and a worker never
opens a pull request of its own, so until 2026-10-12 every worker's `target/` was kept forever: six
of them held 6 to 10 GB each on the day the disk filled. A worker's parent is the one non-`wip`
branch, local, on origin or a pull request's head, whose last `/` segment is `<slug>`. The worker's
`target/` goes, once idle, when the parent's pull request has merged, or when the worker's HEAD is
already contained in the parent branch (local or origin), as an ancestor or as cherry-picks with
the same patch, which is the state a worker reaches once the lane has taken its commits. A worker whose parent cannot be found, or is ambiguous, is kept.

It runs on this Mac from launchd, hourly, beside the other jobs; the plist is kept off-tree by the
maintainers. It cannot run in CI, which never sees this machine's worktrees.

BUGS
  - Merged means GitHub says so for that head branch, from the newest 1,000 merged pull requests.
    A lane older than that keeps its `target/` until someone runs the brief by hand.
  - The recent-write check reads the modification time of `target/` and its first level only, so a
    build that writes only deep inside an old directory can look idle. A deletion under a running
    cargo fails that build, which a rerun fixes.
  - A worker cut from its parent and idle for an hour before its first commit has a HEAD the parent
    already contains, so its `target/` goes. The cost is that worker's next build from scratch.
  - The `wip/<slug>-<x>` shape is a convention the fleet follows, not one script/claim cuts, so a
    worker named any other way is judged as an ordinary branch and kept.
  - A commit the lane took with a conflict resolved, or squashed with others, has a different
    patch, so `git cherry` counts it as missing and the worker is kept until the parent merges.
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


WIP = "wip/"


def parent_lane(branch, branches):
    """The lane branch a `wip/<slug>-<x>` worker belongs to, or None when there is not exactly one."""
    slug = branch[len(WIP):].rpartition("-")[0]
    if not slug:
        return None
    found = {b for b in branches if not b.startswith(WIP) and b.rpartition("/")[2] == slug}
    return found.pop() if len(found) == 1 else None


def contained(path, parent):
    """Whether the parent branch, locally or on origin, already holds the worktree's HEAD.

    Holds means HEAD is an ancestor, or every commit HEAD has beyond the parent is in it as a
    cherry-pick with the same patch (`git cherry`): lanes take their workers' commits both ways.
    """
    for ref in (f"refs/heads/{parent}", f"refs/remotes/origin/{parent}"):
        if run("git", "-C", path, "merge-base", "--is-ancestor", "HEAD", ref).returncode == 0:
            return True
        cherry = run("git", "-C", path, "cherry", ref, "HEAD")
        if cherry.returncode == 0 and not any(l.startswith("+") for l in cherry.stdout.splitlines()):
            return True
    return False


def verdict_for(path, branch, merged, opened, branches, contained):
    """Why to keep a worktree's `target/` on its branch alone, or None when it may go once idle."""
    if branch is None:
        return "keep: no branch checked out"
    if branch in opened:
        return f"keep: {branch} has an open pull request"
    if branch in merged:
        return None
    if not branch.startswith(WIP):
        return f"keep: {branch} has no merged pull request"
    parent = parent_lane(branch, branches)
    if parent is None:
        return f"keep: worker {branch} has no single parent lane branch"
    if parent in merged and parent not in opened:
        return None
    if contained(path, parent):
        return None
    return f"keep: worker {branch} has commits {parent} does not contain"


def decide(trees, merged, opened, now, last_write=last_write, branches=frozenset(),
           contained=contained):
    """[(path, verdict)], verdict 'remove' or a reason to keep."""
    out = []
    for path, branch in trees:
        target = os.path.join(path, "target")
        if not os.path.isdir(target):
            continue
        keep = verdict_for(path, branch, merged, opened, branches, contained)
        if keep:
            out.append((path, keep))
        elif now - last_write(target) < RECENT_MINUTES * 60:
            out.append((path, f"keep: target/ written in the last {RECENT_MINUTES} minutes"))
        else:
            out.append((path, "remove"))
    return out


def known_branches(repo, merged, opened):
    """Every branch name the parent of a worker could be: local, origin's, and pull request heads."""
    out = run("git", "-C", repo, "for-each-ref", "--format=%(refname)", "refs/heads",
              "refs/remotes/origin").stdout.split()
    names = {r[len("refs/heads/"):] if r.startswith("refs/heads/") else r[len("refs/remotes/origin/"):]
             for r in out}
    return (names - {"HEAD"}) | merged | opened


def selftest():
    root = tempfile.mkdtemp()
    try:
        names = ("merged", "merged-busy", "open", "held", "detached", "no-target",
                 "lane-a", "lane-b", "lane-c", "orphan-a", "twice-a", "landed-a", "landed-b")
        trees = []
        for name in names:
            os.makedirs(os.path.join(root, name, "target") if name != "no-target"
                        else os.path.join(root, name))
            branch = "b/" + name
            if name.startswith(("lane-", "orphan-", "twice-", "landed-")):
                branch = WIP + name
            trees.append((os.path.join(root, name), None if name == "detached" else branch))
        now = time.time()
        writes = {os.path.join(root, n, "target"): now - 60 for n in ("merged-busy", "landed-b")}
        # `lane` is still open; `lane-b`'s HEAD is in it and `lane-c`'s is not. `landed` merged, so
        # both its workers go unless busy. `twice` names two parents, `orphan` none.
        branches = {"maintainer/lane", "maintainer/landed", "milestone/twice", "maintainer/twice",
                    "b/merged", "b/open"}
        inside = {(os.path.join(root, "lane-b"), "maintainer/lane")}
        got = dict(decide(trees, {"b/merged", "b/merged-busy", "b/open", "maintainer/landed"},
                          {"b/open", "maintainer/lane"}, now,
                          lambda t: writes.get(t, now - 86400), branches,
                          lambda p, parent: (p, parent) in inside))
        want = {
            os.path.join(root, "merged"): "remove",
            os.path.join(root, "merged-busy"): f"keep: target/ written in the last {RECENT_MINUTES} minutes",
            os.path.join(root, "open"): "keep: b/open has an open pull request",
            os.path.join(root, "held"): "keep: b/held has no merged pull request",
            os.path.join(root, "detached"): "keep: no branch checked out",
            os.path.join(root, "lane-a"): "keep: worker wip/lane-a has commits maintainer/lane does not contain",
            os.path.join(root, "lane-b"): "remove",
            os.path.join(root, "lane-c"): "keep: worker wip/lane-c has commits maintainer/lane does not contain",
            os.path.join(root, "orphan-a"): "keep: worker wip/orphan-a has no single parent lane branch",
            os.path.join(root, "twice-a"): "keep: worker wip/twice-a has no single parent lane branch",
            os.path.join(root, "landed-a"): "remove",
            os.path.join(root, "landed-b"): f"keep: target/ written in the last {RECENT_MINUTES} minutes",
        }
        if got != want:
            print(f"prune-build-output: selftest FAILED\n  got  {got}\n  want {want}", file=sys.stderr)
            return 1
        print(f"prune-build-output: selftest passed ({len(want)} cases)")
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
    branches = known_branches(repo, *heads)
    for path, verdict in decide(worktrees(repo), *heads, time.time(), branches=branches):
        target = os.path.join(path, "target")
        if verdict != "remove":
            print(f"prune-build-output: {verdict.upper().split(':')[0]} {path} ({verdict.split(': ', 1)[1]})")
        elif remove:
            shutil.rmtree(target, ignore_errors=True)
            removed += 1
            print(f"prune-build-output: REMOVED {target}")
        else:
            removed += 1
            print(f"prune-build-output: WOULD REMOVE {target} ({size(target)})")
    if not removed:
        print("prune-build-output: nothing to remove")
    elif not remove:
        print("prune-build-output: dry run; pass --remove to delete these")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
