//! **Bringing this CPU's architecture state up**: the GDT, TSS and IDT, and the control-register
//! and PAT settings that close ring 3 off from what it must not reach.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// Bring this CPU's architecture state up: the GDT and TSS, then the IDT. The order is forced, and
/// not obviously: an IDT entry names a code **selector**, so the GDT that selector indexes has to be
/// the one installed before any trap can be delivered through it.
pub fn init() {
    // First, before this core prints anything or adopts the fine map: the fine map's framebuffer
    // pages select PAT entry 1, and on a core that has not run this, entry 1 is write-through.
    give_write_combining_a_page_attribute_entry();
    // SAFETY: called once per CPU during boot, with a valid stack, before interrupts are unmasked.
    unsafe { segments::init() };
    exceptions::init();
    // And the second door into the kernel, which shares none of the first one's machinery: four
    // MSRs, no gate and no descriptor. It goes here rather than with ring 3 because it is per-CPU
    // state like the GDT and the IDT, and because a `syscall` before it is programmed is a jump to
    // whatever `IA32_LSTAR` holds, which on a cold machine is zero.
    //
    // SAFETY: `segments::init` above installed the GDT these selectors index, and nothing has
    // entered ring 3 (nothing can: this is the boot CPU's own bring-up).
    unsafe { exceptions::init_syscall() };

    close_performance_counters_to_ring3();
    close_ring3_pages_to_ring0_execution();
}

/// **Program `IA32_PAT` so that `PWT` alone means write-combining** (the console-scroll lane,
/// 2026-10-04), which is what `paging::Flags::write_combining()` encodes and what the kernel maps
/// the framebuffer with (`mmu::direct_map_claims`).
///
/// # Why
///
/// Until this, the framebuffer was mapped like a register: `PCD | PWT`, strong uncacheable, one
/// bus transaction per four-byte store and nothing the MTRRs could relax. xenon's 1920x1080
/// aperture is 8 MB, so every full redraw was two million of them, and the boot console's
/// scrolling swept visibly down the screen. Write-combining lets the core gather a run of stores
/// into one burst. A PAT type of WC also wins over whatever the firmware's MTRRs say about the
/// range, so this does not depend on the firmware having done anything.
///
/// # Why entry 1 can simply be rewritten
///
/// The Intel SDM asks for a cache flush around a PAT change because a mapping that already uses
/// the changed entry would otherwise hold lines of the old type. **No mapping uses entry 1 when
/// this runs**: `boot.s`'s coarse map sets neither `PWT` nor `PCD`, every fine-map device page is
/// `PCD | PWT` (entry 3), and on the boot core this runs before `mmu::init` builds the fine map, on
/// a secondary before `mmu::init_secondary` adopts it. What is left is consistency between cores,
/// which is why it runs on every one: `smp::secondary_main` calls [`init`] too.
///
/// # Gated on CPUID
///
/// `CPUID.1:EDX[16]` advertises the PAT. Every `x86_64` processor this tree knows of has one
/// (from memory, not a specification citation). Without it `PWT` alone is write-through, which
/// for a screen nothing reads back is merely slow, and the boot line says so.
///
/// Name: provisional (the console-scroll lane).
pub(super) fn give_write_combining_a_page_attribute_entry() {
    /// `CPUID.1:EDX` bit 16: "PAT".
    const CPUID_1_EDX_PAT: u32 = 1 << 16;
    if isa::cpuid(1).edx & CPUID_1_EDX_PAT == 0 {
        crate::println!(
            "  pat         : not offered by cpuid on core {}; the screen is write-through here",
            crate::cpu::id()
        );
        return;
    }
    // SAFETY: CPUID advertised the MSR, so neither access can `#GP`, and every entry in the value
    // is a defined memory type. The one entry it changes is selected by no mapping on this core
    // (see above), so no cached line or TLB entry has the old type to disagree with.
    let found = unsafe { read_msr(paging::x86_64::pat::MSR) };
    if found != paging::x86_64::pat::VALUE {
        // SAFETY: as the read's.
        unsafe { write_msr(paging::x86_64::pat::MSR, paging::x86_64::pat::VALUE) };
    }
    crate::println!(
        "  pat         : {:#018x} on core {} (was {found:#018x}); entry 1 is write-combining",
        paging::x86_64::pat::VALUE,
        crate::cpu::id()
    );
}

