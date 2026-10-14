//! **Remote fences through SBI RFENCE**: shooting down a translation, an address space, or the
//! instruction cache on other harts, and the count bench builds keep.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// The SBI RFENCE extension id, "RFNC" in ASCII. Both remote-fence calls below live in it.
pub(super) const SBI_RFENCE_EID: usize = 0x5246_4E43;

/// **What the RFENCE calls mean by "the whole thing".** SBI defines a remote fence as covering
/// everything when `size` is all-ones (OpenSBI's `SBI_TLB_FLUSH_ALL`), and separately when `start`
/// and `size` are *both* zero. The two are not the same request and the difference matters here:
/// for [`sbi_remote_sfence_vma_asid`], all-ones is the one that reaches
/// `sfence.vma x0, asid` (every address, that ASID), while `0, 0` makes OpenSBI fall back to
/// `sfence.vma` with no operands and throw the entire TLB away on every target hart, which is
/// precisely the sledgehammer milestone 58 removed. Passing the wrong one would still be *correct*
/// and would silently undo the milestone on every other hart.
pub(super) const SBI_RFENCE_ALL: usize = usize::MAX;

/// **How many remote RFENCEs this boot has issued** (milestone 130's follow-on, bench builds only).
///
/// Every SBI RFENCE is an `ecall` into firmware, so it is the single most expensive thing on an
/// otherwise local TLB path, and whether one is issued at all is decided by a *mask*
/// (`smp::online_harts_mask()`). On a single-hart boot the correct answer is **zero**: there is
/// nobody to shoot down, and both call sites skip the call when the mask names no other hart.
///
/// This counter exists because a reading in notes/benchmarks.md (2026-08-15) inferred that a
/// `map_new` regression was remote RFENCEs fired against an over-reporting mask, and recorded
/// honestly that **nothing had counted a fence**. This counts them.
///
/// `feature = "bench"` rather than always-on: a relaxed increment is cheap but this is the TLB
/// shootdown path, and a diagnostic has no business on it in a shipping kernel. The bench build is
/// its own kernel binary, so the instrument and the shipping path stay separate objects.
#[cfg(feature = "bench")]
pub(super) static REMOTE_FENCES: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(0);

/// Record one issued RFENCE. See [`REMOTE_FENCES`].
#[cfg(feature = "bench")]
pub(super) fn note_remote_fence() {
    REMOTE_FENCES.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
}

/// Remote RFENCEs issued so far this boot. Bench builds only; see [`REMOTE_FENCES`].
#[cfg(feature = "bench")]
pub fn remote_fence_count() -> u64 {
    REMOTE_FENCES.load(core::sync::atomic::Ordering::Relaxed)
}

/// Shoot down a virtual-address translation on the harts in `hart_mask`, via the SBI RFENCE
/// extension. The firmware IPIs those harts and each executes `sfence.vma start, ...` for the range.
/// RISC-V has no hardware TLB broadcast (aarch64's `tlbi ..., is` does), so a kernel page-table change
/// must be pushed to the other harts this way or a migrated thread faults on a stale translation. See
/// `mmu::flush_tlb`.
pub fn sbi_remote_sfence_vma(hart_mask: usize, start: usize, size: usize) {
    const SBI_REMOTE_SFENCE_VMA_FID: usize = 1;
    #[cfg(feature = "bench")]
    note_remote_fence();
    // SAFETY: remote_sfence_vma with (hart bitmap, mask base 0, start, size). TLB maintenance on
    // other harts, which is always sound. The result is ignored, as it always was.
    unsafe {
        sbi::call(
            SBI_RFENCE_EID,
            SBI_REMOTE_SFENCE_VMA_FID,
            [hart_mask, 0, start, size, 0, 0],
        )
    };
}

/// **Discharge every translation tagged with `asid` on the harts in `hart_mask`**: the remote half
/// of `mmu::flush_asid`, and the instruction that makes an ASID safe to hand to a new address space
/// (milestone 58). Each target hart executes `sfence.vma x0, asid`.
///
/// # Why this has to exist at all
///
/// `sfence.vma` is a **local** instruction. It orders and invalidates for the hart that runs it and
/// says nothing about any other, which is the single largest difference between the two ISAs'
/// TLB maintenance: aarch64's `tlbi aside1is` broadcasts across the inner-shareable domain in
/// hardware and needs no software protocol. So the ASID reuse contract (`crates/address_space_identifier`: flush, then
/// the number may tag someone else) is one instruction there and a distributed protocol here.
///
/// # What is ordered, and by whom
///
/// The acknowledgement the contract needs is **the return of this call**. SBI's RFENCE functions are
/// synchronous: OpenSBI queues the request, sends the IPI, and spins in `sbi_tlb_sync` until every
/// target hart has drained it, so by the time `ecall` returns no target holds an entry wearing this
/// tag. Linux depends on the same guarantee (its `flush_tlb_mm` does no waiting of its own), which
/// is the reason to believe it rather than a reading of ours.
///
/// The IPI is delivered as an **M-mode** software interrupt, so a target hart with S-mode interrupts
/// masked still services it. That is not a detail: without it, any code that disables interrupts and
/// spins would deadlock the flusher, and the kernel disables interrupts routinely.
///
/// # BUGS
///
/// - **A firmware that implemented RFENCE asynchronously would break this silently**, and S-mode has
///   no way to detect it: the failure is a stale translation on another hart, arbitrarily far from
///   the cause. The SBI spec's wording is "instructs the remote harts to execute", which OpenSBI
///   reads as synchronous and a different implementation might not. `isa::the_firmware_implements_what_the_kernel_calls`
///   checks the extension is present; nothing checks it is synchronous, because nothing can.
/// - **`hart_mask` is a bitmap of hart ids relative to base 0**, so it only reaches harts 0..63 on
///   rv64. Fine for `MAX_CPUS`, wrong for a machine with more harts than that, which is the same
///   limitation `smp::bring_up_secondaries` documents for logical-id-equals-hart-id.
pub fn sbi_remote_sfence_vma_asid(hart_mask: usize, asid: u16) {
    const SBI_REMOTE_SFENCE_VMA_ASID_FID: usize = 2;
    #[cfg(feature = "bench")]
    note_remote_fence();
    let args = [hart_mask, 0, 0, SBI_RFENCE_ALL, asid as usize, 0];
    // SAFETY: remote_sfence_vma_asid with (hart bitmap, mask base 0, start 0, size all, asid). TLB
    // maintenance on other harts, which is always sound. The result is ignored, as it always was.
    unsafe { sbi::call(SBI_RFENCE_EID, SBI_REMOTE_SFENCE_VMA_ASID_FID, args) };
}

/// Execute `fence.i` on the harts in `hart_mask`, via the SBI RFENCE extension: the cross-hart
/// half of [`sync_icache`], because `fence.i` is hart-local and instruction memory written on one
/// hart is not otherwise ordered against another hart's fetch.
pub fn sbi_remote_fence_i(hart_mask: usize) {
    const SBI_REMOTE_FENCE_I_FID: usize = 0;
    // SAFETY: remote_fence_i with (hart bitmap, mask base 0). Instruction-fetch ordering on other
    // harts, which is always sound. The result is ignored, as it always was.
    unsafe {
        sbi::call(
            SBI_RFENCE_EID,
            SBI_REMOTE_FENCE_I_FID,
            [hart_mask, 0, 0, 0, 0, 0],
        )
    };
}
