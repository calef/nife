"""The module-block ratchet: every module, document and `pub` item carries a `Name:` block, held to a list that only shrinks.

A module is a file (`helpers/name_census.py`'s `is_module_file`): every `.rs` file under a `src/`
directory outside vendored code, whether or not anyone wrote its block. So is a document directly
under `design/`, `notes/` or `briefs/` (`is_top_document`), whose block is its own entry in that
directory's README, and so is every `pub` item in a module (`pub_items`; constants and statics
included, calef 2026-10-18 UTC). That is what lets `script/names` and `script/metrics` count every one, and it
means a name with no block is visible, so it can be held to something. This holds it, with
`helpers/file_length_ratchet.py`'s pattern on purpose: a committed list that only shrinks, a
comparison against the merge base, and a selftest that runs before the check.

    python3 helpers/module_block_ratchet.py --check      # what script/lint runs
    python3 helpers/module_block_ratchet.py --bank       # drop rows whose module gained a block or went
    python3 helpers/module_block_ratchet.py --selftest   # the rules, as fixtures

Name: provisional, minted 2026-10-18 (UTC) by lane maintainer/names-count-every-module. A shared
helper under `helpers/`, which `script/names` puts out of its own scope, so this paragraph is the
record. `module_block` for what it holds each module to, `ratchet` for the family beside
`file_length_ratchet`. calef has not named it.

**The rules.**

- A row is a path (a module or document) or `path::name` (an item), and the list is a multiset: a
  file with three unblocked `pub fn new` holds three identical rows.
- A name with no block beyond what the list holds fails. A new file or a new `pub` item is not on
  the list, so it arrives with its block: `//! Name: provisional, minted <date> (UTC) by <lane>.`
  at the top of a module, `/// Name: ...` in an item's doc comment, or a README entry for a
  document, is enough.
- A row with no unblocked name behind it (the name gained a block, or is gone) fails until it is
  removed, so dead rows cannot accumulate. `--bank` removes them.
- Against the merge base a row may not be added. A rename moves its rows in the same change: a file
  git calls renamed may take its old rows' place, so moving a file does not oblige a block.
- A selection that matches no module fails, because a gate that judges nothing passes.

**What it does not hold.** Status: a `provisional` block passes, as `script/names --check` does.
An item a macro generates is invisible to `pub_items`, so it is never held.
"""

import collections
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import name_census  # noqa: E402

BASELINE = 'design/module-block-baseline.txt'
# The baseline's format. A merge base whose baseline lacks this line holds a list of a smaller
# scope (format 1 had no items), so the change that widens the scope is compared with nothing, as
# the change that first lands a baseline is. This is an exception and a foot gun: bumping the
# format skips the no-additions rule for that one change, so a bump needs a widened scope behind it
# and says so in its pull request.
FORMAT = '# format 2: modules, documents and pub items'

HEADER = FORMAT + """
# The module-block ratchet's baseline: helpers/module_block_ratchet.py, whose header is the manual.
# One row per name that carried no `Name:` block when this list was written: a module (every .rs
# file under a src/ directory) or a document directly under design/, notes/ or briefs/ by its path,
# a `pub` item as path::name, once per item. The list only shrinks. Remove a row in the change that
# writes its block or deletes it (`python3 helpers/module_block_ratchet.py --bank` does it); a row
# may not be added (script/lint compares against the merge base), and a rename moves its rows.
"""

def path_of(row):
    return row.split('::', 1)[0]


def remedy(path):
    if '::' in path:
        return ('write `/// Name: provisional, minted <date> (UTC) by <lane>.` as a paragraph of its '
                'doc comment (design/naming.md)')
    if path.endswith('.md'):
        return (f'give it an entry in {os.path.dirname(path)}/README.md, a paragraph '
                f'`` `{os.path.basename(path)}`: Name: provisional, minted <date> (UTC) by <lane>. ``')
    return ('write its block, `//! Name: provisional, minted <date> (UTC) by <lane>.`, at the top '
            'of the file (design/naming.md)')


# --- the baseline and git -------------------------------------------------------------------------

def read_baseline(text):
    return collections.Counter(line.strip() for line in text.splitlines()
                               if line.strip() and not line.startswith('#'))


def write_baseline(rows):
    with open(BASELINE, 'w') as f:
        f.write(HEADER + ''.join(f'{row}\n' for row in sorted(rows.elements())))


