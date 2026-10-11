//! The System allocator for nife: the untyped-backed heap, inside std.
//!
//! The algorithm is `crates/user_mode_heap`, generated verbatim into `user_mode_heap.rs` next door by
//! `cargo xtask std-src` (so the host-tested source is the only source). This file is the same
//! policy layer as `user_mode_runtime::heap::MemoryRegionHeap`, re-stated for std because std cannot depend on
//! the out-of-tree crate: a spinlock, lazy wiring to the untyped in slot 0, and grow-on-demand
//! by `memory_region::MAP` at the top of the committed range.
//!
//! Lazy, because std has no moment where a program says "init the heap": the first allocation
//! (often std's own rt setup) simply finds committed == 0 and grows. A program whose slot 0 is
//! not an untyped gets a failed allocation, which std turns into the OOM abort: honest, if
//! blunt.
//!
//! **Each thread has a cache in front of the lock** (milestone 561 (a per-CPU allocator is what
//! the current-CPU page was for)): `crates/allocation_cache`, generated into `allocation_cache.rs`
//! next door, kept in the thread's own block (`sys/thread_local/key/nife.rs`). A small request is
//! served from it without the lock; a miss refills, and a full class spills, eight blocks under one
//! acquisition. A thread's end closes its cache and gives every block back
//! ([`spill_thread_cache`]). Before the main thread's block exists, and in a program whose thread
//! pointer could not be set, every request takes the lock, as before. Why per thread and not per
//! CPU, and what was measured, is `notes/std/allocation.md`.

use crate::alloc::Layout;
use crate::ptr::NonNull;
use crate::sys::pal::nife::{abi, rt};
use crate::sys::thread_local::key::nife as tls;

// Generated whole from the crate; its counters and bound are the host tests', hence the allow.
#[allow(dead_code)]
pub(crate) mod allocation_cache;
mod user_mode_heap;

use allocation_cache::{Cache, GRID, class, class_bytes};

const PAGE: u64 = 4096;
const MIN_GROW_PAGES: u64 = 8;

struct Inner {
    heap: user_mode_heap::Heap,
    committed: u64,
}

struct SpinHeap {
    locked: crate::sync::atomic::AtomicBool,
    inner: crate::cell::UnsafeCell<Inner>,
}

// SAFETY: all access to `inner` goes through the `locked` spinlock.
unsafe impl Sync for SpinHeap {}

static HEAP: SpinHeap = SpinHeap {
    locked: crate::sync::atomic::AtomicBool::new(false),
    inner: crate::cell::UnsafeCell::new(Inner { heap: user_mode_heap::Heap::new(), committed: 0 }),
};

struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        HEAP.locked.store(false, crate::sync::atomic::Ordering::Release);
    }
}

fn lock() -> Guard {
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
fn grow(inner: &mut Inner, need: u64) -> bool {
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
fn heap_alloc(inner: &mut Inner, layout: Layout) -> Option<NonNull<u8>> {
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

/// The layout a cached block of `class` has in the heap: its whole effective size, on the grid.
fn class_layout(class: usize) -> Layout {
    // SAFETY: a nonzero multiple of a power of two, aligned to that power of two.
    unsafe { Layout::from_size_align_unchecked(class_bytes(class), GRID) }
}

/// The calling thread's cache, when a request of this shape may use it.
#[inline]
fn cache_for(layout: Layout) -> Option<(&'static mut Cache, usize)> {
    let c = class(layout.size(), layout.align())?;
    let cache = tls::allocation_cache();
    // SAFETY: the calling thread's own cache, which nothing else touches (`tls::allocation_cache`),
    // and nothing here allocates while this borrow is live.
    (!cache.is_null()).then(|| (unsafe { &mut *cache }, c))
}

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

/// A miss: up to eight blocks of the class from the heap under one acquisition, then one of them.
#[cold]
fn refill_and_take(cache: &mut Cache, c: usize) -> *mut u8 {
    {
        let mut g = lock();
        let inner = g.inner();
        // SAFETY: each block is the heap's, `class_bytes(c)` long, on the grid, and handed out
        // to nobody else.
        unsafe { cache.refill(c, || heap_alloc(inner, class_layout(c))) };
    }
    cache.take(c).map_or(crate::ptr::null_mut(), NonNull::as_ptr)
}

/// **A thread's end**: give every block its cache holds back to the heap under one acquisition.
/// The caller has already closed the cache, so nothing refills it after this.
pub(crate) fn spill_thread_cache(cache: &mut Cache) {
    let mut g = lock();
    let inner = g.inner();
    // SAFETY: every cached block of class `c` is the heap's and was cached under `class_layout(c)`.
    cache.drain(|c, b| unsafe { inner.heap.dealloc(b, class_layout(c)) });
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
