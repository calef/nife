---
status: BUILT
raised: 2026-09-17
built: 2026-09-17
---
# 317. The interrupt-remapping flags, and where MSI confinement actually lives

[DECISIONS §86](../decisions/0086-el0-nvme-driver.md)'s research pass found the gap while pricing
an EL0 NVMe driver.

## What this is, and what it deliberately is not

This makes interrupt confinement askable on all three architectures. It confines no interrupt,
and it does not decide who may.

The milestone was cut as "add two runner flags" and the flags are the smallest part of it. What it
found is that MSI confinement lives in a different place on each of the three architectures, and
exactly none of the three is exercised today, for three different reasons, only one of which
anybody had written down.

| | where MSI confinement lives | what this machine offers | what this kernel does |
|---|---|---|---|
| `x86_64` | a separate IOMMU feature: VT-d interrupt remapping | offered (`ECAP.IR` set, and it always was) | never sets `GCMD.IRE` |
| aarch64 | a separate *device*: the GICv3 ITS | absent (`gic-version=2` has no ITS) | cannot reach the question |
| riscv64 | one mode field inside a device context this driver already writes | offered (`CAPS.MSI_FLAT` set) | writes `msiptp = 0`, mode Off |

The asymmetry is the finding. §86's survey covered two architectures and concluded the question was
unaskable; the third one has the mechanism sitting inside a structure this tree already programs.

An MSI or MSI-X message is a memory write to an architecturally special address, so an IOMMU doing
DMA remapping alone does not confine it: a component that can write a device's MSI-X table can aim
an interrupt at a vector it was never given. VFIO refuses to hand a device to an untrusted
userspace driver on a machine without interrupt remapping for this reason, and names its escape
hatch `allow_unsafe_interrupts`.

§86 found that no boot this tree runs could exercise the question even if a claim existed, and
`notes/confinement-claims.md` carries it as a claim stated nowhere. Two runner flags were named as
the cheap first move. This milestone is those two flags, the third architecture §86 did not look
at, and the one thing that makes any of it worth having: each machine description now says, from
inside the guest, what that machine offers.

Both of §86's findings were reached by reading the runner scripts, and one of them is wrong.
`x86_64` has been offering interrupt remapping in every boot this tree has ever run, and the only
reason nobody knew is that no code read the bit. aarch64's finding survives contact with the
machine, and the machine adds something to it. riscv64 was not surveyed at all.

So the first deliverable is not a flag. It is that each architecture now reports its own position
from inside the guest, in its machine description, rather than from a source comment. That is
what makes any of the rest checkable, and it is what turned two of the three answers around.

Who owns the page holding the MSI-X table is not decided here. calef held that decision on
2026-09-17 for want of an experiment behind it. This is the experiment.

## x86_64: the premise was false, and that is the finding

§86 (whether an NVMe driver can leave the kernel) said interrupt remapping is off in every x86_64 boot this tree runs. It has been on the whole
time. §86 read `helpers/qemu-runner-x86_64.sh`, which attaches `-device intel-iommu` with no
`intremap=on`; booting the machine and printing `ECAP` from inside the guest shows `ECAP.IR` set.
QEMU's `intremap` defaults to `auto`, which resolves ON with no in-kernel irqchip, and patagonia has
no KVM. Nothing in this kernel read the bit, so nobody could have noticed.

What was built: `NIFE_INTREMAP` (provisional name) passes `intremap=` through, `on` or `off`, and
unset leaves QEMU's default, so no existing boot changes. The useful value is `off`, the only way to
reach a machine without the capability. The guest's machine description reports what it found,
through `arch::x86_64::iommu::interrupt_remapping_available`, and a test asserts `GSTS.IRES` stays
clear whatever `ECAP.IR` says. The suite passes on both machines. The runner does not add
`kernel-irqchip=split`, which is a KVM constraint this machine cannot check. The knob's default
stays unset, deferring to QEMU.

- [x86-64.md](0317-interrupt-remapping-flags/x86-64.md): the `ECAP` readings by invocation, the
  knob and the lines the guest prints, the suite on both machines, the `kernel-irqchip` check, and
  the reasoning on the default.

## aarch64: it cannot be switched yet, and here is precisely why

