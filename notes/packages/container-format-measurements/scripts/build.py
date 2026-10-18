"""Re-encode basalt's NIFEPKG1 packages as NIFEPKG2 (modeled on §307 (how long a name may be) rule 3) and as tar."""
import sys, os, struct, hashlib
sys.path.insert(0, os.path.dirname(__file__)); import nifepkg1

def pad(n, a): return (-n) % a

def nifepkg2(d):
    h = bytearray(b'NIFEPKG2')
    for f in (d['name'], d['version'], d['arch']):
        fb = f.encode(); h += struct.pack('<H', len(fb)) + fb
    h += struct.pack('<II', len(d['members']), 0)  # count, reserved (the decision's header table counts it)
    # table: u16 name len, name, u32 offset, u32 len, 32-byte digest
    toc_len = sum(2 + len(m.encode()) + 4 + 4 + 32 for m, _ in d['members'])
    off = len(h) + toc_len; off += pad(off, 8)
    toc = bytearray(); body = bytearray(); cur = off
    for m, data in d['members']:
        mb = m.encode()
        toc += struct.pack('<H', len(mb)) + mb + struct.pack('<II', cur, len(data)) + hashlib.sha256(data).digest()
        body += data + b'\0' * pad(len(data), 8); cur += len(data) + pad(len(data), 8)
    out = h + toc; out += b'\0' * pad(len(out), 8); assert len(out) == off
    out += body
    # the last member needs no trailing pad
    last = len(d['members'][-1][1]); out = out[:len(out) - pad(last, 8)]
    return bytes(out), len(h) + toc_len

def manifest(d):
    s = f"name {d['name']}\nversion {d['version']}\narchitecture {d['arch']}\n"
    for m, data in d['members']:
        s += f"member {hashlib.sha256(data).hexdigest()} {m}\n"
    return s.encode()

def octal(n, w): return (b'%0*o' % (w - 1, n)) + b'\0'

def ustar_header(name, size, mode, typeflag=b'0'):
    nb = name.encode(); prefix = b''
    if len(nb) > 100:
        i = nb.rfind(b'/', 0, 156)
        if i > 0 and len(nb) - i - 1 <= 100: prefix, nb = nb[:i], nb[i+1:]
        else: return None
    h = bytearray(512)
    h[0:len(nb)] = nb; h[100:108] = octal(mode, 8); h[108:116] = octal(0, 8); h[116:124] = octal(0, 8)
    h[124:136] = octal(size, 12); h[136:148] = octal(0, 12); h[148:156] = b' ' * 8
    h[156:157] = typeflag; h[257:263] = b'ustar\0'; h[263:265] = b'00'
    h[345:345+len(prefix)] = prefix
    h[148:156] = b'%06o\0 ' % sum(h)
    return bytes(h)

def pax_record(k, v):
    body = f' {k}={v}\n'.encode(); n = len(body)
    while len(str(n).encode()) + len(body) != n: n = len(str(n).encode()) + len(body)
    return str(n).encode() + body

def tar_member(name, data, mode, force_pax=False):
    out = b''
    h = None if force_pax else ustar_header(name, len(data), mode)
    if h is None:
        rec = pax_record('path', name)
        out += ustar_header('PaxHeader/x', len(rec), 0o644, b'x') + rec + b'\0' * pad(len(rec), 512)
        h = ustar_header(name[-100:].lstrip('/') if len(name) > 100 else name, len(data), mode)
        if h is None: h = ustar_header('long', len(data), mode)
    return out + h + data + b'\0' * pad(len(data), 512)

def tar(d, record=512, force_pax=False):
    out = tar_member('MANIFEST', manifest(d), 0o644, force_pax)
    for m, data in d['members']:
        out += tar_member(m, data, 0o755 if m.startswith('bin/') else 0o644, force_pax)
    out += b'\0' * 1024
    out += b'\0' * pad(len(out), record)
    return out

if __name__ == '__main__':
    outdir = sys.argv[1]
    for p in sys.argv[2:]:
        d = nifepkg1.read(p); stem = os.path.basename(p)[:-len('.nifepkg')]
        n2, n2hdr = nifepkg2(d)
        open(f'{outdir}/{stem}.nifepkg2', 'wb').write(n2)
        open(f'{outdir}/{stem}.tar', 'wb').write(tar(d))
        open(f'{outdir}/{stem}.rec10240.tar', 'wb').write(tar(d, 10240))
        print(stem, len(d['raw']), len(n2), n2hdr, len(manifest(d)), len(tar(d)), sep='\t')
