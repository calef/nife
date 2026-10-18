import os, sys
d = sys.argv[1]; pk = sys.argv[2]
sz = lambda p: os.path.getsize(os.path.join(d, p))
cols = [('NIFEPKG1', lambda s: os.path.getsize(f'{pk}/{s}.nifepkg')),
        ('NIFEPKG2', lambda s: sz(f'{s}.nifepkg2')),
        ('N2.zst-3', lambda s: sz(f'{s}.nifepkg2.3.zst')),
        ('N2.zst-19', lambda s: sz(f'{s}.nifepkg2.19.zst')),
        ('N2.zst-20', lambda s: sz(f'{s}.nifepkg2.20.zst')),
        ('tar', lambda s: sz(f'{s}.tar')),
        ('tar-rec10240', lambda s: sz(f'{s}.rec10240.tar')),
        ('tar.zst-3', lambda s: sz(f'{s}.tar.3.zst')),
        ('tar.zst-19', lambda s: sz(f'{s}.tar.19.zst')),
        ('tar.zst-20', lambda s: sz(f'{s}.tar.20.zst')),
        ('N1.zst-19', lambda s: sz(f'{s}.nifepkg.nifepkg1.19.zst'))]
stems = sorted(f[:-len('.nifepkg')] for f in os.listdir(pk) if f.endswith('.nifepkg'))
print('| package | ' + ' | '.join(c for c, _ in cols) + ' |'); print('|---' * (len(cols) + 1) + '|')
tot = {}
for s in stems:
    arch = s.rsplit('-', 1)[1]; row = [f(s) for _, f in cols]
    tot.setdefault(arch, [0] * len(cols)); tot[arch] = [a + b for a, b in zip(tot[arch], row)]
    print(f'| {s} | ' + ' | '.join(f'{v:,}' for v in row) + ' |')
for a, row in sorted(tot.items()):
    print(f'| **all six, {a}** | ' + ' | '.join(f'{v:,}' for v in row) + ' |')
allr = [sum(x) for x in zip(*tot.values())]
print('| **all 18** | ' + ' | '.join(f'{v:,}' for v in allr) + ' |')
print('| ratio to NIFEPKG2 | ' + ' | '.join(f'{v/allr[1]:.3f}' for v in allr) + ' |')
