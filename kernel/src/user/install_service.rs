//! **The offer a booted stick makes, and the two programs that carry it out** (notes/installing.md).
//! This is the boot policy: decide whether this boot could install, say what would be destroyed,
//! ask, then wire the two confined programs that do the work. Nothing here writes a byte to a disk.
//!
//! | | holds | can destroy |
//! |---|---|---|
//! | this module | the boot path, the console | nothing; it has no block endpoint of its own to spend |
//! | `components/src/system_installer.rs` | one disk, one entropy endpoint, a read-only copy of the boot file | that disk |
//! | `redoxfs_server`'s `mkfs` | the same disk, the same entropy endpoint | that disk |
//!
//! **The question is asked here, before the capability exists**: a confirmation a program printed
//! to itself is one it could skip. The offer is made only when this boot came from a file the
//! loader can read back (`memory::boot_file_region`), there is an NVMe controller, the disk does
//! not already carry nife, and somebody answers within a bounded wait. The nife check is
//! `system_installer`'s `ROLE_SURVEY`, spawned without entropy; a `blk` endpoint has no read-only
//! form, so what keeps the survey from writing is its role, not its authority (`confirm` below
//! prices a read-only `blk`). **Refusing by default is the safety property**: the only path to a
//! write is a person typing [`CONFIRMATION`].
//!
//! ```text
//!   install     : this system was booted from a file and can install itself.
//!   install     :   TARGET: the NVMe disk attached to this machine, 1073741824 bytes.
//!   install     :   EVERYTHING ON THAT DISK WILL BE DESTROYED.
//!   install     :   Type INSTALL and press return to proceed; anything else continues the boot.
//!   install     : > INSTALL
//!   install     : partitioning and copying the boot file...
//!   install     : installed. nife data at LBA 2048, EFI system at LBA 1046528.
//!   install     : filesystem created.
//!   install     : DONE. Remove the installation medium and reboot.
//! ```
//!
//! # BUGS
//!
//! - x86_64 only (design/roadmap/0568). The kernel reads the UART to ask, before the input driver.
//! - The disk is named by size, not model, which is the most load-bearing sentence a stranger reads
//!   (design/roadmap/0569).
//! - Not crash-atomic: a power cut between the partitioner and `mkfs` leaves an unmountable disk.
//! - It checks for nife and nothing else, so a disk holding Windows looks empty (design/roadmap/0570).
//! - A disk already carrying nife is never offered an install again, including a half-installed one
//!   (design/roadmap/0572).

use super::*;
use crate::cap::{Rights, memory_region_cap, page_frame_cap, rendezvous_cap};
use crate::sched::RendezvousId;

/// **The word a person types to destroy a disk.** Upper case and unabbreviated, so that a stray
/// newline, a `y`, or a terminal echoing something cannot be it.
pub(crate) const CONFIRMATION: &str = "INSTALL";

/// How long the offer waits for an answer before continuing the boot.
///
/// Long enough that somebody who has just watched a machine come up can read three lines and type
/// seven characters; short enough that a stick left in a machine in a cupboard still reaches a
/// prompt. It is a duration measured against the counter, not a spin count.
const PATIENCE: core::time::Duration = core::time::Duration::from_secs(30);

/// Where the kernel maps the boot file, read-only, into the installer. **Must match
/// `components/src/system_installer.rs`'s `BOOT_FILE_VA`**; the bytes are a run of physical frames the
/// kernel owns, so the program cannot map them itself.
const BOOT_FILE_VA: u64 = address_space_map::runtime_window(0x1000_0000);

/// The installer's capability table. Must match `components/src/system_installer.rs`.
const SLOT_REPORT: u64 = 0;
const SLOT_BLK: u64 = 1;
const SLOT_ENTROPY: u64 = 2;
const SLOT_BUDGET: u64 = 3;
const SLOT_BLK_PAGE: u64 = 4;

/// The untyped the installer spends on the page tables for the one page it maps itself. The same
/// figure `disk_service` gives the partitioner, and for the same reason: it maps one page.
const SYSTEM_INSTALLER_BUDGET_PAGES: u64 = 64;

