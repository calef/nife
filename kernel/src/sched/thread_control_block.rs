//! **Thread control blocks**: creating one from a region, inserting capabilities before it starts,
//! starting it, and the binders that name what a thread is bound to. Configuring an embryo is
//! [`configure`].
//!
//! Cut out of `sched.rs` unchanged by milestone 840 (the scheduler file is split along its seams).
//!
//! Name: provisional (milestone 840's lane, 2026-10-11 UTC; §271 (the scheduler file splits into
//! submodules of `sched`)).

use super::*;

/// **Retype a TCB out of `region`** (milestone 19c.3): an embryo thread, page-resident in a
/// page of the creator's own untyped, in the thread table but in no queue and not runnable.
/// Returns its `ThreadId` (what an `Object::ThreadControlBlock` capability carries) or `None` if the region is out of
/// budget or the table is full.
pub fn create_thread_control_block(region: u64) -> Option<ThreadId> {
    let page =
        crate::memory_region::retype_object_page(region, crate::memory_region::ObjectKind::Thread)?;
    let mut guard = IPC_TABLES.lock();
    let sched = guard.as_mut()?;
    let name = sched.threads.insert_from_page(page, |tid| {
        let mut t = Thread::embryo();
        t.id = tid;
        // Remember which region paid for this TCB. It is the region an rendezvous reap reclaims
        // (DECISIONS §32), and here is the only point where the answer is known rather than
        // inferred: the caller named it, and nothing afterwards can tell us as reliably.
        t.thread_control_block_region = Some(region);
        t
    });
    // On a full table the page stays the region's (spend-only); nothing to recycle. The region
    // is already pinned by retype_object_page, so its destroy is refused regardless.
    name
}

/// **Install a capability into an embryo's capability table** (milestone 19c.3): the child's initial
/// authority, granted one slot at a time before it runs. Refuses a non-embryo. Returns the child
/// slot the capability landed in.
///
/// `target` is `None` for first-free placement (the original behaviour) or `Some(slot)` to place
/// the capability in a specific free slot, which a supervisor uses to put a child's supervision
/// rendezvous in the reserved fault slot (milestone 22). A targeted insert into an occupied or
/// out-of-range slot is `OutOfMemory`, so the reservation cannot be quietly overwritten.
///
/// **For a capability the kernel mints, not one read from a table**: `CAP_INSERT` endows an embryo
/// with a copy of one the spawner holds, and that goes through
/// [`thread_control_block_delegate_cap`] ([`Delegation`] has why).
pub fn thread_control_block_insert_cap(
    tid: ThreadId,
    capability: crate::cap::Cap,
    target: Option<u64>,
) -> Result<u64, abi::Error> {
    thread_control_block_insert_from(tid, target, |_, _| Ok(capability))
}

/// **`ThreadControlBlock::CAP_INSERT`'s body: endow an embryo with a narrowed copy of a capability
/// the running thread holds**, the source read under the `IPC_TABLES` hold that files the copy
/// ([`Delegation`]). Name provisional.
pub(crate) fn thread_control_block_delegate_cap(
    tid: ThreadId,
    delegation: Delegation,
    target: Option<u64>,
) -> Result<u64, abi::Error> {
    thread_control_block_insert_from(tid, target, |sched, current| {
        let caps = sched
            .threads
            .capabilities(current)
            .ok_or(abi::Error::NoSuchSlot)?;
        delegation.derive(&caps.lock())
    })
}

/// The body both share. `source` runs first, so its answer comes before the embryo's, the order the
/// syscall layer answered in when it read the source itself. Its table lock is released before the
/// embryo's is taken: two tables are never held at once (`sync::rank::CAPABILITY_TABLE`).
pub(super) fn thread_control_block_insert_from(
    tid: ThreadId,
    target: Option<u64>,
    source: impl FnOnce(&IpcTables, ThreadId) -> Result<crate::cap::Cap, abi::Error>,
) -> Result<u64, abi::Error> {
    let mut guard = IPC_TABLES.lock();
    let sched = guard.as_mut().ok_or(abi::Error::NoSuchSlot)?;
    let capability = source(sched, current_thread_id())?;
    let (t, caps) = sched
        .threads
        .get_mut_with_capabilities(tid)
        .ok_or(abi::Error::NoSuchSlot)?;
    if t.handshake.state != State::Embryo {
        return Err(abi::Error::WrongObject);
    }
    let landed = match target {
        None => caps.lock().insert(capability),
        Some(slot) => caps.lock().insert_at(slot, capability),
    }
    .map_err(|_| abi::Error::OutOfMemory)?;
    // **The one choke point where a port capability enters a thread** (milestone 299): the boot's
    // own child builder and the progenitor's `ThreadControlBlock::CAP_INSERT` both endow an embryo
    // through here, so caching the grant here is what makes the context switch's port-grant read
    // one field read instead of a capability-table scan. Set only on the `x86_64` build that
    // enforces it, and only after the insert succeeded, so a full table leaves the grant untouched.
    //
    // A thread that is handed more than one port range keeps only the last, which every real
    // consumer is well within (a console driver holds exactly one, for COM1); see this milestone's
    // BUGS. A `PortRange` delegated to an *already running* thread by `SEND_CAP` is likewise not
    // cached here, because no consumer does that and the enforcement is a creation-time grant, the
    // same posture `cycle_counter_grant` takes.
    //
    // **A `PortRange` without `WRITE` grants nothing** (milestone 768 (provisional), calef's ruling
    // of 2026-10-05 UTC on the second outsider pass, DECISIONS §121 (what a device capability is
    // when the device has no page: x86 port I/O)). The TSS I/O bitmap has one bit per port and
    // cannot permit `in` without `out`, so there is no honest read-only grant to install; READ
    // cannot be granted without WRITE, so WRITE is the right that opens the ports and a capability
    // without it opens none. The capability still lands in the table (REVOKE and delegation are
    // unchanged); only the cached grant is withheld.
    #[cfg(target_arch = "x86_64")]
    if let crate::cap::Object::PortRange(base, count) = capability.object
        && capability.rights.allows(crate::cap::Rights::WRITE)
    {
        t.port_range_grant = Some((base, count));
    }
    Ok(landed)
}

