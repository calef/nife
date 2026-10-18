//! `child_thread_tests`: a process builds, starts and runs a child thread, reclaim frees the
//! child's regions, spawn-to-reap repeats without leaking, and a process is composed from two
//! capabilities.
//!
//! Name: provisional, minted 2026-10-12 (UTC) by the lane for milestone 873 (the catch-all
//! userspace test file is split by topic).

use super::*;
use crate::arch::timer;
use crate::sched;

/// **Milestone 19c.3, the whole point: one process builds and starts another, and it runs.**
/// The kernel drives the four verbs the way the progenitor eventually will: retype an address space and
/// a TCB, map a code page (containing a hand-assembled EL0 stub) and a stack into the space,
/// insert a report rendezvous into the child's capability table, configure the TCB (entry, stack, space),
/// and START it. The child, code no wiring wrote and a thread no `spawn` created, drops to
/// EL0, invokes the capability it was granted to SEND a word home, and exits. Receiving that
/// word proves every verb: the retype, the maps, the cap insert, the configure, the start,
/// and a real EL0 thread built from parts.
#[test_case]
fn a_process_can_build_start_and_run_a_child_thread() {
    const CODE_VA: u64 = address_space_map::IMAGE_BASE;
    const STACK_VA: u64 = address_space_map::STACK_TOP_PAGE;
    // The child's program: SEND(slot 0, rendezvous::SEND, REPORT_WORD) then EXIT, nine
    // instructions, with the child's first granted cap in slot 0. This file used to carry three
    // separate aarch64 copies of it; `supervision_tests` already keeps one pair (aarch64 and
    // RISC-V) for its own children, so all three now share that pair. Same shape, one definition,
    // and the tests below run on either machine.
    let code = super::supervision_tests::REPORT_STUB;
    let expect_word = super::supervision_tests::REPORT_WORD;

    // The child's address space, and a region to carve its code and stack frames from.
    let as_region = crate::memory_region::create(8).expect("no address space region");
    let aspace = user_address_space_create(as_region).expect("no aspace");
    let frames_region = crate::memory_region::create(2).expect("no frame region");

    let code_phys = code_page(frames_region, code);
    user_address_space_map(
        aspace,
        CODE_VA,
        code_phys,
        Flags::user_code(),
        crate::revoke::PageMapSource::NoCapability,
    )
    .expect("map code");

    let stack_phys = crate::memory_region::retype_page(frames_region).expect("no stack frame");
    user_address_space_map(
        aspace,
        STACK_VA,
        stack_phys,
        Flags::user_data(),
        crate::revoke::PageMapSource::NoCapability,
    )
    .expect("map stack");

    // The child's one authority: WRITE on a report rendezvous, so it can SEND but not receive.
    let endpoints = crate::memory_region::create(1).expect("no endpoint region");
    let report = crate::sched::create_rendezvous_from(endpoints).expect("no report rendezvous");
    let report_cap = crate::cap::rendezvous_cap(
        report,
        crate::cap::Rights::WRITE.union(crate::cap::Rights::GRANT),
    );

    // Build the thread from parts.
    let thread_control_block_region = crate::memory_region::create(2).expect("no tcb region");
    let tid =
        crate::sched::create_thread_control_block(thread_control_block_region).expect("no tcb");
    let slot =
        crate::sched::thread_control_block_insert_cap(tid, report_cap, None).expect("cap insert");
    assert_eq!(
        slot, 0,
        "the child's first cap must land in slot 0 (the code assumes it)"
    );

    // Not before it is whole: START must refuse an unconfigured embryo.
    assert!(
        crate::sched::start_thread_control_block(tid, [0; 3]).is_err(),
        "START ran a half-built thread (no address space, no entry)",
    );

    crate::sched::configure_thread_control_block(
        tid,
        CODE_VA,
        STACK_VA + page_frames::FRAME_SIZE,
        aspace,
    )
    .expect("configure");
    crate::sched::start_thread_control_block(tid, [0; 3]).expect("start");

    // And starting twice must refuse: it is no longer an embryo.
    assert!(
        crate::sched::start_thread_control_block(tid, [0; 3]).is_err(),
        "START ran a thread that was already running",
    );

    let got = crate::sched::ipc_receive(report)[0];
    assert_eq!(
        got, expect_word,
        "the child never reported: a built-from-parts thread did not reach EL0 and run",
    );
    crate::sched::reclaim_region(endpoints).expect("the endpoint region did not come back");
}

