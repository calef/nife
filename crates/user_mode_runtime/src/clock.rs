//! **Time**: the counter and its frequency on each architecture, the cycle counter and what it
//! counts, the current CPU, and monotonic nanoseconds.
//!
//! Cut out of `lib.rs` unchanged on 2026-10-12 (UTC) by lane maintainer/files-over-1500, for §282
//! (when a file is split, the parent becomes an index); the crate root re-exports every public
//! item, so each keeps its path.
//!
//! Name: provisional, lane maintainer/files-over-1500 (#2017), 2026-10-12 (UTC), for the module and
//! the file. Named for the calls it holds, one module per seam of the old crate root; the pull
//! request records no other name considered.

use abi::cycle_counter::CycleMeaning;

/// The virtual counter, `CNTVCT_EL0`: a monotonic tick count for self-timing. Readable at EL0 only
/// because the kernel opened `CNTKCTL_EL1.EL0VCTEN` (see kernel `timer::init` and notes/abi.md); the
/// read is a plain register move, no syscall. Pair with [`cntfrq`] to turn tick deltas into seconds.
#[cfg(target_arch = "aarch64")]
pub fn now() -> u64 {
    let t: u64;
    // SAFETY: reading a system register the kernel made EL0-readable. No side effects.
    unsafe {
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) t, options(nomem, nostack));
    }
    t
}

/// The monotonic tick count (RISC-V): `rdtime`, which reads the `time` CSR. Readable from U-mode
/// only because the kernel sets `scounteren.TM` in its per-hart timer init, the same shape as
/// aarch64 needing `CNTKCTL_EL1.EL0VCTEN`. That claim was aspirational until 2026-07-30: the bit was
/// never set and this worked only because QEMU's OpenSBI leaves it permitted. Pair with [`cntfrq`]
/// to get seconds.
#[cfg(target_arch = "riscv64")]
pub fn now() -> u64 {
    let t: u64;
    // SAFETY: reading the time CSR the kernel made U-mode-readable. No side effects.
    unsafe {
        core::arch::asm!("rdtime {}", out(reg) t, options(nomem, nostack));
    }
    t
}

/// The counter frequency in Hz, `CNTFRQ_EL0`: how many [`now`] ticks make a second. Constant for the
/// life of the machine (QEMU reports 62.5 MHz under TCG, the host's counter frequency under HVF).
///
/// This is the one architecture where the rate is always knowable, so [`cntfrq_checked`] here is
/// always `Some` and this function cannot fail. The other two read a page the kernel fills, and both
/// can report "unknown"; see [`cntfrq_checked`].
#[cfg(target_arch = "aarch64")]
pub fn cntfrq() -> u64 {
    let f: u64;
    // SAFETY: reading a system register; EL0-readable once EL0VCTEN is set.
    unsafe {
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) f, options(nomem, nostack));
    }
    f
}

/// The counter frequency in Hz, or `None` if this process cannot know it (aarch64: never, the
/// machine states it in `CNTFRQ_EL0`).
#[cfg(target_arch = "aarch64")]
pub fn cntfrq_checked() -> Option<u64> {
    Some(cntfrq())
}

/// The counter frequency in Hz (RISC-V), read from the **timebase page** the kernel maps read-only
/// into every process at [`counter_frequency_protocol::PAGE_VA`].
///
/// RISC-V has **no** register that reports the timebase. The machine states it in the device tree,
/// at `/cpus/timebase-frequency`, and a process holds neither a pointer to the blob nor a capability
/// naming the memory it lives in, so the kernel is the only party that can ever know this number.
/// `kernel::arch::riscv64::timer::init_frequency` reads it on the boot hart and
/// `kernel::user::timebase_page_phys` publishes it; this function is the reader, a plain load, no
/// syscall, the same ambient shape [`now`] already has.
///
/// **It returned a hardcoded `10_000_000` until 2026-09-21**, QEMU `virt`'s rate, with a comment
/// proposing an aux-vector as the eventual fix. radon (the VisionFive 2) runs its `time` CSR at
/// **4 MHz**: every userspace duration measured on that board was 2.5x too large and nothing said
/// so. The aux-vector was never built and is not needed; `x86_64` had already solved the same
/// problem with a page, one directory away, and this now uses it. calef's ruling that forced it:
/// *"We should ensure that the program only returns accurate numbers versus leveraging hard coded
/// ones."*
///
/// # Panics
///
/// If the rate is unknown: see [`cntfrq_checked`], which is this without the panic.
#[cfg(target_arch = "riscv64")]
pub fn cntfrq() -> u64 {
    cntfrq_checked().expect(
        "the counter frequency is unknown: this process's timebase page is zeroed or unrecognized",
    )
}

