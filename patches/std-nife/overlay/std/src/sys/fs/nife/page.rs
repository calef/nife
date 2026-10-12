//! **The shared page**.
//!
//! One page for the whole process, so two concurrent operations would trample each other. A lock
//! guards it, and every request that touches the page holds the guard across its `CALL`, which is
//! also what makes the reply's bytes ours to read.
//!
//! **The lock sleeps; it was a spinlock until 2026-10-12 (UTC)** (lane milestone/121-rg-threads).
//! A spinlock was right while a process had one thread. With `rg`'s parallel walker it was not:
//! the holder is blocked in `CALL` waiting for the file server, and every other worker spun on
//! the flag until its time slice ran out. On `x86_64` at two cores, both cores spun while the
//! caretaker waited behind them, and one walk of 141 files passed the suite's 90 s per-test
//! ceiling. It is now `sys::sync::Mutex`, the futex lock `std`'s own `Mutex` uses: a waiter sleeps
//! on the space's futex (slot 12), or yields where the program holds none.
//!
//! Cut out of `sys/fs/nife.rs` unchanged on 2026-10-12 (UTC) by lane
//! maintainer/near-ceiling-splits, for §266 (a Rust source file stays under 2,000 lines) and §282
//! (when a file is split, the parent becomes an index). What was private to `nife` is `pub(super)`
//! here, which reaches the same modules: `nife` and its children.
//!
//! Name: provisional (lane maintainer/near-ceiling-splits, 2026-10-12 UTC), for the module and the
//! file.

use super::*;

static LOCK: crate::sys::sync::Mutex = crate::sys::sync::Mutex::new();

pub(super) struct Page;

impl Drop for Page {
    fn drop(&mut self) {
        // SAFETY: a `Page` exists only between `page()`'s `lock` and this, on the thread that
        // locked.
        unsafe { LOCK.unlock() };
    }
}

/// Take the shared page. Held across the `CALL` that uses it.
pub(super) fn page() -> Page {
    LOCK.lock();
    Page
}

impl Page {
    /// Put `bytes` in the page: a name to open, or data to write.
    ///
    /// **A plain copy, not a volatile byte loop** (milestone 606 (a directory walk costs what it
    /// does on Linux), provisional number). The page is shared with another address space, and
    /// what keeps the compiler from caching or eliding an access to it is the `CALL` itself: every
    /// exchange is `put`, then `rt::call`, then `get`, and `rt::call` is an `asm!` block without
    /// `nomem`, which the compiler must assume reads and writes any memory. So no access moves
    /// across it or is assumed unchanged by it. The volatile loop bought nothing on top of that
    /// and cost a load or store per byte: 334 KB of them in a walk that reads the fixture tree.
    pub(super) fn put(&mut self, bytes: &[u8]) {
        // SAFETY: PAGE_VA is a mapped, writable page of `PAGE` bytes; callers clamp to it, and it
        // cannot overlap `bytes`, which is this process's own memory.
        unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), PAGE_VA as *mut u8, bytes.len()) };
    }

    /// Put `bytes` at `off` in the page. Only `RENAME` needs this: it is the one verb that carries
    /// two names, back to back with the source first, because one request word cannot hold two
    /// lengths and the page is where the contract puts what does not fit in a word.
    pub(super) fn put_at(&mut self, off: usize, bytes: &[u8]) {
        // SAFETY: PAGE_VA is a mapped, writable page of `PAGE` bytes; the caller checks that
        // `off + bytes.len()` is within it before calling. [`Page::put`] says why a plain copy.
        unsafe {
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                (PAGE_VA as usize + off) as *mut u8,
                bytes.len(),
            )
        };
    }

    /// Take `out.len()` bytes out of the page (a completed read landed there).
    pub(super) fn get(&mut self, out: &mut [u8]) {
        // SAFETY: as [`Page::put`]; callers clamp `out` to the page.
        unsafe {
            core::ptr::copy_nonoverlapping(PAGE_VA as *const u8, out.as_mut_ptr(), out.len())
        };
    }
}
