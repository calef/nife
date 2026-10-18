use core::sync::atomic::{AtomicU64, Ordering};

/// Events per core. 16 is enough to see the whole final approach to a wedge (a block, the
/// wakes around it, the switch that stranded something) without turning the dump into a log.
pub(crate) const DEPTH: usize = 16;

/// What happened. The discriminant is packed into the entry's top byte.
#[derive(Clone, Copy)]
#[repr(u8)]
pub(crate) enum Event {
    /// `schedule()` picked `tid` and marked it Running on this core.
    SwitchTo = 1,
    /// The thread running here marked itself Blocked (aux = low byte of the rendezvous name).
    BlockSelf = 2,
    /// This core moved `tid` Blocked -> Ready onto a run queue (rendezvous, reply, or abort).
    Wake = 3,
    /// This core saw `tid` still on a cpu and parked the wake (`wake_pending`).
    WakeDeferred = 4,
    /// This core completed a parked wake in `finish_switch`.
    WakeCompleted = 5,
    /// This core pushed `tid` into cpu `aux`'s inbox (placement or load-aware wake).
    PlaceRemote = 6,
    /// This core served a steal: handed `tid` to requester cpu `aux`.
    StealServe = 7,
    /// This core drained its inbox; the tid field carries the count moved.
    InboxDrain = 8,
    /// This core REFUSED a wake of `tid`: the target was parked in IPC and the waker had
    /// delivered nothing (no message, no signal, no abort). The boot-8 gate firing; on a
    /// healthy boot this event never appears, so its presence in a bench dump is the finding.
    WakeRefused = 9,
    /// This core switched into `tid`, and the core it last ran on was a different one
    /// (milestone 219). **The cross-core handoff count**, and the only honest one: a rendezvous
    /// wake makes its peer Ready on the *waker's* core, which is local by construction, so
    /// [`Event::PlaceRemote`] misses every migration a pure IPC workload performs. `aux` is the
    /// core it came from.
    ///
    /// Only ever recorded by a `soak` build, since the `last_cpu` comparison that produces it
    /// is gated (it sits in `schedule()`'s switch and cost 5.7% of `ipc_fastpath` on aarch64
    /// when it was not). The variant itself stays in every build: it is compile-time only, it
    /// costs no bytes, and removing it would leave [`dump`]'s `11 => "moved"` arm naming a
    /// number with nothing to point at.
    #[cfg_attr(not(feature = "soak_test"), allow(dead_code))]
    Migrated = 11,
    /// This core set `ipc_served` on `tid`: a delivery completed the thread's parked IPC.
    /// `aux` names the delivering site (1 send, 2 receive-collect, 3 `send_cap`, 4 `receive_cap`-collect,
    /// 5 call, 6 reply, 7 irq signal, 8 death message, 9 a notification signal to a waiter, 10 a
    /// notification signal to its bound receiver), so a bench dump answers "who served
    /// this thread" by reading the ring instead of inferring it from a frozen syscall count,
    /// which is the inference boots 7 through 9 got wrong (notes/visionfive2.md, fifth stop).
    Served = 10,
}

/// How many discriminants [`Event`] has, which sizes the per-core totals below.
///
/// One more than the largest variant, because the discriminants start at 1 so that a zeroed
/// ring slot is distinguishable from a recorded `SwitchTo`. Read only by the `soak`-gated
/// counter array; a `const` costs no bytes either way, so it stays visible to every build.
#[cfg_attr(not(feature = "soak_test"), allow(dead_code))]
pub(crate) const KINDS: usize = 12;

struct Ring {
    seq: AtomicU64,
    slots: [AtomicU64; DEPTH],
    /// **A total per event kind, beside the ring** (milestone 219). The ring holds sixteen
    /// events, which is the right depth for reading the final approach to a wedge and the
    /// wrong one for a soak: a refused wake three hours into an eight-hour run has scrolled
    /// out of it long before anyone looks. A counter cannot scroll.
    ///
    /// Per core, in the same cache line neighbourhood as that core's own ring, for the reason
    /// the ring is per core: one shared counter array would put a contended atomic on the IPC
    /// fastpath, and a soak whose instrument slows the thing it measures is measuring the
    /// instrument. The reader sums across cores and accepts that the sum is a moment that
    /// never quite existed, which is what [`counted`] says out loud.
    ///
    /// **Behind `feature = "soak_test"`.** Per-core rather than shared was not enough: the extra
    /// load-add-store per event, on paths the IPC fastpath runs, is part of what pushed
    /// `ipc_fastpath` 5.7% over milestone 132's bound on aarch64 when this shipped
    /// unconditionally. Only a soak build reads these, so only a soak build carries them.
    #[cfg(feature = "soak_test")]
    counts: [AtomicU64; KINDS],
}