/// The counter frequency in Hz (RISC-V), or `None` if this process cannot know it. See [`cntfrq`]
/// for where the number comes from and [`cntfrq_checked`]'s `x86_64` twin for why "unknown" is a
/// state worth representing rather than papering over.
#[cfg(target_arch = "riscv64")]
pub fn cntfrq_checked() -> Option<u64> {
    // SAFETY: every path that builds a riscv64 process maps a page (real, or zeroed when the rate
    // was never learned) read-only at `counter_frequency_protocol::PAGE_VA` before it runs:
    // `kernel::user::load`, `kernel::user::map_timebase_page`, and
    // `supervision_protocol::build_child_space`. See that crate's `BUGS` section for the one shape
    // that does not, which faults here on purpose.
    let page = unsafe {
        counter_frequency_protocol::TimebasePage::new(counter_frequency_protocol::PAGE_VA)
    };
    page.hz()
}

/// The monotonic tick count (`x86_64`, milestone 161): `rdtsc`, which reads the time-stamp counter.
/// Readable from ring 3 because `CR4.TSD` is clear, which is the reset state and which this kernel
/// does not change; that is the same shape as aarch64 needing `CNTKCTL_EL1.EL0VCTEN` and RISC-V
/// needing `scounteren.TM`, with the difference that here the permissive state is the default and
/// the kernel would have to act to *close* it.
///
/// **`rdtsc` returns the count split across two 32-bit registers** (`edx:eax`), a shape inherited
/// from the Pentium that introduced it, so this is a shift and an or rather than a move. Writing it
/// as a single `out(reg)` compiles and reads only the low half, which is a counter that wraps every
/// four seconds at 1 GHz and looks correct in any test short enough to run.
///
/// **No serialisation, deliberately.** `rdtsc` is not ordered against surrounding instructions, so
/// a sufficiently tight measurement wants `lfence` or `rdtscp` around it. Pair this with [`cntfrq`]
/// and the granularity that buys is nanoseconds; the reordering window is tens of cycles, and the
/// kernel's own calibration accepts the same trade for the same reason
/// (`kernel/src/arch/x86_64/timer.rs`).
///
/// # BUGS
///
/// **On `x86_64` the cycle counter is ambient, and nobody chose that.** The other two architectures
/// give userspace a *coarse* counter and keep the fine one shut: aarch64 opens `CNTVCT_EL0` and, as
/// of milestone 228 (the cycle counters are closed by assumption, and on two architectures the
/// assumption is a comment), writes `PMUSERENR_EL0 = 0` so `PMCCNTR_EL0` stays closed; riscv64 opens
/// `scounteren.TM` and clears `CY`. Here there is one register for both jobs. The TSC *is* the
/// coarse clock and the cycle counter, `CR4.TSD` is clear at reset, this kernel never writes it, and
/// so every ring-3 program on this architecture holds a sub-nanosecond timing instrument it was
/// never granted. That is a state inherited from the reset value, not a position anyone argued for,
/// and it is recorded here rather than in a tracker so the next reader of this function meets it.
///
/// **The `rdpmc` door beside it did close.** `CR4.PCE` is a different bit from `CR4.TSD` and gates a
/// different instruction, and since fixed counter 2 runs at the TSC rate it is a second path to a
/// cycle-rate reading. Milestone 228 established it clear in `arch::init`, per core, because nothing
/// in this tree reads a performance counter from ring 3 and so closing it cost nothing. So the gap
/// below is `rdtsc` specifically, not "x86 counters" in general.
///
/// **It is not closed because closing it today would take the clock away.** Setting `CR4.TSD` with
/// nothing to replace it breaks `Instant`, `thread::sleep`, the random seed, smoltcp's timestamps in
/// `std_net`, and the benchmark harness, all at once and on the same instruction. So this is a
/// limitation with a price rather than an oversight, and paying it needs a second time source first:
/// a coarse monotonic value published in a page, which is DECISIONS §43's move (reading the clock is
/// a page, which put the wall clock in a page rather than a register) one axis over. Nothing here
/// proposes building that.
///
/// **What it costs meanwhile**, stated so §19 (architectural parity is a tenet) reports a known gap
/// rather than a silent one: `x86_64` answers milestone 75 (who may read the cycle counter, and by
/// what authority) with "everyone, always", by inheritance, whatever that decision concludes for the
/// other two. Linux names this exact asymmetry from the other side; its arm64 per-task
/// `PMUSERENR_EL0` work says it opens the counter only on request to avoid "the information leaks
/// x86 has".
#[cfg(target_arch = "x86_64")]
pub fn now() -> u64 {
    let (lo, hi): (u32, u32);
    // SAFETY: reading a counter ring 3 is permitted to read. No side effects, no memory touched.
    unsafe {
        core::arch::asm!("rdtsc", out("eax") lo, out("edx") hi, options(nomem, nostack));
    }
    ((hi as u64) << 32) | (lo as u64)
}