/// **Start an embryo** (milestone 19c.3): the no-start-before-whole gate, then make it runnable.
/// Refuses a TCB that is not an embryo, or one with no bound address space or no entry set: a
/// half-built thread must never run. On success the thread gets its kernel stack and entry
/// context and joins this core's run queue.
pub fn start_thread_control_block(tid: ThreadId, args: [u64; 3]) -> Result<(), abi::Error> {
    // **The kernel stack is built before `IPC_TABLES` is taken** (2026-10-04), for the reason the
    // reaper frees one after releasing it: building it maps six pages and allocates their frames,
    // and every other core's IPC and capability lookups waited behind that while it ran under the
    // lock (notes/job-mix/null-syscall-under-load.md). Declared before the guard, so a refusal below
    // releases the lock first and frees the unused stack afterwards. The refusals keep their order:
    // a missing stack is still reported only after the embryo checks have passed.
    let stack = crate::thread::KernelStack::new();
    let mut guard = IPC_TABLES.lock();
    let sched = guard.as_mut().ok_or(abi::Error::NoSuchSlot)?;
    // A member of a process that has ended may not start (milestone 812): `Gone`.
    if sched
        .threads
        .get(tid)
        .is_some_and(|t| process::refuses_start(sched, t))
    {
        return Err(abi::Error::Gone);
    }
    let (t, capabilities) = sched
        .threads
        .get_mut_with_capabilities(tid)
        .ok_or(abi::Error::NoSuchSlot)?;

    if t.handshake.state != State::Embryo {
        return Err(abi::Error::WrongObject); // already started (or not a TCB)
    }
    // WHOLE, or refuse: a bound address space and an entry point. Either missing is a half-built
    // thread, and starting it would drop to EL0 with no low half or no code.
    if t.space.is_none() || t.entry.0 == 0 {
        return Err(abi::Error::NotPermitted); // configure it first
    }

    // **The spawn-slot convention** (milestone 22, DECISIONS §26). If the reserved fault slot holds
    // a Rendezvous capability, this thread is supervised: record it as the fault target
    // and consume the slot, so the child cannot forge fault messages on it (the kernel stays the
    // only sender on this path, §26.5). Supervision is fixed here, at spawn, and never changes.
    //
    // **The capability's badge is the child's label, and it is kept** (milestone 105, DECISIONS
    // §148 as amended 2026-10-04, ruling R3). The builder sets it with `rendezvous::BADGE` before
    // the insert, and the kernel delivers it with the death message so a supervisor of several
    // children can say which one died. Consuming the slot below is also what keeps it from the
    // child: no capability it holds carries it, and nothing else reports it.
    let mut table = capabilities.lock();
    if let Ok(fault_cap) = table.get(abi::fault::FAULT_EP_SLOT)
        && let crate::cap::Object::Rendezvous(ep, label) = fault_cap.object
    {
        t.fault_ep = Some(ep);
        t.fault_label = label;
        let _ = table.delete(abi::fault::FAULT_EP_SLOT);
    }
    drop(table);

    t.start_args = args; // the child's x0, x1, x2 (19d/19e)
    let Some(stack) = stack else {
        return Err(abi::Error::OutOfMemory); // no kernel stack to be had
    };
    t.arm_for_start(stack);
    t.handshake.state = State::Ready;
    // Placement is the power of two choices (DECISIONS §28), the same as `spawn`: a freshly started
    // user thread lands on the lighter of two sampled cores rather than always the starter's, so a
    // process that spawns a pipeline does not pile it all onto one core. `place_on` enqueues locally
    // or hands the thread to the target's inbox; the SGI that makes a remote target pick it up goes
    // out after IPC_TABLES is released.
    // The decision, once, under the lock. Asking `target != cpu::id()` again after `drop(guard)`
    // unmasks interrupts first, so this thread can be stolen onto `target` in between and the
    // second answer skips the SGI the first one owed (see `place_on`).
    let target = pick_spawn_target();
    // Record it on the thread, so a supervisor can survey it (`abi::survey::record::PLACEMENT`).
    // This is the path every *user* thread takes, so it is the one a survey actually reads: a
    // supervised thread is always one a `START` put on a core. `t` is still borrowed here, which is
    // why this is before `take_token` re-borrows `sched`. The token it takes is the one the thread
    // table minted when this embryo was inserted; `Embryo -> Ready` above happens once, so it is
    // taken once.
    t.placement = target as u8;
    let member_of = t.process;
    let remote = place_on(target, take_token(sched, tid));
    if let Some(pid) = member_of {
        process::count_start(sched, pid);
    }
    drop(guard);
    if let Some(target) = remote {
        crate::arch::irq::send_reschedule(target);
    }
    Ok(())
}

