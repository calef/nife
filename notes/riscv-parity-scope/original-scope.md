# The original parity scope, and the records it superseded

An appendix to [notes/riscv-parity-scope.md](../riscv-parity-scope.md), moved there on 2026-10-11
(UTC) under §212 (a prose budget). Every workstream is done. These are the scopes as written
before each was built, the superseded "blocked" record for C, and the recommended sequence, kept
because the plan is what was built and the wrong record is instructive.

## A (original scope). SMP (multi-hart): L, high risk. The only pure-kernel gap.

RISC-V runs single-hart today; `send_reschedule` is a no-op and the runner passes `-smp 1`. This is
the last big *primitive* aarch64 claims and riscv does not, and the only parity item that is genuine
new kernel work rather than porting userspace.

- Prerequisite: per-hart trap state. The leak-free `tp` fix uses a single global `KERNEL_TP`,
  and the trap frame is placed below `sp`. SMP needs each hart to have its own per-CPU pointer and
  trap-frame area: the standard approach is `sscratch` holding this hart's kernel context, swapped in
  at trap entry, set up per-hart at bring-up. This refactor touches the trap path (trap.s) and is the
  enabling step; it was flagged as a follow-up during the `tp` saga.
- Secondary bring-up via SBI HSM (`sbi_hart_start(hartid, addr, opaque)`). The boot hart starts
  the others into a secondary entry path that mirrors aarch64's `secondary_main`. It must set `stvec`, `tp`/
  `sscratch`, adopt the kernel `satp`, arm the timer (`sie.STIE`), create an idle thread and run
  queue, become a scheduler participant.
- Per-hart PLIC context. `plic::init` hardcodes context 1 (hart 0 S-mode). Each hart's context
  is `2*hart + 1` and needs its own threshold and enable bits; wire this into `arch::irq::init_this_cpu`
  (a no-op today).
- IPIs via the SBI IPI extension (`sbi_send_ipi`) → supervisor software interrupt (`scause` = 1)
  → drain inbox + reschedule, the riscv twin of the `RESCHED_SGI` path. This lights up the
  `send_reschedule` no-op.
- Cross-hart shootdown: `sfence.vma` / `fence.i` IPIs for TLB and icache coherence.
- Proves: the scheduler and capability model are SMP-safe on a second *weakly-ordered* ISA. The
  weak-memory discipline (built for ARM) should carry over; SMP is where it gets its second witness.

## B (original scope). In-kernel test suite on RISC-V: M, low-medium risk. Highest value per effort.

The 116 kernel tests boot under QEMU on aarch64 and signal pass/fail via semihosting exit. RISC-V has
no in-kernel test run. The hard part is already done: `arch::semihosting::exit` exists on riscv
(QEMU virt's test-finisher). What remains:

- `xtask test()` grows a riscv kernel-test build + boot (the `riscv64imac` target, the riscv runner,
  TCG for deterministic semihosting).
- Gate the arch-specific tests (the SGI-triggered interrupt tests) to aarch64; the rest test portable
  logic (scheduler, caps, page-table math) and should pass on riscv unchanged.
- Proves: the same verified behavior holds on both arches. This is the strongest *parity signal*
  there is, and it aligns with the verified-Rust thesis. Do it first: cheap, and it makes every later
  parity claim checkable on riscv.

## C (superseded record). PARTIAL (kernel-side discovery done; blocked on QEMU transport).

The kernel-side enumeration is arch-correct on RISC-V now. The virtio-mmio slot layout (32 slots
0x200 apart on aarch64, 8 slots 0x1000 apart on riscv) moved into arch constants. The transport
window is mapped device-typed, and `find_block_device` probes it and reads valid magic on riscv. The
userspace driver itself is nearly portable (342 lines, one `dmb ish` to arch-gate to `fence`).

Blocked, and honestly: QEMU 11's riscv `virt` does not auto-plug `-device virtio-blk-device` into
the virtio-mmio slots (all eight read magic ok but device-id 0 = empty); it prefers the PCIe
transport. So there is no mmio block device for the kernel's mmio driver to find. Finishing C needs
either a way to force a virtio-mmio disk on riscv `virt`, or a PCIe virtio transport (a larger driver
change). Then it needs the userspace driver extracted from `hello` into a portable binary. That
binary is granted the DMA region, the device MMIO and an Irq cap (the PLIC path from parity's
earlier work), and needs the `virtio_service` wiring. aarch64's virtio works fully (the
userspace-driver-reads-a-disk test passes);
this is a transport-availability gap on riscv, not a kernel defect. Original scope below.

