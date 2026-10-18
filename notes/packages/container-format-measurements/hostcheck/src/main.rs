use readers::{n2, tar_own};
use sha2::{Digest, Sha256};
use std::io::Read;
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mode = a[0].as_str();
    for p in &a[1..] {
        let b = std::fs::read(p).unwrap();
        match mode {
            "n2" => { let k = n2::Package::parse(&b).expect(p); let mut n = 0; for i in 0..k.len() { let e = k.member(i).unwrap(); assert_eq!(Sha256::digest(e.data).as_slice(), e.digest); n += 1; } println!("n2 ok {p} {} {} {} {n}", k.name(), k.version(), k.architecture()); }
            "tar" => { let k = tar_own::Package::parse(&b).expect(p); for i in 0..k.len() { let e = k.member(i).unwrap(); assert_eq!(Sha256::digest(e.data).as_slice(), e.digest); println!("  {}{}{} {}", e.name.prefix, if e.name.prefix.is_empty() {""} else {"/"}, e.name.name.len(), e.data.len()); } println!("tar_own ok {p} {}", k.len()); }
            "tns" => { match tar_no_std::TarArchiveRef::new(&b) { Ok(t) => for e in t.entries() { println!("  tar-no-std: name len {} {:?}... size {}", e.filename().as_str().map(|s| s.len()).unwrap_or(0), e.filename().as_str().map(|s| &s[..s.len().min(12)]), e.size()); }, Err(e) => println!("  tar-no-std refused: {e:?}") } }
            "zst" => { let t = std::time::Instant::now(); let mut d = ruzstd::decoding::StreamingDecoder::new(&b[..]).unwrap(); let mut out = Vec::new(); d.read_to_end(&mut out).unwrap(); let el = t.elapsed(); let orig = std::fs::read(p.rsplitn(3, '.').nth(2).unwrap()).unwrap(); assert!(out == orig, "{p}"); println!("zst ok {p} {} bytes {:.1} ms", out.len(), el.as_secs_f64()*1e3); }
            _ => panic!(),
        }
    }
}