/// Establish `CR4.SMEP` set, so ring 0 cannot execute an instruction fetched from a page whose
/// `U/S` bit says it belongs to ring 3.
///
/// # Why this is a confinement matter and not a hardening nicety
///
/// `crates/paging`'s x86 decoder reports a user page as *not* kernel-executable, and every
/// confinement test that reads a mapping through `Flags::is_kernel_executable` believes it. The
/// hardware disagrees unless this bit is set: x86 has one execute permission, `XD`, and it applies
/// at every ring, so a user code page with `XD` clear is executable at ring 0 too. RISC-V refuses a
/// supervisor fetch from a `U` page unconditionally and aarch64 has `PXN`; x86 gates the same
/// refusal behind a control-register bit that nothing here set. Milestone 313's audit found the
/// decoder's claim, and `notes/confinement-claims.md`'s sentence that "the hardware really does make
/// a user page non-executable in supervisor mode", both true only with this bit on.
///
/// What it costs: nothing on any path. SMEP is checked by the fetch unit against the leaf's `U/S`
/// bit; there is no per-access software toggle, which is what separates it from `SMAP` (`stac`/`clac`
/// around every kernel access to user memory, and the reason SMAP stays off, per
/// `mmu::permit_kernel_access_to_user_pages`'s `BUGS`). What it forbids: nothing this kernel does.
/// Every kernel page is mapped `U/S` clear, and a user program's code is only ever *entered* through
/// `iretq`/`sysret` at ring 3, never called from ring 0.
///
/// # Gated on CPUID, and honest on the console when absent
///
/// `CPUID.(EAX=7,ECX=0):EBX[7]` is the feature bit (Ivy Bridge and every Intel since; AMD from
/// Excavator; QEMU's `-cpu max`, HVF and the `q35` defaults all offer it). Setting `CR4.SMEP` on a
/// CPU that does not advertise it is `#GP`, so this reads first. A machine that does not offer it
/// gets a boot line saying that ring 0 can execute ring-3 pages there, because the decoder's answer
/// is then wrong on that machine and a reader of the transcript should know.
///
/// Per core, like `close_performance_counters_to_ring3` above it: `CR4` is per-CPU state.
pub(super) fn close_ring3_pages_to_ring0_execution() {
    /// `CR4.SMEP`: "Supervisor Mode Execution Prevention".
    const SMEP: u64 = 1 << 20;
    /// `CPUID.(7,0):EBX` bit 7 advertises SMEP.
    const CPUID_7_EBX_SMEP: u32 = 1 << 7;

    // Leaf 7 exists only when leaf 0 says the maximum basic leaf reaches it; `isa::init` makes
    // the same check before reading RDSEED out of the same word.
    let offered = isa::cpuid(0).eax >= 7 && isa::cpuid_count(7, 0).ebx & CPUID_7_EBX_SMEP != 0;
    if !offered {
        crate::println!(
            "  cr4.smep    : not offered by cpuid on core {}; ring 0 can execute ring-3 pages here",
            crate::cpu::id()
        );
        return;
    }

    let cr4 = instructions::read_cr4();
    if cr4 & SMEP != 0 {
        return;
    }
    // SAFETY: setting `CR4.SMEP` only *removes* a ring-0 permission the kernel never uses (no
    // kernel code lives in a `U/S` page). CPUID advertised the bit, so the write cannot `#GP`. Paging
    // bits are preserved; no TLB entry is invalidated, and the bit takes effect on the next fetch
    // without one, because SMEP is evaluated against the leaf at fetch time.
    unsafe { instructions::write_cr4(cr4 | SMEP) };
    crate::println!(
        "  cr4.smep    : set on core {}; ring 0 faults on a fetch from a ring-3 page",
        crate::cpu::id()
    );
}