This section is the tree of 2026-09-17. Milestone 227 (a GICv3 driver) was built on 2026-09-19:
the kernel now reads `compatible`, drives both versions and boots `gic-version=3` at 5 of 5, and
the runner no longer warns. Items 1 and 2 of the bill below are paid; the ITS (item 3), and the
parity gap that waits on it (item 4), are still owed.

`gic-version=3` is one flag. Surviving it is milestone 227. QEMU accepts the machine; this
kernel does not survive it, and the failure is silent rather than loud, which is the part worth
recording.

Measured with `dumpdtb` on QEMU 11.1.1, both versions of the same `virt` machine:

| | `gic-version=2` | `gic-version=3` |
|---|---|---|
| node | `intc@8000000` | `intc@8000000` |
| `compatible` | `arm,cortex-a15-gic` | `arm,gic-v3` |
| `reg[0]` | `0x8000000 + 0x10000` (GICD) | `0x8000000 + 0x10000` (GICD) |
| `reg[1]` | `0x8010000 + 0x10000` (GICC) | `0x80a0000 + 0xf60000` (GICR) |
| MSI child | `v2m@8020000` | `its@8080000` |

The ITS is right there for the taking, which is what makes this tempting and what makes the rest of
the row matter.

`memory::init` finds the controller by the `intc@` name prefix and never reads `compatible`, so
it matches a GICv3 node just as happily, takes `reg[1]`, and hands it to `arch::aarch64::irq::init`
as "the CPU interface". On a GICv3 that region is the redistributor frame array, whose register
layout has nothing to do with a GICv2 CPU interface. Nothing anywhere refuses the mismatch.

`NIFE_GIC=3 cargo xtask boot-check --arch aarch64` (the flag is this milestone's, and the runner
prints a warning saying the configuration is unsupported):

    interrupts      : GICv2, distributor 0x0000000008000000, cpu interface 0x00000000080a0000
    self-test       : timer      FAILED  the 62MHz counter advanced 1250000 and 0 tick(s) arrived in ~20ms
    self-test       : scheduler  FAILED  thread 5 did not run within 2s (saw 0x0)
    nife self-test: 3 of 5 passed, 2 FAILED: timer scheduler

Against `gic-version=2`, the same command is 5 of 5.

Read the first line twice. The kernel says "GICv2" and prints the redistributor base under the
heading "cpu interface". It is not confused; it never asked. The machine description is stating a
belief, and on this machine the belief is false. That is the shape of failure this tree fears most:
a record that is wrong and looks like a reading.

### What milestone 227 would owe

Four things, and only the second is the large one its block warns about:

1. A machine that says which controller it has. `memory::init`'s `intc@` prefix must read
   `compatible` and distinguish `arm,cortex-a15-gic` from `arm,gic-v3`, and
   `arch::aarch64::irq::print_summary` must report what it found rather than the literal `GICv2` it
   prints today. This is worth doing even if 227 never starts, because it converts a silent
   wrong answer into a refusal; it is not done here only because it is 227's file and this lane is
   not 227.
2. The GICv3 driver. Distributor programming moves to affinity routing, the CPU interface moves
   from MMIO to system registers (`ICC_*`, which is real `arch/` work), and the redistributor gets
   a per-core frame with its own wake protocol. This is the large piece.
3. The ITS, which is the whole reason interrupt remapping wanted GICv3. Command queue, device
   table, interrupt translation tables, and `MAPD`/`MAPTI`/`INV`. Without it `gic-version=3` buys a
   different interrupt controller and no MSI translation, which is not what this question is about.
4. Parity's bill. DECISIONS §19 makes a kernel capability ship on every architecture or carry a
   scope note. x86_64 can offer interrupt remapping today and aarch64 cannot, so any claim built on
   it starts life with a recorded gap and this block is where it points.

## riscv64: offered, declined in one word, and untestable on silicon we own

§86 did not survey this architecture, and it is the one where the mechanism is closest to hand: MSI
confinement sits in the device context `arch::riscv64::iommu::attach` already writes. A boot reads
`CAPS.MSI_FLAT` set, the machine description now says `MSI page table offered (mode off)`, and
`attach` writes the four MSI words zero, so `msiptp.MODE = Off`. No flag is needed or available, and
`helpers/qemu-runner-riscv64.sh` is unchanged. riscv64 is also the inverse of `x86_64`: no silicon
ships the ratified IOMMU (milestone 143 (silicon IOMMU)), so its answer can never get a second witness, where xenon
waits for `x86_64`'s. What `MODE = Off` does to an MSI-shaped write is a reading of the
specification, not a measurement.

