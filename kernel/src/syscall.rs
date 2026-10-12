//! The syscall boundary. **Five calls <!--count:syscalls-->.**
//!
//! DECISIONS §4 rule 3 said the syscall surface stays narrow and explicit, *"a boundary, not a
//! habit."* §8 said milestone 7 was a hard decision point and that hacking one in without the
//! conversation meant the plan had failed. §10 had the conversation and chose capabilities.
//!
//! This is what that buys:
//!
//! ```text
//!   exit(code)                          authority over yourself
//!   exit_thread(word)                   likewise, one thread of yourself (milestone 812)
//!   yield()                             likewise
//!   cap_delete(slot)                    likewise: your own capability table is your own
//!   invoke(cap, method, a0, a1, a2)     EVERYTHING ELSE
//! ```
//!
//! All but one are authority over yourself and the last is everything else, which is the
//! property worth remembering rather than the count. This header said "three calls" from
//! 2026-07-14 until the 2026-08-17 documentation sweep, `abi::SYS_CAP_DELETE` having arrived on
//! 2026-07-24 without it; the count is now a re-derived `<!--count:syscalls-->` claim.
//!
//! No `open`. No `read`. No `write`. No `fork`. **A process can only act on things it was
//! handed.** The ABI lives in `crates/abi`, which both the kernel and every user program depend
//! on, so the boundary is *one artifact* rather than two files that agree by luck.
//!
//! # No pointer ever crosses this boundary
//!
//! There used to be a `user_slice` here: the console `write` syscall took a `(ptr, len)` from
//! userspace and the kernel read the user's memory, which is why it needed the `AT S1E0R`
//! confused-deputy defence. Milestone 8 moved the console to a userspace server and deleted that
//! path. Today every argument is a scalar in a register (a capability slot, a method, a `va`, a
//! word), so the kernel follows no user pointer and there is no deputy to confuse. The primitive
//! that made the old check possible, `mmu::user_can_read`, is kept for the next syscall that does
//! take a user pointer.

use abi::Error;

use crate::arch::exceptions::TrapFrame;
use crate::cap::{Object, Rights};
use crate::sched;

// **The `ThreadControlBlock` methods, in a file of their own** because this one reached §266 (a Rust
// source file stays under 2,000 lines)'s ceiling when milestone 812 (`std::thread::spawn` runs
// real threads in one address space) added `SET_THREAD_POINTER`. They were already one
// out-of-line seam; [`invoke`] reaches them through one call. The other objects' methods followed
// on 2026-10-12 (UTC), one child per object family, for §282 (when a file is split, the parent
// becomes an index).
mod address_space;
mod memory_region;
mod object_methods;
mod page_frame;
mod retype;
mod thread_control_block;
use address_space::address_space_invoke;
use memory_region::{
    memory_region_destroy, memory_region_map, memory_region_retype, memory_region_split,
    memory_region_usage,
};
#[cfg(target_arch = "x86_64")]
use object_methods::port_range_invoke;
use object_methods::{
    aborted_send_error, irq_ack, irq_wait, notification_invoke, process_invoke, reboot_invoke,
    rendezvous_badge, rendezvous_reap, reply_other, timer_invoke, virtio_invoke,
};
use page_frame::{page_frame_map, page_frame_revoke, page_frame_slice};
use thread_control_block::thread_control_block_invoke;

