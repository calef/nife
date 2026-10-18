#![no_main]
use libfuzzer_sys::fuzz_target;
use std::io::Read;
fuzz_target!(|data: &[u8]| {
    if let Ok(mut d) = ruzstd::decoding::StreamingDecoder::new(data) {
        let mut out = Vec::new();
        let _ = (&mut d).take(64 << 20).read_to_end(&mut out);
    }
});
