use crate::host::fifo_step;
use crate::partition::{Entry, Mbr};
use crate::sd::in_range;

/// No partition table makes the write test's range touch a partition or sector 0.
///
/// Falsification: replayable `crates/designware_mobile_storage/falsifications/proofs.the_scratch_range_never_touches_a_partition_or_sector_zero.patch`
#[kani::proof]
#[kani::unwind(5)]
fn the_scratch_range_never_touches_a_partition_or_sector_zero() {
    let entry = || Entry {
        kind: kani::any(),
        start: kani::any(),
        sectors: kani::any(),
    };
    let mbr = Mbr {
        entries: [entry(), entry(), entry(), entry()],
    };
    let want: u32 = kani::any();
    if let Some((first, count)) = mbr.scratch(want) {
        assert!(first >= 1);
        assert!((1..=want).contains(&count));
        let k: u32 = kani::any();
        kani::assume(k < count);
        let sector = u64::from(first) + u64::from(k);
        for e in &mbr.entries {
            assert!(!e.contains(sector));
        }
    }
}

/// No block request the server translates reaches a sector outside its window.
///
/// Falsification: replayable `crates/designware_mobile_storage/falsifications/proofs.a_served_block_never_leaves_its_window.patch`
#[kani::proof]
fn a_served_block_never_leaves_its_window() {
    let w = crate::serve::Window {
        first: kani::any(),
        sectors: kani::any(),
        writable: kani::any(),
    };
    kani::assume(w.first.checked_add(w.sectors).is_some());
    let block: u64 = kani::any();
    let count: u64 = kani::any();
    if let Some(lba) = w.translate(block, count) {
        assert!(lba >= w.first);
        let span = count * crate::serve::SECTORS_PER_BLOCK;
        assert!(lba + span <= w.first + w.sectors);
    }
}

/// A FIFO step never moves more than is owed, and a write step never more than the FIFO has
/// room for, whatever `STATUS` says.
///
/// Falsification: replayable `crates/designware_mobile_storage/falsifications/proofs.a_fifo_step_never_overruns_the_fifo_or_the_buffer.patch`
#[kani::proof]
fn a_fifo_step_never_overruns_the_fifo_or_the_buffer() {
    let status: u32 = kani::any();
    let depth: u32 = kani::any();
    kani::assume((1..=4096).contains(&depth));
    let write: bool = kani::any();
    let remaining: u32 = kani::any();
    let n = fifo_step(status, depth, write, remaining);
    assert!(n <= remaining);
    assert!(n <= depth * 4);
    let filled = crate::regs::fifo_count(status);
    if write && filled <= depth {
        assert!(n <= (depth - filled) * 4);
    }
}

/// A range the driver admits is inside the card and inside what the command's 32-bit address
/// can name.
///
/// Falsification: replayable `crates/designware_mobile_storage/falsifications/proofs.an_admitted_range_is_inside_the_card_and_the_address.patch`
#[kani::proof]
fn an_admitted_range_is_inside_the_card_and_the_address() {
    let blocks: u64 = kani::any();
    let block_addressed: bool = kani::any();
    let lba: u64 = kani::any();
    let count: u64 = kani::any();
    if in_range(blocks, block_addressed, lba, count) {
        assert!(count >= 1);
        assert!(lba + count <= blocks);
        let last = lba + count - 1;
        if block_addressed {
            assert!(last <= u64::from(u32::MAX));
        } else {
            assert!((last + 1) * 512 <= 1 << 32);
        }
    }
}