/// Hand the current thread an address space, and install it.
///
/// The space goes into the address-space registry, bound to this thread (§249: the registry owns
/// every space), and the thread keeps the copy the context switch reads. From here the reaper takes
/// it out of the registry and drops it when the thread dies, and every context switch back to this
/// thread re-installs it.
///
/// Returns the space's name in the registry, which an address-space capability carries, for a
/// spawn that hands a process its own space (`user::run_with_own_space`).
pub fn adopt_address_space(space: crate::user::AddressSpace) -> u64 {
    let current = current_thread_id();
    // Before `IPC_TABLES`: the registry's lock ranks above it. The thread is live and running, so
    // the region sweep cannot take the entry in the moment before the thread holds its copy.
    let bound = crate::user::register_bound_address_space(space, current);

    // **The current-CPU page is written here as well as at switch-in**, and the test that found
    // this is the reason it is not obvious. `schedule()` writes the page of the thread it is
    // switching TO, which covers every thread that has a space before it first runs. A thread that
    // adopts one *while already running* (the `sched::spawn(|| run(image, ...))` shape every
    // kernel-side spawn uses) was switched in before it had a space at all, so its first user
    // instruction ran against a page nobody had touched and `current_cpu` answered `None` until the
    // next preemption. This is that thread's switch-in, arriving late; `cpu::id()` is the core it
    // is standing on right now, which is exactly what the switch would have written.
    bound.publish_current_cpu(cpu::id() as u64);

    {
        let mut guard = IPC_TABLES.lock();
        let sched = guard.as_mut().expect("no scheduler");
        sched
            .threads
            .get_mut(current)
            .expect("no current thread")
            .space = Some(bound);
    }

    // SAFETY: `bound` is the copy of a space the registry now owns, bound to the *current* thread.
    // The current thread is the one executing this line, so it is on a CPU and cannot be reaped,
    // and the space is dropped only once its thread can never be switched in again.
    unsafe { crate::arch::mmu::switch_user_root(bound.ttbr0()) };
    bound.name()
}

/// **Can the thread a bound address space names still run?** (§249; name provisional.) What the
/// region sweep asks before it takes a bound space: `user::reap_address_spaces_in_region` has the
/// rule each answer feeds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Binder {
    /// The name no longer resolves: the thread was reaped. Generational, so it is never a newer
    /// thread in the same slot.
    Gone,
    /// `Dead` or `Finished`, and no core is standing on its stack: it will never be switched in
    /// again, so nothing will install its root.
    Corpse,
    /// Anything else, an embryo included (a `START` would run it), and a corpse still on a core.
    CanRun,
}

/// **Answer [`Binder`] questions under one hold of `IPC_TABLES`**: `f` is given the question and may
/// ask it as often as it likes. One hold rather than one per thread, because the sweep asks about
/// every bound space on every scan. Taken under the address-space registry's lock (61 above 60). A
/// scheduler that does not exist yet has bound nothing, and answers `CanRun` so nothing is freed on
/// a guess.
#[cfg_attr(not(feature = "system_tests"), allow(dead_code))] // the region sweep uses `with_holders`
pub fn with_binders<R>(f: impl FnOnce(&dyn Fn(ThreadId) -> Binder) -> R) -> R {
    let guard = IPC_TABLES.lock();
    let Some(sched) = guard.as_ref() else {
        return f(&|_| Binder::CanRun);
    };
    f(&|tid| binder_of(sched, tid))
}

/// [`with_binders`]'s answer for one thread, for `process::with_holders` too.
pub(super) fn binder_of(sched: &IpcTables, tid: ThreadId) -> Binder {
    match sched.threads.get(tid) {
        None => Binder::Gone,
        Some(t)
            if matches!(t.handshake.state, State::Dead | State::Finished)
                && !t.handshake.on_cpu =>
        {
            Binder::Corpse
        }
        Some(_) => Binder::CanRun,
    }
}