/// **Object revocation, piece 3: a started thread and its bound address space are reclaimed
/// after it exits.** Build and start a child as above, but carve its code and stack from the
/// *same* region as its address space, so one region holds the child's whole world (root,
/// tables, code, stack) and its TCB is in another. Once the child has run, exited, and been
/// reaped, both regions reclaim and the free-frame count returns *exactly* to baseline. The bound
/// address space died with the thread (the `Drop` chain), leaving its region object-free for
/// `reclaim_region` to unpin and free.
///
/// # This test used to probe the refusal first, and that probe was killing its own child
///
/// Milestone 72. It opened by asserting `reclaim_region(thread_control_block_region).is_err()` while the child was
/// still runnable, over a comment reading "the refusal leaves the region untouched". That comment
/// went stale when DECISIONS §16 was amended: a refused reclaim is **not** passive, it *arms the
/// kill* on every live thread in the region so the owner's retry can tear a runaway down (§24's
/// `^C` escalation depends on it). So the probe marked this child `killed`, and `schedule()`
/// converts a killed thread to a corpse at its next preemption. Win the race and the child reaches
/// its `SEND` first and the test passes; lose it and the child is reaped without ever sending, the
/// `ipc_receive` below never returns, and the machine goes fully idle: the intermittent lost-wakeup
/// hang that kept `cpu-matrix` red on branches that could not have caused it.
///
/// Proved by widening the window rather than by waiting for the race: a call-free delay loop in
/// front of `REPORT_STUB` guarantees a preemption before the `SEND`, and with the probe present
/// that hangs the watchdog on **both** ISAs, first run, every run. It is not a RISC-V defect; the
/// riscv64 leg simply lost the race more often. With the probe gone the same widened child passes.
///
/// The refusal itself is not lost coverage: `force_kill_tests` proves refuse-then-arm-then-reclaim
/// directly, on a runaway that is *meant* to die, which is the only subject a destructive probe can
/// honestly be pointed at.
#[test_case]
fn reclaim_frees_a_started_then_exited_childs_regions() {
    const CODE_VA: u64 = address_space_map::IMAGE_BASE;
    const STACK_VA: u64 = address_space_map::STACK_TOP_PAGE;
    // SEND(slot 0, rendezvous::SEND, REPORT_WORD) then EXIT, the shared stub (see the test above).
    let code = super::supervision_tests::REPORT_STUB;
    let expect_word = super::supervision_tests::REPORT_WORD;

    // The report rendezvous lives in a region of its own, reclaimed last, so it is not part of what
    // this test accounts for: the accounting below is per-region, and the frames it spends cannot
    // be confused with the child's.
    let endpoints = crate::memory_region::create(1).expect("no endpoint region");
    let report = crate::sched::create_rendezvous_from(endpoints).expect("no report rendezvous");

    // The child's whole address space in one region: root, tables, code, and stack.
    let as_region = crate::memory_region::create(8).expect("no address space region");
    let aspace = user_address_space_create(as_region).expect("no aspace");

    let code_phys = code_page(as_region, code);
    user_address_space_map(
        aspace,
        CODE_VA,
        code_phys,
        Flags::user_code(),
        crate::revoke::PageMapSource::NoCapability,
    )
    .expect("map code");

    let stack_phys = crate::memory_region::retype_page(as_region).expect("no stack frame");
    user_address_space_map(
        aspace,
        STACK_VA,
        stack_phys,
        Flags::user_data(),
        crate::revoke::PageMapSource::NoCapability,
    )
    .expect("map stack");

    let report_cap = crate::cap::rendezvous_cap(
        report,
        crate::cap::Rights::WRITE.union(crate::cap::Rights::GRANT),
    );
    let thread_control_block_region = crate::memory_region::create(2).expect("no tcb region");
    // The two runs this test is responsible for, named while their regions are alive. A
    // machine-wide free-frame delta could not survive this window: the child runs on whichever
    // core §28 (SMP placement: two random choices at spawn) placed it on, the reaper frees its
    // address space from that core's switch, and a neighbouring test's own late teardown lands
    // here too. These ten frames cannot be moved by any of that. See `testing::RegionRun` and
    // notes/load-sensitive-assertions.md.
    let as_run = crate::testing::RegionRun::of(as_region);
    let tcb_run = crate::testing::RegionRun::of(thread_control_block_region);
    let tid =
        crate::sched::create_thread_control_block(thread_control_block_region).expect("no tcb");
    crate::sched::thread_control_block_insert_cap(tid, report_cap, None).expect("cap insert");
    crate::sched::configure_thread_control_block(
        tid,
        CODE_VA,
        STACK_VA + page_frames::FRAME_SIZE,
        aspace,
    )
    .expect("configure");
    crate::sched::start_thread_control_block(tid, [0; 3]).expect("start");

    // **Nothing may touch `thread_control_block_region` between here and the child's report.** A refused
    // `reclaim_region` arms §16's kill on the child and dooms it; see this test's own note above.

    // Let it run: it SENDs the word and exits. Receiving proves it reached EL0.
    let got = crate::sched::ipc_receive(report)[0];
    assert_eq!(got, expect_word, "the child never reported");

    // Let the reaper collect the now-Finished child. A Finished thread is removed when its own
    // core switches away from it, and DECISIONS §28's placement can have put this child on
    // ANOTHER core, so yielding on THIS core cannot make that happen: a hundred cheap yields
    // complete long before the remote core's next timer tick. So wait on the clock, not on a
    // yield count. Still a leak trap rather than a masked failure: a child that is never reaped
    // times out and fails; only cross-core reap lag is tolerated.
    //
    // **And wait on this child, not on a headcount.** This asked whether `thread_count()` had
    // returned to a baseline sampled at the top of the test, which is the size of the WHOLE
    // thread table: the previous test's processes are still tearing down at that instant, so the
    // baseline was a number the system would move on its own, and the wait was really waiting
    // for everything else to hold still. It failed exactly that way once on RISC-V, where the
    // slower machine leaves more teardown in flight. `is_thread_present` asks the question the test
    // means. The whole history here is a wait that keeps being written against something wider
    // than the property: it was a yield count until §28's scattering broke it, then a
    // clock-bounded headcount until this. A sibling wait below had the same defect.
    assert!(
        wait_for(|| !crate::sched::is_thread_present(tid)),
        "the exited child was never reaped",
    );

    // Both regions reclaim now: the TCB's, and the address space's (its bound space died with
    // the thread, so the region is object-free, needing only unpin and free).
    crate::sched::reclaim_region(thread_control_block_region)
        .expect("reclaim the TCB region after exit");
    crate::sched::reclaim_region(as_region).expect("reclaim the address-space region after exit");

    tcb_run.assert_returned("the child's TCB region did not give its frames back");
    as_run.assert_returned("the child's address-space region did not give its frames back");
    crate::sched::reclaim_region(endpoints).expect("the endpoint region did not come back");
}

