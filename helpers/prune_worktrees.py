#!/usr/bin/env python3
"""List, and on request remove, every lane worktree whose work has landed and is safe on the remote.

    python3 helpers/prune_worktrees.py            # dry run: say what would go and why the rest stays
    python3 helpers/prune_worktrees.py --remove   # remove exactly those, and their local branches
    python3 helpers/prune_worktrees.py --selftest

PROVISIONAL NAME, minted 2026-10-19 (UTC) by lane/118-budget-rules-2-3 for milestone 118 (CLAUDE.md has a budget)'s budget
rule 2, after its sibling helpers/prune_build_output.py. calef has not ruled on it.

Why this exists. A lane's worktree is pruned the moment its pull request merges
(briefs/merge-and-cleanup.md step 3; notes/skills/maintainer/SKILL.md), and nothing did it: the
step waited for a person to remember it. A merge lands in GitHub's queue, not on this Mac, so there
is no local event to hang it on. This runs on a clock instead, beside the build-output prune, and
removes a worktree only when every one of these holds:

  - its branch has a merged pull request and no open one, or it is a `wip/` worker whose lane has
    landed (the rule helpers/prune_build_output.py applies, imported from there so the two agree);
  - `git status --porcelain` prints nothing, so no modified, staged or untracked file is in it;
  - its HEAD is held by a branch on origin (origin/main, once a lane merges) or by the head its
    merged pull request recorded, so no commit exists only here;
  - nothing in it, or in its git directory, was written in the last RECENT_MINUTES.

Removal is `git worktree remove` without `--force`, which refuses a dirty or locked worktree on its
own, then a delete of the local branch conditional on it still pointing at that HEAD. Anything that
refuses is printed and kept.

It runs on this Mac from launchd, hourly, with the build-output prune; the plist is kept off-tree by
the maintainers. It cannot run in CI, which never sees this machine's worktrees.

BUGS
  - Idle is read from the modification times of the worktree's top level and its git directory, so
    a session that only reads files in a merged, clean, pushed worktree looks idle and loses its
    working directory. It loses no work: everything the worktree held is on origin.
  - Merged means GitHub says so, from the newest 1,000 merged pull requests, as for the build-output
    prune. An older lane is kept until someone runs briefs/merge-and-cleanup.md by hand.
  - A worker whose commits its lane took as cherry-picks holds commits no remote branch contains, so
    it is kept; helpers/prune_build_output.py still removes its `target/`.
"""

import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time

_spec = importlib.util.spec_from_file_location(
    "prune_build_output", os.path.join(os.path.dirname(os.path.abspath(__file__)), "prune_build_output.py"))
pbo = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pbo)

RECENT_MINUTES = pbo.RECENT_MINUTES
run = pbo.run


def pull_requests(repo):
    """({merged head branch: {head oid}}, {open head branch}), or None when GitHub cannot be read."""
    out = {}
    for state in ("merged", "open"):
        r = run("gh", "pr", "list", "--state", state, "--limit", "1000", "--json",
                "headRefName,headRefOid", cwd=repo)
        if r.returncode != 0:
            print(f"prune-worktrees: cannot list {state} pull requests: {r.stderr.strip()}", file=sys.stderr)
            return None
        out[state] = json.loads(r.stdout)
    merged = {}
    for p in out["merged"]:
        merged.setdefault(p["headRefName"], set()).add(p["headRefOid"])
    return merged, {p["headRefName"] for p in out["open"]}


def clean(path):
    r = run("git", "--no-optional-locks", "-C", path, "status", "--porcelain")
    return r.returncode == 0 and not r.stdout.strip()


def pushed(path, oids):
    """Whether a branch on origin, or a merged pull request's recorded head, holds this HEAD."""
    held = run("git", "-C", path, "for-each-ref", "--contains", "HEAD", "--count=1",
               "--format=%(refname)", "refs/remotes/origin")
    if held.returncode == 0 and held.stdout.strip():
        return True
    return any(run("git", "-C", path, "merge-base", "--is-ancestor", "HEAD", oid).returncode == 0
               for oid in oids)


