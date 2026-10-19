//! **Wiring for the EL0 NVMe block server** (milestone 261; [DECISIONS §86]'s option 2a,
//! DECIDED 2026-09-03; notes/non-volatile-memory-express.md).
//!
//! [DECISIONS §86]: ../../../design/decisions/0086-el0-nvme-driver.md
//!
//! `kernel/src/non_volatile_memory_express.rs` resets the controller, builds the admin queues,
//! IDENTIFYs the namespace and creates the one I/O queue pair; this file decides what the process
//! that drives that queue pair is handed. The whole confinement claim is the [`Spawn`] literal.
//!
//! # What the server holds, and what it is denied
//!
//! Held:
//!
//! - slot 0, the **request** endpoint (RECEIVE): `filesystem_contract::blk`'s wire format, the
//!   same one the virtio block server speaks;
//! - slot 1, a **readiness** endpoint (WRITE): one message once the first command has
//!   round-tripped, so a hang in bring-up is distinguishable from a hang in the first read;
//! - mapped: **one page of BAR0**, device-typed, the **doorbell page** at `bar0 + 0x1000`;
//! - mapped: **the data plane's run of its DMA region**, read/write: the I/O submission and
//!   completion rings and one window of [`crate::non_volatile_memory_express::TRANSFER_PAGES`]
//!   pages of buffer per client. A client is granted the request endpoint badged with its window
//!   ([`Wiring::client_capability`]) and is mapped that window alone ([`Wiring::window_phys`]), so two
//!   clients staging at once cannot reach each other's pages.
//!
//! Denied:
//!
//! - **BAR0's controller register page** (0x000..0x1000: `CC`, `CSTS`, `AQA`, `ASQ`, `ACQ`), so
//!   it cannot reset the controller or move a ring. NVMe 1.4 section 3.1 puts a page boundary there.
//! - **The admin plane's pages of the DMA region** (the two admin rings and the IDENTIFY buffer).
//! - **An `Irq` capability.** The server polls the completion ring's phase tag;
//!   `Command::create_io_cq` sets `IEN=0` and names no vector, so there is no interrupt to grant.
//! - **A `Virtio` capability**, because this is not a virtio device.
//! - **A `DeviceFrame` or `PageFrame` capability.** The pages arrive as [`Mapping`]s installed
//!   before `_start`, so the server holds no name for either window and cannot delegate, remap or
//!   revoke them.
//! - **The physical address of anything but its own data plane.**
//!   [`non_volatile_memory_express::Handoff`] carries page
//!   [`crate::non_volatile_memory_express::DATA_PLANE_PAGE`] of the region, not page 0.
//!

//! # BUGS
//!
//! **The server can aim DMA anywhere inside its region, its own admin rings included.** The IOMMU
//! bounds the controller to the whole allocation, because it fetches admin commands out of it, so
//! a PRP pointing at page 0 can overwrite the admin submission ring. That is denial of service
//! against the controller, not an escape, as §86 (whether an NVMe driver can leave the kernel)'s option 2a states; its option 4 (a doorbell
//! validator) would close it.
//!
//! **The doorbell page carries the admin doorbells too when `CAP.DSTRD` is 0** (`SQ0TDBL` at
//! `0x1000`, `CQ0HDBL` at `0x1004`). The server cannot write the admin ring, so ringing it only
//! makes the controller re-fetch slots: the same denial-of-service class. QEMU reports stride 0.
//!
//! **The server cannot read `CSTS`, so a hung controller presents as a timeout rather than as
//! `CSTS.CFS`**: the bounded poll answers `EIO`. That is the price of not mapping the register page.
//!
//! **One command in flight, so the ring depth buys nothing**, and any throughput measured against
//! this server is a lower bound.
//!
//! Name: recorded (crates/non_volatile_memory_express), renamed 2026-09-18 under DECISIONS §154
//! (the acronym test is whether the phrase is spoken). A `<program>_service` like
//! `entropy_service`, so the name is the program's plus the suffix; the ruling and refusals live
//! in `crates/non_volatile_memory_express`.

use super::*;
use crate::cap::{Rights, rendezvous_cap};
use crate::sched::RendezvousId;