/// The counter frequency in Hz (`x86_64`), read from the **timebase page** the kernel maps
/// read-only into every process at [`counter_frequency_protocol::PAGE_VA`] (milestone 161's `cntfrq`
/// follow-up).
///
/// aarch64 has `CNTFRQ_EL0`, which states the rate. RISC-V has none, but the device tree does,
/// and the process cannot read it. x86 has **no architected rate a ring-3 program can ask for
/// directly**: `CPUID` leaf `0x15` gives the TSC's ratio to a crystal clock on the parts that
/// implement it, but a ring-3 program cannot calibrate one for itself the way the kernel's
/// fallback does, against the 8254 PIT: the PIT is at I/O ports `0x40..0x43`, `IOPL` is 0 and the
/// TSS's I/O permission bitmap is empty, so `in`/`out` from a process is a general protection
/// fault. That is not an oversight to route around, it is
/// [DECISIONS §121](../../../design/decisions/0121-port-io-capability.md), which closed port I/O
/// to userspace **permanently**: a program that could calibrate its own clock by touching the PIT
/// would be a program that had escaped the confinement this kernel exists to enforce.
///
/// So the kernel is the only party that can ever know this number, and it publishes rather than
/// gates: `arch::x86_64::timer::init_frequency` reads `CPUID` leaf `0x15` first and falls back to
/// PIT calibration only if the part does not report one (see that function's own docs; under this
/// project's QEMU invocation the leaf is unavailable and calibration is what every boot actually
/// uses), then every kernel-side function that builds a top-level process's address space writes
/// the result into a page and maps it, read-only, before the process ever runs: `kernel::user::load`
/// (the generic ELF loader every ordinary test fixture calls) and the handful of functions that
/// build one by hand for a narrower world (`spawn_init`, and the `spawn_<program>`-shaped test
/// harnesses: `timetable_tests::spawn_timetable`, `authority_tests`' `root_supervisor` spawn,
/// `c_seam_tests::spawn_confiner`, `login_service`, `live_swap_tests`' `swapper` spawn; see
/// `kernel::user::map_x86_timebase_page`, which all of them call). This function is the reader: a
/// plain load through an unsafe pointer, no syscall, the same "ambient, no capability" shape
/// [`now`] already has.
///
/// # Panics
///
/// If the rate is unknown, which is a zeroed or unrecognized page. **It used to return 1 GHz
/// instead**, and that fallback was deleted on 2026-09-21 under calef's ruling: *"We should ensure
/// that the program only returns accurate numbers versus leveraging hard coded ones."* A plausible
/// wrong rate is worse than no rate, because it produces a benchmark figure somebody quotes; a
/// process that dies on the read produces nothing to quote. [`cntfrq_checked`] is this without the
/// panic, for a caller that would rather cope than die.
///
/// **The argument that fallback rested on has expired.** It was that widening this one architecture
/// to `Option<u64>` would make every caller of the portable [`monotonic_nanos`] handle a case the
/// other two could not produce. Two of three architectures read a page now, so the case is no
/// longer peculiar, and the shape this tree already uses for exactly this question is the kernel's
/// own: `arch::timer::frequency` panics and `arch::timer::frequency_checked` returns an `Option`,
/// with the reason written beside them ("a plausible zero is worse than a panic naming this file").
/// This is that pair, one privilege level down, so `monotonic_nanos` keeps its `u64` and nothing
/// fabricates a rate.
///
/// **What the panic costs, honestly**: a userspace panic here traps (see [`trap`]) and the kernel
/// kills the process, with no message, because a program with no console cannot print one. So the
/// symptom a reader meets is a killed process and a fault line, not a diagnostic naming the page.
/// That is the same floor every `expect` in userspace has and it is not special to this one.
///
/// # BUGS
///
/// **The one case that used to reach the fallback is gone, not merely refused.** A process built by
/// `supervision_protocol::build_child_space` (the tree's one userspace ELF loader, used by
/// `root_supervisor`, `spawner`, `system_initializer`, and every role `hello` builds, `coremark` and
/// `timetable`'s own `least_authority_demo` included) used to map a freshly retyped, *zeroed*
/// placeholder and so read 1 GHz. It now gets a page that crate fills from its own rate, read
/// through [`cntfrq_checked`], so a child inherits its parent's measured number. The residue is
/// real and small: a parent that does not know the rate writes a zeroed page, and the whole subtree
/// below it refuses rather than guessing.
#[cfg(target_arch = "x86_64")]
pub fn cntfrq() -> u64 {
    cntfrq_checked().expect(
        "the counter frequency is unknown: this process's timebase page is zeroed or unrecognized",
    )
}

