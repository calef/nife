//! Host tests for the userspace heap algorithm. These prove the invariants QEMU never would
//! cheaply: coalescing back to one block, alignment beyond the grid, sliver-free splitting, and
//! the grow-at-top pattern `user_mode_runtime::heap` relies on.

use core::alloc::Layout;
use core::marker::PhantomData;
use core::ops::Deref;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

use user_mode_heap::{Heap, MIN_ALIGN, effective_size};

/// A 16-aligned arena on the host stack-of-the-test; the heap only ever sees pointers into it.
#[repr(align(4096))]
struct Arena([u8; 64 * 1024]);

fn arena() -> Box<Arena> {
    Box::new(Arena([0; 64 * 1024]))
}

fn layout(size: usize, align: usize) -> Layout {
    Layout::from_size_align(size, align).unwrap()
}

/// **A [`Heap`] over memory these tests lend it**, and the one place they argue its two safety
/// contracts. The compiler holds both arguments rather than a comment at each call: a donation is
/// a `&'a mut` borrow, so nothing else reaches the region while the heap can, and two donations
/// are disjoint because two live `&mut` borrows are; a [`Block`] is not `Clone` and [`Lent::free`]
/// takes it by value, so it is freed once and with the layout it was allocated with.
///
/// It derefs to the [`Heap`] for the read-only counters. `add_region` and `dealloc` are reachable
/// only through it, because there is no `DerefMut`.
struct Lent<'a> {
    heap: Heap,
    /// Which `Lent` minted a [`Block`], so one cannot be freed into another heap.
    id: usize,
    lent: PhantomData<&'a mut [u8]>,
}

/// One live allocation from a [`Lent`]. Not `Clone`: freeing consumes it.
struct Block {
    ptr: NonNull<u8>,
    layout: Layout,
    heap: usize,
}

impl Block {
    fn addr(&self) -> usize {
        self.ptr.as_ptr().addr()
    }
}

static NEXT_HEAP: AtomicUsize = AtomicUsize::new(0);

impl<'a> Lent<'a> {
    fn new() -> Self {
        Self {
            heap: Heap::new(),
            id: NEXT_HEAP.fetch_add(1, Ordering::Relaxed),
            lent: PhantomData,
        }
    }

    fn donate(&mut self, region: &'a mut [u8]) {
        // SAFETY: `region` is borrowed mutably for `'a`, which outlives every use of this heap,
        // so nothing else reaches it; an earlier donation is a different live `&mut`, so disjoint.
        unsafe { self.heap.add_region(region.as_mut_ptr(), region.len()) }
    }

    fn alloc(&mut self, layout: Layout) -> Option<Block> {
        let ptr = self.heap.alloc(layout)?;
        Some(Block {
            ptr,
            layout,
            heap: self.id,
        })
    }

    /// Write `byte` over the whole of `block`, which is how a test catches two live blocks that
    /// overlap: one scribble corrupts the other's free-list node.
    fn scribble(&mut self, block: &Block, byte: u8) {
        assert_eq!(block.heap, self.id, "a block from another heap");
        // SAFETY: `block` came from this heap's `alloc` and is still live, because freeing it
        // consumes it; its layout's size is what the heap handed out.
        unsafe { core::ptr::write_bytes(block.ptr.as_ptr(), byte, block.layout.size()) };
    }

    fn free(&mut self, block: Block) {
        assert_eq!(block.heap, self.id, "a block from another heap");
        // SAFETY: `block` came from this heap's `alloc` with `block.layout`, and is moved in
        // here, so it is freed once.
        unsafe { self.heap.dealloc(block.ptr, block.layout) };
    }
}

impl Deref for Lent<'_> {
    type Target = Heap;
    fn deref(&self) -> &Heap {
        &self.heap
    }
}

#[test]
fn everything_freed_coalesces_to_one_block() {
    let mut a = arena();
    let total = a.0.len();
    let mut h = Lent::new();
    h.donate(&mut a.0);
    assert_eq!(h.block_count(), 1);

    // Allocate a mix of sizes, then free in an order that is neither LIFO nor FIFO. If any
    // adjacency case in insert_free is wrong, the final block count betrays it.
    let sizes = [24usize, 1, 4096, 100, 16, 333, 2048, 7];
    let mut blocks: Vec<_> = sizes
        .iter()
        .map(|&s| Some(h.alloc(layout(s, 8)).unwrap()))
        .collect();
    for i in [3, 0, 6, 1, 7, 2, 5, 4] {
        h.free(blocks[i].take().unwrap());
    }
    assert_eq!(h.block_count(), 1);
    assert_eq!(h.free_bytes(), total);
}

#[test]
fn effective_size_is_the_grid() {
    assert_eq!(effective_size(layout(1, 1)), MIN_ALIGN);
    assert_eq!(effective_size(layout(16, 8)), 16);
    assert_eq!(effective_size(layout(17, 1)), 32);
    // A zero-size allocation still costs one grid cell, so the pointer is unique and freeable.
    assert_eq!(
        effective_size(Layout::from_size_align(0, 1).unwrap()),
        MIN_ALIGN
    );
}