def last_write(path):
    """The newest of what a working session leaves: a top-level entry other than `target/`, and the
    worktree's own HEAD and its reflog (any commit, checkout or reset).

    Not the worktree directory, its git directory or its index: the build-output prune deleting
    `target/` touches the first, and any tool's `git status` touches the other two with its lock
    file and its index refresh, so every worktree on the machine would always look busy."""
    gitdir = run("git", "-C", path, "rev-parse", "--absolute-git-dir").stdout.strip()
    with os.scandir(path) as entries:
        paths = [e.path for e in entries if e.name != "target"]
    if gitdir:
        paths += [os.path.join(gitdir, n) for n in ("HEAD", "logs/HEAD")]
    newest = 0.0
    for p in paths:
        try:
            newest = max(newest, os.lstat(p).st_mtime)
        except OSError:
            pass
    return newest


def decide(trees, merged, opened, now, branches=frozenset(), contained=pbo.contained,
           clean=clean, pushed=pushed, last_write=last_write):
    """[(path, branch, verdict)], verdict 'remove' or a reason to keep."""
    out = []
    for path, branch in trees:
        keep = pbo.verdict_for(path, branch, set(merged), opened, branches, contained)
        if keep:
            pass
        elif not clean(path):
            keep = "keep: it holds modified, staged or untracked files"
        elif not pushed(path, merged.get(branch, ())):
            keep = "keep: its HEAD has commits no origin branch or merged pull request holds"
        elif now - last_write(path) < RECENT_MINUTES * 60:
            keep = f"keep: written in the last {RECENT_MINUTES} minutes"
        out.append((path, branch, keep or "remove"))
    return out


def remove(repo, path, branch):
    """Remove the worktree and its branch, or say why not. Never forces."""
    head = run("git", "-C", path, "rev-parse", "HEAD").stdout.strip()
    r = run("git", "-C", repo, "worktree", "remove", path)
    if r.returncode != 0:
        return f"REFUSED {path} (git worktree remove: {r.stderr.strip()})"
    d = run("git", "-C", repo, "update-ref", "-d", f"refs/heads/{branch}", head)
    if d.returncode != 0:
        return f"REMOVED {path}; kept branch {branch} ({d.stderr.strip()})"
    return f"REMOVED {path} and branch {branch}"


