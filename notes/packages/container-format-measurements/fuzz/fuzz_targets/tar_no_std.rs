#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    if let Ok(a) = tar_no_std::TarArchiveRef::new(data) {
        for e in a.entries() {
            let _ = e.filename().as_str();
            let d = e.data();
            assert!(d.as_ptr() >= data.as_ptr() && d.as_ptr() as usize + d.len() <= data.as_ptr() as usize + data.len());
            let _ = e.data_as_str();
            let _ = e.size();
        }
        for h in a.headers() { let _ = h.1.size.as_number::<u64>(); let _ = h.1.typeflag.try_to_type_flag(); }
    }
});
