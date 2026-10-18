---
status: DECIDED
raised: 2026-08-15
decided: 2026-09-04
ratified_by: calef
---

# 86. Whether an NVMe driver can leave the kernel, and what capability would let it

Answered 2026-09-03 by calef, after the research pass below: option 2a now,
designed so option 4 can be added without reshaping the EL0 driver, and the choice between them made
by measurement rather than argument. The reasoning is in "The decision" at the end of this section.
Raised by milestone 53's storage lane (2026-08-15, pull request #193), which
built the NVMe driver kernel-resident and stopped exactly here, correctly: the alternative needs
new syscall surface, and that is a boundary a lane does not cross (§10, §16).

## What is being decided

Every other DMA driver in this tree runs at EL0 behind a `Virtio` capability: the kernel owns the
queue addresses, parses each descriptor on its way to the device, and refuses any address outside
the driver's DMA region. That checkpoint exists because virtio's descriptors pass through a
doorbell path the kernel mediates.

NVMe has no such point to stand on. The controller fetches 64-byte commands directly out of
driver-written memory, and the DMA targets (the PRP fields) ride inside those commands. Nothing
kernel-side sees a command on its way past, so the `Virtio` capability's design does not transfer;
a confined EL0 NVMe driver needs a different contract. The question: does the NVMe driver stay in
the kernel, or does the syscall surface grow whatever lets it leave?

## The options

1. Stay kernel-resident, with the IOMMU as the whole confinement story. This is what is built.
   `bring_up` confines the controller's requester id to its six-page DMA region before enabling it,
   and on both `virt` machines an unconfined controller cannot fetch its first command, so
   forgetting the confinement fails loudly. The cost is architectural: the driver joins the TCB,
   against the microkernel thesis that drivers are user programs (§21, §23 spent real work getting
   virtio out).
2. A kernel-owned admin plane. The kernel keeps the admin queues (queue creation names physical
   addresses; that is the dangerous authority). It hands EL0 a capability to a pre-built I/O queue
   pair whose rings and data buffers all live inside the driver's own confined DMA region. The
   IOMMU bounds every fetch and every transfer to that region, so the kernel never parses commands;
   it only guarantees the geometry inside which any command is harmless. This is the `Virtio`
   capability's *shape* (kernel owns setup, EL0 owns the data path) with the IOMMU replacing
   per-descriptor validation.
3. Delegate the whole controller. Map BAR0 into the driver and let the IOMMU alone bound it,
   admin queues included. Simplest surface, but the driver can then re-point queues anywhere inside
   its region and the kernel cannot name which pages are rings versus data; and BAR0 contains every
   doorbell including the admin pair, so revocation semantics get murky. Recorded because someone
   will propose it, not because it is recommended.

## Recommendation

