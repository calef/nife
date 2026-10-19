"""Per-section wall time from CI lint job logs (`gh run view <run> --log --job <job>` saved to files). Usage: cistages.py LOG..."""
import sys,re,datetime as d,statistics as st
from collections import defaultdict
agg=defaultdict(list); tot=[]
for fn in sys.argv[1:]:
    rows=[]
    for l in open(fn,errors='replace'):
        p=l.split('\t')
        if len(p)<3 or not p[1].startswith('Run script/ci-build lint'): continue
        m=re.match(r'(\S+Z) (.*)',p[2].rstrip('\n').lstrip('\ufeff'))
        if not m: continue
        ts=d.datetime.fromisoformat(m.group(1)[:26]+'+00:00'); rows.append((ts,m.group(2)))
    if not rows: continue
    st_=[(ts,t[4:].strip()) for ts,t in rows if t.startswith('==> ')]
    end=rows[-1][0]
    for i,(ts,n) in enumerate(st_):
        nx=st_[i+1][0] if i+1<len(st_) else end
        agg[n].append((nx-ts).total_seconds())
    tot.append((end-rows[0][0]).total_seconds())
print('runs',len(tot),'total median',st.median(tot))
cargo=0;nonc=0
for n,v in sorted(agg.items(),key=lambda kv:-st.median(kv[1])):
    m=st.median(v)
    if n.startswith(('clippy','uefi_loader','rustdoc','the lint ratchet (','unused dep','the host pass','mutation','the kernel\'s dep','every crate with')): cargo+=m
    else: nonc+=m
    print(f"{m:7.1f} {n}")
print('cargo-ish',cargo,'other',nonc)