/// Extra stack pages for the installer, below the single page `run` maps.
///
/// It keeps a 16 KiB entry array in `.bss` rather than on the stack and recurses nowhere, so this
/// is the partitioner's number rather than a measurement of its own; the deepest thing it does is
/// `GloballyUniqueIdentifierPartitionTable::create`.
const SYSTEM_INSTALLER_STACK_PAGES: usize = 8;

/// The report's first word, from `components/src/system_installer.rs`. Duplicated here rather than shared
/// through a crate because it is the kernel's own convention with one program it spawns, the same
/// terms `disk_service` and `disk_tests` already state for `R_PARTITIONED` and `R_MADE`.
const R_INSTALLED: u64 = 0x_49_4E_53_54_44;

/// `mkfs`'s create role, from `redoxfs_server/src/bin/mkfs.rs`. Spelled here for the same reason
/// `disk_tests` spells it: the kernel's convention with a program it spawns.
const ROLE_MAKE: u64 = 0;
/// `mkfs`'s "a filesystem was created and a file written into it" verdict, ASCII `MKFSD`.
const R_MADE: u64 = 0x_4D_4B_46_53_44;

// The installer's roles, in `a0`. Must match `components/src/system_installer.rs`.
const ROLE_INSTALL: u64 = 0;
const ROLE_SURVEY: u64 = 1;
const ROLE_CONFIRM: u64 = 2;
/// The survey's "this disk already carries a nife data partition" verdict, ASCII `HAVIT`.
const R_ALREADY: u64 = 0x_48_41_56_49_54;
/// The confirm role's "that slot is successful" verdict, ASCII `CNFRM`.
const R_CONFIRMED: u64 = 0x_43_4E_46_52_4D;
/// Its "there is no such boot slot on this disk" refusal, ASCII `NOSLT`.
const R_NO_SLOT: u64 = 0x_4E_4F_53_4C_54;

/// **Offer to install, and carry it out if a person says so.** Returns after either, so the boot
/// continues in every case.
///
/// The three programs it needs are named rather than passed, because a caller on the boot path
/// has no business knowing which binaries an install is made of.
pub(crate) fn offer() {
    let Some((_, boot_file_len)) = crate::memory::boot_file_region() else {
        return; // Not booted from a file: nothing to install.
    };
    let Some(system_installer) = program("system_installer") else {
        crate::println!(
            "  install     : this boot came from a file, but the archive has no system_installer."
        );
        return;
    };
    let Some(nvme) = program("non_volatile_memory_express") else {
        return;
    };
    let Some(disk) = non_volatile_memory_express_service::ensure(nvme) else {
        return; // No NVMe controller: nothing to install onto.
    };
    disk.wait_for_ready();

    // **Before the question, and by a process that cannot write.** An installed machine boots from
    // a file too, so an offer that did not look at the disk first would ask every installed machine
    // to wipe itself, once per boot.
    let report = spawn_system_installer(system_installer, &disk, None, ROLE_SURVEY, 0, 0);
    if crate::sched::ipc_receive(report)[0] == R_ALREADY {
        return; // nife is already on this disk. See BUGS: there is no reinstall.
    }

    if !asked(disk.size_bytes) {
        crate::println!("  install     : not confirmed; continuing the boot.");
        return;
    }

    // Entropy from the instruction rather than from a device: `RDSEED` is on every x86_64 part a
    // stranger would install this on, and needing no virtio-rng is what lets the offer work on a
    // machine that has only a disk. A machine without it gets `None` here and the installer's own
    // refusal below, which is the honest answer rather than a GPT full of invented ids.
    let entropy = program("entropy")
        .and_then(|image| entropy_service::ensure(image, entropy_service::Bus::Instruction))
        .map(|w| {
            w.wait_for_ready();
            w.request
        });
    let Some(entropy) = entropy else {
        crate::println!(
            "  install     : this machine has no source of randomness, so a partition table \
             written here could not carry unique ids. Nothing was written."
        );
        return;
    };

    crate::println!("  install     : partitioning and copying the boot file...");
    let report = spawn_system_installer(
        system_installer,
        &disk,
        Some(entropy),
        ROLE_INSTALL,
        boot_file_len,
        boot_file_len,
    );
    let answer = crate::sched::ipc_receive(report);
    if answer[0] != R_INSTALLED {
        crate::println!(
            "  install     : FAILED ({:#x}, {}, {}). The disk may be in any state.",
            answer[0],
            answer[1],
            answer[2]
        );
        return;
    }
    crate::println!(
        "  install     : installed. nife data at LBA {}, EFI system at LBA {}.",
        answer[1],
        answer[2]
    );

    // The filesystem, by the program that already knows how to make one inside a partition it finds
    // by type GUID. It is `mkfs` unmodified, on the same disk and the same entropy endpoint.
    let Some(maker) = program("mkfs") else {
        crate::println!("  install     : the archive has no mkfs, so the data partition is empty.");
        return;
    };
    let target = disk_service::BlankDisk {
        blk_ep: disk.request,
        blk_shared: disk.transfer_phys,
        blk_ready: None,
    };
    let report = disk_service::start_maker(maker, &target, ROLE_MAKE, Some(entropy), true);
    let answer = crate::sched::ipc_receive(report);
    if answer[0] != R_MADE {
        crate::println!(
            "  install     : the partition table is written but mkfs refused ({:#x}).",
            answer[0]
        );
        return;
    }
    crate::println!("  install     : filesystem created.");
    crate::println!("  install     : DONE. Remove the installation medium and reboot.");
}