Option 2, **but not yet**. The recommendation is to hold this PROPOSED until the board-side work
(the JH7110's PLDA XpressRICH root complex, tracked as milestone 163, NOT-STARTED) exists. §23 built
multi-queue confinement only after single-queue worked for the same reason: the second data point
tells you which parts of the contract are QEMU artifacts. A second, possibly faster path to the same
data point may exist through milestone 87's x86 machine instead; see milestone 163's own text for
why that does not make 163 unnecessary. Option 1 is honest in the meantime because the limitation is
recorded in notes/non-volatile-memory-express.md's BUGS section rather than implied away.

A scoping lane has now reported on x86_64 (milestone 165, PARTIAL), and it is half an answer
rather than a whole one. Milestone 165 wired real PCI enumeration on x86_64 through ACPI's MCFG
and proved it under QEMU's `q35`. That is stronger evidence than an equivalent RISC-V QEMU proof
would be, because x86's discovery mechanism (ACPI tables naming an ECAM window) is the same one
real x86 hardware uses, unlike RISC-V's QEMU-only fake ECAM device. That closes the *discovery*
half of what a real data point needs. It does not close the *confinement* half this decision turns
on. x86 has no IOMMU driver at all (VT-d, roadmap item 6 of milestone 161, unbuilt), so a real
NVMe-behind-IOMMU controller cannot yet be brought up confined on either machine this tree targets.
This stays PROPOSED, pending either the JH7110 board bring-up or VT-d, whichever lands first. (A
separate lane was scoping a dedicated milestone for the JH7110 driver as this report was being
written; check the roadmap index for its number before citing it.)

The VT-d half has now landed and been exercised against this driver (2026-08-25), and the
*confinement* gap this section named is closed on x86_64. VT-d itself landed earlier this session
(milestone 161 item 6, `kernel/src/arch/x86_64/iommu.rs`) but had confined no real PCI device; this
is that exercise. `helpers/qemu-runner-x86_64.sh` now attaches `-device nvme` the same way the
aarch64 and riscv64 runners do (no `iommu_platform` flag, same as the other two, since a real PCI
device's DMA is not virtio's opt-in).
`kernel/src/nvme.rs::tests::the_nvme_disk_serves_the_block_interface_end_to_end` now runs for real
on all three architectures instead of skipping on x86_64. (An account: that module is
`kernel/src/non_volatile_memory_express.rs` since 2026-09-18, and milestone 261 replaced that test
with the EL0 one, so neither the path nor the symbol resolves today.) The result: the confinement
claim holds under VT-d exactly as it does under SMMUv3 and the RISC-V IOMMU. The existing driver,
unmodified in shape, enumerates the controller over ACPI's real MCFG, confines its requester id to
its six-page DMA region before enabling it, and serves SIZE/WRITE/READ over the blk-IPC verbs, on
QEMU's `q35` with `-device intel-iommu`.

Moved to [`vt-d-and-interrupts.md`](0086-el0-nvme-driver/vt-d-and-interrupts.md): the fixes the VT-d
exercise needed, and the paragraphs that closed this section.

## What is blocked until it is answered

- An EL0 `nvme_server` program (the block-server shape `fs_proto::blk` already speaks).
- `block_roster` growing an NVMe transport kind (the surveyor cannot list the NVMe disk today);
  small, but its wire shape depends on who owns the controller.
- Milestone 55's storage stack picking its final backend: it can benchmark against the
  kernel-resident driver meanwhile, and those numbers stay honest if they are labeled with it.

---

## The research pass, 2026-09-03

Everything above this line is the record as it stood. What follows is a research lane
(`maintainer/research-86-el0-nvme`, no code) answering the six questions AGENTS.md asks of a fork
before it reaches calef, so that this section can be decided by reading it. It appends rather than
rewrites, the way the two findings above it did.

### The hold is spent, and it was measuring the wrong variable

Moved to [`prior-art.md`](0086-el0-nvme-driver/prior-art.md): the research pass on the hold
condition.

### The premise is false, and that is the largest single finding

Moved to [`prior-art.md`](0086-el0-nvme-driver/prior-art.md): the NVM Express specification's
account of the doorbell, and what the premise garbled.

### The options, repriced

The three above stand, with one correction to option 2 and one addition.

Option 1, stay kernel-resident. Unchanged and still honest. The limitation is recorded in
notes/non-volatile-memory-express.md's `BUGS`.

Option 2, a kernel-owned admin plane. Still the right shape, and it is also what the field does
(see the prior art below). One thing it never priced: the split it describes is already a page
boundary in the hardware. NVMe puts the controller and admin registers at offsets 00h through
0FFFh (CC, CSTS, AQA, ASQ, ACQ among them) and the first doorbell at 1000h, with SQyTDBL at
`1000h + ((2y) * (4 << CAP.DSTRD))` and CQyHDBL at `1000h + ((2y + 1) * (4 << CAP.DSTRD))`
(revision 1.4c, sections 3.1.25 and 3.1.26). So "the kernel keeps the admin plane and EL0 gets the
data path" is expressible by mapping one page of BAR0 and not the other, with the `DeviceFrame`
capability this tree already has and **no new syscall surface at all.** That splits option 2 in two:

- 2a, map the doorbell page. Zero new surface. The catch is real and must be stated where a reader
  meets it. With `CAP.DSTRD` = 0, which section 8.6 calls "the expected doorbell stride value" for
  hardware implementations, the **admin** doorbells share that page with the I/O ones, so the EL0
  driver can ring the admin doorbell. It cannot write the admin submission queue if the kernel keeps
  the `PageFrame` capabilities for the pages the admin rings live in. Keeping them is entirely in
  the kernel's gift, since the kernel allocates and confines the region and mints every capability
  to it. What is left is that the driver can make the controller re-fetch admin slots the kernel
  wrote or never filled. That is a denial-of-service surface against the controller, not an escape,
  and the region-relative confinement is untouched by it.
- 2b, keep BAR0 and add a capability. The kernel maps no part of BAR0 to EL0 and exposes a
  doorbell-ring method, mirroring `Object::Virtio` exactly. Costs one enum variant and a few method
  constants, priced below.

Option 3, delegate the whole controller. Unchanged, and the interrupt finding below is one more
reason it is not recommended.

Option 4, new: mediate at the doorbell, the way §23 mediates virtio. 2b plus validation: on the
ring, the kernel walks the newly-published submission queue entries and bounds PRP1 and PRP2 into
the driver's DMA region. It copies each validated entry into a kernel-private shadow submission
queue (the one the controller was actually told about at Create I/O SQ time), and only then writes
the hardware doorbell. This is `dma_validator::validate_and_shadow`'s design transferred, and the
whole reason to want it is that **it does not depend on an IOMMU.** On a board with none, which is
every board this project owns, it is the only option in this list that confines anything.

### What this tree already does, item by item

Moved to [`prior-art.md`](0086-el0-nvme-driver/prior-art.md): the table of what an EL0 NVMe driver
needs and what supplies it today.

### What each option costs, counted

Adding a capability, measured against the precedent. `Object::Virtio` is one enum variant
(`kernel/src/cap.rs:124`), **six sites** across `kernel/src`, and **four method constants** in
`crates/abi`. It added **no new syscall number**: it dispatches through the existing `invoke`, which
is the pattern §16 (object revocation) established when it grew `Untyped`. One constraint to respect
if a variant is added: `kernel/src/cap.rs` asserts `size_of::<Object>() == 24` at compile time, so
the new variant must carry no more than `Virtio(usize)` does.

How much kernel code leaves, and how much stays. `kernel/src/non_volatile_memory_express.rs` is 474 lines, 416 of them
before `mod tests`.

| Piece | Lines | Under option 1 | Under 2 or 4 |
|---|---|---|---|
| `Nvme::new`, the admin plane (reset, admin queues, enable, identify, create the I/O pair) | 111 to 186, **76** | kernel | kernel |
| The data path (`size_bytes` through `next_cid`: transfer, transact, the polling loop) | 187 to 343, **157** | kernel | EL0 |
| `wait_rdy` and the register accessors | 344 to 378, **35** | kernel | split |
| `bring_up`, the policy function | 391 to 414, **24** | kernel | kernel |
| `crates/nvme`, pure arithmetic, already host-tested | **716** | linked by the kernel | linked by the EL0 program |

So roughly 157 lines leave the kernel and about 100 stay. The EL0 program's template is
`components/src/block_driver.rs`, which is **65 lines**: it is a thin shell around a driver's logic
serving `filesystem_proto::blk`, which is the shape an `nvme_server` takes.

Option 4's validator is smaller than the thing it is modeled on. `crates/dma_validator` is 1082
lines including its harnesses, and most of that is walking descriptor **chains** with indirect
descriptors and a per-queue high-water mark. NVMe has no chain: per newly-published entry it is two
`u64` range checks (PRP1, PRP2), an opcode check, and a 64-byte copy into the shadow slot. It grows
one level of recursion the day a PRP list appears, which today it cannot, and which milestone 55
(Time Machine) is what would change.

What option 4 costs at run time is the honest open question and it is measurable rather than
arguable. Today's driver completes one command before submitting the next (notes/non-volatile-memory-express.md's `BUGS`),
so the per-command copy is invisible against a QEMU round trip. At real queue depth it is a syscall
per batch plus 64 bytes copied per command, and `script/bench` is what would say whether that
matters. Nobody should assert it either way from this section.

