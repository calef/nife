"""Every exception marker a ratchet honors, counted, so debt that comes back by exception is seen.

A ratchet holds a measure to a baseline that only falls, and a marked exception is the one way an
item passes over its limit without a baseline row. Once a ratchet's debt reaches zero its chart can
go, and from then on an exception is the only way that debt returns. `script/lint` runs this after
the ratchets and prints a warning per marker, never a failure; with no markers it prints nothing.

    python3 helpers/exception_markers.py            # the warning (what script/lint runs)
    python3 helpers/exception_markers.py --list     # kind<TAB>item, one line per marker
    python3 helpers/exception_markers.py --selftest # a planted marker of each kind is caught

Name: provisional, minted 2026-10-13 (UTC) by lane/exception-count. A shared python module under
`helpers/`, which `script/names` puts out of its own scope, so its provenance is this paragraph.
calef names things; expect this to change.

**Each kind is read by its own gate's parser**, so this count cannot disagree with what the gate
accepts:

- `prose-budget`, §212 (a prose budget), and `writing-standards`, §213 (writing standards):
  `prose_ratchet.exceptions` over `prose_ratchet.documents`. One item per document and family.
- `index-exception`, §282 (when a file is split, the parent becomes an index):
  `parent_index.judge`'s `marked` items, one per marked item.
- `line-citation`, §277 (cite a heading or a symbol, never a line number): the citations
  `line_citations.file_hits` excuses, which is its count with the marker ignored minus its count
  with the marker honored. A marker that excuses nothing, such as one quoted in a note about the
  marker, is not an exception and is not counted.

**What has no marker to count.** The file-length ratchet, §266 (a Rust source file stays under
2,000 lines), and the comment-block ratchet, §267 (a comment states the constraint as it is now),
provide no exception. The lint ratchet's function length, milestone 888 (a ratchet on function
length), is measured from source text by `function_length.py`, which does not read
`#[allow(clippy::too_many_lines)]`, so an `allow` does not excuse a function there either. A
ratchet that gains a marker adds a kind here, with a fixture.
"""

import os
import subprocess
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import line_citations as lc  # noqa: E402
import parent_index as pi  # noqa: E402
import prose_ratchet as pr  # noqa: E402

# (kind, the rule it excuses), in the order the warning prints them.
KINDS = (
    ('prose-budget', '§212 prose budget'),
    ('writing-standards', '§213 writing standards'),
    ('index-exception', '§282 parent index'),
    ('line-citation', '§277 line citations'),
)


def markers(files):
    """{kind: [item]} for every honored marker in `files`, a {path: text} map."""
    found = {kind: [] for kind, _ in KINDS}
    for path in pr.documents(files):
        for family in sorted(pr.exceptions(files[path])):
            found[family].append(path)
    for path in sorted(files):
        text = files[path]
        if path.endswith('.rs') and not path.startswith('vendor/') and \
                pi.MAYBE_PARENT.search(text):
            judged = pi.judge(text)
            for item, _date, _reason in (judged or {}).get('marked', ()):
                found['index-exception'].append(f'{path}:{item["first"]} ({pi.describe(item)})')
        if lc.in_scope(path) and lc.MARKER in text and '\0' not in text:
            honored = set(lc.file_hits(text, path))
            for line, target, _kind in lc.file_hits(text, path, honor_marks=False):
                if (line, target, _kind) not in honored:
                    found['line-citation'].append(f'{path}:{line} ({target})')
    return found


def tree():
    """{path: text} over the union of the scopes the three gates read."""
    paths = pr.tracked() | pi.tracked_rust() | set(lc.tracked())
    out = {}
    for path in paths:
        if os.path.isfile(path) and not os.path.islink(path):
            text = lc.read(path)
            if text is not None:
                out[path] = text
    return out


