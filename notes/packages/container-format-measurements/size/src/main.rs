#![no_std]
#![no_main]
use core::hint::black_box as bb;
#[cfg(feature = "n2")] use readers::n2;
#[cfg(any(feature = "tar_own", feature = "tar_own_ustar", feature = "tar_no_std", feature = "manifest_only"))] use readers::manifest;
#[cfg(any(feature = "tar_own", feature = "tar_own_ustar"))] use readers::tar_own;

fn s(x: &[u8]) { bb(x.as_ptr()); bb(x.len()); }

#[cfg(feature = "none")]
fn exercise(buf: &[u8], want: &str) { s(buf); s(want.as_bytes()); }

#[cfg(feature = "n1")]
fn exercise(buf: &[u8], want: &str) {
    let Ok(p) = package_archive::Package::parse(buf) else { bb(0); return };
    s(p.name().as_bytes()); s(p.version().as_bytes()); s(p.architecture().as_bytes());
    for i in 0..p.len() {
        s(p.member_name(i).unwrap_or("").as_bytes()); s(p.member(i).unwrap_or(&[]));
        bb(p.member_digest(i).unwrap_or([0; 32]));
    }
    s(p.read(want).unwrap_or(&[]));
}

#[cfg(feature = "n2")]
fn exercise(buf: &[u8], want: &str) {
    let Some(p) = n2::Package::parse(buf) else { bb(0); return };
    s(p.name().as_bytes()); s(p.version().as_bytes()); s(p.architecture().as_bytes());
    for i in 0..p.len() { if let Some(e) = p.member(i) { s(e.name.as_bytes()); s(e.data); bb(e.digest); } }
    s(p.read(want).unwrap_or(&[]));
}

#[cfg(any(feature = "tar_own", feature = "tar_own_ustar"))]
fn exercise(buf: &[u8], want: &str) {
    let Some(p) = tar_own::Package::parse(buf) else { bb(0); return };
    s(p.name().as_bytes()); s(p.version().as_bytes()); s(p.architecture().as_bytes());
    for i in 0..p.len() { if let Some(e) = p.member(i) { s(e.name.prefix.as_bytes()); s(e.name.name.as_bytes()); s(e.data); bb(e.digest); } }
    s(p.read(want).unwrap_or(&[]));
}

#[cfg(feature = "tar_no_std")]
fn exercise(buf: &[u8], want: &str) {
    let Ok(a) = tar_no_std::TarArchiveRef::new(buf) else { bb(0); return };
    let mut it = a.entries();
    let Some(first) = it.next() else { bb(0); return };
    if first.filename().as_str() != Ok("MANIFEST") { bb(0); return; }
    let Some(m) = manifest::Manifest::parse(first.data()) else { bb(0); return };
    s(m.name.as_bytes()); s(m.version.as_bytes()); s(m.architecture.as_bytes());
    let mut n = 0;
    for e in it {
        let f = e.filename(); let Ok(name) = f.as_str() else { bb(0); return };
        let Some(d) = m.digest(name) else { bb(0); return };
        n += 1; s(name.as_bytes()); s(e.data()); bb(d);
        if name == want { s(e.data()); }
    }
    bb(n == m.members);
}

// A bump allocator from a crate, so this harness writes no `unsafe impl GlobalAlloc` of its own.
#[cfg(feature = "ruzstd")]
#[global_allocator]
static A: static_alloc::Bump<[u8; 1 << 26]> = static_alloc::Bump::uninit();
#[cfg(feature = "ruzstd")] extern crate alloc;
#[cfg(feature = "ruzstd")]
fn exercise(buf: &[u8], _want: &str) {
    let mut out = alloc::vec::Vec::new();
    #[cfg(feature = "ruzstd_on")]
    {
        use ruzstd::io::Read;
        let Ok(mut d) = ruzstd::decoding::StreamingDecoder::new(buf) else { bb(0); return };
        let mut chunk = [0u8; 4096];
        loop { match d.read(&mut chunk) { Ok(0) => break, Ok(n) => out.extend_from_slice(&chunk[..n]), Err(_) => { bb(0); return } } }
    }
    #[cfg(not(feature = "ruzstd_on"))] out.extend_from_slice(buf);
    s(&out);
}

static BUF: [u8; 4096] = [0; 4096];

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    exercise(bb(&BUF[..]), bb("bin/rg"));
    loop {}
}
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }
#[cfg(feature = "manifest_only")]
fn exercise(buf: &[u8], want: &str) {
    let Some(m) = manifest::Manifest::parse(buf) else { bb(0); return };
    s(m.name.as_bytes()); s(m.version.as_bytes()); s(m.architecture.as_bytes()); bb(m.members);
    bb(m.digest(want));
}