/// Where the server's data-plane run is mapped. Must match `components/src/non_volatile_memory_express.rs`.
/// Deliberately distinct from the `0x0090_0000` DMA convention `entropy.rs` and
/// `keyboard_driver.rs` share, and from milestone 159's `0x0094_0000`, so no two of these
/// constants can mean two things at once.
const DATA_PLANE_VA: u64 = address_space_map::pair_page(0x0000_0000_0098_0000);

/// Where the server's one page of BAR0 is mapped, device-typed. Must match
/// `components/src/non_volatile_memory_express.rs`. Far enough above [`DATA_PLANE_VA`] that the data plane's run
/// can grow without colliding with it.
const DOORBELL_VA: u64 = address_space_map::pair_page(0x0000_0000_009c_0000);

/// How many pages the data plane is mapped: the two I/O rings, then every client's transfer window.
const DATA_PLANE_PAGES: usize = 2 + crate::non_volatile_memory_express::TRANSFER_PAGES as usize
    * ::non_volatile_memory_express::CLIENT_WINDOWS;

/// Everything one wiring of the server is, from the outside.
pub struct Wiring {
    /// The server's readiness endpoint, and **only on the call that did the wiring**: the report
    /// is sent once and whoever asked first has taken it.
    pub ready: Option<RendezvousId>,
    /// The request endpoint. **This is the capability a client is given**, with WRITE; the server
    /// holds READ. Nothing about it names the device, the doorbells, or the DMA region.
    pub request: RendezvousId,
    /// The **physical** base of window 0's transfer buffer
    /// (`non_volatile_memory_express::FILESYSTEM_WINDOW`), the window an unbadged request stages
    /// in: the filesystem server's, and the kernel's own when it plays a client. The other
    /// windows follow it contiguously; [`Wiring::window_phys`] names one.
    pub transfer_phys: u64,
    /// True when the IOMMU unit this kernel programmed is **the one that owns the controller's
    /// requester id** ([`crate::iommu::Scope::is_confining`]), and the controller was confined to the
    /// DMA region before it was enabled. False means the driver is as unconfined as its
    /// arithmetic. Until milestone 261's bench rehearsal this was `iommu::is_active()`, which is true
    /// on any machine where *some* unit is translating, owner or not.
    pub confined_by_iommu: bool,
    /// Which unit owns the controller, and how: the evidence behind `confined_by_iommu`, printed
    /// by the bench preflight (fatal risk 6's first night-of condition).
    pub scope: crate::iommu::Scope,
    /// The controller's PCIe requester id.
    pub rid: u32,
    /// Logical blocks per 4096-byte filesystem block, as the handoff carried it into ring 3:
    /// always in `1..=8` for a server that started (fatal risk 6's second night-of condition).
    pub blocks_per: u16,
    /// **The namespace's size in bytes as the kernel's admin plane read it from IDENTIFY**, which
    /// is the number [`non_volatile_memory_express::Handoff`] carried into ring 3. A test compares the server's answers
    /// against *this* rather than against a constant, so the same assertions hold on QEMU's 8 MiB
    /// image and on a 256 GB disk (milestone 318).
    pub size_bytes: u64,
}

/// **One NVMe server per boot**, for the reason the entropy service is wired once: a second
/// bring-up would reset the controller and recreate the queues out from under the first, and the
/// first would then poll forever for a completion nothing was told to make.
static WIRED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
static REQUEST: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
static TRANSFER: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
static CONFINED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
static SIZE_BYTES: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
static RID: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
static BLOCKS_PER: core::sync::atomic::AtomicU16 = core::sync::atomic::AtomicU16::new(0);

/// Wire the NVMe server if this boot has not already, else hand back what is already running.
/// `None` when there is no NVMe controller on the bus, or when one is present and failed
/// bring-up. For a caller that must tell those two apart (a test, a bench), [`ensure_or_why`].
pub(crate) fn ensure(image: &'static [u8]) -> Option<Wiring> {
    ensure_or_why(image).ok()
}

