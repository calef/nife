//! The tar candidates' MANIFEST member: `name X`, `version X`, `architecture X`, then one
//! `member <64 hex> <path>` line per member.
pub struct Manifest<'a> { pub text: &'a str, pub name: &'a str, pub version: &'a str, pub architecture: &'a str, pub members: usize }
fn hex(c: u8) -> Option<u8> { match c { b'0'..=b'9' => Some(c - b'0'), b'a'..=b'f' => Some(c - b'a' + 10), _ => None } }
impl<'a> Manifest<'a> {
    pub fn parse(bytes: &'a [u8]) -> Option<Self> {
        let text = core::str::from_utf8(bytes).ok()?;
        let mut m = Manifest { text, name: "", version: "", architecture: "", members: 0 };
        for line in text.split_terminator('\n') {
            let (k, v) = line.split_once(' ')?;
            match k {
                "name" => m.name = v, "version" => m.version = v, "architecture" => m.architecture = v,
                "member" => { let (d, p) = v.split_once(' ')?; if d.len() != 64 || p.is_empty() { return None; } decode(d)?; m.members += 1; }
                _ => return None,
            }
        }
        (!m.name.is_empty() && !m.version.is_empty() && !m.architecture.is_empty()).then_some(m)
    }
    pub fn digest(&self, path: &str) -> Option<[u8; 32]> {
        self.text.split_terminator('\n').filter_map(|l| l.strip_prefix("member ")).find_map(|v| {
            let (d, p) = v.split_once(' ')?; if p == path { decode(d) } else { None }
        })
    }
}
fn decode(d: &str) -> Option<[u8; 32]> {
    let b = d.as_bytes(); let mut out = [0u8; 32];
    for (i, o) in out.iter_mut().enumerate() { *o = hex(b[2 * i])? << 4 | hex(b[2 * i + 1])?; }
    Some(out)
}
pub fn decode_pub(d: &str) -> Option<[u8; 32]> { if d.len() != 64 { return None; } decode(d) }
