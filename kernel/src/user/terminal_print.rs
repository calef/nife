//! **Printing to a display terminal as an application would**, for the terminal's tests.
//!
//! Cut out of `user.rs` unchanged by milestone 872 (the kernel's user-mode file is split along its
//! seams), which left the parent an index.
//!
//! Name: provisional (milestone 872's lane, 2026-10-11 UTC; §281 (the kernel's user-mode file
//! splits into child modules of `user`)).

use super::*;

/// **Play an application printing to a display terminal**: put `text` in its output page and
/// `OPERATION_WRITE` it.
///
/// Shared by both of the terminal's wirings (the whole scanout, and a compositor window) because the
/// terminal contract does not know which one it is in: an `OPERATION_WRITE` is an `OPERATION_WRITE`. Returns when
/// the reply arrives, which the contract says means the bytes are on the console's side, so a test
/// needs no polling and no sleep between writes.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))] // the milestone-29 tests are the callers
pub(super) fn term_print(out: u64, ep: crate::sched::RendezvousId, text: &[u8]) {
    assert!(
        text.len() <= FRAME_SIZE as usize,
        "an OPERATION_WRITE past its output page",
    );
    let base = mmu::phys_to_virt(out);
    for (i, &b) in text.iter().enumerate() {
        // SAFETY: inside the output frame this kernel allocated and shares with the terminal.
        unsafe { core::ptr::write_volatile((base + i as u64) as *mut u8, b) };
    }
    // The bytes must be visible to the terminal before the request that names them.
    //
    // PAIR: no acquire fence, and none is needed. The terminal is blocked in `receive_cap` and the
    // `ipc_call` below is what wakes it, so the kernel's release of the `IPC_TABLES` lock and the
    // terminal's acquire of it are the pair (`spin::Mutex` locks `Acquire` and unlocks `Release`).
    // Redundant, kept: it is one `dmb` on a path that prints a line, and the contract does not
    // forbid a terminal that polls its page instead of blocking. See notes/memory-ordering.md.
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    let w0 = line_editor::proto::req(line_editor::proto::OPERATION_WRITE, text.len() as u64);
    let r = crate::sched::ipc_call(ep, [w0, 0]);
    assert_eq!(
        r[0],
        text.len() as u64,
        "the terminal consumed {} of {} bytes",
        r[0],
        text.len(),
    );
}
