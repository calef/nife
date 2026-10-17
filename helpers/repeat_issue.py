#!/usr/bin/env python3
"""Open or update one issue per scheduled workflow whose failures say the same thing again.

    helpers/repeat_issue.py sync          # REPEAT rows on stdin; one issue per workflow
    helpers/repeat_issue.py --selftest

Milestone 422 (a cadence job whose healthy state is red, and nothing that notices it repeating),
provisional number and name.

# WHY

The repeat row alone is rung four with no reader named, and what it asks for ("run the audit") is
a lane and a day, not a fix applied between tasks. calef ruled the home on 2026-10-16 (UTC), option
2 of the ask on pull request #2227: one issue per distinct repeat, on the helpers/watcher_watch.py
pattern this workflow already runs, closed by the audit landing. What the ruling refused stands:
the row is not a gate, no empty pull request carries a label, and no audit-lane gate reads the row.

# THE RULE

sync reads REPEAT rows, one per workflow, in the format `helpers/trunk-health.sh` prints. CHURN
rows and everything else are ignored: a workflow whose failures disagree with each other gets no
issue, per the same ruling. Per workflow, then: no open issue with its title opens one; an open
issue whose body no longer matches the row's facts is edited to match; a matching open issue is
left alone. The state is GitHub's own, never a file, because the watcher that calls this holds
none either (trunk-health.yml's BUGS owns the re-announce every five minutes; the issue is what
makes that re-announce cheap to ignore).

The close is deliberate and human, and never this script's: the audit lane's report pull request
says `Closes #N` naming this issue. That routing lives in audit-cadence.yml's step summary, which
is what hands the audit lane out.

# BUGS

- Dedupe is by exact title, and the title carries only the workflow name. Two distinct findings
  for one workflow (a repeat of one line while another line fails differently) are one issue, on
  the ruling's "one open issue per workflow"; the body names the newest line.
- A workflow whose repeat issue stays open while its row disappears keeps its issue open. Only a
  close condition closes an issue; the row stopping is not the work being done, and the 2026-09-14
  FileNotFoundError run is the reason a missing row must not read as "fixed".
"""

import json
import os
import re
import subprocess
import sys

REPO = "nifeos/nife"
TITLE_PREFIX = "scheduled failure repeats: "

# One row as helpers/trunk-health.sh prints it: name, qualifier, then REPEAT: with the line and
# the span of dates. The span is anchored at the end, because the failing line itself may hold
# parentheses, pipes and dates of its own.
ROW = re.compile(
    r"^(?P<name>\S+)\s+(?P<qualifier>\d+ of last 6 failed)\s+REPEAT: "
    r"(?P<line>.*) \((?P<first>\d{4}-\d{2}-\d{2})\.\.(?P<last>\d{4}-\d{2}-\d{2}) UTC\)$"
)


def parse_rows(text):
    """`[(name, qualifier, line, first, last)]` for every REPEAT row in `text`. Pure.

    CHURN rows, DUE rows, cadence rows and anything else that reaches stdin are not findings this
    home owns, and are dropped here rather than trusted upstream."""
    rows = []
    for line in text.splitlines():
        m = ROW.match(line)
        if m:
            rows.append((m.group("name"), m.group("qualifier"), m.group("line"),
                         m.group("first"), m.group("last")))
    return rows


def issue_body(name, qualifier, line, first, last):
    """The issue body for one repeat. Pure, and the only place the wording lives."""
    return (
        "`%s` %s, failing with the same final line each time.\n\n"
        "The line, as the newest failing run's step printed it:\n\n"
        "    %s\n\n"
        "Span of the repeated runs: %s to %s (UTC). A repeat is not a color: the same red kept\n"
        "arriving on schedule, and for `%s` that is usually the audit being due, which is the\n"
        "mechanism working, but a broken run can hide among correct reds, which is why the line\n"
        "is named rather than only the count.\n\n"
        "**What closes this issue:** the audit lane's report pull request lands with `Closes #N`\n"
        "naming this issue in its body. The audit-cadence workflow's step summary routes that.\n"
        "Closing it any other way leaves the row this issue exists for still firing.\n\n"
        "Opened by `helpers/repeat_issue.py`, milestone 422 (a cadence job whose healthy state is red),\n"
        "provisional. The thresholds behind the row are in `helpers/trunk-health.sh`'s repeat section."
        % (name, qualifier, line, first, last, name)
    )


def plan(rows, open_issues):
    """Issue actions for `rows` given `{title: (number, body)}` of open issues. Pure.

    One action per workflow at most: `open` when none exists, `edit` when the body drifted from
    the row's facts, nothing when they agree."""
    acts = []
    seen = set()
    for name, qualifier, line, first, last in rows:
        title = TITLE_PREFIX + name
        body = issue_body(name, qualifier, line, first, last)
        if title in open_issues:
            number, current = open_issues[title]
            if current != body:
                acts.append(("edit", number, title, body))
        elif title not in seen:
            acts.append(("open", None, title, body))
        seen.add(title)
    return acts