- [riscv64.md](0317-interrupt-remapping-flags/riscv64.md): the `CAPS` reading, why no flag exists,
  the table of the three architectures emulated and on silicon, and the specification reading.

## EXAMPLES

Run the x86_64 suite against a unit that does not offer interrupt remapping, which is the side
of the comparison that needed a flag to reach:

    NIFE_INTREMAP=off script/test --arch x86_64

See what the guest makes of the machine, both ways:

    cargo xtask boot-check --arch x86_64                    # ... interrupt remapping offered
    NIFE_INTREMAP=off cargo xtask boot-check --arch x86_64  # ... interrupt remapping absent

The `iommu` line is in `target/boot-check-x86_64.log`.

Ask each machine where it stands, which is the one command that makes this milestone checkable:

    cargo xtask boot-check --arch x86_64   # ... interrupt remapping offered (unused)
    cargo xtask boot-check --arch aarch64  # interrupts : GICv2, ... (no ITS to report)
    cargo xtask boot-check --arch riscv64  # ... MSI page table offered (mode off)

Boot the GICv3 machine, which was 3 of 5 here on 2026-09-17 and is 5 of 5 since milestone 227:

    NIFE_GIC=3 cargo xtask boot-check --arch aarch64      # interrupts : GICv3, ... 5 of 5

Compare the device trees the two GIC versions produce, which is how the table above was made:

    qemu-system-aarch64 -machine virt,gic-version=3,iommu=smmuv3 -cpu cortex-a72 \
        -display none -dumpdtb=/tmp/gic3.dtb
    dtc -I dtb -O dts /tmp/gic3.dtb | grep -A 12 'intc@'

## BUGS

- Nothing here remaps an interrupt, and the reported bit only says the hardware offers it. No
  `IRTE` is allocated or written, `GCMD.IRE` is never set, and no MSI is forged and traced to a
  vector it was not given. `interrupt_remapping_is_reported_and_never_enabled` asserts `GSTS.IRES`
  is clear precisely so this sentence cannot quietly stop being true, but an assertion that
  remapping is *off* is not evidence about what remapping would *do*.
- DECISIONS §86 and `notes/confinement-claims.md` both carry the false half of the premise, and
  a lane may not edit the former. §86's "interrupt remapping is off in every x86_64 boot this tree
  runs" needs the correction above; the note's copy of the same sentence has it already. This is
  flagged for the integrator rather than fixed here.
- riscv64's `MODE = Off` semantics are a spec reading, not a measurement. What the hardware
  does with an MSI-shaped write from a device whose context has no MSI page table is stated in the
  specification and has not been observed here. It is flagged in
  `kernel/src/arch/riscv64/iommu.rs`'s `BUGS` as well, where a reader meets the code.
- riscv64's base 32-byte device-context branch has run zero times and cannot be made to. No
  silicon ships the ratified RISC-V IOMMU (milestone 143), so the `MSI_FLAT`-absent path in
  `attach` is dead code kept for a machine that does not exist. Recorded rather than removed,
  because deleting it would assert that no such machine ever will.
- So the confinement claim is still stated nowhere, and `notes/confinement-claims.md` still
  carries it as such, now pointing here for the flags. What would settle it is a test in milestone
  202's shape: give a component the MSI-X table page, have it aim an interrupt at a vector it was
  never granted, and assert the hardware refuses. That needs a driver outside the kernel that wants
  interrupts, which is §86's question and not this one.