/// [`ensure`], keeping the reason there is no server: no controller on the bus, or a controller
/// this driver refused and why (milestone 261's bench rehearsal). The second is never a skip.
pub fn ensure_or_why(
    image: &'static [u8],
) -> Result<Wiring, crate::non_volatile_memory_express::Absent> {
    use core::sync::atomic::Ordering;

    if WIRED.load(Ordering::Acquire) {
        let rid = RID.load(Ordering::Relaxed);
        return Ok(Wiring {
            ready: None,
            request: REQUEST.load(Ordering::Relaxed),
            transfer_phys: TRANSFER.load(Ordering::Relaxed),
            confined_by_iommu: CONFINED.load(Ordering::Relaxed),
            scope: crate::iommu::scope_of(rid),
            rid,
            blocks_per: BLOCKS_PER.load(Ordering::Relaxed),
            size_bytes: SIZE_BYTES.load(Ordering::Relaxed),
        });
    }
    let w = start(image)?;
    REQUEST.store(w.request, Ordering::Relaxed);
    TRANSFER.store(w.transfer_phys, Ordering::Relaxed);
    CONFINED.store(w.confined_by_iommu, Ordering::Relaxed);
    SIZE_BYTES.store(w.size_bytes, Ordering::Relaxed);
    RID.store(w.rid, Ordering::Relaxed);
    BLOCKS_PER.store(w.blocks_per, Ordering::Relaxed);
    WIRED.store(true, Ordering::Release);
    Ok(w)
}

/// **Bring the controller up in the kernel, then hand its data plane to a process.**
fn start(image: &'static [u8]) -> Result<Wiring, crate::non_volatile_memory_express::Absent> {
    let found = crate::non_volatile_memory_express::bring_up()?;
    let rid = found.rid;
    let scope = crate::iommu::scope_of(rid);
    let confined_by_iommu = scope.is_confining();
    let handoff = found.controller.handoff();
    let words = handoff.pack();

    // The doorbell page, and only it. BAR0's first page holds `CC`, `CSTS`, `AQA`, `ASQ` and
    // `ACQ`; the doorbells start at 0x1000 (NVMe 1.4 §3.1.25), which is why "the kernel keeps the
    // admin plane and EL0 gets the data path" needs no new capability to express.
    let doorbell_phys = found.bar0 + page_frames::FRAME_SIZE;
    let data_plane_phys = handoff.data_plane_phys;
    let transfer_phys = data_plane_phys + 2 * page_frames::FRAME_SIZE;

    let ready = crate::sched::create_rendezvous();
    let request = crate::sched::create_rendezvous();

    // `found.controller` is dropped here and that is correct today, because `NonVolatileMemoryExpress` has no `Drop`:
    // the controller stays enabled and its queues stay where the admin plane put them, which is
    // exactly what a live EL0 data plane needs. **If it ever grows one that resets the
    // controller, this line becomes a bug**, and the fix is to keep the value alive rather than
    // to forget it, so the admin plane is still reachable for a re-wire.

    crate::sched::spawn(move || {
        // The two I/O rings, then the transfer buffer, contiguous at DATA_PLANE_VA; then one
        // device-typed page of BAR0. Normal-memory flags for the first run because the controller
        // and the driver share ordinary RAM there, device flags for the last because a cached or
        // reordered doorbell write is a command the controller never hears about.
        let mut maps = [Mapping {
            va: 0,
            phys: 0,
            flags: Flags::user_data(),
        }; DATA_PLANE_PAGES + 1];
        super::fs_service::map_channel(
            &mut maps[..DATA_PLANE_PAGES],
            DATA_PLANE_VA,
            data_plane_phys,
            DATA_PLANE_PAGES,
        );
        maps[DATA_PLANE_PAGES] = Mapping {
            va: DOORBELL_VA,
            phys: doorbell_phys,
            flags: Flags::user_device(),
        };
        run(
            image,
            Spawn {
                arg0: words[0], // the packed geometry: see `non_volatile_memory_express::Handoff`
                arg1: words[1], // the data plane's PHYSICAL base: PRP fields speak physical
                arg2: words[2], // the namespace's size in bytes, the `SIZE` answer
                grants: &[
                    rendezvous_cap(request, Rights::READ), // slot 0: RECEIVE blk requests
                    rendezvous_cap(ready, Rights::WRITE),  // slot 1: signal readiness once
                ],
                maps: &maps,
            },
        )
    })
    .expect("could not spawn the NVMe server");

    Ok(Wiring {
        ready: Some(ready),
        request,
        transfer_phys,
        confined_by_iommu,
        scope,
        rid,
        blocks_per: handoff.blocks_per,
        size_bytes: handoff.size_bytes,
    })
}

