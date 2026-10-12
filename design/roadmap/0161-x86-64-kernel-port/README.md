# The x86_64 kernel port: the appendices to milestone 161

[`design/roadmap/0161-x86-64-kernel-port.md`](../0161-x86-64-kernel-port.md) carries the status,
why x86_64 is the third architecture, a one-line summary of each build step, where each open item
ended, the follow-on and the closing lane's results. This directory carries the working its prose
budget has no room for.

- [`history.md`](history.md): the status history the block kept, the scope carried forward from
  [milestone 20 (a portable HAL, proven on a second architecture)](../0020-portable-hal.md), and the
  physical machine it did not wait for.
- [`early-boot.md`](early-boot.md): steps 1 to 10, from the PVH boot path to a PIT line routed
  through the IO APIC.
- [`ring-3.md`](ring-3.md): step 11 and item 3, the `syscall` pair, the `iretq` choice, the syscall
  ABI decision and the two `CR4` bits.
- [`processes-and-userspace.md`](processes-and-userspace.md): steps 12 and 13 and item 4, the
  scheduler, real processes, userspace, the latent bugs and the 67 skips.
- [`open-list.md`](open-list.md): items 0 to 2 as recorded, the port-I/O question and the renames.
- [`smp-and-vt-d.md`](smp-and-vt-d.md): items 5 and 6, the three SMP failures and the VT-d driver.
- [`closing.md`](closing.md): the 2026-09-19 lane's working and measurements.

*Name: provisional, minted 2026-10-12 (UTC) by maintainer/prose-budget-paydown, which split the
parent under §212 (a prose budget). The directory follows §212's siting default. calef names
directories.*

- `history` Name: provisional, minted 2026-10-12 (UTC) by maintainer/prose-budget-paydown, for the
  parent's status history and its scope and sequencing sections. calef names files.
- `early-boot` Name: provisional, minted 2026-10-12 (UTC) by maintainer/prose-budget-paydown, for
  steps 1 to 10 of the parent's "What was built". calef names files.
- `ring-3` Name: provisional, minted 2026-10-12 (UTC) by maintainer/prose-budget-paydown, for step 11
  and item 3. calef names files.
- `processes-and-userspace` Name: provisional, minted 2026-10-12 (UTC) by
  maintainer/prose-budget-paydown, for steps 12 and 13 and item 4. calef names files.
- `open-list` Name: provisional, minted 2026-10-12 (UTC) by maintainer/prose-budget-paydown, for
  items 0 to 2 of the parent's "What is still open" and the port-I/O question. calef names files.
- `smp-and-vt-d` Name: provisional, minted 2026-10-12 (UTC) by maintainer/prose-budget-paydown, for
  items 5 and 6. calef names files.
- `closing` Name: provisional, minted 2026-10-12 (UTC) by maintainer/prose-budget-paydown, for the
  parent's "Closing the follow-on (2026-09-19)". calef names files.
