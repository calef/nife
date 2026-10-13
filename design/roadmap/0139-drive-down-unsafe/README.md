# Drive the unsafe count down: the appendices to milestone 139

[`design/roadmap/0139-drive-down-unsafe.md`](../0139-drive-down-unsafe.md) carries what the block
is for, a table of every round, what is still open with calef's rulings, what counts as a
reduction, the ratchet, the BUGS and the Follow-on. This directory carries each round's full
account, which the page's prose budget has no room for. Each round was measured from the diff
against its own base commit, and the appendices keep those numbers.

- [`rounds-1-and-2.md`](rounds-1-and-2.md): `MappedWindow` and its first seven programs, then
  `user_rt`'s `invoke5`, the `ipc` reading and the FS page-copy cluster.
- [`round-3.md`](round-3.md): the `swish`, `disk_surveyor` and `net_stack` windows, and the two
  investigations (device registers, framebuffer) with the ns16550 correction.
- [`rounds-4-and-5.md`](rounds-4-and-5.md): the bounds-check measurement and the graphics
  migration, then `tock-registers` for the fixed-layout device blocks.
- [`round-6.md`](round-6.md): the `user/` survey, `initrd_bytes`, the slice accessors and their
  count regression, and the `invoke` cluster counted.
- [`round-7.md`](round-7.md): the `invoke` cluster read method by method and wrapped.
- [`round-8.md`](round-8.md): the kernel outside `arch/`, its per-category table, and the page
  zeroing and device-tree collapses.
- [`rounds-9-to-11.md`](rounds-9-to-11.md): the identified work finished, the ownership token, the
  ceiling set at the tree's value, and question 2's renames.

The split was ordered by calef on 2026-10-13 (UTC), superseding his 2026-09-25 ruling that the
block is not split; the page records both.

*Name: provisional, minted 2026-10-13 (UTC) by maintainer/cap-0047-0139, which split the parent
under §212 (a prose budget). The directory follows §212's siting default, `design/roadmap/X/`
beside `design/roadmap/X.md`. calef names directories.*

- `rounds-1-and-2` Name: provisional, minted 2026-10-13 (UTC) by maintainer/cap-0047-0139, after
  the rounds it holds. calef names files.
- `round-3` Name: provisional, minted 2026-10-13 (UTC) by maintainer/cap-0047-0139, likewise.
  calef names files.
- `rounds-4-and-5` Name: provisional, minted 2026-10-13 (UTC) by maintainer/cap-0047-0139,
  likewise. calef names files.
- `round-6` Name: provisional, minted 2026-10-13 (UTC) by maintainer/cap-0047-0139, likewise.
  calef names files.
- `round-7` Name: provisional, minted 2026-10-13 (UTC) by maintainer/cap-0047-0139, likewise.
  calef names files.
- `round-8` Name: provisional, minted 2026-10-13 (UTC) by maintainer/cap-0047-0139, likewise.
  calef names files.
- `rounds-9-to-11` Name: provisional, minted 2026-10-13 (UTC) by maintainer/cap-0047-0139,
  likewise. calef names files.