/// The counter frequency in Hz (`x86_64`), or `None` if this process cannot know it: a zeroed page
/// (the rate was never measured, or the parent that built this process did not know it) or one
/// whose magic does not match. See [`cntfrq`] for where the number comes from and why the
/// unchecked twin refuses rather than substituting a constant.
#[cfg(target_arch = "x86_64")]
pub fn cntfrq_checked() -> Option<u64> {
    // SAFETY: every kernel-side space-building function this crate's own docs list maps a page
    // (real, or zeroed when the rate was never learned) read-only at
    // `counter_frequency_protocol::PAGE_VA` into every x86_64 process before it ever runs.
    let page = unsafe {
        counter_frequency_protocol::TimebasePage::new(counter_frequency_protocol::PAGE_VA)
    };
    page.hz()
}

/// **A cycle-counter read and what it counts**, returned together by [`cycle_reading`].
///
/// The count alone is not a quantity a reader can compare across machines: aarch64 and riscv64
/// count core cycles (over different privilege levels), and `x86_64` counts reference cycles at a
/// constant rate. So the count never travels without its [`CycleMeaning`].
///
/// Name: provisional, minted by the lane for milestone 353 (the aarch64 half of 74) on 2026-10-07
/// (UTC), the noun for what
/// [`cycle_reading`] returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CycleReading {
    /// The raw counter value. **Only a difference of two reads means anything**, and only when
    /// both were taken on the same core (aarch64 and riscv64 counters are per core and start at
    /// different times).
    pub count: u64,
    /// What [`Self::count`] counts on this architecture.
    pub meaning: CycleMeaning,
}

