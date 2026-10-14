"""The outsider-pass launcher (`script/launch-pass`): one reviewer, pinned, with a record of what ran.

Milestone 920 (an outsider pass is launched pinned and floor-checked). Pass 8 was started with a
milestone number as its prompt and read the lane's instructions as its own; pass 9 was configured
for one model and served by another, which nobody saw until it had finished. This makes both
mistakes impossible to make quietly.

    script/launch-pass --reviewer glm --commit SHA      # GLM 5.3 through opencode
    script/launch-pass --reviewer fable --commit SHA    # Claude Fable through Claude Code
    script/launch-pass ... --dry-run                    # the clone and the prompt, no session
    script/launch-pass --complete RECORD --short-rows FILE   # the one completion round
    script/launch-pass --selftest

Name: provisional, minted by milestone 920's lane on 2026-10-14 (UTC). A shared python module
under `helpers/`, which `script/names` puts out of its own scope, so its provenance is this
paragraph; the entry point's `Name:` block is in `script/launch-pass`.

**What one launch does, in order.** A standalone clone with no remote and one branch, `pass`, at
the pinned commit. The prompt: the reviewer's brief from `briefs/confinement-outsider-pass.md` and
the runner section from `briefs/confinement-outsider-pass-launch.md`, placeholders filled, and
refused if it names a milestone or a roadmap block. Then a setup turn in a new session, with the
model pinned on the harness's command line and no tools: the launcher reads back the model the
session reports and stops there, before the brief is ever sent, if it differs from the pin. Only
then does the same session get the prompt. The launch record (`launch.json`, beside the clone and
outside every repository) holds the harness and its version, the model pinned and reported, the
session id, the commit, the prompt's SHA-256 and the start time.

**The completion round** resumes that session once with the checker's short-row lines as the whole
message, and writes to the record before it sends, so a second `--complete` is refused even after
a crash.
"""
from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
import uuid

REVIEWERS = {
    'glm': {'harness': 'opencode', 'model': 'zai-coding-plan/glm-5.3',
            'trailer': 'GLM 5.3 <noreply@z.ai>'},
    'fable': {'harness': 'claude', 'model': 'claude-fable-5-1',
              'trailer': 'Claude Fable 5.1 <noreply@anthropic.com>'},
}
PROBE = ('This is a setup check before your task arrives. Reply with the single word READY. '
         'Run no tool and read no file.')
# What a prompt may never be: the lane's own instructions. A reviewer handed a milestone number or a
# roadmap block follows the lane's brief instead of its own (pass 8).
REFUSED = (
    (re.compile(r'(?i)\bmilestones?\s+\d+'), 'names a milestone number'),
    (re.compile(r'design/roadmap/'), 'names a roadmap block'),
    (re.compile(r'(?m)^(milestone_dependencies|decision_dependencies|needs_person):'),
     'holds a roadmap block\'s frontmatter'),
    (re.compile(r'^\s*\d+\s*$'), 'is a bare milestone number'),
)
SHORT_ROW = re.compile(r'^row \d+: (?!booted|host \(exempt\)).+$')


def check_prompt(text):
    """Refuse a prompt that is not the reviewer's brief and the runner section."""
    for pattern, why in REFUSED:
        if pattern.search(text):
            raise SystemExit('launch-pass: refused: the prompt %s; it must be the reviewer\'s brief '
                             'and the runner section, nothing else' % why)
    for heading in ('## Before you start', '### Your job', '### Output'):
        if heading not in text:
            raise SystemExit('launch-pass: refused: the prompt has no `%s`' % heading)


def section(text, heading):
    """The body under one `## ` heading, up to the next heading of the same level."""
    found = re.search(r'(?ms)^%s\n(.*?)(?=^## |\Z)' % re.escape(heading), text)
    if not found:
        raise SystemExit('launch-pass: no `%s` section' % heading)
    return found.group(1).strip() + '\n'