### Prior art, fetched rather than recalled

Moved to [`prior-art.md`](0086-el0-nvme-driver/prior-art.md): seL4, Genode, Redox, Linux VFIO and
SPDK, and software mediation of NVMe queues.

### The axis nothing in this section has named: who may forge an interrupt

Moved to [`vt-d-and-interrupts.md`](0086-el0-nvme-driver/vt-d-and-interrupts.md): interrupt
remapping on each architecture, with its 2026-09-18 amendment, and where the gap is recorded.

### Reversibility, and who has acted

Moved to [`vt-d-and-interrupts.md`](0086-el0-nvme-driver/vt-d-and-interrupts.md): each option's
reversibility, and who has acted on it.

### What this lane would decide, and what is left to calef

Moved to [`prior-art.md`](0086-el0-nvme-driver/prior-art.md): the research lane's reading of the
options, and what it left to calef.

## The decision, 2026-09-03

Option 2a now, designed so option 4 can be added without reshaping the EL0 driver, and the choice
between them settled by measurement. calef, after the head-to-head below: *"I agree with your
recommendation."*

### A correction to the research pass, which changes option 4's central claim

The pass argues that option 4 is the only entry that confines anything on hardware this project
owns, because no board here has an IOMMU. That is true of radon and false of xenon, and milestone
87's own requirements list says so: xenon was selected partly *for* VT-d, "because IOMMU-backed
driver isolation (milestone 16) is a parity theme (§19), and the x86 side of the DMA-confinement
story needs real hardware eventually."

