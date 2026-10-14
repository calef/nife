//! **A child's code page**, built once for every test that spawns one.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// **A fresh page from `region` holding `code`, ready for the instruction fetcher**: retyped,
/// written through the direct map, and made coherent with `sync_icache`. Panics when the region is
/// spent or `code` does not fit one page.
///
/// Nine tests in five files built a child's code page this way by hand, each with its own
/// `// SAFETY:` comment over the same raw-pointer loop and none checking that the stub fit
/// (milestone 139 (drive the unsafe count down), 2026-10-07 UTC; name provisional). The fact they
/// each restated is `retype_page`'s own postcondition, so it is asserted once here, and the copy is
/// a bounds-checked slice copy rather than pointer arithmetic.
#[cfg(test)]
pub(super) fn code_page(region: u64, code: &[u32]) -> u64 {
    let phys = crate::memory_region::retype_page(region).expect("no code frame");
    let va = mmu::phys_to_virt(phys);
    // SAFETY: `retype_page` hands back a whole frame carved from `region` for this caller alone,
    // and the direct map names every RAM page, so the slice is the frame and nothing aliases it.
    let page = unsafe {
        core::slice::from_raw_parts_mut(va as *mut u32, FRAME_SIZE as usize / size_of::<u32>())
    };
    page[..code.len()].copy_from_slice(code);
    sync_icache(va, size_of_val(code));
    phys
}