## C (original scope). virtio-blk + on-disk filesystem: M. The driver + DMA model.

aarch64 runs a userspace virtio-blk driver that reads nifefs off a virtio-mmio disk, with the
kernel touching no DMA. RISC-V has the MMIO constants (`VIRTIO_MMIO_BASE`, `VIRTIO_IRQ_BASE`) but no
driver run. The virtio-mmio driver is largely portable (MMIO + virtqueues + DMA).

- Kernel: `find_block_device` (from the DTB `virtio_mmio@` nodes or by probing), route the device's
  PLIC IRQ to the userspace driver (the routing mechanism is done), hand it DMA-capable frames.
- Runner: attach a `virtio-blk` disk (`NIFE_DISK`, as aarch64 does).
- Proves: userspace device drivers *with DMA* on the second arch, and the "kernel issued no
  virtio command and touched no DMA" claim on riscv. Self-contained; depends on nothing else here.

## D (original scope). Full integrated boot + interactive shell: M–L. Mostly userspace porting.

aarch64 boots the userspace progenitor as the boot process, which builds the whole system (console + input +
shell + spawn service). RISC-V demonstrates the progenitor building *one* worker, then halts. Closing this is
mostly porting userspace, not proving new kernel behavior.

- Port the device-specific programs to the NS16550: `console.rs` (writes the UART, ~6 PL011 register
  sites) and `input.rs` (reads RX + the UART IRQ, ~2 sites). Either parameterize the register layout
  or ship NS16550 variants. `swish.rs` is already mostly portable (IPC, no direct hardware).
- A riscv `spawn_progenitor` (or a generalized one) that grants the PLIC/NS16550 equivalents of the
  GIC/PL011/IRQ capabilities aarch64's grants.
- Wire the riscv boot to hand off to progenitor-as-PID-1 instead of halting.
- Proves: the full interactive system runs on riscv. Lowest *kernel* value of the list; highest
  app-porting cost. Do last, or skip if the goal is "prove the kernel," not "ship the system."

## E (original scope). Benchmarks: M. Cross-arch numbers.

aarch64 runs CoreMark and the os_primitives_benchmarker EL0 primitive suite (null syscall, context
switch, IPC RTT, map,
spawn), plus cross-OS comparisons. RISC-V runs none. The workloads are userspace and mostly portable
(`coremark` is compute; `os_primitives_benchmarker` uses `user_mode_runtime::now`, which is `rdtime`
on riscv).

- Make the `bench` boot mode reachable on riscv.
- ~~Resolve the timing caveat honestly. `user_mode_runtime::cntfrq` is hardcoded to the QEMU virt
  10 MHz timebase on riscv (there is no `CNTFRQ` register). A real number needs the frequency handed
  to userspace (an aux-vector entry from the DTB `timebase-frequency`).~~ Done 2026-09-21, and not
  the way this
  line predicted. No aux-vector: the kernel writes the device tree's rate into a page and maps it
  read-only into every process, which is the mechanism `x86_64` already used
  (`counter_frequency_protocol`). Userspace reads the machine's number on all three architectures now,
  and a process that cannot learn it refuses rather than substituting a constant.
- Proves: comparable performance on a second arch: the "measure, don't argue" ethos, with riscv
  as a new data point next to the L4 lineage. Depends on the timing fix for the numbers to be honest.

## Dependencies and sequence

```
B (tests) ─────────────── independent, enables checking everything after
A (SMP) ── needs per-hart trap-state refactor (self-contained otherwise)
C (virtio) ────────────── independent
E (bench) ── needs the timebase-to-userspace fix for honest numbers
D (boot/shell) ── needs console/input ported to NS16550
```

Recommended order, by kernel-value-per-effort:

1. B: tests. Cheapest real win; the semihosting primitive is already there. Makes the same suite
   green on both arches and every later claim checkable. Start here.
2. A: SMP. The last true primitive, and the only new kernel work. Highest value for "the kernel
   is portable," highest risk. The per-hart trap refactor is the gate.
3. C: virtio + DMA. Self-contained; proves the driver/DMA model on riscv.
4. E: benchmarks. Cross-arch numbers, once the timebase caveat is fixed.
5. D: full boot / shell. Most app-porting, least new kernel proof. Do last, or treat as optional.

If the goal is "the kernel is at parity," A + B + C + E is the set, and D is system integration
rather than a kernel claim. If the goal is "the whole system runs on riscv," add D.
