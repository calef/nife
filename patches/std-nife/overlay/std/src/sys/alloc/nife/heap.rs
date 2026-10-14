//! **The heap underneath every allocation**: one spin-locked region, grown a page run at a time,
//! and the allocation from it that the thread caches refill from.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

pub(super) const PAGE: u64 = 4096;

pub(super) const MIN_GROW_PAGES: u64 = 8;

pub(super) struct Inner {
    heap: user_mode_heap::Heap,
    committed: u64,
}

pub(super) struct SpinHeap {
    locked: crate::sync::atomic::AtomicBool,
    inner: crate::cell::UnsafeCell<Inner>,
}

// SAFETY: all access to `inner` goes through the `locked` spinlock.
unsafe impl Sync for SpinHeap {}

pub(super) static HEAP: SpinHeap = SpinHeap {
    locked: crate::sync::atomic::AtomicBool::new(false),
    inner: crate::cell::UnsafeCell::new(Inner { heap: user_mode_heap::Heap::new(), committed: 0 }),
};

pub(super) struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        HEAP.locked.store(false, crate::sync::atomic::Ordering::Release);
    }
}

pub(super) fn lock() -> Guard {
    use crate::sync::atomic::Ordering;
    while HEAP
        .locked
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        crate::hint::spin_loop();
    }
    Guard
}

impl Guard {
    #[allow(clippy::mut_from_ref)]
    fn inner(&mut self) -> &mut Inner {
        // SAFETY: holding the guard means holding the spinlock; access is exclusive.
        unsafe { &mut *HEAP.inner.get() }
    }
}

/// Map at least `need` more contiguous bytes at the top of the committed range, from the
/// untyped in slot 0. Same growth policy as `user_mode_runtime::heap`: geometric, floor of 8 pages,
/// capped by `rt::HEAP_MAX`; only a fresh run big enough for the request counts as success.
pub(super) fn grow(inner: &mut Inner, need: u64) -> bool {
    let need_pages = need.div_ceil(PAGE);
    let want_pages = need_pages
        .max(inner.committed / PAGE)
        .max(MIN_GROW_PAGES)
        .min((rt::HEAP_MAX - inner.committed) / PAGE);
    if want_pages < need_pages {
        return false;
    }
    let start = rt::HEAP_BASE + inner.committed;
    let mut got: u64 = 0;
    while got < want_pages {
        let va = start + got * PAGE;
        // SAFETY: plain syscall; the kernel validates the slot, the va, and the budget.
        if unsafe { rt::invoke(rt::MEMORY_REGION_SLOT, abi::memory_region::MAP, va, 0, 0) } != 0 {
            break;
        }
        got += 1;
    }
    if got > 0 {
        inner.committed += got * PAGE;
        // SAFETY: the range was just mapped writable for this process alone, adjacent to (or
        // the start of) the range already donated.
        unsafe { inner.heap.add_region(start as *mut u8, (got * PAGE) as usize) };
    }
    got >= need_pages
}

/// A block from the heap itself, growing it as needed; `None` when the region is spent.
pub(super) fn heap_alloc(inner: &mut Inner, layout: Layout) -> Option<NonNull<u8>> {
    loop {
        if let Some(p) = inner.heap.alloc(layout) {
            return Some(p);
        }
        let slack = layout.align().saturating_sub(PAGE as usize) as u64;
        let need = user_mode_heap::effective_size(layout) as u64 + slack;
        if !grow(inner, need) {
            return None;
        }
    }
}