/// **Say that this boot worked**, so an upgrade that came up is not rolled back once its tries run
/// out (rung 2b of milestone 198 (a package manager)'s other half). A no-op on every boot no
/// chooser started (a stick, a `-kernel` boot, an installed machine whose chooser fell back to its
/// own file).
///
/// **The criterion: the machine got far enough that a person can log in and fix what else is
/// wrong.** At this call site the kernel passed its self-tests and reached userspace, the NVMe
/// server answered from ring 3, the filesystem server **mounted the installed disk and reported
/// ready**, and the progenitor was loaded, measured and built. Earlier would confirm an upgrade
/// with no working disk; later needs a person, and an unattended machine has none (the Boot Loader
/// Specification and ChromeOS's update engine make the same call). Such a boot can still be wrong
/// about anything boot does not exercise (the network, the compositor), about the shell, which
/// nothing has typed at yet, and about anything that fails after the first seconds; this is a
/// statement about coming up, repeated in `crates/boot_slot`'s `BUGS`.
///
/// **The authority**: `components/src/system_installer.rs`'s [`ROLE_CONFIRM`], spawned with four
/// capabilities: the report endpoint, the disk's `blk` endpoint, the page it shares with that
/// disk's server, and a budget for the one page it maps. **No entropy endpoint**, so the only table
/// it can write is the one it read; **no boot file**, so it cannot rewrite the image it vouches
/// for. A `blk` endpoint is the whole disk (nothing in `filesystem_contract::blk` bounds a client
/// to a block range), so it *could* write anywhere on that disk, as `system_installer`'s `BUGS`
/// records.
///
/// **It runs beside a live filesystem server, and the NVMe server keeps them apart.** The
/// installer is granted the request endpoint badged with its own transfer window
/// (`non_volatile_memory_express::INSTALLER_WINDOW`) and is mapped that window alone, so its
/// staging and the filesystem server's land in different pages whenever either runs. What it
/// writes is the partition table, which the filesystem server never reads after mounting, so the
/// two do not contend for a block either.
pub(crate) fn confirm() {
    let Some(slot) = crate::memory::boot_slot() else {
        return; // No chooser started this boot: there is nothing to confirm.
    };
    let Some(system_installer) = program("system_installer") else {
        crate::println!(
            "  boot slot   : slot {slot} cannot be confirmed: the archive has no system_installer."
        );
        return;
    };
    let Some(nvme) = program("non_volatile_memory_express") else {
        return;
    };
    let Some(disk) = non_volatile_memory_express_service::ensure(nvme) else {
        // A machine that booted from a slot and has no NVMe controller is a machine whose disk
        // went away between the chooser and here. Nothing to write to, and saying so is better
        // than a silent revert three boots later.
        crate::println!("  boot slot   : slot {slot} cannot be confirmed: no disk.");
        return;
    };
    disk.wait_for_ready();

    let report =
        spawn_system_installer(system_installer, &disk, None, ROLE_CONFIRM, slot as u64, 0);
    let answer = crate::sched::ipc_receive(report);
    match answer[0] {
        R_CONFIRMED if answer[2] == 0 => {
            crate::println!("  boot slot   : slot {slot} was already confirmed; nothing written.");
        }
        R_CONFIRMED => {
            crate::println!(
                "  boot slot   : slot {slot} confirmed. This upgrade will not roll back."
            );
        }
        R_NO_SLOT => {
            // The number crossed a boot and did not match anything, so nothing was written. A
            // machine in this state rolls back, which is the safe direction, and this line is the
            // only place a person would find out why.
            crate::println!(
                "  boot slot   : slot {slot} is not on this disk ({} boot slots found); NOT confirmed.",
                answer[1]
            );
        }
        other => {
            crate::println!(
                "  boot slot   : slot {slot} NOT confirmed ({other:#x}, step {}). This boot will roll back.",
                answer[1]
            );
        }
    }
}