/// **The byte a confinement-falsification canary is planted with.** Distinct from `0x00` (a zeroed
/// frame and the NVMe image's own contents) so "untouched" means "still the canary" rather than
/// "still zero", and a `READ` escape that landed, copying the image's zeros over it, is caught.
#[cfg(feature = "system_tests")]
const CANARY_BYTE: u8 = 0xC1;

/// **One DMA-escape attacker, wired and running** (milestone 261's Outstanding item; milestone 202
/// (every confinement test is a ritual until somebody breaks the confinement and watches it fail)).
/// What [`start_dma_escape`] hands its test caller.
#[cfg(feature = "system_tests")]
pub struct DmaEscape {
    /// The attacker's readiness endpoint. One message once it has tried both escapes: word 0 the
    /// read-escape status, word 1 the write-escape status (see `run_dma_escape`).
    pub report: RendezvousId,
    /// The physical base of the victim frame, outside the controller's DMA region. The test looks
    /// for this in the IOMMU's fault queue and reads its canary back through the direct map.
    pub victim: u64,
    /// True when the unit this kernel programmed owns the controller's requester id
    /// ([`crate::iommu::Scope::is_confining`]). False means the escape would be refused by nothing,
    /// so the test must not read a pass into it.
    pub confined_by_iommu: bool,
    /// The evidence behind `confined_by_iommu`, for the failure message.
    pub scope: crate::iommu::Scope,
    /// The controller's PCIe requester id, which ties a fault to it when the IOMMU reports no
    /// address (AMD-Vi under QEMU 11.1.1; see `arch/x86_64/iommu.rs`'s `Fault::addr`).
    pub rid: u32,
    /// The kernel thread running the attacker, which [`DmaEscape::release`] waits out.
    thread: crate::thread::ThreadId,
    /// The one-page region `report` lives in, so [`DmaEscape::release`] can destroy it. A kernel
    /// rendezvous (`create_rendezvous`) is never freed, and this one extra pushed the suite over a
    /// 32-page chunk boundary that a later test then paid for.
    ep_region: u64,
    /// The controller the attacker was given, kept so [`DmaEscape::release`] can take it down
    /// before handing its region back.
    controller: crate::non_volatile_memory_express::NonVolatileMemoryExpress,
}

#[cfg(feature = "system_tests")]
impl DmaEscape {
    /// Block for the attacker's report (it sends once, after both escapes).
    pub fn wait(&self) -> [u64; 5] {
        crate::sched::ipc_receive(self.report)
    }

    /// True when every byte of the victim frame is still the canary, i.e. the controller's DMA
    /// never reached it. A `READ` escape that the IOMMU failed to refuse would have copied the
    /// disk's bytes over it, so this goes false exactly when the confinement does.
    pub fn canary_intact(&self) -> bool {
        let p = crate::arch::mmu::phys_to_virt(self.victim) as *const u8;
        (0..page_frames::FRAME_SIZE as usize).all(|i| {
            // SAFETY: `victim` is a frame this wiring allocated and frees only in `release`, which
            // consumes `self`, so it is still ours here. The direct map covers all of RAM, and `i`
            // stays inside the one frame. CPU reads bypass the IOMMU, which is the point: only the
            // device's view of this frame is unmapped.
            let byte = unsafe { p.add(i).read_volatile() };
            byte == CANARY_BYTE
        })
    }

