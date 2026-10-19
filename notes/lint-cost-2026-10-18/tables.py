"""Median stage tables from stagetime.py output named <mode>-<n>.tsv in DIR. Usage: tables.py DIR"""
import glob,statistics as st,sys,re
S=sys.argv[1]
def load(f):
    d={}; tot=None
    for l in open(f):
        a,b=l.rstrip('\n').split('\t',1)
        if a=='#total': tot=float(b)
        elif not a.startswith('#'): d[b]=d.get(b,0)+float(a)
    return tot,d
CARGO=lambda n: n.startswith(('clippy','uefi_loader','rustdoc','the lint ratchet (clippy','unused dep','the host pass','mutation','the kernel\'s dep','every crate with'))
for mode in ['nocargo-cold','nocargo-warm','full-cold','full-warm']:
    fs=sorted(glob.glob(f'{S}/{mode}-*.tsv'))
    runs=[load(f) for f in fs]
    if not runs: continue
    tots=[r[0] for r in runs]
    c=[sum(v for k,v in r[1].items() if CARGO(k)) for r in runs]
    print(f"{mode}: n={len(runs)} totals={[round(t) for t in tots]} median={st.median(tots):.0f} cargo={[round(x) for x in c]}")
    names=runs[0][1].keys()
    med={n:st.median([r[1].get(n,0) for r in runs]) for n in names}
    for n,v in sorted(med.items(),key=lambda kv:-kv[1])[:12]: print(f"   {v:6.1f} {n[:90]}")
