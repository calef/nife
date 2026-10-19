#!/usr/bin/env python3
"""How often merged pull requests touch the prose ratchet's baseline, and why.

    python3 helpers/prose_baseline_churn.py                  # the first week, and the life to date
    python3 helpers/prose_baseline_churn.py --days N         # the first N days instead of 7
    python3 helpers/prose_baseline_churn.py --rev REV        # read history up to REV (default origin/main)
    python3 helpers/prose_baseline_churn.py --selftest

Name: provisional, minted 2026-10-19 (UTC) by lane milestone/586-baseline-churn. A python helper
under `helpers/`, outside `script/names`' scope, so this paragraph is its provenance. calef names
things; expect this to change.

# WHY

Milestone 586 (a prose ratchet in lint) names the one finding that would make the ratchet not worth
keeping: a baseline that churns on every pull request costs more attention than it saves. This is
that measurement. It reads git only, so any commit of `main` reproduces it.

# WHAT IT COUNTS

The unit is a merge on `main`'s first-parent line, which is one merged pull request. The window
opens at the merge that added `design/prose-baseline.tsv`, so "the first week" means the gate's
first seven days in force, not a calendar week.

Three populations, each a subset of the one before:

- merged: every pull request merged in the window.
- prose: those whose diff changes a document the ratchet measures, by `prose_ratchet.documents`
  read at that merge. Only these can meet the gate at all.
- touched: those whose diff changes the baseline file.

Each touch is classified by what it did to the rows, read from the file's two sides of the merge:

- initial: the merge that wrote the file.
- forced: a change the gate made the author make. A renamed document carries its row (a row for a
  missing file fails), and a bump of the `measure:` line re-measures the rows it moved.
- banking: rows removed or lowered and nothing else. `--bank` is optional (the merge-base
  comparison closes the slack without it), so this is a choice, not a toll.
- other: anything left, such as a row added or raised. The gate refuses both against the merge
  base, so a nonzero count here is a finding.

The `debt_paydown` column is read from `notes/project-metrics/merged-pull-requests.csv`, the label
as it stood at that file's last fetch.

# BUGS

- The prose population uses today's `documents` scope at every merge, not the scope as it was then.
  The scope widened once in the window measured (appendices, at the gate's own merge), so early
  counts can include a document the gate of that day did not read.
- A pull request that met the gate and was rewritten to pass before it merged leaves no trace in
  the baseline. This counts what the file cost, not what the gate cost; the gate's cost to authors
  is the exception markers (`helpers/exception_markers.py`) and failed lint runs, which CI keeps and
  git does not.
"""

import csv
import datetime
import os
import re
import subprocess
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import prose_ratchet as pr  # noqa: E402

LABELS_CSV = 'notes/project-metrics/merged-pull-requests.csv'
PR_SUBJECT = re.compile(r'^Merge pull request #(\d+) ')


def git(*args):
    return subprocess.run(('git',) + args, capture_output=True, text=True, check=True).stdout


def rows_of(text):
    return pr.read_baseline(text or '')


def header_of(text):
    return [line for line in (text or '').splitlines() if line.startswith('#')]


def classify(old, new, renames):
    """What one change did to the baseline. `old` and `new` are the file's text (None if absent);
    `renames` maps an old document path to its new path, from git's rename detection."""
    if old is None:
        return {'class': 'initial', 'rows': len(rows_of(new))}
    a, b = rows_of(old), rows_of(new)
    renamed = {p: q for p, q in renames.items() if p in a and p not in b and q in b and q not in a}
    gone = set(a) - set(b) - set(renamed)
    came = set(b) - set(a) - set(renamed.values())
    lowered = raised = 0
    pairs = [(p, p) for p in set(a) & set(b)] + list(renamed.items())
    # Counted in rows, not cells: a row is one document, and a document is what an author meets.
    for p, q in pairs:
        up = down = False
        for c in pr.COLUMNS:
            x, y = a[p].get(c), b[q].get(c)
            if x == y:
                continue
            # `-` is "within the limit", so a number becoming `-` is a fall and the reverse a rise.
            if y is None or (x is not None and y < x):
                down = True
            else:
                up = True
        lowered += down and not up
        raised += up
    measure = pr.baseline_measure(old) != pr.baseline_measure(new)
    out = {'removed': len(gone), 'lowered': lowered, 'raised': raised, 'renamed': len(renamed),
           'added': len(came), 'header': header_of(old) != header_of(new), 'measure': measure}
    if came or (raised and not measure):
        out['class'] = 'other'
    elif renamed or measure:
        out['class'] = 'forced'
    elif gone or lowered:
        out['class'] = 'banking'
    else:
        out['class'] = 'other'
    return out