/// Print what is about to be destroyed and read the answer. `true` only for exactly
/// [`CONFIRMATION`].
///
/// Name: provisional, flagged 2026-09-24 by the boolean-predicate pass
/// (design/naming/functions-that-answer-yes-or-no-worklist.md). It acts, so the Rust predicate rule calef
/// ratified 2026-09-24 exempts it, but it is a participle that reads as a question, on a function
/// that prompts and reads the reply; recommended `ask_to_confirm`.
fn asked(size_bytes: u64) -> bool {
    crate::println!("  install     : this system was booted from a file and can install itself.");
    crate::println!(
        "  install     :   TARGET: the NVMe disk attached to this machine, {size_bytes} bytes."
    );
    crate::println!("  install     :   EVERYTHING ON THAT DISK WILL BE DESTROYED.");
    crate::println!(
        "  install     :   Type {CONFIRMATION} and press return to proceed; anything else \
         continues the boot."
    );
    crate::print!("  install     : > ");

    let mut typed = [0u8; 32];
    let Some(n) = crate::console::read_line(&mut typed, PATIENCE) else {
        crate::println!();
        return false;
    };
    &typed[..n] == CONFIRMATION.as_bytes()
}

/// Spawn the installer with the four things it needs and nothing else: the disk, the randomness,
/// the page it shares with the disk's server, and a read-only copy of the file this machine booted
/// from.
fn spawn_system_installer(
    image: &'static [u8],
    disk: &non_volatile_memory_express_service::Wiring,
    entropy: Option<RendezvousId>,
    role: u64,
    // What the role reads out of `a1`: the boot file's length for [`ROLE_INSTALL`], the slot
    // number for [`ROLE_CONFIRM`], nothing for [`ROLE_SURVEY`].
    arg1: u64,
    // **How many bytes of the boot file to map**, which is a separate number from `arg1` because
    // only one role wants both. A role that maps none of it cannot rewrite the image it is
    // looking at, which is the same narrowing the missing entropy endpoint is.
    boot_file_len: u64,
) -> RendezvousId {
    let report = crate::sched::create_rendezvous();
    let budget = crate::memory_region::create(SYSTEM_INSTALLER_BUDGET_PAGES)
        .expect("no budget for system_installer");
    // The installer's own window (milestone 834), so a confirm beside a live filesystem server
    // stages in pages that server never reads.
    let blk_cap = disk.client_cap(::non_volatile_memory_express::INSTALLER_WINDOW);
    let blk_shared = disk.window_phys(::non_volatile_memory_express::INSTALLER_WINDOW);
    // Read only when `entropy` is `Some`; copied out here so the closure captures a plain `u64`,
    // the same shape `disk_service::start_partitioner` uses for the same reason.
    let ep = entropy.unwrap_or_default();

    crate::sched::spawn(move || {
        let maps = system_installer_maps(boot_file_len);
        crate::sched::grant_at(SLOT_REPORT, rendezvous_cap(report, Rights::WRITE))
            .expect("system_installer slot 0 was occupied");
        crate::sched::grant_at(SLOT_BLK, blk_cap).expect("system_installer slot 1 was occupied");
        // **Slot 2 is the difference between the two roles and nothing else is.** A survey holds
        // the same disk, writable; without entropy it cannot draw the unique ids a *new* table
        // carries, which narrows what the program would write and not what it can (module docs,
        // "When the offer is made", item 3).
        if entropy.is_some() {
            crate::sched::grant_at(SLOT_ENTROPY, rendezvous_cap(ep, Rights::WRITE))
                .expect("system_installer slot 2 was occupied");
        }
        crate::sched::grant_at(SLOT_BUDGET, memory_region_cap(budget))
            .expect("system_installer slot 3 was occupied");
        crate::sched::grant_at(
            SLOT_BLK_PAGE,
            page_frame_cap(blk_shared, Rights::READ.union(Rights::WRITE)),
        )
        .expect("system_installer slot 4 was occupied");
        run(
            image,
            Spawn {
                arg0: role,
                arg1,
                arg2: 0,
                grants: &[], // every one of them is placed above, at its own slot
                maps,
            },
        )
    })
    .expect("could not spawn system_installer");

    report
}