// Index exception (2026-10-12): `script/fastpath-footprint` finds the syscall entry by this
// path, `kernel7syscall8dispatch` in the mangled name, and counts it flat as `syscall_entry`.
/// Called from the `svc` arm of `exception_body` (`ecall` on riscv64, in `riscv_trap_body`). The
/// *body*, not the dispatcher, since milestone 124 split the two: a syscall arrives from user mode,
/// which is the case that deliberately does NOT move to the interrupt stack, because this path can
/// block and a blocked thread's frames must live on its own stack.
///
/// **`#[inline(never)]` keeps this a symbol `script/fastpath-footprint` can name**, which is the
/// third instance of a pattern the tree has now recorded twice
/// (`design/roadmap/0368-a-flat-entry-set-counts-bytes-no-syscall-fetches.md`): that gate's
/// `syscall_entry` half sums a flat list of symbols, so an LLVM inlining flip moves bytes into or
/// out of the measurement without anything on the syscall path changing. Milestone 220's lane hit
/// the *outward* direction, which is the one that reads as good news and is not: adding an
/// unrelated dependency to this crate made LLVM fold these 1160 bytes into the aarch64 exception
/// handler, and the gate reported `syscall_entry` shrinking 35% while the code a syscall fetches
/// was identical. Re-recording the baseline there would have locked in an under-measurement.
///
/// It also stands on its own, which is the test the tree asks of every `#[inline(never)]` it
/// carries (`timer::tick`, `plic::disable`, `sched::grant_cycle_counter`, milestone 156's spawn
/// bodies): this is a large match executed once per syscall, and one `bl` is not a cost worth
/// duplicating a kilobyte of dispatcher into a handler that also serves faults and interrupts.
#[inline(never)]
// In the pinned hot section: milestone 796 (pin the hot trap path's placement).
#[cfg_attr(
    target_os = "none",
    unsafe(link_section = ".text.hot.syscall.dispatch")
)]
pub fn dispatch(frame: &mut TrapFrame) {
    // The syscall number and arguments come from the trap frame through arch accessors, not raw
    // register indices, because the ABI register file differs per architecture (aarch64 `svc` with
    // the number in x8 and args in x0..x5; RISC-V `ecall` with the number in a7 and args in a0..a5).
    // `TrapFrame::{syscall_nr, arg, set_arg}` hide that mapping so this dispatcher stays portable.
    // See DECISIONS §10/§17.
    let nr = frame.syscall_nr();
    // Milestone 134's per-IPC stack-depth instrument keys each sample by method, and the method
    // register may be overwritten by a result before the end of this function. Not in any build
    // that measures time or footprint: see `crate::ipc_stack_depth`.
    #[cfg(any(test, feature = "ipc_stack_depth"))]
    let method = frame.arg(1);

    // `exit` never comes back, so it is not part of the result-writing path below.
    if nr == abi::SYS_EXIT {
        sched::exit();
    }
    // End this thread only (milestone 812): beside `exit`, and like it never back here.
    if nr == abi::SYS_EXIT_THREAD {
        sched::exit_thread(frame.arg(0));
    }

    let result: Result<i64, Error> = match nr {
        abi::SYS_YIELD => {
            sched::yield_now();
            Ok(0)
        }
        // Drop a capability from the caller's own capability table (milestone 19d). Deleting an empty slot
        // is a no-op, not an error: a loader recycling slots should not have to track emptiness.
        abi::SYS_CAP_DELETE => {
            let _ = sched::delete_current_cap(frame.arg(0));
            Ok(0)
        }
        abi::SYS_INVOKE => invoke(
            frame,
            frame.arg(0),
            frame.arg(1),
            frame.arg(2),
            frame.arg(3),
            frame.arg(4),
        ),
        _ => Err(Error::BadSyscall),
    };

    // The return value goes back in the first argument register, which the trap-restore path pops
    // into the register the user is waiting on. Writing to the trap frame IS writing to the user's
    // registers.
    frame.set_arg(
        0,
        match result {
            Ok(v) => v as u64,
            Err(e) => (e as i64) as u64,
        },
    );

    // Last, so the sample covers the whole syscall; one relaxed load for every thread the
    // instrument is not measuring.
    #[cfg(any(test, feature = "ipc_stack_depth"))]
    crate::ipc_stack_depth::after_syscall(nr, method);
}