def warning_lines(found, actions=None):
    """The warning, one line per kind and one per item; empty when nothing is marked.

    Under GitHub Actions a kind's line is a `::warning::` annotation and its items are plain lines
    beneath it, which keeps the annotations under the ten a step shows."""
    if actions is None:
        actions = bool(os.environ.get('GITHUB_ACTIONS'))
    head, item_head = ('::warning::', '    ') if actions else ('warning: ', 'warning:   ')
    total = sum(len(items) for items in found.values())
    if not total:
        return []
    out = [f'{head}{total} exception markers the ratchets honor; each lets an item past its limit '
           f'with no baseline row']
    for kind, rule in KINDS:
        items = found[kind]
        if items:
            out.append(f'{head}{kind} ({rule}): {len(items)}')
            out += [f'{item_head}{item}' for item in items]
    return out


def selftest():
    failed = []
    at = ':'  # joins a path and a line number, so line_citations.py does not count the fixtures
    planted = {
        'design/roadmap/0001-a.md': '# A\n\n<!-- prose-budget: exception. 2026-10-13. Reason: x -->\n',
        'design/roadmap/0002-b.md': '# B\n\n<!-- writing-standards: exception. 2026-10-13. Reason: x -->\n',
        'kernel/src/a.rs': ('mod b;\n\n// Index exception (2026-10-13): every child reads it.\n'
                            'pub fn f() {\n    let x = 1;\n    let y = 2;\n    g(x, y);\n}\n'),
        'notes/c.md': ('<!-- line-citation: exception. Reason: quoted output -->\n'
                       f'notes/net.md{at}63: x\n'),
    }
    found = markers(planted)
    want = {
        'prose-budget': ['design/roadmap/0001-a.md'],
        'writing-standards': ['design/roadmap/0002-b.md'],
        'index-exception': [f'kernel/src/a.rs{at}4 (fn f)'],
        'line-citation': [f'notes/c.md{at}2 (notes/net.md{at}63)'],
    }
    for kind, _ in KINDS:
        if found[kind] != want[kind]:
            failed.append(f'a planted {kind} marker read as {found[kind]}, wanted {want[kind]}')
    # Each kind on its own, so one kind's reader cannot be passing on another's fixture.
    for path, text in planted.items():
        alone = markers({path: text})
        if sum(len(v) for v in alone.values()) != 1:
            failed.append(f'{path} alone read as {alone}')
    # The same tree with every marker quoted, unmarked, or excusing nothing: all silent.
    quiet = {
        'design/roadmap/0001-a.md': '# A\n\nSpell it `<!-- prose-budget: exception. 2026-10-13. Reason: x -->`.\n',
        'kernel/src/a.rs': 'mod b;\n\npub fn f() {\n    let x = 1;\n    let y = 2;\n    g(x, y);\n}\n',
        'notes/c.md': 'Write `line-citation: exception. Reason: ...` above it.\n\nNo citation here.\n',
        'vendor/x/src/a.rs': planted['kernel/src/a.rs'],
    }
    if any(markers(quiet).values()):
        failed.append(f'markers that excuse nothing were counted: {markers(quiet)}')
    for actions, annotated in ((False, 9), (True, 5)):
        lines = warning_lines(found, actions)
        if len(lines) != 9 or sum(line.startswith(('warning: ', '::warning::'))
                                  for line in lines) != annotated:
            failed.append(f'the warning reads {lines}')
    if warning_lines(markers(quiet), False) or warning_lines(markers(quiet), True):
        failed.append('the warning is not silent at zero')
    for name in failed:
        print(f'exception markers selftest: {name}', file=sys.stderr)
    if failed:
        return 1
    print(f'exception markers selftest: {len(KINDS)} kinds caught when planted, silent when quoted')
    return 0


def main(argv):
    root = subprocess.run(['git', 'rev-parse', '--show-toplevel'], capture_output=True,
                          text=True).stdout.strip()
    if root:
        os.chdir(root)
    if argv == ['--selftest']:
        return selftest()
    found = markers(tree())
    if argv == ['--list']:
        for kind, _ in KINDS:
            for item in found[kind]:
                print(f'{kind}\t{item}')
        return 0
    if argv:
        print(__doc__.split('\n\n')[1], file=sys.stderr)
        return 2
    for line in warning_lines(found):
        print(line)
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
