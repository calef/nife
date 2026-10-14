//! **The allocator's four entry points**, as `std`'s `System` calls them.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

#[inline]
pub unsafe fn alloc(layout: Layout) -> *mut u8 {
    if let Some((cache, c)) = cache_for(layout) {
        if let Some(p) = cache.take(c) {
            return p.as_ptr();
        }
        return refill_and_take(cache, c);
    }
    let mut g = lock();
    heap_alloc(g.inner(), layout).map_or(crate::ptr::null_mut(), NonNull::as_ptr)
}

#[inline]
pub unsafe fn alloc_zeroed(layout: Layout) -> *mut u8 {
    // Fresh pages from memory_region::MAP arrive zeroed, but recycled free-list memory does not, so
    // zero unconditionally rather than track provenance.
    let p = unsafe { alloc(layout) };
    if !p.is_null() {
        unsafe { crate::ptr::write_bytes(p, 0, layout.size()) };
    }
    p
}

#[inline]
pub unsafe fn dealloc(ptr: *mut u8, layout: Layout) {
    let Some(p) = NonNull::new(ptr) else {
        return;
    };
    if let Some((cache, c)) = cache_for(layout) {
        // SAFETY: `p` came from `alloc(layout)`, so it is `class_bytes(c)` grid-aligned bytes
        // of the heap's (a cached block is fungible within its class), and it is ours now.
        if let Err(p) = unsafe { cache.put(c, p) } {
            {
                let mut g = lock();
                let inner = g.inner();
                // SAFETY: as `spill_thread_cache`.
                cache.spill(c, |b| unsafe { inner.heap.dealloc(b, class_layout(c)) });
            }
            // SAFETY: as the first `put`; the spill made room.
            let kept = unsafe { cache.put(c, p) };
            debug_assert!(kept.is_ok());
        }
        return;
    }
    let mut g = lock();
    // SAFETY: forwarded GlobalAlloc contract: `ptr` came from `alloc(layout)` on this heap.
    unsafe { g.inner().heap.dealloc(p, layout) };
}

#[inline]
pub unsafe fn realloc(ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
    // SAFETY: the GlobalAlloc realloc contract guarantees `new_size` is valid for the align.
    unsafe {
        let new_layout = Layout::from_size_align_unchecked(new_size, old_layout.align());
        let new_ptr = alloc(new_layout);
        if !new_ptr.is_null() {
            crate::ptr::copy_nonoverlapping(ptr, new_ptr, usize::min(old_layout.size(), new_size));
            dealloc(ptr, old_layout);
        }
        new_ptr
    }
}