#[allow(clippy::declare_interior_mutable_const)]
const EMPTY_RING: Ring = Ring {
    seq: AtomicU64::new(0),
    slots: [const { AtomicU64::new(0) }; DEPTH],
    #[cfg(feature = "soak_test")]
    counts: [const { AtomicU64::new(0) }; KINDS],
};

static RINGS: [Ring; crate::cpu::MAX_CPUS] = [EMPTY_RING; crate::cpu::MAX_CPUS];

/// Record an event on the calling core's ring. Every call site runs with interrupts masked
/// (under `IPC_TABLES` or in IRQ context), so the owning core cannot interleave with itself.
///
/// **`#[inline(never)]`, so the fast paths carry one copy rather than one per event** (milestone
/// 758 (the IPC fast paths shrink back inside their band), provisional). Inlined, every site
/// carried its own per-CPU read, `RINGS` bounds check and panic landing pad, sequence bump and
/// packed store: measured on 2026-10-04 UTC, the ring index alone was the largest single line in
/// `script/fastpath-footprint`'s closures on riscv64 and the third largest on aarch64 and x86_64,
/// and an IPC round trip records two or three events in each of four or five functions. Out of
/// line, a site is its argument moves and a call. The ring is a diagnostic nobody branches on,
/// so the call's few instructions buy back several hundred bytes of every core's L1i.
#[inline(never)]
// In the pinned hot section: milestone 796 (pin the hot trap path's placement).
#[cfg_attr(
    target_os = "none",
    unsafe(link_section = ".text.hot.sched.trace.record")
)]
pub(crate) fn record(kind: Event, tid: u64, aux: u8) {
    let ring = &RINGS[crate::cpu::id()];
    let seq = ring.seq.fetch_add(1, Ordering::Relaxed);
    // kind in the top byte, aux below it, the tid's low 48 bits under that. A tid is
    // (generation << 32) | slot with slot < 256; 48 bits keeps 16 bits of generation,
    // plenty to disambiguate in a dump.
    let entry = ((kind as u64) << 56) | ((aux as u64) << 48) | (tid & 0x0000_FFFF_FFFF_FFFF);
    ring.slots[(seq as usize) % DEPTH].store(entry, Ordering::Relaxed);
    // The running total, on this core's own line. Relaxed and non-atomic-read-modify-write in
    // effect (one writer per core), so it costs a load, an add and a store next to the two
    // stores above it. **Soak builds only**, because those three instructions are on the IPC
    // fastpath and nothing but a soak reads what they produce; see the field's own note.
    #[cfg(feature = "soak_test")]
    {
        let total = &ring.counts[kind as usize];
        total.store(
            total.load(Ordering::Relaxed).wrapping_add(1),
            Ordering::Relaxed,
        );
    }
}

/// **How many times `kind` has happened on this machine since boot**, summed over every core.
///
/// The one number the soak's heartbeat is actually watching is `WakeRefused`
/// (`soak`), and the reason it can be watched at all is that this is a total rather than a
/// window: a refusal at minute nine of an eight-hour run is still in this number at hour eight.
///
/// **The sum is racy and deliberately so.** Cores keep counting while it is read, so the value
/// is not a snapshot of any single instant. Nothing here needs one: the soak asks "is this
/// still zero" and "how much did it grow since the last beat", and both survive a reader that
/// is a few events behind.
#[cfg(feature = "soak_test")]
pub(crate) fn counted(kind: Event) -> u64 {
    let mut total = 0u64;
    for ring in &RINGS {
        total = total.wrapping_add(ring.counts[kind as usize].load(Ordering::Relaxed));
    }
    total
}

/// Print core `cpu`'s ring, oldest first. Racy against that core's own writes, by design.
pub(crate) fn dump(cpu: usize) {
    let ring = &RINGS[cpu];
    let seq = ring.seq.load(Ordering::Relaxed);
    if seq == 0 {
        return;
    }
    let start = seq.saturating_sub(DEPTH as u64);
    crate::print!("    core {cpu} events [{start}..{seq}):");
    for s in start..seq {
        let e = ring.slots[(s as usize) % DEPTH].load(Ordering::Relaxed);
        let (kind, aux, tid) = (e >> 56, (e >> 48) & 0xff, e & 0x0000_FFFF_FFFF_FFFF);
        let name = match kind {
            1 => "switch",
            2 => "block",
            3 => "wake",
            4 => "wake?",
            5 => "wake+",
            6 => "place",
            7 => "steal",
            8 => "drain",
            9 => "refuse",
            10 => "serve",
            11 => "moved",
            _ => "?",
        };
        crate::print!(" {name}:{tid:#x}");
        if matches!(kind, 2 | 6 | 7 | 10 | 11) {
            crate::print!("/{aux}");
        }
    }
    crate::println!();
}
