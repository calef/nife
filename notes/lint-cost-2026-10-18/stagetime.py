#!/usr/bin/env python3
"""Run a command, timestamp each '==> ' line, write per-stage seconds as TSV. Usage: stagetime.py OUT.tsv LOG -- cmd..."""
import subprocess, sys, time
out, log = sys.argv[1], sys.argv[2]
cmd = sys.argv[sys.argv.index('--')+1:]
t0 = time.monotonic(); stages = []; cur = ('<preamble>', t0)
with open(log, 'w') as lf:
    p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors='replace')
    for line in p.stdout:
        now = time.monotonic()
        lf.write(f"{now-t0:9.2f} {line}")
        if line.startswith('==> '):
            stages.append((cur[0], now - cur[1])); cur = (line[4:].strip(), now)
    rc = p.wait(); end = time.monotonic()
    stages.append((cur[0], end - cur[1]))
with open(out, 'w') as f:
    f.write(f"#exit\t{rc}\n#total\t{end-t0:.2f}\n")
    for n, s in stages: f.write(f"{s:.2f}\t{n}\n")
print(f"exit={rc} total={end-t0:.1f}s")