/// **Read the CPU's cycle counter from user mode, paired with what it counts** (milestone 353,
/// calef's ruling B4 step 1, 2026-10-07 UTC).
///
/// One instruction per architecture, no syscall, which is the property DECISIONS §139 (who may read the cycle counter, and by what
/// authority) option 4
/// chose the grant to keep:
///
/// | architecture | instruction | [`CycleMeaning`] |
/// |---|---|---|
/// | aarch64 | `mrs PMCCNTR_EL0` | [`CycleMeaning::CoreCyclesUserAndKernel`]: EL0 and EL1, not EL2 |
/// | riscv64 | `csrr cycle` | [`CycleMeaning::CoreCyclesEveryMode`]: every privilege mode |
/// | `x86_64` | `rdtsc` | [`CycleMeaning::ConstantRateReferenceCycles`]: not core cycles |
///
/// # Who may call it
///
/// **Only a program granted the cycle counter, on aarch64 and riscv64.** There the read is legal
/// at EL0 or U-mode only if the kernel opened `PMUSERENR_EL0` or `scounteren.CY` for this thread
/// (milestone 229 (the cycle-counter grant)). An ungranted call traps, and the kernel kills the thread: the read does not
/// fail softly, because a program is expected to know from its own manifest whether it holds the
/// grant, and there is no question to ask the kernel first. On `x86_64` the TSC is ambient in ring
/// 3 and every program may call it. A kernel-provided way to ask "may I read" is step 2 of the
/// ruling, a syscall-surface fork that is recorded and not built.
///
/// # Examples
///
/// ```no_run
/// let before = user_mode_runtime::cycle_reading();
/// // ... the work being measured ...
/// let after = user_mode_runtime::cycle_reading();
/// let spent = after.count.wrapping_sub(before.count);
/// if !after.meaning.is_core_cycles() {
///     // x86_64: `spent` is reference cycles at the TSC's constant rate, not core cycles.
/// }
/// # let _ = spent;
/// ```
///
/// # BUGS
///
/// - **No manifest field grants the cycle counter yet**, so outside the kernel's own test (which
///   grants through a test-only door) no aarch64 or riscv64 program can call this without being
///   killed. DECISIONS §139 put the grant in the spawn manifest; carrying it there is milestone 75
///   (who may read the cycle counter, and by what authority), and milestone 229 left the kernel
///   side without a setter on purpose.
/// - **riscv64 cannot say whether its counter is the kernel's.** The `cycle` CSR is `mcycle`, and the
///   kernel's bench probe may have been handed `hpmcounter3` by firmware instead. Flagging that in
///   the reading needs a kernel-to-process channel that does not exist; milestone 856 (a program
///   asks whether it may read the cycle counter) proposes it with step 2.
/// - **Under QEMU every count is emulator time**, an instruction count or a virtual clock, never a
///   measurement of a core.
///
/// Name: provisional, minted by milestone 353's lane on 2026-10-07 (UTC). A noun, Rust's getter
/// shape (`now`, `cntfrq`). Refused `cycles` (the kernel's `arch::pmu::cycles` already means core
/// cycles on all three architectures, and on `x86_64` this is the TSC, which is not), and
/// `read_cycle_counter` (the fixture's private name for the raw read, and a verb).
pub fn cycle_reading() -> CycleReading {
    CycleReading {
        count: read_cycle_counter(),
        meaning: CYCLE_MEANING,
    }
}

/// What [`cycle_reading`] counts here: core cycles at EL0 and EL1, under the `PMCCFILTR_EL0` of
/// zero the kernel writes on every core.
#[cfg(target_arch = "aarch64")]
const CYCLE_MEANING: CycleMeaning = CycleMeaning::CoreCyclesUserAndKernel;
/// What [`cycle_reading`] counts here: `mcycle`, every privilege mode.
#[cfg(target_arch = "riscv64")]
const CYCLE_MEANING: CycleMeaning = CycleMeaning::CoreCyclesEveryMode;
/// What [`cycle_reading`] counts here: the TSC, constant-rate reference cycles.
#[cfg(target_arch = "x86_64")]
const CYCLE_MEANING: CycleMeaning = CycleMeaning::ConstantRateReferenceCycles;