    /// **Give back the frames this wiring took**: the controller's DMA region, the victim and the
    /// report endpoint's region, which the suite's ledger otherwise counts against it for the rest
    /// of the boot. Only once
    /// the attacker's thread is gone (its address space mapped the data plane) and the controller
    /// is disabled ([`crate::non_volatile_memory_express::NonVolatileMemoryExpress::retire`]),
    /// because until both hold, something may still read or DMA into these frames. Returns whether
    /// everything came back, which the test asserts. The IOMMU domain's tables stay, for the
    /// reason `retire` gives.
    ///
    /// Name: ratified 2026-10-05 (calef, #1647).
    pub fn release(self) -> bool {
        // Ten seconds of TCG, far past an `exit` that follows the report this caller already took.
        let deadline = crate::arch::timer::now() + 10 * crate::arch::timer::frequency();
        while crate::sched::is_thread_present(self.thread) && crate::arch::timer::now() < deadline {
            crate::sched::yield_now();
        }
        if crate::sched::is_thread_present(self.thread) || self.controller.retire().is_err() {
            return false;
        }
        crate::memory::free(page_frames::PageFrame::from_addr(self.victim));
        crate::sched::reclaim_region(self.ep_region).is_ok()
    }
}

/// **Bring the controller up confined, then spawn an EL0 server that aims a PRP outside its region**
/// (milestone 261's Outstanding item; milestone 202's convention). The storage twin of
/// `display_service::start_backing_escape`: the attacker gets exactly the honest server's world
/// (the same confined controller, the same data plane, the same doorbell) and differs only in the
/// physical address it writes into a command's PRP. `None` when no NVMe controller is on the bus.
///
/// The kernel picks the victim frame and plants its canary, the way milestone 16b (IOMMU-backed
/// driver isolation)'s confinement test and the GPU backing-escape test both do: the caller has to know the exact address to look
/// for in the IOMMU's fault queue, and a server guessing "the frame past my region" guesses wrong
/// (the next allocation may be inside the domain). The frame is freed only by
/// [`DmaEscape::release`], after the controller is disabled: returning it to the allocator while
/// the controller has been told to read it is the use-after-free-by-hardware notes/dma.md warns
/// about.
#[cfg(feature = "system_tests")]
pub fn start_dma_escape(image: &'static [u8]) -> Option<DmaEscape> {
    // The victim, outside any region bring_up will confine the controller to. Planted with the
    // canary before the controller is enabled, so a landed escape is the only thing that could
    // change it.
    let victim = crate::memory::alloc()?.addr();
    // SAFETY: a freshly allocated frame, reached through the direct map; nothing else names it yet.
    unsafe {
        core::ptr::write_bytes(
            crate::arch::mmu::phys_to_virt(victim) as *mut u8,
            CANARY_BYTE,
            page_frames::FRAME_SIZE as usize,
        );
    }

    // Bring the controller up and confine it to its own DMA region. In the shipped tree
    // `ESCAPE_TARGET` is zero, so `victim` is not in the controller's domain and the escape below
    // faults. The replayable falsification inserts a store of `victim` into `ESCAPE_TARGET` right
    // here, which widens the domain to cover it; that one line is the whole patch.
    let found = crate::non_volatile_memory_express::bring_up().ok()?;
    let rid = found.rid;
    let scope = crate::iommu::scope_of(rid);
    let confined_by_iommu = scope.is_confining();
    let controller = found.controller;
    let handoff = controller.handoff();
    let words = handoff.pack();

    let doorbell_phys = found.bar0 + page_frames::FRAME_SIZE;
    let data_plane_phys = handoff.data_plane_phys;
    let transfer_phys = data_plane_phys + 2 * page_frames::FRAME_SIZE;

    // The control block the EL0 role reads first thing: `[ESCAPE_MAGIC, victim]` at the start of
    // the transfer buffer. No fence orders it against the reader: these writes complete before the
    // `crate::sched::spawn` below, and the spawn is the happens-before edge (the scheduler
    // publishes the new thread after this closure and its captured writes are in place), the same
    // ordering the honest `start` relies on for the geometry it writes.
    // SAFETY: `transfer_phys` is the data plane's transfer buffer, a frame this bring-up allocated,
    // reached through the direct map.
    unsafe {
        let p = crate::arch::mmu::phys_to_virt(transfer_phys) as *mut u64;
        p.write_volatile(non_volatile_memory_express::ESCAPE_MAGIC);
        p.add(1).write_volatile(victim);
    }

    // In a region of its own rather than on the kernel's chunk, so `release` can hand it back; the
    // way `fs_service::start_std_bound` makes its report endpoint.
    let ep_region = crate::memory_region::create(1).expect("no endpoint region for the attacker");
    let report = crate::sched::create_rendezvous_from(ep_region)
        .expect("no report endpoint for the attacker");

    let thread = crate::sched::spawn(move || {
        let mut maps = [Mapping {
            va: 0,
            phys: 0,
            flags: Flags::user_data(),
        }; DATA_PLANE_PAGES + 1];
        super::fs_service::map_channel(
            &mut maps[..DATA_PLANE_PAGES],
            DATA_PLANE_VA,
            data_plane_phys,
            DATA_PLANE_PAGES,
        );
        maps[DATA_PLANE_PAGES] = Mapping {
            va: DOORBELL_VA,
            phys: doorbell_phys,
            flags: Flags::user_device(),
        };
        run(
            image,
            Spawn {
                arg0: words[0],
                arg1: words[1],
                arg2: words[2],
                grants: &[
                    // slot 0 is unused by the attacker (it never serves a client); it keeps the
                    // slot shape the honest server has. slot 1 is where it reports.
                    rendezvous_cap(report, Rights::READ),
                    rendezvous_cap(report, Rights::WRITE),
                ],
                maps: &maps,
            },
        )
    })
    .expect("could not spawn the NVMe DMA-escape attacker");

    Some(DmaEscape {
        report,
        victim,
        confined_by_iommu,
        scope,
        rid,
        thread,
        ep_region,
        controller,
    })
}