/// **Spawn-to-reap repeats without leaking: the whole milestone's payoff.** Build, start, run,
/// exit, reap, and reclaim a region-backed EL0 child, in a loop. Every iteration returns the
/// free-frame count to the same baseline, and the region slots are reused (generational), so the
/// loop neither leaks memory nor exhausts the region table. This is the property "spawn's
/// prerequisite" was always about: not retype (that had shipped), but reclamation, so a workload
/// can come and go over and over. A few iterations under TCG is enough to catch any per-cycle
/// leak; the real magnitudes wait on the EL0 `lat_proc` benchmark.
#[test_case]
fn spawn_to_reap_repeats_without_leaking() {
    const CODE_VA: u64 = address_space_map::IMAGE_BASE;
    const STACK_VA: u64 = address_space_map::STACK_TOP_PAGE;
    let code = super::supervision_tests::REPORT_STUB;
    let expect_word = super::supervision_tests::REPORT_WORD;

    let endpoints = crate::memory_region::create(1).expect("no endpoint region");
    let report = crate::sched::create_rendezvous_from(endpoints).expect("no report rendezvous");

    for round in 0..6 {
        let as_region = crate::memory_region::create(8).expect("address space region");
        let aspace = user_address_space_create(as_region).expect("aspace");
        let code_phys = code_page(as_region, code);
        user_address_space_map(
            aspace,
            CODE_VA,
            code_phys,
            Flags::user_code(),
            crate::revoke::PageMapSource::NoCapability,
        )
        .expect("map code");
        let stack_phys = crate::memory_region::retype_page(as_region).expect("stack frame");
        user_address_space_map(
            aspace,
            STACK_VA,
            stack_phys,
            Flags::user_data(),
            crate::revoke::PageMapSource::NoCapability,
        )
        .expect("map stack");

        let report_cap = crate::cap::rendezvous_cap(
            report,
            crate::cap::Rights::WRITE.union(crate::cap::Rights::GRANT),
        );
        let thread_control_block_region = crate::memory_region::create(2).expect("tcb region");
        // This round's own ten frames, by name. The leak this test hunts is a per-cycle one, so
        // asking per round about the pages that round allocated is the question; the machine's
        // free total was the question the assertion used to ask, and it could be moved by any
        // other core in a window that contains a child running, a remote reap and two reclaims.
        let as_run = crate::testing::RegionRun::of(as_region);
        let tcb_run = crate::testing::RegionRun::of(thread_control_block_region);
        let tid =
            crate::sched::create_thread_control_block(thread_control_block_region).expect("tcb");
        crate::sched::thread_control_block_insert_cap(tid, report_cap, None).expect("cap insert");
        crate::sched::configure_thread_control_block(
            tid,
            CODE_VA,
            STACK_VA + page_frames::FRAME_SIZE,
            aspace,
        )
        .expect("configure");
        crate::sched::start_thread_control_block(tid, [0; 3]).expect("start");

        assert_eq!(
            crate::sched::ipc_receive(report)[0],
            expect_word,
            "round {round}: the child never reported"
        );
        // On the clock, not on yields, and on THIS child rather than on a headcount, both for
        // the reasons spelled out in the test above: §28 can place the child on another core and
        // only that core's switch reaps it, and `thread_count()` is the size of the whole table,
        // so waiting for it to return to a baseline is waiting for the rest of the system.
        // A lagging reap here would surface as the reclaim below refusing a live thread.
        assert!(
            wait_for(|| !crate::sched::is_thread_present(tid)),
            "round {round}: the child was never reaped",
        );
        crate::sched::reclaim_region(thread_control_block_region).expect("reclaim tcb region");
        crate::sched::reclaim_region(as_region).expect("reclaim address space region");

        // The frame's address is in the failure, which is what says which round leaked; the
        // whole-boot half of "nothing leaked anywhere" is the frame ledger's job
        // (`testing::report_page_frame_ledger`), measured where it is not load-sensitive.
        tcb_run.assert_returned("spawn-to-reap leaked the cycle's TCB region");
        as_run.assert_returned("spawn-to-reap leaked the cycle's address-space region");
    }
    crate::sched::reclaim_region(endpoints).expect("the endpoint region did not come back");
}

