//! **`std_heap_contention`: how the `std` heap's spinlock holds up under four threads** (milestone 812
//! (`std::thread::spawn` runs real threads in one address space), the block's item 12: "measure it
//! under four threads before replacing it").
//!
//! The same work twice: `ALLOCATIONS` small allocations and frees, done by one thread, then split
//! across four. It prints the counter ticks each took, and the test that runs it
//! (`system_tests::user::std_threads_tests`) prints them; `notes/std/threads.md` records what they
//! were. A number under QEMU's emulation is a ratio to read, not a speed: the heap is the one lock,
//! so four threads on four cores finishing in no less time than one thread is contention.
//!
//! Then [`ENDED`] threads, one after another, each leave a full allocation cache behind when they
//! end (milestone 561 (a per-CPU allocator is what the current-CPU page was for)). A full cache
//! holds 34,816 bytes, so unless a thread's end gives its cache back to the heap, the one-mebibyte
//! heap budget `system_tests` grants runs out before the last of them and the program aborts.

use std::alloc::{Layout, alloc, dealloc};
use std::time::Instant;

const ALLOCATIONS: usize = 200_000;

/// How many threads in turn fill their cache and end: 64 full caches are 2.2 MB, twice the budget.
const ENDED: usize = 64;

/// Allocate sixteen blocks of every size a cache holds, 16 to 256 bytes, then free them all, which
/// leaves this thread's cache full.
fn fill_my_cache() {
    let layouts: Vec<Layout> = (1..=16)
        .map(|k| Layout::from_size_align(k * 16, 8).expect("a valid layout"))
        .collect();
    let mut blocks = Vec::with_capacity(16 * layouts.len());
    for l in &layouts {
        for _ in 0..16 {
            // SAFETY: a nonzero layout.
            let p = unsafe { alloc(*l) };
            assert!(!p.is_null(), "out of memory filling a cache");
            blocks.push((p, *l));
        }
    }
    for (p, l) in blocks {
        // SAFETY: allocated just above with this layout, and freed once.
        unsafe { dealloc(p, l) };
    }
}

fn churn(n: usize) {
    for i in 0..n {
        let b = Box::new([i as u8; 48]);
        std::hint::black_box(&b);
    }
}

fn timed(threads: usize) -> u128 {
    let start = Instant::now();
    let handles: Vec<_> = (0..threads)
        .map(|_| std::thread::spawn(move || churn(ALLOCATIONS / threads)))
        .collect();
    for h in handles {
        h.join().expect("a churning thread panicked");
    }
    start.elapsed().as_micros()
}

fn main() {
    let one = timed(1);
    let four = timed(4);
    println!("heap allocations {ALLOCATIONS}");
    println!("one thread us {one}");
    println!("four threads us {four}");
    for _ in 0..ENDED {
        std::thread::spawn(fill_my_cache)
            .join()
            .expect("a cache-filling thread panicked");
    }
    println!("threads ended with full caches {ENDED}");
}
