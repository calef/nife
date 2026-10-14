# The MMU (Memory Management Unit)

The piece of hardware that makes an operating system possible at all.

## The problem

Without an MMU, `load x0, [0x40001000]` reads physical byte `0x40001000` from the RAM
chip. Directly. Every program can read and write every byte of memory, including the
kernel's and every other program's. There is no such thing as a "process" in any
meaningful sense, because there is no wall between them.

## What it does

The MMU is hardware between the CPU and the memory bus. When it's on, every address the
CPU emits is a virtual address, and the MMU translates it to a physical address
before it reaches RAM.

Translation is driven by **page tables**: a tree the *kernel* builds in RAM and hands to
the hardware by writing its physical address into `TTBR0_EL1` / `TTBR1_EL1`. The kernel
writes the rules; the hardware then enforces them on every memory access, at full speed,
forever.

Three things fall out, and they are the foundation of modern computing:

Isolation. Each process gets its own page tables. Process A's virtual `0x1000` and
process B's virtual `0x1000` map to *different physical RAM*. Neither can name the
other's memory. Not "isn't allowed to." *Cannot express the address.*

Protection. Each mapping carries permission bits: readable, writable, executable.
Map code read-execute and data read-write, and a buffer overflow can no longer turn
injected data into running code.

Illusion. A process sees a clean, contiguous address space even though the physical
pages backing it are scattered wherever RAM was free. This later buys demand paging,
copy-on-write `fork`, memory-mapped files, and swap.

## Page faults

Touch an unmapped address, or violate the permissions, and the MMU raises an exception
and hands control to the kernel. That's a page fault.

Sometimes the kernel fixes it and resumes (the page was swapped out; copy-on-write needs
to duplicate it). Sometimes it kills the process. That's a segfault.

## Mechanics

Translation happens per page (4 KiB, typically), not per byte, because a per-byte
lookup table would be larger than the memory it describes.

Page tables are a radix tree. On aarch64 with 4 KiB pages and 48-bit addresses: 4
levels of 9 bits each (512 entries per table), plus 12 bits of offset within the
page. 9 x 4 + 12 = 48.

Walking a 4-level tree on every memory access would be brutally slow, so the CPU caches
recent translations in a TLB (translation lookaside buffer). Which means: **when the
kernel changes a mapping, it must invalidate the stale TLB entries.** Forgetting to is
one of the great classic kernel bugs, because everything works fine until it
catastrophically doesn't.

## The aarch64 two-register trick

aarch64 has two page table base registers, not one:

- `TTBR0_EL1` translates low addresses → userspace
- `TTBR1_EL1` translates high addresses → the kernel

The kernel lives permanently in the high half. On a context switch we swap only `TTBR0`;
the kernel's mappings never move. So a syscall requires no address space change at all.
x86_64 has one such register and achieves the same effect by convention. This is one of
the places aarch64's clean-sheet design visibly pays off.

## Turning it on is the sketchiest moment in the kernel

The MMU is off at boot. Addresses are physical.

The instant you flip the enable bit in `SCTLR_EL1`, the *very next instruction* is fetched
through the MMU. If you have not mapped the code that is currently executing, the CPU
tries to fetch from an address that no longer means anything, and the machine simply
stops existing.

You get one shot, and you cannot printf your way out of it. This is milestone 4, and it
is where the GDB stub earns its keep.

## Aside: why some chips can't run this OS

Microcontroller-class RISC-V and ARM parts (ESP32-C3, many Cortex-M) have no MMU.
No MMU means no virtual addresses, no isolation, no user mode as we mean it. They can run
an RTOS, but not the kind of OS we are building. This is why the RISC-V hardware we
considered had to be JH7110-class or better.

## BUGS

- **A user thread can lose the translation of its own text while it runs, cause unknown.** Seen
  twice, on two architectures, so the cause is in shared kernel code rather than in one port's TLB
  handling:
  - riscv64, merge-group run 38284891615 (2026-10-12 UTC), `thead-c906`: `network_time_client`
    killed by an instruction page fault with `pc == stval == 0x60000f36`, the second instruction of
    a function in its first text page.
  - x86_64, merge-group run 38506029610 (2026-10-14 UTC), the AMD-Vi leg: `date`, the first stage of
    `pipeline_tests`' `date | wc`, killed with `rip == addr == 0x60002102` and error code `0x14`
    (a ring-3 fetch from a page not present). That address is the instruction after the `syscall`
    in `user_mode_runtime::syscall::trap6`. The per-core trace rings show `date` ran that page twice
    on core 1, blocked sending to `wc`, `wc` ran on core 1 and woke it, and `date` faulted on its
    first fetch after being switched back in. The VT-d leg of the same job ran the same suite clean.

  QEMU's TCG caches neither a not-present entry nor a paging-structure entry on either ISA, and
  every root change in those windows flushed. So at the refill, either the live root was not the
  thread's or the thread's own tables no longer mapped its text. Neither has reproduced. On riscv64,
  fifteen local full `thead-c906` legs ran with two probes, and they saw no return to U-mode under a
  root other than the thread's own and no write into a frame between its free and its next
  allocation. On x86_64, 206 runs of `pipeline_tests` ran under round-robin TCG, 265 under
  multi-threaded TCG (what CI's x86_64 host runs) and 300 under multi-threaded TCG with
  `NIFE_IOMMU=amd`. Reading the x86_64 code ruled out lazy text mapping, since the loader maps every
  page before the thread adopts the space, and the `CR3` comparison in `switch_user_root`, which
  reads the register. It also ruled out revocation and table-cut records outliving their space, a
  double free (the allocator asserts) and the e1000e DMA plane, which is never freed.

  The riscv64 and x86_64 kill reports both name the half next time. riscv64 prints the live `satp`
  and the leaf a walk finds. x86_64 prints the live `CR3` root and every level of the walk with
  whether each table's frame is still allocated (`Mapper::walk`), so a cleared entry, a table freed
  while linked and a frame written over each read differently. Both say, in a `system_tests`
  build, whether the live root is the thread's own. aarch64's kill report prints neither, and
  should before the next occurrence lands there.

---

*Add to this file as new memory concepts come up.*