#[test]
fn over_aligned_allocations_are_aligned_and_leak_nothing() {
    let mut a = arena();
    let total = a.0.len();
    let mut h = Lent::new();
    h.donate(&mut a.0);

    // Force front padding: a small allocation first skews the next block off 4096 alignment.
    let small = h.alloc(layout(16, 16)).unwrap();
    let big = h.alloc(layout(8192, 4096)).unwrap();
    assert_eq!(big.addr() % 4096, 0);

    h.free(big);
    h.free(small);
    // The front-padding remainder must have been a real free block, not a leaked sliver.
    assert_eq!(h.block_count(), 1);
    assert_eq!(h.free_bytes(), total);
}

#[test]
fn exhaustion_returns_none_and_growth_at_the_top_coalesces() {
    let mut a = arena();
    let mut h = Lent::new();
    // Donate only the first 8 KiB; keep the rest as "unmapped pages" to grow into.
    let (first, rest) = a.0.split_at_mut(8 * 1024);
    h.donate(first);

    assert!(
        h.alloc(layout(16 * 1024, 8)).is_none(),
        "must refuse, not corrupt"
    );

    // Grow the way user_mode_runtime::heap does: donate the pages right above the committed top.
    h.donate(&mut rest[..16 * 1024]);
    // The donation is adjacent to the free tail of the first region, so a block bigger than
    // either donation alone must now exist.
    let p = h.alloc(layout(16 * 1024, 8)).unwrap();
    h.free(p);
    assert_eq!(h.block_count(), 1);
}

#[test]
fn first_fit_reuses_a_hole_of_exactly_the_right_size() {
    let mut a = arena();
    let mut h = Lent::new();
    h.donate(&mut a.0);

    let keep_a = h.alloc(layout(64, 8)).unwrap();
    let hole = h.alloc(layout(128, 8)).unwrap();
    let keep_b = h.alloc(layout(64, 8)).unwrap();
    let hole_addr = hole.addr();
    h.free(hole);

    // The freed hole sits below the big tail block; first-fit must land exactly in it.
    let p = h.alloc(layout(128, 8)).unwrap();
    assert_eq!(p.addr(), hole_addr);

    h.free(p);
    h.free(keep_a);
    h.free(keep_b);
    assert_eq!(h.block_count(), 1);
}

#[test]
fn a_block_that_fits_exactly_leaves_nothing_behind() {
    let mut a = arena();
    let mut h = Lent::new();
    // Only the first 4 KiB is donated and the rest of the arena is slack on purpose. An allocator
    // that carved a remainder out of an exact fit would write a free-list node at the region's end,
    // and inside a Box that is the test process's own heap rather than an assertion failure.
    const DONATED: usize = 4096;
    h.donate(&mut a.0[..DONATED]);

    let p = h.alloc(layout(DONATED, MIN_ALIGN)).unwrap();
    // Nothing is left, and "nothing" is no blocks rather than one empty one: a zero-length block on
    // the list is a node written past the end of what was donated.
    assert_eq!(h.block_count(), 0);
    assert_eq!(h.free_bytes(), 0);
    assert!(h.alloc(layout(1, 1)).is_none());

    h.free(p);
    assert_eq!(h.block_count(), 1);
    assert_eq!(h.free_bytes(), DONATED);
}

#[test]
fn a_split_never_invents_bytes_nobody_donated() {
    let mut a = arena();
    let base = a.0.as_ptr().addr();
    let mut h = Lent::new();
    const DONATED: usize = 4096;
    h.donate(&mut a.0[..DONATED]);

    // A front-padded split, so both remainders are computed from the block's size. Nothing exposes
    // a block's size to read back, and `free_bytes` is an independent counter that a wrong split
    // does not touch, so the only observable consequence of the arithmetic rotting is a heap that
    // later hands out memory nobody gave it.
    let pad = h.alloc(layout(16, 16)).unwrap();
    let over = h.alloc(layout(64, 64)).unwrap();
    h.free(over);
    h.free(pad);
    assert_eq!(h.block_count(), 1);
    assert_eq!(h.free_bytes(), DONATED);

    // The whole donation is available again, and not one grid cell more.
    assert!(h.alloc(layout(DONATED + MIN_ALIGN, 8)).is_none());
    let all = h.alloc(layout(DONATED, MIN_ALIGN)).unwrap();
    assert_eq!(all.addr(), base);
    h.free(all);
}

#[test]
fn thrashing_does_not_fragment_the_heap_to_death() {
    // The kernel heap's milestone-4 test, re-proven for the userspace twin: interleaved
    // allocate/free churn must end with the heap as one block, not confetti.
    let mut a = arena();
    let total = a.0.len();
    let mut h = Lent::new();
    h.donate(&mut a.0);

    let mut live: Vec<Block> = Vec::new();
    for round in 0..1000usize {
        let size = 1 + (round * 37) % 500;
        if let Some(p) = h.alloc(layout(size, 8)) {
            // Scribble over the whole allocation: if the heap ever hands out overlapping
            // blocks, the free-list nodes get corrupted and a later operation explodes.
            h.scribble(&p, 0xA5);
            live.push(p);
        }
        if round % 3 == 0 && !live.is_empty() {
            let p = live.remove(round % live.len());
            h.free(p);
        }
    }
    for p in live.drain(..) {
        h.free(p);
    }
    assert_eq!(h.block_count(), 1);
    assert_eq!(h.free_bytes(), total);
}