def gh(args, token=None):
    env = dict(os.environ)
    if token:
        env["GH_TOKEN"] = token
    return subprocess.run(["gh"] + args, capture_output=True, text=True, env=env)


def sync(text):
    rows = parse_rows(text)
    if not rows:
        return 0
    listed = gh(["issue", "list", "--repo", REPO, "--state", "open", "--limit", "200",
                 "--json", "number,title,body"])
    if listed.returncode:
        print("repeat_issue: could not list issues: %s" % listed.stderr.strip(), file=sys.stderr)
        return 2
    open_issues = {i["title"]: (i["number"], i["body"] or "")
                   for i in (json.loads(listed.stdout) if listed.stdout.strip() else [])}
    status = 0
    write = os.environ.get("GH_TOKEN") or None
    fallback = os.environ.get("WATCHER_FALLBACK_TOKEN") or None
    for action, number, title, body in plan(rows, open_issues):
        if action == "open":
            args = ["issue", "create", "--repo", REPO, "--title", title, "--body", body]
        else:
            args = ["issue", "edit", str(number), "--repo", REPO, "--body", body]
        for who, token in (("nife-smelter", write), ("fallback token", fallback)):
            if token is None and who != "nife-smelter":
                continue
            done = gh(args, token)
            if done.returncode == 0:
                print("repeat_issue: %s %s as %s: %s"
                      % (action, title, who, (done.stdout or "").strip()))
                break
            print("repeat_issue: %s as %s failed: %s" % (action, who, done.stderr.strip()),
                  file=sys.stderr)
        else:
            status = 2
    return status


def selftest():
    ok = True

    def expect(label, got, want):
        nonlocal ok
        if got != want:
            ok = False
            print("FAIL %s: got %r want %r" % (label, got, want), file=sys.stderr)

    audit = ("audit-cadence                    5 of last 6 failed       REPEAT: design/audit-reports/"
             "README.md says how, and which lens the last one lacked. (2026-09-07..2026-10-12 UTC)")
    churn = ("falsifications                   2 of last 6 failed       CHURN: 2 of the last 6 "
             "failed, all 2 different")
    due = "audits                           2 overdue                DUE: documentation, security"

    # Only REPEAT rows are findings this home owns; every other line shape drops.
    rows = parse_rows("\n".join([audit, churn, due, "main is green at de74157d", ""]))
    expect("one row parsed", len(rows), 1)
    expect("row fields", rows,
           [("audit-cadence", "5 of last 6 failed",
             "design/audit-reports/README.md says how, and which lens the last one lacked.",
             "2026-09-07", "2026-10-12")])

    # A failing line with its own parentheses and dates must not confuse the trailing span.
    tricky = ("mutation                     3 of last 6 failed       REPEAT: or set it to 0 to run "
              "unbounded (helpers/memory-bounded-runner.sh) on 2026-09-07 (2026-09-07..2026-10-12 UTC)")
    expect("trailing span wins", parse_rows(tricky)[0][3:], ("2026-09-07", "2026-10-12"))

    body = issue_body("audit-cadence", "5 of last 6 failed", "the line", "2026-09-07", "2026-10-12")
    for needle in ("audit-cadence", "the line", "2026-09-07", "2026-10-12", "Closes #N"):
        expect("body names %s" % needle, needle in body, True)

    # One open issue per workflow: open, then leave alone, then edit only on drift. CHURN never
    # reaches here, but a row for a second workflow is independent.
    title = TITLE_PREFIX + "audit-cadence"
    rows = [("audit-cadence", "5 of last 6 failed", "the line", "2026-09-07", "2026-10-12")]
    expect("opens with none", plan(rows, {}), [("open", None, title, body)])
    expect("matching body is a no-op", plan(rows, {title: (7, body)}), [])
    drifted = issue_body("audit-cadence", "5 of last 6 failed", "the line", "2026-09-01", "2026-10-12")
    expect("drifted body edits", plan(rows, {title: (7, drifted)}), [("edit", 7, title, body)])
    both = rows + [("mutation", "3 of last 6 failed", "other", "2026-09-21", "2026-10-12")]
    expect("two workflows, two actions", len(plan(both, {})), 2)

    print("repeat_issue selftest: %s" % ("ok" if ok else "FAILED"))
    return 0 if ok else 1


def main(argv):
    if argv[1:] == ["--selftest"]:
        return selftest()
    if argv[1:] == ["sync"]:
        return sync(sys.stdin.read())
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