- `NIFE_GIC=3` was added to reproduce a known-broken boot for milestone 227, which fixed that boot
  on 2026-09-19. It is now an ordinary runner choice (`helpers/qemu-runner-aarch64.sh`: TCG
  defaults to 2, HVF to 3). The ITS is still undriven (`kernel/src/drivers/gicv3.rs`'s BUGS).
- The correction is about QEMU under TCG on one machine, not about VT-d. `intremap`'s `auto`
  resolving ON is QEMU's behavior with no in-kernel irqchip. Under KVM it resolves differently and
  the original reading could well be right there. Nobody has checked what the CI Linux runners do,
  and milestone 87's `OptiPlex` (xenon) is real silicon whose firmware answers this question in its
  own way.
- `NIFE_INTREMAP` and `NIFE_GIC` are provisional names. Names are an architect's
  (`AGENTS.md`); these follow the existing `NIFE_*` runner-knob convention (`NIFE_SMP`, `NIFE_CPU`,
  `NIFE_EL2`, `NIFE_DISK`) and nothing outside this repository has acted on either.
- The x86_64 measurement is one machine. QEMU 11.1.1, TCG, macOS on Apple Silicon, no KVM. The
  `kernel-irqchip` finding above is specifically a statement about that machine, and the CI Linux
  runners have not been checked.
- `ECAP.IR` is a capability bit, not a working unit. It says QEMU's model advertises interrupt
  remapping. Whether that model's remapping behaves like silicon's is a question no boot here asks,
  and milestone 87's `OptiPlex` (xenon) is where it would get a second witness.

## Follow-on

- **Recorded.** That nothing here remaps an interrupt and that the reported bit only says the
  hardware offers it, in `kernel/src/arch/x86_64/iommu.rs`'s `BUGS` section beside the driver, and
  in this block's own `BUGS`.
- **Decision.** `design/decisions/0086-el0-nvme-driver.md` states that interrupt remapping is off in
  every x86_64 boot this tree runs, and the machine says otherwise. A lane does not edit
  `design/decisions/`, so the correction is recorded here, in the driver's `BUGS`, and in
  `notes/confinement-claims.md`, and the integrator owes §86 the amendment.
- **Recorded.** That the confinement claim about where a device may *interrupt* is still stated
  nowhere, in `notes/confinement-claims.md`, whose entry now carries what the flags measured and
  what is left.
- **Recorded.** Milestone 227 (a GICv3 driver) paid items 1 and 2 of "What milestone 227 would
  owe" on 2026-09-19. The ITS, item 3, is still undriven, recorded in
  `kernel/src/drivers/gicv3.rs`'s BUGS, so item 4's parity gap stands.
- **Decision.** `design/decisions/0086-el0-nvme-driver.md`: who owns the page holding the MSI-X table.
  calef held it on 2026-09-17 for want of an experiment behind it, and this milestone is that
  experiment. It is now answerable on x86_64 without any further machinery, and the honest cost of
  answering it on aarch64 is milestone 227.
- **Recorded.** riscv64's position, in `kernel/src/arch/riscv64/iommu.rs`'s new `BUGS` section
  beside `attach`: the MSI page table is offered (`CAPS.MSI_FLAT`, measured), every device context
  is the extended 64-byte format, and all four MSI words are written zero. The `MODE = Off`
  semantics are marked there as a spec reading rather than a measurement.
- **Recorded.** That riscv64's base 32-byte context branch is unreachable on any silicon that
  exists, in the same `BUGS` section, pointing at milestone 143's hardware gate.

## Index row

An MSI is a memory write to a special address, so DMA remapping does not confine it and a component
that can write a device's MSI-X table can aim an interrupt at a vector it was never given. DECISIONS
§86 surveyed two architectures from the runner scripts and concluded the question was unaskable
here. Booting all three says something different: MSI confinement lives in a different place on
each of them, and none of the three is exercised. `x86_64` has had it offered in every boot this
tree ever ran (QEMU's `intremap` defaults to `auto`, which resolves ON with no in-kernel irqchip),
and nothing read the bit; `NIFE_INTREMAP=off` is the flag that reaches the machine without it, and
a test asserts `GSTS.IRES` stays clear whatever `ECAP.IR` says. riscv64 was never surveyed and is
the closest of the three: `CAPS.MSI_FLAT` is set, every device context is the extended format, and
`attach` writes `msiptp = 0`, so the mechanism is one word inside a structure this driver already
programs, and no flag is needed or available. It is also the one architecture whose answer can
never get a second witness, since no silicon ships the ratified IOMMU (milestone 143), which
inverts `x86_64`, where xenon is waiting. aarch64's half of §86 survives, and turned out not to be
one flag away, and finding out how was the point: `gic-version=3` moves `reg[1]` from the CPU
interface to the redistributor, `memory::init` matches the node by the `intc@` name prefix and never
reads `compatible`, and the result is a boot that prints `GICv2, cpu interface 0x80a0000` and
receives no timer tick. Milestone 227's bill is itemised here, and §86's MSI-X ownership decision is
now answerable on x86_64 without further machinery.