So option 2 has a hardware home on a machine already on the desk. Option 4's argument survives for
radon and for any board without an IOMMU; it is not the categorical advantage the pass states.
Whether xenon's firmware actually exposes a DMAR is unverified and is already the last item of
notes/x86-uefi-boot.md's bench procedure.

### Why 2a first, and it is a sequence rather than a preference

**The irreversible part of this decision is not in 2a.** It costs zero new syscall surface: the
doorbell page is a `DeviceFrame`, the DMA run a `PageFrame` (§102), the physical base a spawn
argument the way `components/src/entropy.rs` already passes one. A capability variant belongs only to 2b
and 4. So building 2a produces what fatal risk 6 (a capability-confined userspace driver cannot
drive real hardware at real speed) needs while deferring the expensive choice, and the *move fast
on what can be undone* test, "who else has already acted on this", answers nobody.

It moves 157 lines out of the kernel, which is the microkernel thesis cashing out and the same
win §23 bought for virtio.

And option 4 is a superset of 2b rather than a rival. Which submission queue the controller is
told about is an admin-plane decision the kernel owns under both, so adding mediation later changes
`bring_up` and the ring path and almost nothing in the EL0 driver. That is the property the
implementation must preserve, and it is this decision's one design constraint.

### The two options answer different risks, which is why measurement decides

| | option 2a | option 4 |
|---|---|---|
| what confines it | the IOMMU | kernel validation at the doorbell |
| hardware needed | an IOMMU: xenon yes, radon no | none |
| new syscall surface | **zero** | one capability variant, no new syscall number |
| kernel on the hot path | no, EL0 writes the doorbell | yes, a trap per ring plus two range checks and a 64-byte copy per command |
| reachable by Kani | no, it is a hardware claim | yes, `crates/dma_validator` has harnesses |
| prior art | mainstream: seL4, Genode, VFIO, SPDK | novel |

Option 2a optimizes risk 6 by removing the kernel from the data path. Option 4 optimizes
risk 7 (the confinement claim is false) and risk 2 (the proofs prove trivia), and pays for it
on the hot path.

### What would move this to option 4, stated in advance

**Three measurements, and two need no new code.**

1. What doorbell mediation costs, priced from the mechanism already shipped. §23's virtio path
   *is* option 4's design: `crates/dma_validator` walks descriptors on `NOTIFY` and shadows them,
   and `script/bench` carries icount baselines on all three ISAs. NVMe's validator is strictly
   cheaper (two range checks and a copy against chain-walking), so virtio's number is an upper
   bound.
2. The depth at which it stops mattering. One trap amortized over a 32-command batch is noise;
   one trap per command at depth 1 is the whole cost. Milestone 101 (the L4 calibration, read from
   the IPC number that pays for the trap) is the calibration to read it against.
3. Whether xenon exposes a DMAR, which decides whether option 2 is viable on owned hardware at
   all. One boot, already the last item of notes/x86-uefi-boot.md's bench procedure.

If mediation is cheap at realistic queue depths, take option 4. A confinement claim that holds
without special hardware, and that Kani can reach, is worth more to this project than the last few
percent of throughput; that is the demonstrator's own argument. The reason not to buy it tonight is
that nobody knows the price.

### What this does not decide

The interrupt gap the pass found is orthogonal and survives both options. A confined component's
MSI-X write is a memory write to a special address, so DMA remapping does not cover it, and this
machine runs `-device intel-iommu` with no `intremap=on` and `gic-version=2` with no ITS. It is
latent while every BAR-reaching component is the kernel and live the moment one is not, which is
what this decision creates. It is recorded in notes/confinement-claims.md and is not answered here.
