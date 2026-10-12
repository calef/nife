//! **The FS server's stack, and the instrument that measures it**: the stack size, the
//! poison its pages are filled with, and [`fs_stack_used`], which reads the high-water back.
//!
//! Cut out of `fs_service.rs` unchanged by lane `maintainer/last-three-over-2000`, which left
//! the parent an index (§282 (when a file is split, the parent becomes an index)) and under §266
//! (a Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

use super::*;

/// **Extra stack pages for the FS server**, below the single page `run` maps, so the process
/// gets `1 + FS_STACK_PAGES` pages in one contiguous run down from [`USER_STACK_TOP`].
///
/// RedoxFS recurses through its tree, htree and transaction code with **8 KiB frames** (one
/// `read_block::<TreeList<..>>` activation carries a whole 4096-byte block plus scratch), so this
/// is not a "generous round number" question, it is a measured one. It used to be 32, and 32 was
/// **528 bytes short** the moment milestone 31 phase 2 added `CREATE` and `TRUNCATE`: the FS
/// server took one more level of tree recursion, ran off the bottom of its stack, and was killed
/// by a data abort mid-request, which left every client blocked on a `CALL` that would never be
/// answered. See [`fs_stack_used`] for the instrument that now measures this instead of guessing,
/// and `the_redoxfs_servers_stack_still_has_headroom` for the test that fails the day it is too small
/// again. notes/fs-server.md carries the story.
///
/// 96 is chosen against the measurement, not above it: the high-water is **135,696 bytes** (and
/// 127,408 as measured by milestone 37, an unattributed 8 KiB lower; notes/fs-server.md carries
/// both and the reasoning), and
/// `1 + 96` pages is 397,312, which leaves room for roughly thirty more 8 KiB activations. That
/// margin is the point, because recursion depth here tracks the *tree* depth, which grows with
/// the image; a size proven on a 16 MiB fixture is not proven on a real disk. 384 KiB of frames
/// once per boot is cheap next to the FS server's 8 MiB heap budget.
pub(super) const FS_STACK_PAGES: u64 = 96;

/// The pattern every FS-server stack page is filled with before the process starts. Nothing but
/// the FS server's own stack writes ever touch these frames, so a word that still reads as this
/// is a word the stack never reached, and the deepest changed word is the high-water mark. The
/// value is deliberately not 0 and not a plausible pointer, so a poisoned word cannot be mistaken
/// for real data (or vice versa) in a dump.
const STACK_POISON: u64 = 0xC71C_5E57_C71C_5E57;

/// How many FS servers one boot can start, and therefore how many banks of poisoned stack pages
/// [`FS_STACK_PHYS`] holds: the ordinary one, and milestone 37's two (the server that is killed
/// mid-transaction, and the one that mounts the disk it left behind).
///
/// They cannot share stack frames. The killed server is still executing its trap when the
/// recovery server starts, so reusing its pages would have one process writing another's stack,
/// and the bug would present as the recovery mount failing for no reason.
pub(super) const FS_SERVERS: usize = 3;

/// A fresh frame filled with [`STACK_POISON`], for one of the FS server's stack pages, and
/// remembered in [`FS_STACK_PHYS`] so the depth actually reached can be read back afterwards.
pub(super) fn poisoned_stack_page_frame(server: usize, index: usize) -> u64 {
    let p = crate::memory::alloc()
        .expect("no frame for the fs server's stack")
        .addr();
    // SAFETY: fresh frame, reachable through the direct map, exactly FRAME_SIZE bytes.
    let words = unsafe {
        core::slice::from_raw_parts_mut(mmu::phys_to_virt(p) as *mut u64, FRAME_SIZE as usize / 8)
    };
    words.fill(STACK_POISON);
    FS_STACK_PHYS[server][index].store(p, core::sync::atomic::Ordering::Relaxed);
    p
}

/// The physical frames behind every FS server's extra stack pages: one bank per server, index 0
/// being the page directly below [`USER_STACK_VA`]. Written once at wiring, read by
/// [`fs_stack_used`]. Zero means "this boot never started that server".
#[allow(clippy::declare_interior_mutable_const)] // an atomic array is exactly what this is for
pub(super) static FS_STACK_PHYS: [[core::sync::atomic::AtomicU64; FS_STACK_PAGES as usize];
    FS_SERVERS] = [const { [const { core::sync::atomic::AtomicU64::new(0) }; FS_STACK_PAGES as usize] };
    FS_SERVERS];

/// **How deep the FS server's stack actually went**, in bytes below [`USER_STACK_TOP`], and how
/// much it was given. `None` if this boot wired no FS service.
///
/// Read by scanning the poison: the deepest word that is no longer [`STACK_POISON`] is the
/// deepest the process ever wrote. This is a measurement of the whole run so far, not a sample,
/// because nothing ever un-writes a stack word. The base page `run` maps is counted as fully used
/// (it holds the entry frame and cannot be scanned from here), which is true and is why the
/// number starts at one page.
///
/// The point of it is that a stack size is otherwise a number nobody can defend. The one before
/// this was 528 bytes too small, and the way we found out was a mystery hang.
///
/// The high-water is a **maximum over every FS server this boot started**, which is why
/// milestone 37's recovery mount is covered by it too. A mount that has to walk back a
/// generation is the case most likely to recurse further than a clean one, so it is exactly the
/// case this instrument should be watching.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
pub fn fs_stack_used() -> Option<(u64, u64)> {
    use core::sync::atomic::Ordering;
    let total = (FS_STACK_PAGES + 1) * FRAME_SIZE;
    let mut deepest = FRAME_SIZE; // the base page, always used
    let mut wired = false;
    for bank in FS_STACK_PHYS.iter() {
        for (i, slot) in bank.iter().enumerate() {
            let phys = slot.load(Ordering::Relaxed);
            if phys == 0 {
                continue;
            }
            wired = true;
            // SAFETY: a frame this module allocated and still owns, via the direct map.
            let words = unsafe {
                core::slice::from_raw_parts(
                    mmu::phys_to_virt(phys) as *const u64,
                    FRAME_SIZE as usize / 8,
                )
            };
            // Page `i` spans [USER_STACK_VA - (i+1)*FRAME, USER_STACK_VA - i*FRAME). Word `w` in
            // it sits that far above the page's base, so a touched word means at least this much
            // depth.
            if let Some(w) = words.iter().position(|&x| x != STACK_POISON) {
                let depth = (i as u64 + 2) * FRAME_SIZE - w as u64 * 8;
                deepest = deepest.max(depth);
            }
        }
    }
    wired.then_some((deepest, total))
}