// Index exception (2026-10-12): the decoder inlines into `dispatch` above, and moving it to a
// child would let a codegen-unit split change what `syscall_entry` measures.
/// Act on a capability.
///
/// **The lookup is the security mechanism, and it is a bounds check.** `slot` is an index into
/// *this thread's* table, which lives in kernel memory. An empty slot is `NoSuchSlot`: not
/// "permission denied", but *there is nothing there*. That difference is what no-ambient-authority
/// feels like from the inside.
///
/// `pub(crate)` so kernel tests in other modules can drive **this** path rather than a
/// re-implementation of it: an authorization test that calls `sched` directly proves the helper, not
/// the boundary. See `user/reap_tests.rs`.
pub fn invoke(
    frame: &mut TrapFrame,
    slot: u64,
    method: u64,
    a0: u64,
    a1: u64,
    a2: u64,
) -> Result<i64, Error> {
    let cap = sched::current_cap(slot).map_err(|_| Error::NoSuchSlot)?;

    match cap.object {
        Object::Rendezvous(ep, badge) => match method {
            // SEND takes WRITE, RECEIVE takes READ. The *same* endpoint, handed out with different
            // rights, is a one-way pipe in whichever direction each holder was trusted with.
            abi::rendezvous::SEND => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                // The three words are already in registers. **Nothing is read from user memory**,
                // so there is no pointer to validate and no confused-deputy question to ask. That
                // is the fastpath, and it is why IPC carries control and not bulk data (§10).
                // The badge rides in word 3 (milestone 613 (a system log service), provisional):
                // a byte-sink writer's identity is its capability's badge, never its own claim.
                sched::ipc_send_badged(ep, [a0, a1, a2], badge);
                // If the endpoint was revoked (stale, or reclaimed while we blocked), the send never
                // happened: report it rather than a silent success. Object revocation, notes/.
                //
                // `Gone`, not `NoSuchSlot`, since milestone 50. The slot is not empty; a real
                // capability names an object that has been destroyed, and a writer branches on the
                // difference in opposite directions (`abi::Error::Gone`, notes/sink-protocol.md).
                // This one line is what turns a dead pipe reader into something the producer can
                // act on, which is why the ABI grew a variant rather than the sink protocol growing
                // a heartbeat.
                //
                // **Or it was refused** (milestone 603 (provisional), DECISIONS §101 (notification objects) ruling B): the
                // endpoint carries a hardware interrupt, whose driver reads `w0 = 1` as "the device
                // fired", so no program may deposit anything there. `NotPermitted`, because it is
                // the answer to an operation the capability names but may not perform, and it is
                // asked only once the abort branch is taken, so a send that went through pays
                // nothing for it.
                if sched::take_ipc_aborted() {
                    return Err(aborted_send_error());
                }
                Ok(0)
            }
            abi::rendezvous::RECEIVE => {
                if !cap.rights.allows(Rights::READ) {
                    return Err(Error::NotPermitted);
                }
                let msg = sched::ipc_receive(ep);
                if sched::take_ipc_aborted() {
                    return Err(Error::Gone); // endpoint revoked; the message is a placeholder
                }
                // Word 0 goes back the way every syscall result does, in x0 (dispatch writes it
                // from our return value). Words 1..4 we place directly, because a syscall return is
                // one register and a message is up to five: ordinary IPC fills three and leaves the
                // top two zero, and a fault/exit notification (DECISIONS §26) fills all five.
                frame.set_arg(1, msg[1]);
                frame.set_arg(2, msg[2]);
                frame.set_arg(3, msg[3]);
                frame.set_arg(4, msg[4]);
                Ok(msg[0] as i64)
            }

            // Delegation. `a0` is the slot of the capability to pass on, `a1` the rights to narrow
            // it to, `a2` one data word. Two rights are in play and they are different questions:
            // WRITE on *this* endpoint (may I send here?) and GRANT on the *delegated* capability
            // (was I trusted to pass it on?). Without GRANT you may use a thing and not lend it.
            abi::rendezvous::SEND_CAP => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                // The source is read, checked (GRANT, narrow-only) and copied inside `sched`, under
                // the hold that files the copy, so no revocation sweep can fall between the read and
                // the filing. Reading it here first was that gap (`sched::Delegation`).
                #[cfg(feature = "system_tests")]
                crate::delegation_pause::here(); // no lock held
                sched::ipc_delegate_cap(
                    ep,
                    a2,
                    sched::Delegation {
                        slot: a0,
                        rights: Rights::from_bits(a1 as u32),
                    },
                    badge, // the endpoint we send on may be badged (milestone 599 (a frame per filesystem client channel))
                )?;
                if sched::take_ipc_aborted() {
                    // Revoked, or refused (§101 ruling B); either way the delegation did not happen
                    // and the capability is still the sender's.
                    return Err(aborted_send_error());
                }
                Ok(0)
            }
            abi::rendezvous::RECEIVE_CAP => {
                if !cap.rights.allows(Rights::READ) {
                    return Err(Error::NotPermitted);
                }
                let msg = sched::ipc_receive_cap(ep);
                if sched::take_ipc_aborted() {
                    return Err(Error::Gone); // endpoint revoked; the message is a placeholder
                }
                // x1 carries the slot the received capability landed in, or NO_CAP if the message
                // brought none; x2 the second data word (a CALL's, or 0); x3 the sender's badge
                // (milestone 599), 0 when its capability was unbadged. x0 returns the first word.
                // x4 is `abi::notification::BOUND` when the bound notification ended this receive,
                // `abi::rendezvous::REPLY_DELIVERED` when x1 is a CALL's Reply (§245 (a `CALL`
                // server tells a Reply from a delegation)), and 0 otherwise (milestone 151
                // (notification objects)): the one register here no sender can write, so the one a
                // bound server, and every CALL server, tests.
                frame.set_arg(1, msg[1]);
                frame.set_arg(2, msg[2]);
                frame.set_arg(3, msg[3]);
                frame.set_arg(4, msg[4]);
                Ok(msg[0] as i64)
            }

            // Call: send two words and block until replied. The kernel mints a one-shot Reply cap
            // naming us into the server (delivered by its RECEIVE_CAP); we return here only when the
            // server invokes it. See §12 and notes/ipc-naming.md. Sending needs WRITE, like SEND.
            abi::rendezvous::CALL => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                let reply = sched::ipc_call_badged(ep, [a0, a1], badge);
                if sched::take_ipc_aborted() {
                    return Err(aborted_send_error()); // revoked or refused; no call, no reply
                }
                frame.set_arg(1, reply[1]); // r1; r0 returns in x0 below
                // x2: the slot a `REPLY_CAPABILITY` filed in our table, or `NO_CAP` (§255 (each socket is
                // its own capability)). Written on every CALL so a caller reads the kernel's word,
                // never what it left in x2 itself.
                frame.set_arg(2, reply[2]);
                Ok(reply[0] as i64)
            }

            // **Collect a corpse this endpoint supervises** (DECISIONS §32). The method that lets a
            // supervisor reap without holding the authority to *build*: reaping used to mean
            // `MemoryRegion::DESTROY`, which needs WRITE on the region, and WRITE on a region is also what
            // retypes a thread and an address space out of it.
            //
            // READ, not WRITE: the authority to collect a death is the authority to *receive* deaths
            // here, which is what a supervisor holds and what a send-only holder (a peer that can
            // report to this supervisor) deliberately does not. `a0` is the tid the kernel stamped on
            // the death message, and it is authorized *relative to this endpoint* (sched's
            // reap_supervised), so it is a name inside a relationship rather than a global handle.
            abi::rendezvous::REAP => {
                if !cap.rights.allows(Rights::READ) {
                    return Err(Error::NotPermitted);
                }
                rendezvous_reap(ep, a0)
            }

            // **Read one entry of the domain this endpoint supervises** (milestone 126 (the `procps` package)). The view
            // half of what REAP is the control half of, and scoped by the same relationship, so a
            // supervisor sees exactly the children whose deaths would arrive here.
            //
            // READ for the same reason REAP takes READ: the authority to see who may die here is
            // the authority to receive deaths here. **A send-only holder is refused rather than
            // shown an empty domain**, which is the whole point of the method; a monitor that
            // reports nothing because it could not look is the worst failure this tool has, and an
            // empty answer is reserved for a domain that really is empty.
            //
            // `a0` is the cursor: 0 to start, then whatever the last call returned, until a
            // `survey::DONE` comes back. `a1` is the RECORD: which per-thread fact the caller
            // wants. x1 carries the tid and x2 that record's word.
            //
            // **The selector is calef's 2026-09-21 ruling, and it replaces growing this row.** The
            // earlier plan put each new fact in a further return register; he expects a third and a
            // fourth fact, and a mechanism that has to be redesigned at the sixth field is the
            // wrong mechanism at the fourth. Nobody grows a register row: Linux reached 52 fields
            // through a pseudo-file and Zircon 41 topics through a selector. So x0 and x1 are the
            // frame, the same for every record, and only x2 belongs to the record.
            //
            // **`record::STATE` is 0 so that every caller written before the selector keeps
            // working**, having passed 0 into an argument that was then unused. That is a claim
            // about a wire rather than a hope, and it is asserted in `abi`'s own tests.
            //
            // **Why the placement record needs no authority beyond the `ENUMERATE` below**, said
            // here rather than in a decisions file the reader would have to go and find. §150 (how does a
            // thread's CPU time reach userspace?)
            // already weighed that a viewer holding `ENUMERATE` learns something aggregate about
            // threads it cannot otherwise name, and accepted it for a CPU-time counter. A placement
            // is strictly less than what was accepted: one bounded value out of at most 64, written
            // once when the thread started and never again, against a counter that moves
            // continuously and can therefore be differenced into a timing channel. A viewer that
            // can already see a thread's tid and run state learns which of a handful of cores it
            // was put on. **It can never name a thread outside the domain it was handed**; what
            // it can learn about them is one noisy bit per spawn (2026-09-24 security audit):
            // `pick_spawn_target` samples two cores and takes the one with the shorter run queue,
            // and that queue counts every domain's threads, so a placement says which of two
            // random cores was lighter at that instant. That is the counting channel the
            // 2026-08-17 audit recorded, at lower bandwidth, and it is accepted on the same
            // reasoning. The right to look is still the capability, and nothing here widens it.
            abi::rendezvous::SURVEY => {
                // `ENUMERATE`, not `READ`, and the distinction is the method's whole safety
                // argument: `READ` here also unlocks `RECEIVE` and `REAP`, so a viewer granted it
                // could reap a child. A domain names its members and does not act on them, and one
                // bit for three operations cannot say that. See `Rights::ENUMERATE`.
                if !cap.rights.allows(Rights::ENUMERATE) {
                    return Err(Error::NotPermitted);
                }
                let (next, tid, word) = sched::survey_supervised(ep, a0, a1)?;
                frame.set_arg(1, tid);
                frame.set_arg(2, word);
                Ok(next as i64)
            }

            // Body extracted, the pattern of milestone 156 (`syscall_entry`'s measured size is every method
            // combined): minting a badge is spawn-time delegation, never a step of the IPC round
            // trip, so it stays out of `invoke`'s own bytes (milestone 599).
            abi::rendezvous::BADGE => rendezvous_badge(ep, cap.rights, badge, a0),
            _ => Err(Error::BadMethod),
        },

        // A one-shot reply to a specific caller (§12). Minted by the kernel at a CALL rendezvous,
        // named by the caller's tid, consumed on use.
        Object::Reply(tid) => match method {
            abi::reply::REPLY => {
                // Minted WRITE-only, so a narrowed derivative could not answer; and minted without
                // GRANT, so it could not have been delegated here in the first place.
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                sched::ipc_reply(tid, [a0, a1]);
                // One-shot: consume it, so a second reply is NoSuchSlot and the caller cannot be
                // answered twice. This is the guarantee a pre-wired reply endpoint cannot make.
                let _ = sched::delete_current_cap(slot);
                Ok(0)
            }
            // Every other method, `REPLY_CAPABILITY` and `BadMethod` alike, is decided out of line.
            _ => reply_other(slot, method, a0, a1, a2),
        },

        // Another process's memory, under construction (19b). WRITE on the address space cap is the
        // authority to shape it; the frame's own rights gate what kind of mapping, exactly as
        // frame::MAP; the va gate is the proved paging::is_user_page_va, as everywhere.
        Object::AddressSpace(name) => address_space_invoke(frame, cap, name, method, [a0, a1, a2]),

        // A thread under construction (19c.3). WRITE on the TCB cap is the authority to shape
        // and start it. Every method refuses a thread that is not an embryo, in the scheduler.
        Object::ThreadControlBlock(tid) => {
            thread_control_block_invoke(frame, cap.rights, tid, method, [a0, a1, a2])
        }

        // A notification (milestone 151, DECISIONS §101 (notification objects)). Extracted and `#[inline(never)]` for
        // `memory_region_map`'s reason: none of these is a step of the IPC round trip
        // `script/fastpath-footprint` bounds, so the new arm costs the flat `syscall_entry` one
        // call rather than four method bodies.
        Object::Notification(id) => notification_invoke(cap.rights, id, method, a0),

        // A timer (milestone 106, DECISIONS §147 (a timer a userspace service cannot hold)). Out of line for the notification arm's reason:
        // neither method is a step of the IPC round trip the fastpath footprint bounds.
        Object::Timer(id) => timer_invoke(cap.rights, id, method, a0, a1, a2),

        // A process (milestone 812): its one method is the supervisor's kill. Out of line for the
        // timer arm's reason.
        Object::Process(pid) => process_invoke(cap.rights, pid, method),

        // The reboot object (milestone 805 (`reboot` at the prompt), DECISIONS §251 (restarting the
        // machine is a kernel object the progenitor hands out)). Out of line for the timer arm's
        // reason, and colder than any of them: a successful call never returns. Holding the
        // capability is the whole authority, so no rights bit is asked for (§251, "The method").
        Object::Reboot => reboot_invoke(method),

        Object::MemoryRegion(region) => match method {
            // Body extracted (milestone 156): all five `MemoryRegion` methods are memory-management
            // administration a spawner runs while building a process, never a step of the IPC
            // round trip `script/fastpath-footprint` bounds, so each stays out of `invoke`'s own
            // bytes and `#[inline(never)]` on purpose. See `address_space_list`, the pattern this copies.
            abi::memory_region::MAP => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                memory_region_map(region, a0)
            }
            abi::memory_region::RETYPE_OBJ => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                retype::memory_region_retype_obj(region, a0, a1, a2)
            }
            abi::memory_region::RETYPE => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                memory_region_retype(region, a0)
            }
            abi::memory_region::SPLIT => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                memory_region_split(cap, region, a0)
            }
            abi::memory_region::DESTROY => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                memory_region_destroy(region)
            }
            // What the region was spent on (milestone 126's `free`, DECISIONS §225 (`free` sees the machine and your share) part 1). Under
            // `ENUMERATE` alone, `address_space::LIST`'s rule one object type over: learning what
            // a budget went to is not the authority to spend it. An unknown record is refused
            // before the region is looked up, `SURVEY`'s order.
            abi::memory_region::USAGE => memory_region_usage(cap, region, a0),
            _ => Err(Error::BadMethod),
        },

        Object::PageFrame(phys, count) => match method {
            // Body extracted (milestone 156), the same reason as `MemoryRegion`'s five methods:
            // neither `MAP` nor `REVOKE` is a step of the IPC round trip, so both move out of
            // `invoke`'s own bytes. `MAP`'s rights check is data-dependent (branches on `a1`), so
            // it lives inside `page_frame_map` rather than at the call site here, unlike the fixed
            // single-right checks the other extractions keep in `invoke`.
            //
            // §102 (2026-08-20): `count` rides on the capability, not on the syscall's arguments,
            // so `MAP`'s and `REVOKE`'s wire shape is exactly what it was before the object could
            // name a run. A single-page frame (`count: 1`) runs each loop below once.
            abi::page_frame::MAP => page_frame_map(slot, a0, a1, a2),
            abi::page_frame::REVOKE => {
                if !cap.rights.allows(Rights::GRANT) {
                    return Err(Error::NotPermitted);
                }
                page_frame_revoke(phys, count.get())
            }
            abi::page_frame::SLICE => page_frame_slice(slot, a0, a1),
            _ => Err(Error::BadMethod),
        },

        // A device's MMIO page is almost passive: it is handed to MAP_INTO as the page to map
        // (19d.2), and since milestone 23 it answers exactly one invocation, `REVOKE`.
        Object::DeviceFrame(phys) => match method {
            // **Take the registers back from everyone else** (DECISIONS §41). The step live
            // replacement needs between tearing one driver down and endowing the next, so that a
            // device never has two owners. Needs `GRANT`, the same rule `PageFrame::REVOKE` uses: you
            // were trusted to lend the device on, so you may take it back.
            //
            // Unlike a frame revoke this **spares the invoker's own** capability and mapping, and
            // it must: only the kernel mints a `DeviceFrame`, and it does so once at boot, so a
            // symmetric revoke would make the device unreachable for the rest of the machine's
            // life. `revoke::revoke_device_from_others` carries the full argument.
            abi::page_frame::REVOKE => {
                if !cap.rights.allows(Rights::GRANT) {
                    return Err(Error::NotPermitted);
                }
                crate::revoke::revoke_device_from_others(phys);
                Ok(0)
            }
            _ => Err(Error::BadMethod),
        },

        Object::Virtio(id) => {
            if !cap.rights.allows(Rights::WRITE) {
                return Err(Error::NotPermitted);
            }
            virtio_invoke(id, method, a0, a1)
        }

        Object::Irq(intid) => match method {
            // Body extracted (milestone 156). `WAIT` does block on the same `sched::ipc_receive`
            // the `Rendezvous` fastpath uses, but the caller here is a driver waiting on a device
            // interrupt, not the IPC round trip this gate bounds, so it moves out of `invoke`'s
            // own bytes with the rest. See `memory_region_map`'s doc comment for the full reasoning.
            abi::irq::WAIT => {
                if !cap.rights.allows(Rights::READ) {
                    return Err(Error::NotPermitted);
                }
                irq_wait(frame, intid)
            }
            abi::irq::ACK => {
                if !cap.rights.allows(Rights::READ) {
                    return Err(Error::NotPermitted);
                }
                irq_ack(intid)
            }
            _ => Err(Error::BadMethod),
        },

        // A port range is enforced at the context switch (the TSS I/O bitmap), so like a
        // `DeviceFrame` it is almost passive on the syscall path: it answers exactly one invocation,
        // `REVOKE`, the take-back a live driver replacement needs (DECISIONS §121, milestone 299).
        // Extracted, `#[inline(never)]`, for `memory_region_map`'s reason: a rare administrative
        // method has no business growing the flat `syscall_entry` footprint the IPC round trip is
        // measured against (`script/fastpath-footprint`). The variant is `x86_64`-only, so on the
        // other two architectures this match is exhaustive without it and their dispatcher is
        // unchanged.
        #[cfg(target_arch = "x86_64")]
        Object::PortRange(base, count) => port_range_invoke(cap.rights, base, count, method),
    }
}