def assemble(tree, scratch, commit, reviewer):
    """The one prompt: the runner section, then the reviewer's brief, placeholders filled."""
    with open(os.path.join(tree, 'briefs/confinement-outsider-pass.md')) as brief, \
            open(os.path.join(tree, 'briefs/confinement-outsider-pass-launch.md')) as launch:
        reviewer_brief = section(brief.read(), "## The reviewer's brief")
        runner = section(launch.read(), '## The runner section')
    fill = {'<checkout>': os.path.join(scratch, 'nife'), '<worktree>': os.path.join(scratch, 'nife'),
            '<commit>': commit, '<trailer>': reviewer['trailer'],
            '<path>': os.path.join(scratch, 'nife', 'notes/confinement-claims.md'),
            '<results path>': os.path.join(scratch, 'results.md'),
            '<logs>': os.path.join(scratch, 'logs')}
    text = '## Before you start\n\n' + runner + '\n' + reviewer_brief
    for placeholder, value in fill.items():
        text = text.replace(placeholder, value)
    return text


def git(*args, cwd=None):
    done = subprocess.run(['git', *args], cwd=cwd, capture_output=True, text=True)
    if done.returncode != 0:
        raise SystemExit('launch-pass: git %s: %s' % (' '.join(args), done.stderr.strip()))
    return done.stdout.strip()


def make_clone(tree, scratch, commit):
    """A standalone clone with no remote and one branch, `pass`, at the pinned commit."""
    if not re.fullmatch(r'[0-9a-f]{40}', commit):
        raise SystemExit('launch-pass: --commit wants a full 40-character SHA, not %r' % commit)
    source = git('rev-parse', '--path-format=absolute', '--git-common-dir', cwd=tree)
    git('cat-file', '-e', commit + '^{commit}', cwd=tree)
    clone = os.path.join(scratch, 'nife')
    git('clone', '--quiet', '--no-hardlinks', source, clone)
    git('remote', 'remove', 'origin', cwd=clone)
    git('switch', '--quiet', '-c', 'pass', commit, cwd=clone)
    for branch in git('for-each-ref', '--format=%(refname:short)', 'refs/heads', cwd=clone).split():
        if branch != 'pass':
            git('branch', '--quiet', '-D', branch, cwd=clone)
    os.makedirs(os.path.join(scratch, 'logs'), exist_ok=True)
    return clone


def run_harness(argv, log_path, cwd):
    """Run one harness turn, keep its event stream, and return the stream's JSON objects."""
    with open(log_path, 'a') as log:
        done = subprocess.run(argv, cwd=cwd, stdout=subprocess.PIPE, stderr=log, text=True)
        log.write(done.stdout)
    events = []
    for line in done.stdout.splitlines():
        try:
            events.append(json.loads(line))
        except ValueError:
            continue
    return done.returncode, events


class Claude:
    """Claude Code headless: `init` names the session and model; each assistant message, the model."""
    name = 'claude'
    # The reviewer runs with permission prompts off, so it gets the shell and the files and nothing
    # else: no web, no MCP server (the maintainers' connectors reach mail and documents), and no
    # user-level settings or instructions, which hold machine-local facts the tree keeps private.
    TOOLS = 'Bash,Read,Edit,Write,Glob,Grep'
    FENCE = ('--strict-mcp-config', '--setting-sources', 'project')

    def __init__(self):
        self.binary = os.environ.get('LAUNCH_PASS_CLAUDE', 'claude')

    def version(self):
        return subprocess.run([self.binary, '--version'], capture_output=True, text=True).stdout.strip()

    def models(self, events):
        found = set()
        for event in events:
            if event.get('type') == 'system' and event.get('subtype') == 'init':
                found.add(event.get('model'))
            if event.get('type') == 'assistant':
                found.add(event.get('message', {}).get('model'))
        return {m for m in found if m}

    def probe(self, model, clone, log):
        session = str(uuid.uuid4())
        argv = [self.binary, '-p', '--model', model, '--session-id', session, '--tools', '',
                *self.FENCE, '--output-format', 'stream-json', '--verbose', PROBE]
        _, events = run_harness(argv, log, clone)
        return session, self.models(events)

    def send(self, session, model, message, clone, log):
        argv = [self.binary, '-p', '--resume', session, '--model', model, '--tools', self.TOOLS,
                *self.FENCE, '--permission-mode', 'bypassPermissions', '--output-format',
                'stream-json', '--verbose', message]
        status, events = run_harness(argv, log, clone)
        return status, self.models(events)


