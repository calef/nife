import struct, hashlib
NAME_LEN=32; HEADER_LEN=8+96+8; MEMBER_LEN=72
def field(b): return b.split(b'\0',1)[0].decode()
def read(path):
    b=open(path,'rb').read()
    assert b[:8]==b'NIFEPKG1'
    name,ver,arch=(field(b[8+32*i:8+32*(i+1)]) for i in range(3))
    count,=struct.unpack_from('<I',b,104)
    members=[]
    for i in range(count):
        e=b[HEADER_LEN+i*MEMBER_LEN:HEADER_LEN+(i+1)*MEMBER_LEN]
        mn=field(e[:32]); off,ln=struct.unpack_from('<II',e,32); dg=e[40:72]
        data=b[off:off+ln]; assert hashlib.sha256(data).digest()==dg
        members.append((mn,data))
    return dict(name=name,version=ver,arch=arch,members=members,raw=b)