#[cfg(target_arch = "aarch64")]
fn read_cycle_counter() -> u64 {
    let value: u64;
    // SAFETY: `mrs` from `PMCCNTR_EL0` reads a counter and touches no memory. It is UNDEFINED at
    // EL0 unless `PMUSERENR_EL0` permits it, which is the grant `cycle_reading`'s caller must hold;
    // an ungranted read traps and the kernel kills the thread, which is not memory unsafety.
    unsafe {
        core::arch::asm!("mrs {}, pmccntr_el0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

#[cfg(target_arch = "riscv64")]
fn read_cycle_counter() -> u64 {
    let value: u64;
    // SAFETY: `csrr` from the `cycle` CSR reads a counter and touches no memory. It is an illegal
    // instruction in U-mode unless `scounteren.CY` permits it; see the aarch64 twin above.
    unsafe {
        core::arch::asm!("csrr {}, cycle", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// The TSC. The same instruction [`now`] executes on this architecture, kept separate because
/// the two answer different questions: `now` is the clock, this is the counter with its meaning.
#[cfg(target_arch = "x86_64")]
fn read_cycle_counter() -> u64 {
    now()
}

/// **Which CPU this thread is running on**, or `None` if this process has no page to read it from
/// or has somehow not been switched in yet.
///
/// A plain load, no syscall, and that is the point rather than an optimization. calef ruled on
/// 2026-09-21 that a thread observing *itself* gets a page and a thread observing *another* gets a
/// selector, because the consumer here is a memory allocator keeping a per-CPU cache: it asks once
/// per allocation, and a crossing at that rate is not a cost you can amortize away. (The ruling's
/// `design/decisions/` section is on another branch and not on `main` yet, so it is named here
/// rather than cited.) `crates/current_cpu_protocol` carries the layout, the ordering argument and
/// the reason this is not a register on any of the three targets.
///
/// **The answer can be stale before you use it**, because the kernel may migrate this thread the
/// instruction after the load, and nothing in this function or that crate prevents it. A per-CPU
/// cache built on this must be *correct* when the answer turns out to be the previous core's and
/// merely faster when it is not. Linux solves the same problem with `rseq`'s restartable
/// sequences; this tree does not have those and is not pretending to.
///
/// **The value is a CPU id, not an index into anything you have counted.** Size a per-CPU array by
/// [`current_cpu_protocol::CPU_ID_BOUND`] and nothing else: the VisionFive 2's online set is
/// `{1, 2, 3}`, so counting cores and indexing by the count reaches a core that is not there and
/// misses one that is.
///
/// # EXAMPLES
///
/// ```no_run
/// # use user_mode_runtime::current_cpu;
/// let mut per_cpu = [0u64; current_cpu_protocol::CPU_ID_BOUND];
/// if let Some(cpu) = current_cpu() {
///     per_cpu[cpu] += 1;
/// }
/// ```
///
/// # BUGS
///
/// - **A process whose address space was never bound to a TCB has no page here**, and this faults
///   on an unmapped read rather than answering `None`. Every space that runs a thread gets one
///   (`kernel::user::AddressSpace::new` for the kernel-built ones,
///   `kernel::sched::configure_thread_control_block` for the userspace-built ones), so the shape
///   that faults is a space with no thread in it, which has nothing to ask. Stated because the
///   failure is a fault rather than a value, which is the one thing a caller cannot handle.
pub fn current_cpu() -> Option<usize> {
    // SAFETY: the kernel maps a page it owns, read-only, at `current_cpu_protocol::PAGE_VA` into
    // every address space that a TCB binds, before that thread's first instruction runs. See this
    // function's own BUGS section for the one shape that has no page.
    let page = unsafe { current_cpu_protocol::CurrentCpuPage::new(current_cpu_protocol::PAGE_VA) };
    page.cpu()
}

/// **Monotonic nanoseconds since boot**, from [`now`] and [`cntfrq`].
///
/// Here rather than in each program because two of them need it and the naive form is wrong:
/// `ticks * 1_000_000_000` overflows a `u64` about five minutes into a boot at 62.5 MHz, so the
/// conversion splits into whole seconds and a remainder. `components/src/clock.rs` found that the hard
/// way; `date` and the shell's `time` then wanted the same five lines, which is CLAUDE.md rule 7 at
/// the smallest size it comes in.
///
/// **This needs no capability**, and that is worth knowing when reading anything that calls it: the
/// counter is ambient (the kernel opened it to EL0), so a *duration* is measurable by any process.
/// What needs a capability is the **wall clock**, which is this plus an offset only the clock page
/// carries. See notes/clock.md.
pub fn monotonic_nanos() -> u64 {
    /// Nanoseconds in a second. Spelled here rather than taken from `clock_protocol`, because this
    /// crate is the syscall runtime and depends on `abi` alone; a runtime that pulled in a wire
    /// contract to name a unit would be the wrong direction for the dependency.
    const NANOS_PER_SEC: u64 = 1_000_000_000;
    let freq = cntfrq();
    let ticks = now();
    let secs = ticks / freq;
    let rem = ticks % freq;
    secs * NANOS_PER_SEC + rem * NANOS_PER_SEC / freq
}
