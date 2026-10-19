#!/usr/bin/env python3
"""Run each script/lint section alone (preamble + that section). Usage: sections.py TREE OUT.tsv [--no-cargo] [--only REGEX]"""
import os, re, subprocess, sys, tempfile, time
tree, out = sys.argv[1], sys.argv[2]
nocargo = '--no-cargo' in sys.argv
only = sys.argv[sys.argv.index('--only')+1] if '--only' in sys.argv else None
lint = os.path.join(tree, 'script/lint')
text = open(lint).read().split('\n')
# Reuse the tree's own partition so "cargo section" means what --no-cargo means.
lst = subprocess.run(['awk','-v','mode=list','-f',os.path.join(tree,'helpers/lint-no-cargo.awk'),lint],capture_output=True,text=True).stdout.split('\n')
kind = {l[6:].removeprefix('==> '): l[:4].strip() for l in lst if l}
pre, secs, cur = [], [], None
for line in text:
    if line.startswith('echo "==> '):
        cur = [line[10:-1], [line]]; secs.append(cur)
    elif cur is None: pre.append(line)
    else: cur[1].append(line)
# The preamble ends in the mode dispatch (`--baselines`, `--no-cargo`); argv is empty here so both fall through.
# LINT_SECTION skips the preamble's run-once block (the heavy-job slot, the mode dispatch), as
# helpers/lint-sections.sh does.
env = dict(os.environ, LINT_SECTION='1', **({'LINT_NO_CARGO': '1'} if nocargo else {}))
with open(out, 'w') as f:
    for i, (name, body) in enumerate(secs):
        k = kind.get(name, '?')
        if nocargo and k == 'skip': continue
        if only and not re.search(only, name): continue
        fd, path = tempfile.mkstemp(suffix='.sh'); os.write(fd, ('\n'.join(pre + body) + '\n').encode()); os.close(fd)
        t0 = time.monotonic()
        p = subprocess.run(['sh','-c','lint_tmp=$1; shift; . "$lint_tmp"', lint, path], cwd=tree, env=env, capture_output=True, text=True)
        dt = time.monotonic() - t0; os.unlink(path)
        tail = (p.stdout + p.stderr).strip().split('\n')[-1][:200].replace('\t',' ')
        f.write(f"{i}\t{k}\t{p.returncode}\t{dt:.2f}\t{name}\t{tail}\n"); f.flush()