def git(*args):
    r = subprocess.run(('git',) + args, capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else None


def merge_base():
    base = (git('merge-base', 'HEAD', 'origin/main') or '').strip()
    head = (git('rev-parse', 'HEAD') or '').strip()
    return base if base and base != head else None


def renamed_rust(base):
    """[(old, new)] for every `.rs` file git itself calls renamed between `base` and the tree."""
    pairs = []
    for line in (git('diff', '--name-status', '-M', base, '--', '*.rs') or '').splitlines():
        cells = line.split('\t')
        if cells[0].startswith('R') and len(cells) == 3:
            pairs.append((cells[1], cells[2]))
    return pairs


def tree_modules():
    """`(modules, unblocked)` for the working tree, untracked files included, out of the census."""
    listed = git('ls-files', '-z', '--cached', '--others', '--exclude-standard') or ''
    paths = [p for p in listed.split('\0') if p and os.path.isfile(p)]

    def read(rel):
        try:
            return open(rel, encoding='utf-8', errors='replace').read()
        except OSError:
            return None

    _records, _problems, _missing, unblocked = name_census.census(paths, read)
    judged = {p for p in paths if name_census.is_module_file(p) or name_census.is_top_document(p)}
    return judged, collections.Counter(unblocked)


# --- the rules, pure so the selftest can drive them ------------------------------------------------

def problems(modules, unblocked, rows, base_rows=None, renames=()):
    """Every failure in one tree. `base_rows` is None where the merge base holds no baseline."""
    if not modules:
        return ['the check judged no file at all. A selector that matches nothing passes by '
                'checking nothing (design/roadmap/0401-a-gate-that-selects-the-set-it-judges.md)']
    bad = []
    for row, n in sorted((unblocked - rows).items()):
        bad.append(f'{row}: no `Name:` block, and {"it is" if n == 1 else f"{n} more are"} not on '
                   f'the list. A new name carries its block; {remedy(row)}')
    for row, n in sorted((rows - unblocked).items()):
        gone = path_of(row) not in modules or '::' in row
        why = 'has no unblocked name behind it' if gone else 'carries a block now'
        bad.append(f'{BASELINE}: {row} {why}{f" ({n} rows)" if n > 1 else ""}. Remove it (--bank); '
                   f'the list cannot go stale')
    if base_rows is not None:
        was_called = {new: old for old, new in renames}

        def before(row):
            path = path_of(row)
            old = was_called.get(path)
            return row if old is None else old + row[len(path):]

        moved = collections.Counter()
        for row, n in rows.items():
            moved[before(row)] += n
        for row, n in sorted((moved - base_rows).items()):
            bad.append(f'{BASELINE}: {row} was added. The list only shrinks; {remedy(row)}')
    return bad


# --- the verbs ------------------------------------------------------------------------------------

def run_check():
    if not os.path.exists(BASELINE):
        print(f'module-block ratchet: {BASELINE} is missing. The gate and its list are one change; '
              f'deleting one deletes the gate', file=sys.stderr)
        return 1
    modules, unblocked = tree_modules()
    rows = read_baseline(open(BASELINE).read())
    base, base_rows, renames = merge_base(), None, ()
    if base:
        renames = renamed_rust(base)
        old = git('show', f'{base}:{BASELINE}')
        if old is not None and FORMAT in old.splitlines():
            base_rows = read_baseline(old)
    bad = problems(modules, unblocked, rows, base_rows, renames)
    if bad:
        print('module-block ratchet: the tree breaks its baseline:', file=sys.stderr)
        for b in bad:
            print(f'  {b}', file=sys.stderr)
        return 1
    print(f'module-block ratchet: {len(modules)} module and document files, '
          f'{sum(unblocked.values())} names with no block, held to {sum(rows.values())} baseline rows')
    return 0


def bank():
    if not os.path.exists(BASELINE):
        print(f'module-block ratchet: {BASELINE} does not exist; there is nothing to bank',
              file=sys.stderr)
        return 2
    _modules, unblocked = tree_modules()
    rows = read_baseline(open(BASELINE).read()) & unblocked
    write_baseline(rows)
    print(f'module-block ratchet: {sum(rows.values())} rows written to {BASELINE}')
    return 0


def selftest():
    def m(*names):
        return collections.Counter(f'x/src/{n}.rs' for n in names)

    def i(*names):
        return collections.Counter(f'x/src/a.rs::{n}' for n in names)

    files = {'x/src/a.rs'}

    cases = [
        ('a new module with no block fails', bool(problems(m('a', 'new'), m('new'), collections.Counter()))),
        ('a new module with a block passes', not problems(m('a', 'new'), collections.Counter(), collections.Counter())),
        ('a listed module with no block passes', not problems(m('a'), m('a'), m('a'), m('a'))),
        ('a listed module that gained a block fails', bool(problems(m('a'), collections.Counter(), m('a')))),
        ('a row for a deleted module fails', bool(problems(m('b'), collections.Counter(), m('a')))),
        ('a row added against the merge base fails',
         bool(problems(m('a', 'b'), m('a', 'b'), m('a', 'b'), m('a')))),
        ('a rename that moves its row passes',
         not problems(m('new'), m('new'), m('new'), m('old'), [('x/src/old.rs', 'x/src/new.rs')])),
        ('a rename that leaves its row behind fails',
         bool(problems(m('new'), m('new'), m('old'), m('old'), [('x/src/old.rs', 'x/src/new.rs')]))),
        ('a selector that matches nothing fails loudly',
         bool(problems(set(), collections.Counter(), collections.Counter()))),
        ('a new pub item with no block fails', bool(problems(files, i('f', 'g'), i('f')))),
        ('a third unblocked `new` beside two listed fails',
         bool(problems(files, i('new', 'new', 'new'), i('new', 'new')))),
        ('two listed `new`, one now blocked, leaves a stale row',
         bool(problems(files, i('new'), i('new', 'new')))),
        ('an item row added against the merge base fails',
         bool(problems(files, i('f', 'g'), i('f', 'g'), i('f')))),
        ('a rename carries its item rows',
         not problems({'x/src/b.rs'}, collections.Counter({'x/src/b.rs::f': 1}),
                      collections.Counter({'x/src/b.rs::f': 1}), i('f'),
                      [('x/src/a.rs', 'x/src/b.rs')])),
        ('pub constants, statics, methods and qualified fns are items',
         name_census.pub_items('pub const A: u8 = 0;\npub static mut B: u8 = 0;\n'
                               '    pub fn new() {}\npub const unsafe fn c() {}\n'
                               'pub struct D;\npub trait E {}') == ['A', 'B', 'new', 'c', 'D', 'E']),
        ('pub(crate), mod and private items are not',
         name_census.pub_items('pub(crate) fn a() {}\npub mod b;\nfn c() {}\n') == []),
        ('a file under src/ is a module', name_census.is_module_file('crates/a/src/b/c.rs')),
        ('a file under tests/ is not', not name_census.is_module_file('crates/a/tests/c.rs')),
        ('build.rs is not', not name_census.is_module_file('kernel/build.rs')),
        ('vendored code is not', not name_census.is_module_file('vendor/a/src/b.rs')),
        ('code quoted in a note is not', not name_census.is_module_file('notes/a/src/b.rs')),
        ('a top-level note is a document', name_census.is_top_document('notes/a.md')),
        ('a numbered record is not', not name_census.is_top_document('design/0001-a.md')),
        ('a README is not', not name_census.is_top_document('notes/README.md')),
        ('a nested document is not (its directory gates it)',
         not name_census.is_top_document('notes/a/b.md')),
    ]
    msg = problems(set(m('a', 'new')), m('new'), collections.Counter())[0]
    cases.append(('the failure names the file and the block to write',
                  'x/src/new.rs' in msg and '//! Name: provisional' in msg))
    failed = [name for name, ok in cases if not ok]
    for name in failed:
        print(f'module-block ratchet selftest: {name}', file=sys.stderr)
    if not failed:
        print(f'module-block ratchet selftest: {len(cases)} cases pass')
    return 1 if failed else 0


def main(argv):
    root = git('rev-parse', '--show-toplevel')
    if root:
        os.chdir(root.strip())
    if argv and argv[0] == '--selftest':
        return selftest()
    if not argv or argv[0] == '--check':
        return run_check()
    if argv[0] == '--bank':
        return bank()
    if argv[0] == '--init' and not os.path.exists(BASELINE):
        write_baseline(tree_modules()[1])
        print(f'module-block ratchet: wrote {BASELINE}')
        return 0
    print(__doc__.split('\n\n')[1], file=sys.stderr)
    return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