/// **A process composed from two capabilities runs in the space it was built in** (milestone 19b
/// (run a real workload), extended by §185 (what carries the claim that userspace composes a process
/// from an authority you can count on one hand)).
///
/// The witness holds a memory region and a report line, and nothing else. It reads a child out of
/// the archive by name, mints a rendezvous, builds the child's space through the shared loader, maps
/// a frame of its own into that space and is refused the same address twice, then configures and
/// starts the thread and receives its answer. Every bit set is one verb the kernel allowed a
/// two-capability process; `CHILD_ANSWERED` is the one nothing else in the suite asserts from
/// userspace, that a thread a process built ran in the space it built.
///
/// **The wait has a deadline**, for `c_seam_tests::wait_for_report`'s reason: a child that never
/// runs leaves the witness parked in its receive, and a bare receive here would surface that as a
/// watchdog dump rather than as this sentence.
///
/// Falsification: replayable `system_tests/falsifications/user.child_thread_tests.a_process_composed_from_two_capabilities_runs_in_the_space_it_built.patch`
#[cfg(initial_ramdisk)]
#[test_case]
fn a_process_composed_from_two_capabilities_runs_in_the_space_it_built() {
    use capability_witness_contract::process_composition as bits;
    let witness = process_composition_service::wire();
    let report = witness.report;
    // Each predicate is asked once per pass and its first `true` is the answer: the reclaim below
    // is a predicate with a side effect, and asking it again after it succeeded would ask about a
    // region that no longer exists.
    let within_30_s = |done: &mut dyn FnMut() -> bool| {
        let deadline = timer::now() + 30 * timer::frequency();
        loop {
            if done() {
                return true;
            }
            if timer::now() >= deadline {
                return false;
            }
            sched::yield_now();
        }
    };
    assert!(
        within_30_s(&mut || sched::rendezvous_waiting_senders(report) > 0),
        "the witness sent no verdict in 30 s: it is still waiting for a child it started, so the \
         thread it built never ran or never answered on the capability it was given",
    );
    let verdict = sched::ipc_receive(report)[0];

    // Hand the memory back before judging, so a wrong verdict does not also read as a leak in the
    // frame ledger. The witness exits after its send; its space is bound to it and goes with it.
    // The child exited before the witness sent anything, so the budget it was built from is free
    // once the reaper has it; the retry is for that reap, which may land on another core.
    assert!(
        within_30_s(&mut || !sched::is_thread_present(witness.tid)),
        "the witness sent its verdict and was never reaped",
    );
    assert!(
        within_30_s(&mut || sched::reclaim_region(witness.budget).is_ok()),
        "the witness's budget would not reclaim: something it built is still alive",
    );
    sched::reclaim_region(witness.thread_control_block_region)
        .expect("reclaim the witness's thread control block region");
    sched::reclaim_region(witness.report_region).expect("reclaim the witness's report line");

    assert_eq!(
        verdict,
        bits::WHOLE,
        "process composition verdict {verdict:#08b}: bit0 child read from the archive, bit1 space \
         built and child laid into it, bit2 own frame mapped into it, bit3 double map refused, bit4 \
         configured and started, bit5 the child answered",
    );
}