/// **Build the installer's extra mappings**: its stack, then the boot file read-only, one entry per
/// page.
///
/// The boot file is about ten megabytes, so this is a few thousand entries and they do not fit on a
/// stack or in `.bss` worth spending on a feature used once in the life of a machine. They go in
/// frames allocated for the purpose and are never freed, which costs about sixty kilobytes for the
/// rest of this boot and is the cheapest honest answer: the alternative is a `pages` field on
/// [`Mapping`], and that struct has ninety-six literals in this tree.
fn system_installer_maps(boot_file_len: u64) -> &'static [Mapping] {
    let boot_file_phys = crate::memory::boot_file_region().map_or(0, |(at, _)| at);
    let pages = boot_file_len.div_ceil(FRAME_SIZE) as usize;
    let total = SYSTEM_INSTALLER_STACK_PAGES + pages;

    let bytes = total * core::mem::size_of::<Mapping>();
    let scratch = crate::memory::alloc_contiguous_zeroed(bytes.div_ceil(FRAME_SIZE as usize))
        .expect("no scratch for the installer's mappings")
        .addr();
    // SAFETY: `total` `Mapping`s fit in the run of frames just allocated, which nothing else holds;
    // the direct map reaches them, and `Mapping` is a plain `Copy` struct of integers, so the
    // zeroed frames are already a valid initialised slice of them.
    let maps: &'static mut [Mapping] = unsafe {
        core::slice::from_raw_parts_mut(mmu::phys_to_virt(scratch) as *mut Mapping, total)
    };

    for (k, m) in maps[..SYSTEM_INSTALLER_STACK_PAGES].iter_mut().enumerate() {
        m.va = USER_STACK_VA - (k as u64 + 1) * FRAME_SIZE;
        m.phys = crate::memory::alloc()
            .expect("no frame for the installer's stack")
            .addr();
        m.flags = Flags::user_data();
    }
    for (i, m) in maps[SYSTEM_INSTALLER_STACK_PAGES..].iter_mut().enumerate() {
        m.va = BOOT_FILE_VA + i as u64 * FRAME_SIZE;
        m.phys = boot_file_phys + i as u64 * FRAME_SIZE;
        // Read-only, and it is the whole of what keeps a program holding a disk from also being
        // able to rewrite the image it is about to install.
        m.flags = Flags::user_rodata();
    }
    maps
}