class Opencode:
    """opencode headless: the event stream names the session; its exported record, the model."""
    name = 'opencode'

    def __init__(self):
        self.binary = os.environ.get('LAUNCH_PASS_OPENCODE', 'opencode')

    def version(self):
        return subprocess.run([self.binary, '--version'], capture_output=True, text=True).stdout.strip()

    def models(self, session, clone):
        done = subprocess.run([self.binary, 'export', session], cwd=clone, capture_output=True,
                              text=True)
        record = json.loads(done.stdout[done.stdout.find('{'):] or '{}')
        return {'%s/%s' % (m['info'].get('providerID'), m['info'].get('modelID'))
                for m in record.get('messages', []) if m.get('info', {}).get('role') == 'assistant'}

    def probe(self, model, clone, log):
        argv = [self.binary, 'run', '-m', model, '--dir', clone, '--format', 'json', PROBE]
        _, events = run_harness(argv, log, clone)
        sessions = {e.get('sessionID') or e.get('part', {}).get('sessionID') for e in events}
        sessions.discard(None)
        if len(sessions) != 1:
            raise SystemExit('launch-pass: opencode named %d sessions in its setup turn' % len(sessions))
        session = sessions.pop()
        return session, self.models(session, clone)

    def send(self, session, model, message, clone, log):
        argv = [self.binary, 'run', '--session', session, '-m', model, '--auto', '--dir', clone,
                '--format', 'json', message]
        status, _ = run_harness(argv, log, clone)
        return status, self.models(session, clone)


HARNESSES = {'claude': Claude, 'opencode': Opencode}