def selftest():
    for v in ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR", "GIT_OBJECT_DIRECTORY"):
        os.environ.pop(v, None)
    root = tempfile.mkdtemp()
    env = dict(os.environ, GIT_AUTHOR_NAME="t", GIT_AUTHOR_EMAIL="t@t", GIT_COMMITTER_NAME="t",
               GIT_COMMITTER_EMAIL="t@t", GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM="1")

    def git(*a, cwd=None):
        r = subprocess.run(("git",) + a, cwd=cwd, env=env, capture_output=True, text=True)
        if r.returncode != 0:
            raise RuntimeError(f"git {' '.join(a)}: {r.stderr}")
        return r.stdout.strip()

    try:
        origin, repo = os.path.join(root, "origin.git"), os.path.join(root, "nife")
        git("init", "-q", "--bare", "-b", "main", origin)
        git("clone", "-q", origin, repo)
        git("commit", "-q", "--allow-empty", "-m", "root", cwd=repo)
        git("push", "-q", "origin", "HEAD:main", cwd=repo)
        cases = ("landed", "dirty", "untracked", "unpushed", "busy", "open", "held", "detached")
        trees = []
        for name in cases:
            path = os.path.join(root, "lanes", name)
            git("worktree", "add", "-q", path, "-b", f"lane/{name}", "origin/main", cwd=repo)
            git("commit", "-q", "--allow-empty", "-m", name, cwd=path)
            if name != "unpushed":
                git("push", "-q", "origin", f"HEAD:lane/{name}", cwd=path)
            trees.append((path, None if name == "detached" else f"lane/{name}"))
        git("checkout", "-q", "--detach", cwd=os.path.join(root, "lanes", "detached"))
        git("fetch", "-q", "origin", cwd=repo)
        with open(os.path.join(root, "lanes", "dirty", "f"), "w") as f:
            f.write("x")
        git("add", "f", cwd=os.path.join(root, "lanes", "dirty"))
        with open(os.path.join(root, "lanes", "untracked", "notes.txt"), "w") as f:
            f.write("a lane's unsaved finding")
        now = time.time() + 2 * RECENT_MINUTES * 60
        busy = os.path.join(root, "lanes", "busy", "notes")
        os.mkdir(busy)
        os.utime(busy, (now, now))
        # A fresh index, git directory or worktree directory is not a session: the build-output
        # prune and any `git status` touch those on every worktree.
        landed = os.path.join(root, "lanes", "landed")
        for p in (landed, git("rev-parse", "--absolute-git-dir", cwd=landed)):
            os.utime(p, (now, now))
        merged = {f"lane/{n}": set() for n in cases if n not in ("held",)}
        got = {os.path.basename(p): v for p, _, v in decide(trees, merged, {"lane/open"}, now)}
        want = {
            "landed": "remove",
            "dirty": "keep: it holds modified, staged or untracked files",
            "untracked": "keep: it holds modified, staged or untracked files",
            "unpushed": "keep: its HEAD has commits no origin branch or merged pull request holds",
            "busy": f"keep: written in the last {RECENT_MINUTES} minutes",
            "open": "keep: lane/open has an open pull request",
            "held": "keep: lane/held has no merged pull request",
            "detached": "keep: no branch checked out",
        }
        if got != want:
            print(f"prune-worktrees: selftest FAILED\n  got  {got}\n  want {want}", file=sys.stderr)
            return 1
        # The removal itself, and git's own refusal behind the checks: a dirty worktree that slipped
        # past them is still not removed.
        said = remove(repo, os.path.join(root, "lanes", "landed"), "lane/landed")
        refused = remove(repo, os.path.join(root, "lanes", "dirty"), "lane/dirty")
        branches = git("for-each-ref", "--format=%(refname:short)", "refs/heads", cwd=repo).split()
        if (not said.startswith("REMOVED") or os.path.exists(os.path.join(root, "lanes", "landed"))
                or "lane/landed" in branches or not refused.startswith("REFUSED")
                or not os.path.exists(os.path.join(root, "lanes", "dirty", "f"))):
            print(f"prune-worktrees: selftest FAILED on removal: {said}; {refused}; {branches}",
                  file=sys.stderr)
            return 1
        print(f"prune-worktrees: selftest passed ({len(want)} cases, a removal and a refusal)")
        return 0
    finally:
        shutil.rmtree(root)


def main(argv):
    if argv == ["--selftest"]:
        return selftest()
    if any(a != "--remove" for a in argv):
        print(__doc__.split("\n\n")[1], file=sys.stderr)
        return 2
    do_remove = "--remove" in argv
    repo = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    heads = pull_requests(repo)
    if heads is None:
        return 1
    merged, opened = heads
    run("git", "-C", repo, "fetch", "--quiet", "origin")
    branches = pbo.known_branches(repo, set(merged), opened)
    count = 0
    for path, branch, verdict in decide(pbo.worktrees(repo), merged, opened, time.time(), branches):
        if verdict != "remove":
            print(f"prune-worktrees: KEEP {path} ({verdict.split(': ', 1)[1]})")
            continue
        count += 1
        print("prune-worktrees: " + (remove(repo, path, branch) if do_remove else f"WOULD REMOVE {path}"))
    if do_remove and count:
        run("git", "-C", repo, "worktree", "prune")
    if not count:
        print("prune-worktrees: nothing to remove")
    elif not do_remove:
        print("prune-worktrees: dry run; pass --remove to remove these")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
