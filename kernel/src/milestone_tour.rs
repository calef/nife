//! **The milestone tour**: aarch64's demonstrations, between the self-test and the hand-over.
//!
//! Compiled out of the `shell`, `bench` and test builds. Moved out of the crate root by
//! milestone 876 (the kernel crate root is split along its seams), bodies unchanged.
//!
//! Name: provisional (milestone 876's lane, 2026-10-12 UTC).

use super::*;

// **The milestone tour, and what is left of it after milestone 267.**
//
// It was three things wearing one name: a machine description, a narrative, and a set of
// demonstrations. The description is `print_machine_description` above and prints on every
// boot. The narrative was `user/src/narrator.rs`, a program at EL0, and is deleted
// (milestone 267's block records what it said and why it went). What remains here is the
// third thing, and every entry is here because it needs a privilege a program does not
// have. The list is short on purpose, and it is the whole list:
//
//   - **The virtio and PCIe block demos.** The kernel enumerates the bus, mints the
//     device's registers, a DMA page and an interrupt into a driver's world, and receives
//     the driver's report with `sched::ipc_receive`. Every one of those is an authority whose
//     whole point is that the driver did not have it until the kernel granted it, so the
//     granter cannot be the grantee.
//
// The never-yield threads, the outlaw and the memory-region demo are `self_test::run`'s
// `preemption`, `userspace` and `budget` checks, on all three architectures (calef's ruling of
// 2026-10-19 (UTC) on #2434). What is left needs a virtio disk the machine may not have.
//
// The console server is started at the top of this block rather than being on that list.
// It is not a survivor either, and since the narrator was deleted it is not the move
// either: it comes up with nothing to print. See the block below.
//
// Compiled out by `cargo xtask shell`, which boots straight to the system instead of
// scrolling all of this first. There were two features here until milestone 296: `initboot`
// named nothing `shell` did not, and a build of each produced the same 3,109 symbols at the
// same total size. calef ruled it deleted rather than renamed on 2026-09-14.
#[cfg(not(any(test, feature = "system_tests", feature = "bench", feature = "shell")))]
pub(super) fn milestone_tour() {
    // **The console server, with nothing to print through it.** Milestone 267 moved the
    // nine-line milestone narrative out of `kernel_main` into `user/src/narrator.rs` and
    // spawned it here as this server's client; calef ruled the narrator deleted on
    // 2026-09-13 and the client went with it. What is left is the server: `user/src/
    // console.rs`, a UART driver at EL0, spawned and then idle on its request endpoint
    // because nothing else in this boot holds a capability to it.
    //
    // **That is a question, not a design**, and it is deliberately left open here rather
    // than answered by deleting one more thing: whether a boot-time console server earns
    // its bring-up once its only client is gone is an architect's call, because deleting it
    // removes infrastructure rather than a demonstration. Milestone 267's block states the
    // case both ways. The interactive system does not reach this code at all; it builds
    // its own console through `boot_progenitor` further down.
    //
    // The initial ramdisk is asked for first because both `expect`s inside
    // `console_service::start` are about the archive rather than the machine: a run with no
    // `-initrd` has no console program to start, and the initial ramdisk step further down is where
    // that boot says so.
    //
    // The `map` form is milestone 267's and is kept rather than rewritten. It was chosen
    // over `is_some() && let` on a measurement (340 bytes of `.text`), but that comparison
    // was made against a two-condition `if let` that no longer exists here, so the number
    // is history rather than a live claim about this line.
    let _console = user::initial_ramdisk().map(|_| user::console_service::start());

    tour_el0();
}

/// 7a. EL0.
#[cfg(not(any(test, feature = "system_tests", feature = "bench", feature = "shell")))]
fn tour_el0() {
    println!();
    println!("  and now the other side of the boundary:");
    println!();

    // Milestone 9: a virtio block device, driven from userspace.
    match crate::virtio::find_block_device() {
        None => println!("    virtio : no block device attached"),
        Some(d) => {
            println!(
                "    virtio : block device at {:#x}, INTID {}, handing it to a driver at EL0",
                d.mmio_phys, d.intid,
            );
            if let Some(report) = user::virtio_service::start(image_for_virtio()) {
                // The driver reads block 0 and sends us its first 8 bytes. We check they
                // are the nifefs magic, which proves real disk bytes crossed DMA and
                // the EL0 boundary. This RECEIVE blocks until the driver has done the read.
                let word = sched::ipc_receive(report)[0];
                let head = word.to_le_bytes();
                println!();
                if &head == b"nife: re" {
                    println!("      a driver at EL0 read the file 'motd' off a virtio disk,");
                    println!("      through a nifefs superblock it parsed itself,");
                    println!("      woken by the device's interrupt delivered as a message.");
                    println!("      the kernel issued no virtio command and touched no DMA.");
                } else {
                    println!("      the driver reported {head:?}, not the motd contents");
                }
            }
        }
    }

    // The same disk again, over PCIe (DECISIONS §18): found by ECAM enumeration,
    // BARs placed by the kernel, INTx through the GIC, the identical driver and
    // confinement behind the transport seam.
    match crate::pci::find_block_device() {
        None => println!("    pcie   : no virtio-blk on the bus"),
        Some(d) => {
            println!(
                "    pcie   : virtio-blk at {:02x}:{:02x}.{}, INTID {}, same driver, same confinement",
                d.bdf.bus, d.bdf.dev, d.bdf.func, d.intid,
            );
            if let Some(report) = user::virtio_service::start_pci(image_for_virtio()) {
                let word = sched::ipc_receive(report)[0];
                if &word.to_le_bytes() == b"nife: re" {
                    println!("      the same file, over the transport real hardware uses.");
                } else {
                    println!("      the pcie driver reported the wrong bytes");
                }
            }
        }
    }

    // Milestone 8. The console driver is no longer in the kernel.
    match user::initial_ramdisk() {
        None => println!("    initial ramdisk : none (no -initrd passed to QEMU)"),
        Some(image) => {
            println!(
                "    initial ramdisk : {} bytes at {:#x}, from the device tree",
                image.len(),
                memory::initial_ramdisk_region().unwrap().0,
            );
            // The console server and its client both came up at the top of the tour
            // (milestone 267): the narrative above is what they printed. This line
            // says where their images came from, which is the milestone-8 claim, and
            // no longer starts a second server to restate it.
        }
    }
}

/// The driver the virtio service spawns. `block_driver` on every architecture since milestone 291;
/// on aarch64 this used to be a role of `hello`, which was the same `crates/virtio` code behind a
/// second dispatch table. Panics if absent (the demo checked `initial_ramdisk()` above).
#[cfg(not(any(test, feature = "system_tests")))]
// Tour-only: the shell and bench boots both skip the milestone tour where it is used.
#[cfg_attr(feature = "shell", allow(dead_code))]
#[cfg(not(feature = "bench"))]
fn image_for_virtio() -> &'static [u8] {
    user::program("block_driver").expect("no block_driver program in the initial ramdisk")
}