def now():
    return datetime.datetime.now(datetime.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')


def write_record(path, record):
    with open(path, 'w') as out:
        json.dump(record, out, indent=2, sort_keys=True)
        out.write('\n')


def launch(args):
    reviewer = dict(REVIEWERS[args.reviewer])
    reviewer['model'] = args.model or reviewer['model']
    given = None
    if args.prompt:
        with open(args.prompt) as source:
            given = source.read()
        check_prompt(given)
    scratch = os.path.abspath(args.scratch or tempfile.mkdtemp(prefix='nife-review-'))
    clone = make_clone(args.tree, scratch, args.commit)
    prompt = given or assemble(args.tree, scratch, args.commit, reviewer)
    check_prompt(prompt)
    with open(os.path.join(scratch, 'prompt.md'), 'w') as out:
        out.write(prompt)
    print('launch-pass: clone %s at %s; prompt %s' % (clone, args.commit, os.path.join(scratch, 'prompt.md')))
    if args.dry_run:
        return 0
    harness = HARNESSES[reviewer['harness']]()
    record_path = os.path.join(scratch, 'launch.json')
    record = {'harness': harness.name, 'harness_version': harness.version(),
              'model_pinned': reviewer['model'], 'commit': args.commit, 'clone': clone,
              'prompt_sha256': hashlib.sha256(prompt.encode()).hexdigest(), 'started': now()}
    session, reported = harness.probe(reviewer['model'], clone, os.path.join(scratch, 'setup.log'))
    record.update(session_id=session, model_reported=sorted(reported))
    if reported != {reviewer['model']}:
        record['status'] = 'stopped: the session reported %s, not the pinned model' % sorted(reported)
        write_record(record_path, record)
        print('launch-pass: %s; the brief was not sent. Record: %s' % (record['status'], record_path))
        return 1
    record['status'] = 'attacking'
    write_record(record_path, record)
    print('launch-pass: session %s on %s; sending the brief. Record: %s' % (session, reviewer['model'], record_path))
    status, seen = harness.send(session, reviewer['model'], prompt, clone, os.path.join(scratch, 'attack.log'))
    record.update(status='reported' if status == 0 else 'harness exited %d' % status,
                  models_seen=sorted(seen), finished=now())
    write_record(record_path, record)
    return status


def complete(args):
    """Resume the recorded session once, with the checker's short rows as the whole message."""
    with open(args.complete) as source:
        record = json.load(source)
    if 'completion' in record:
        raise SystemExit('launch-pass: refused: this session already had its completion round (%s)'
                         % record['completion']['started'])
    if record.get('status') != 'reported':
        raise SystemExit('launch-pass: refused: the record says %r, not a finished pass' % record.get('status'))
    with (sys.stdin if args.short_rows == '-' else open(args.short_rows)) as source:
        lines = [line.rstrip() for line in source if line.strip()]
    if not lines or not all(SHORT_ROW.match(line) for line in lines):
        raise SystemExit('launch-pass: refused: --short-rows must be `script/booted-rows --short-only` '
                         'output, short rows only, one `row N: reason` per line')
    message = '\n'.join(lines) + '\n'
    record['completion'] = {'started': now(), 'message_sha256': hashlib.sha256(message.encode()).hexdigest()}
    write_record(args.complete, record)
    harness = HARNESSES[record['harness']]()
    scratch = os.path.dirname(os.path.abspath(args.complete))
    status, seen = harness.send(record['session_id'], record['model_pinned'], message,
                                record['clone'], os.path.join(scratch, 'completion.log'))
    record['completion'].update(finished=now(), status=status, models_seen=sorted(seen))
    write_record(args.complete, record)
    return status


# ----------------------------------------------------------------------------------------------
# The selftest: a fake harness on PATH, a two-file repository, and no model call at all.
# ----------------------------------------------------------------------------------------------

FAKE_HARNESS = r'''#!/usr/bin/env python3
import json, os, sys
argv = sys.argv[1:]
with open(os.environ['FAKE_CALLS'], 'a') as calls:
    calls.write(json.dumps(argv) + '\n')
model = os.environ['FAKE_SERVED']
if argv[:1] == ['--version']:
    print('fake 1.0')
elif argv[:1] == ['export']:
    print('Exporting session'); print(json.dumps({'messages': [{'info': {'role': 'assistant',
        'providerID': model.split('/')[0], 'modelID': model.split('/')[-1]}}]}))
elif argv[:1] == ['run']:
    print(json.dumps({'type': 'text', 'sessionID': 'ses_fake'}))
else:
    print(json.dumps({'type': 'system', 'subtype': 'init', 'model': model, 'session_id': 's'}))
    print(json.dumps({'type': 'assistant', 'message': {'model': model}}))
'''
BRIEF = ("# Brief\n\n## The reviewer's brief\n\n### Your job\n\nAttack `<worktree>` at `<path>`.\n\n"
         "### Output\n\nWrite `<results path>`.\n")
LAUNCH = '# Launch\n\n## The runner section\n\nCommit `<commit>`; end with `<trailer>`.\n'


def selftest_tree(tmp):
    tree = os.path.join(tmp, 'tree')
    os.makedirs(os.path.join(tree, 'briefs'))
    for name, text in (('confinement-outsider-pass.md', BRIEF), ('confinement-outsider-pass-launch.md', LAUNCH)):
        with open(os.path.join(tree, 'briefs', name), 'w') as out:
            out.write(text)
    git('init', '-q', cwd=tree)
    git('add', '-A', cwd=tree)
    git('-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-qm', 'pinned', cwd=tree)
    fake = os.path.join(tmp, 'fake-harness')
    with open(fake, 'w') as out:
        out.write(FAKE_HARNESS)
    os.chmod(fake, 0o755)
    os.environ.update(LAUNCH_PASS_CLAUDE=fake, LAUNCH_PASS_OPENCODE=fake,
                      FAKE_CALLS=os.path.join(tmp, 'calls'))
    return tree, git('rev-parse', 'HEAD', cwd=tree)


def selftest_launch(tree, tmp, reviewer, served, commit):
    """One launch against the fake harness; returns its exit, its record, and every harness call."""
    os.environ['FAKE_SERVED'] = served
    open(os.environ['FAKE_CALLS'], 'w').close()
    scratch = os.path.join(tmp, 'pass-%s-%s' % (reviewer, served.replace('/', '-')))
    args = argparse.Namespace(tree=tree, reviewer=reviewer, model=None, scratch=scratch,
                              commit=commit, dry_run=False, prompt=None)
    status = launch(args)
    with open(os.path.join(scratch, 'launch.json')) as source:
        record = json.load(source)
    with open(os.environ['FAKE_CALLS']) as source:
        calls = [json.loads(line) for line in source]
    return status, record, calls, os.path.join(scratch, 'launch.json')


def expect_refusal(action, needle):
    try:
        action()
    except SystemExit as refused:
        assert needle in str(refused), refused
        return
    raise AssertionError('expected a refusal naming %r' % needle)


def selftest():
    expect_refusal(lambda: check_prompt('Do milestone ' + '913'), 'milestone number')
    expect_refusal(lambda: check_prompt('913'), 'milestone number')
    expect_refusal(lambda: check_prompt('Read design/roadmap/0913-x.md'), 'roadmap block')
    with tempfile.TemporaryDirectory() as tmp:
        tree, commit = selftest_tree(tmp)
        prompt = assemble(tree, '/s', commit, REVIEWERS['glm'])
        check_prompt(prompt)
        assert '/s/nife' in prompt and commit in prompt and 'noreply@z.ai' in prompt, prompt
        hand = os.path.join(tmp, 'hand-prompt')
        with open(hand, 'w') as out:
            out.write('Do milestone ' + '913\n')
        open(os.environ['FAKE_CALLS'], 'w').close()
        args = argparse.Namespace(tree=tree, reviewer='glm', model=None, scratch=os.path.join(tmp, 'm'),
                                  commit=commit, dry_run=False, prompt=hand)
        expect_refusal(lambda: launch(args), 'milestone number')
        assert not os.path.exists(args.scratch) and os.path.getsize(os.environ['FAKE_CALLS']) == 0
        for reviewer, wrong in (('fable', 'claude-opus-4-8'), ('glm', 'zai-coding-plan/glm-4.7')):
            status, record, calls, _ = selftest_launch(tree, tmp, reviewer, wrong, commit)
            assert status == 1 and record['status'].startswith('stopped'), record
            assert not any(prompt.splitlines()[0] in ' '.join(c) for c in calls), 'the brief went out'
            assert not any('--resume' in c or '--session' in c for c in calls), calls
        status, record, calls, path = selftest_launch(tree, tmp, 'fable', 'claude-fable-5-1', commit)
        assert status == 0 and record['status'] == 'reported' and record['model_reported'] == ['claude-fable-5-1']
        assert git('branch', '--format=%(refname:short)', cwd=record['clone']) == 'pass'
        assert git('remote', cwd=record['clone']) == ''
        rows = os.path.join(tmp, 'short')
        with open(rows, 'w') as out:
            out.write('row 10: host only\nrow 22: timed out on every ISA (inconclusive)\n')
        args = argparse.Namespace(complete=path, short_rows=rows)
        assert complete(args) == 0
        expect_refusal(lambda: complete(args), 'already had its completion round')
    print('launch-pass: selftest passed')
    return 0


def main(argv):
    parser = argparse.ArgumentParser(prog='script/launch-pass')
    parser.add_argument('--tree', default=os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    parser.add_argument('--reviewer', choices=sorted(REVIEWERS))
    parser.add_argument('--model', help='override the reviewer\'s pinned model id')
    parser.add_argument('--commit')
    parser.add_argument('--scratch', help='a new directory for the clone, prompt and record')
    parser.add_argument('--prompt', help='a hand-assembled prompt, checked like the assembled one')
    parser.add_argument('--dry-run', action='store_true')
    parser.add_argument('--complete', metavar='RECORD')
    parser.add_argument('--short-rows', default='-')
    parser.add_argument('--selftest', action='store_true')
    args = parser.parse_args(argv)
    if args.selftest:
        return selftest()
    if args.complete:
        return complete(args)
    if not (args.reviewer and args.commit):
        parser.error('--reviewer and --commit are required to launch')
    return launch(args)


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