/// **The first proofs over the kernel's own source** (milestone 193).
///
/// Every other `#[kani::proof]` in this tree sits in a crate under `crates/`, because until this
/// milestone `script/verify` could not reach `kernel/src` at all: its own header says
/// `cargo kani -p <crate>` never compiles the kernel. Milestone 191 measured what that cost, and
/// the answer was every defect the corpus actually had. This module is the other side of that
/// door, and `notes/kernel-proofs.md` is what a reader should open first, because a proof over
/// this crate carries stubs that a proof over a pure crate does not.
///
/// # What is stubbed, and therefore what these proofs do NOT say
///
/// The list is short and it is exhaustive, which is the only reason it is worth writing down:
///
/// - **Global assembly is skipped**, by `--ignore-global-asm` in `script/verify`. That is the boot
///   entry, the vector table and the context switch on all three architectures. Nothing below
///   reaches any of it, and nothing below claims anything about it.
/// - **`asm!` is an unsupported construct to Kani**, so every function that reaches one is
///   unverifiable rather than verified: all of `kernel/src/arch/`, plus `cpu.rs` once and `user.rs`
///   twice. If a harness ever calls into that code Kani reports the construct rather than proving
///   past it, which is the failure direction we want.
/// - **The panic handler is absent under `cfg(kani)`** (`panic.rs` says why: Kani links `std`,
///   which already defines the lang item). Nothing here says what the kernel does after a panic.
///
/// The functions proved below touch none of that. They are integer arithmetic on words a user
/// program chose, which is exactly where milestone 142's review found four defects by reading.
#[cfg(kani)]
mod proofs;

#[cfg(test)]
mod tests;