def show(rev, path):
    r = subprocess.run(('git', 'show', f'{rev}:{path}'), capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else None


def renames_between(a, b):
    out = {}
    for line in git('diff', '-M', '--name-status', a, b).splitlines():
        cells = line.split('\t')
        if cells[0].startswith('R'):
            out[cells[1]] = cells[2]
    return out


def labels():
    try:
        with open(LABELS_CSV) as f:
            return {int(r['number']): r['debt_paydown'] == '1' for r in csv.DictReader(f)}
    except OSError:
        return {}


def merges(rev, since):
    out = []
    for line in git('log', '--first-parent', '--merges', '--format=%H%x09%ct%x09%s',
                    f'--since=@{since}', rev).splitlines():
        sha, ts, subject = line.split('\t', 2)
        m = PR_SUBJECT.match(subject)
        if m:
            title = git('log', '-1', '--format=%b', sha).strip().splitlines()
            out.append({'sha': sha, 'ts': int(ts), 'pr': int(m.group(1)),
                        'title': title[0] if title else subject})
    return sorted(out, key=lambda m: m['ts'])


def measure(rev):
    added = git('log', '--first-parent', '--diff-filter=A', '--format=%H%x09%ct', rev, '--',
                pr.BASELINE).split()
    start_sha, start = added[-2], int(added[-1])
    # The baseline arrived inside a merge's second parent; the window opens at that merge.
    first = next(m for m in merges(rev, start) if m['sha'] == start_sha or
                 subprocess.run(('git', 'merge-base', '--is-ancestor', start_sha, m['sha']),
                                capture_output=True).returncode == 0)
    open_ts = first['ts']
    paydown = labels()
    rows = []
    for m in merges(rev, open_ts):
        parent = f"{m['sha']}^1"
        changed = git('diff', '--name-only', parent, m['sha']).split('\n')
        paths = set(git('ls-tree', '-r', '--name-only', m['sha']).split('\n'))
        scope = set(pr.documents(paths))
        m['prose'] = any(p in scope for p in changed)
        m['paydown'] = paydown.get(m['pr'])
        m['touch'] = None
        if pr.BASELINE in changed:
            m['touch'] = classify(show(parent, pr.BASELINE), show(m['sha'], pr.BASELINE),
                                  renames_between(parent, m['sha']))
            m['rows_after'] = len(rows_of(show(m['sha'], pr.BASELINE)))
        rows.append(m)
    return open_ts, rows


def day(ts):
    return datetime.datetime.fromtimestamp(ts, datetime.timezone.utc).strftime('%Y-%m-%dT%H:%MZ')


def summary(name, rows):
    merged = len(rows)
    prose = sum(m['prose'] for m in rows)
    touched = [m for m in rows if m['touch']]
    by = {}
    for m in touched:
        by[m['touch']['class']] = by.get(m['touch']['class'], 0) + 1
    print(f'{name}: {merged} merged, {prose} changed a measured document, {len(touched)} touched the baseline')
    if merged:
        print(f'  touched / merged {len(touched) / merged:6.1%}    touched / prose {len(touched) / max(prose, 1):6.1%}')
    for c in ('initial', 'forced', 'banking', 'other'):
        n = by.get(c, 0)
        lab = sum(1 for m in touched if m['touch']['class'] == c and m['paydown'])
        print(f'  {c:8} {n:4}   of which labeled debt-paydown: {lab}')


def report(rev, days):
    open_ts, rows = measure(rev)
    close_ts = open_ts + days * 86400
    print(f'baseline in force from {day(open_ts)}; first {days} days close {day(close_ts)}; '
          f'read at {git("rev-parse", "--short", rev).strip()}\n')
    week = [m for m in rows if m['ts'] < close_ts]
    summary(f'first {days} days', week)
    summary('life to date', rows)
    print('\nevery touch: when, pull request, class, rows after, what moved, debt-paydown, title')
    for m in rows:
        t = m['touch']
        if not t:
            continue
        moved = ' '.join(f'{k}={t[k]}' for k in ('removed', 'lowered', 'renamed', 'raised', 'added')
                         if t.get(k))
        if t.get('measure'):
            moved += ' measure'
        elif t.get('header'):
            moved += ' header'
        flag = {True: 'paydown', False: '-', None: '?'}[m['paydown']]
        print(f"  {day(m['ts'])}  #{m['pr']:<5} {t['class']:8} {m['rows_after']:5}  "
              f"{moved or '-':28} {flag:8} {m['title'][:60]}")
    return 0


def selftest():
    head = '# measure: 3\npath\t' + '\t'.join(pr.COLUMNS) + '\n'
    old = head + 'a.md\t4000\t-\t50\t-\t-\nb.md\t-\t22\t-\t-\t-\nc.md\t-\t-\t45\t-\t-\n'
    cases = [
        # A rename read as remove-plus-add would call a forced edit banking and an illegal add.
        ('rename', head + 'a.md\t4000\t-\t50\t-\t-\nb.md\t-\t22\t-\t-\t-\nd.md\t-\t-\t45\t-\t-\n',
         {'c.md': 'd.md'}, 'forced'),
        ('bank', head + 'a.md\t3900\t-\t-\t-\t-\nb.md\t-\t22\t-\t-\t-\n', {}, 'banking'),
        ('raise', head + 'a.md\t4100\t-\t50\t-\t-\nb.md\t-\t22\t-\t-\t-\nc.md\t-\t-\t45\t-\t-\n',
         {}, 'other'),
        ('measure', old.replace('measure: 3', 'measure: 4').replace('4000', '4100'), {}, 'forced'),
    ]
    ok = classify(None, old, {})['class'] == 'initial'
    for name, new, renames, want in cases:
        got = classify(old, new, renames)['class']
        if got != want:
            print(f'prose_baseline_churn selftest: {name}: {got}, want {want}')
            ok = False
    print('prose_baseline_churn selftest: %s' % ('ok' if ok else 'FAILED'))
    return 0 if ok else 1


def main(argv):
    if argv[1:] == ['--selftest']:
        return selftest()
    rev, days = 'origin/main', 7
    args = argv[1:]
    while args:
        flag = args.pop(0)
        if flag == '--rev' and args:
            rev = args.pop(0)
        elif flag == '--days' and args:
            days = int(args.pop(0))
        else:
            print(__doc__.split('\n\n')[1], file=sys.stderr)
            return 2
    return report(rev, days)


if __name__ == '__main__':
    sys.exit(main(sys.argv))