/// Establish `CR4.PCE` clear, so ring 3 cannot execute `RDPMC`.
///
/// # The second door, and it is not the one milestone 228's block talks about
///
/// That block names `CR4.TSD` (bit 2), which gates `RDTSC`, and deliberately leaves it alone: this
/// architecture's `user_mode_runtime::now()` **is** `rdtsc` and there is no coarse counter to fall back to, so
/// closing it would take `Instant`, `thread::sleep`, the random seed, smoltcp's timestamps and the
/// benchmark harness away on one instruction. That trade is recorded in `notes/x86-port/user-mode-runtime.md` and in a
/// `BUGS` section beside `now()`, and nothing here changes it.
///
/// **`CR4.PCE` is bit 8 and gates a different instruction.** `RDPMC` reads a performance counter by
/// index, and fixed counter 2 (`CPU_CLK_UNHALTED.REF_TSC`) runs at the TSC rate, so an open `PCE` is
/// a second path to a cycle-rate instrument that has nothing to do with `TSD`. Nobody had looked
/// when 228 was minted, which is why its block named only two architectures as fixable.
///
/// This is the same defect the other two architectures had: a counter whose openness depended on
/// what firmware left rather than on anything this kernel decided. `boot.s` writes `CR4` exactly
/// twice, at the BSP and AP long-mode transitions, and both are `or eax, 1 << 5`, which is `PAE`.
/// Nothing has ever written bit 8 or programmed a perf-counter MSR.
///
/// # Why clearing it costs nothing, and what it would mean if it did
///
/// Nothing in this tree reads a performance counter from ring 3, or from ring 0 for that matter:
/// there is no perf-MSR programming anywhere under `arch/x86_64/`, so with the counters unprogrammed
/// an open `PCE` would let a process read zeros or firmware's leftovers rather than anything useful.
/// If clearing this ever breaks something, that is a finding rather than a reason to skip it, because
/// it would mean this tree already depends on a counter it never granted itself.
///
/// # It is read back, not assumed
///
/// The reset value of `CR4` is zero and this could have been a blind `and`. Assuming a reset value
/// is the exact habit milestone 228 exists to stop, so this reads `CR4` first and says so out loud
/// when the bit was set, which is the only case worth a line of boot output.
///
/// **On QEMU it is clear and this is silent, and that was read rather than assumed.** A temporary
/// probe printed `CR4` here on 2026-09-02 and got **0x20** on the PVH boot (`PAE` alone, which is
/// what `boot.s` sets) and **0x668** under OVMF (`DE`, `PAE`, `MCE`, `OSFXSR`, `OSXMMEXCPT`). Bit 8
/// was clear in both, so nothing was actually closed. But five bits this kernel never wrote were
/// already set by firmware before any of our code ran, on the one "firmware" this port has ever
/// booted under, and that is the argument for the read in one number. On the **Dell `OptiPlex`**
/// (xenon), real firmware runs first and the value is unknown.
///
/// Per core, like the GDT and the IDT above it: `CR4` is per-CPU state, so a secondary that skipped
/// this would run on whatever its own path left behind.
pub(super) fn close_performance_counters_to_ring3() {
    /// `CR4.PCE`: "Performance-Monitoring Counter Enable". Set means `RDPMC` is legal at any
    /// privilege level; clear means ring 0 only.
    const PCE: u64 = 1 << 8;

    let cr4 = instructions::read_cr4();
    if cr4 & PCE == 0 {
        return;
    }

    crate::println!("  cr4.pce     : firmware left RDPMC open to ring 3; closing it");
    // SAFETY: clearing `CR4.PCE` only *removes* a ring-3 permission. It does not touch paging
    // (`PAE`, bit 5, is preserved by the mask), does not invalidate any TLB entry, and cannot make
    // a kernel access illegal, since `RDPMC` at CPL 0 is legal whatever this bit says. Every other
    // bit is written back as it was read.
    unsafe { instructions::write_cr4(cr4 & !PCE) };
}