impl Wiring {
    /// The physical base of client window `window`'s transfer buffer: the pages a client granted
    /// [`Wiring::client_capability`]`(window)` must be mapped, and no others.
    ///
    /// Name: provisional, minted 2026-10-18 (UTC) by lane/ioping-direct-io.
    pub fn window_phys(&self, window: u64) -> u64 {
        assert!(
            ::non_volatile_memory_express::client_window(window).is_some(),
            "no NVMe client was given window {window}",
        );
        self.transfer_phys
            + window * crate::non_volatile_memory_express::TRANSFER_PAGES * page_frames::FRAME_SIZE
    }

    /// The request endpoint as client window `window` is granted it: `WRITE`, badged with the
    /// window, so the server stages that client's requests in [`Wiring::window_phys`]`(window)`.
    /// Window 0 is unbadged, as the filesystem server has always held it.
    ///
    /// Name: provisional, minted 2026-10-18 (UTC) by lane/ioping-direct-io.
    pub fn client_capability(&self, window: u64) -> crate::cap::Cap {
        assert!(::non_volatile_memory_express::client_window(window).is_some());
        crate::cap::rendezvous_cap_badged(self.request, Rights::WRITE, window)
    }

    /// Take the startup report, or `None` when this caller was not the one that wired the server.
    /// Word 0 is [`filesystem_contract::READY`] when the first command round-tripped, and
    /// a `0xDEAD_..` word naming the failing step otherwise; word 1 is the namespace size the
    /// server was handed, so a bench transcript can see the geometry reached ring 3 intact.
    pub fn wait_for_ready(&self) -> Option<[u64; 5]> {
        self.ready.map(crate::sched::ipc_receive)
    }

    /// **Play a client**: one blk request over the request endpoint, the way any holder of that
    /// endpoint would. The kernel exercises the contract rather than reaching into the server,
    /// which is the whole point of the server being a process.
    pub fn blk(&self, operation: u64, block: u64) -> i64 {
        crate::sched::ipc_call(self.request, [filesystem_contract::req(operation), block])[0] as i64
    }

    /// The transfer buffer's first block, through the direct map, for a caller staging a write or
    /// checking a read.
    ///
    /// # Safety
    /// The server must not be mid-transfer, which every caller gets from the blk contract's own
    /// turn-taking: a request is a `CALL`, so the client is blocked exactly while the server runs.
    pub unsafe fn transfer_block(
        &self,
    ) -> &'static mut [u8; crate::non_volatile_memory_express::BLOCK_SIZE] {
        // SAFETY: forwarded from this function's own contract; `transfer_phys` is a frame this
        // wiring allocated and the direct map covers all of RAM.
        unsafe {
            &mut *(crate::arch::mmu::phys_to_virt(self.transfer_phys)
                as *mut [u8; crate::non_volatile_memory_express::BLOCK_SIZE])
        }
    }
}
