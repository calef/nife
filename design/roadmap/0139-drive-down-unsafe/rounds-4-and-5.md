# Milestone 139 appendix: rounds 4 and 5

An appendix to [milestone 139 (drive the unsafe count down, and cinch the ratchet behind it)](../0139-drive-down-unsafe.md),
moved out of that page on 2026-10-13 (UTC) under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md).
The main page is complete without it. This file holds round 4, which measured `MappedWindow`'s
bounds check and migrated the graphics cluster, and round 5, which took `tock-registers` for the
fixed-layout device blocks. Long sentences were split as they moved; no figure, date, name or
quotation was dropped.

## Round 4 (2026-08-24): measuring `MappedWindow`'s bounds-check cost, then migrating the framebuffer/graphics cluster round 3 narrowed to

[Round 3](round-3.md)'s own conclusion set this round's exact scope. It named `painter.rs`'s and
`window.rs`'s `px_write`/`px_read`, `display.rs`'s `surface_pixel`, and `display_terminal.rs`'s
`paint`. All are bounded: 2,048-8,192 accesses per one-shot test run, or a keystroke-driven repaint
far smaller than that. None of them is the sustained 60fps path the milestone's original framing
worried about. The open question was no longer structural; it was a number. Does `MappedWindow`'s
bounds check cost enough at these volumes to matter? In round 3's own words it was "worth
`script/icount` or `script/bench` rather than reasoning about it further." This round got that
number, then migrated on it.

The measurement. A temporary comparison loop ran over a page-sized buffer. One loop did a raw
`write_volatile`; the other performed `MappedWindow::check`'s own arithmetic first
(`off.checked_add(size).is_some_and(|end| end <= len)`). It ran at three volumes:

- 56: one glyph cell, `bitmap_font::GLYPH_W * GLYPH_H`, `display_terminal.rs`'s smallest
  keystroke-driven repaint.
- 2,048: `window.rs`'s largest `compositor::SCENE` surface.
- 8,192: `graphics_protocol::PIXELS`, the one-shot digest volume that `painter.rs`, `display.rs` and
  `window.rs`'s own largest surface all share.

It ran via `script/bench` on both ISAs. That is icount: deterministic instruction counts, the right
instrument for a path-length question rather than a magnitude one. notes/benchmarks.md's own table
names icount for exactly this job.

| | aarch64 (ticks/access) | riscv64 (ticks/access) |
|---|---|---|
| raw `write_volatile` | 8 | ~1.4 |
| `MappedWindow`-checked | 12 | ~2.0 |
| delta | 4 | ~0.6 |

The cost is flat across all three volumes on both ISAs. The check's cost does not depend on how
many accesses follow it, only on the arithmetic itself. So the total overhead scales linearly and
stays small in absolute terms even at the largest bounded volume. 8,192 accesses cost ~29,000
extra aarch64 ticks, under 30 `ipc_rtt` round trips (1,017 ticks each). That sits inside a test
that already pays several real round trips (at least one `CALL` to a driver or the compositor).
For `display.rs` it also pays a real device DMA completion at ~200 us wall clock (`fs_read`'s own
reading of what a completion-interrupt wait costs). Negligible on both ISAs. That settles round 3's
open question: the bounds check is not worth avoiding at any volume this cluster actually sees.

The comparison itself was not kept in the tree. It answered a one-time design question rather than
measuring an ongoing primitive worth regression-gating forever. The same reasoning keeps
`bench.rs`'s `map_new` shootdown probe a `bench-probe:` line rather than a baseline row. Keeping it
would also have cost this milestone's own ceiling two more `unsafe {` blocks (one per comparison
loop) for a question that is now closed. That works directly against the number this round exists
to lower. The numbers above are the reproducible record instead.

The migration, on that number. All four sites round 3 named moved onto `MappedWindow`:

- `painter.rs`'s `px_write`/`px_read`: a `const WINDOW` at `SURFACE_VA`, sized to
  `gfx::SURFACE_BYTES`, the same shape round 1's cluster used. 2 `unsafe {` blocks removed, 1
  added, net -1.
- `window.rs`'s `px_write`/`px_read`: not a `const`, unlike `painter.rs`'s twin. The compositor's
  `spawn_client` maps a different frame count per client (`SCENE[i].frames()`,
  `kernel/src/user/compositor_service.rs`), not knowable at this program's compile time. So the
  window is constructed once in `_start`, sized to `bytes`. That is this client's own `w * h * 4`,
  already computed and bound-checked against `compositor::MAX_SURFACE_BYTES` before any pixel is
  painted (milestone 43 (a second security audit, with a different lens),
  notes/shared-page-audit.md finding 4). It is threaded through both call sites: `px_write`, and
  `px_read` via a closure into `compositor::surface_checksum`. This is the genuinely per-client,
  runtime-sized case that `net_stack.rs`'s socket cluster was in round 3. 2 removed, 1 added, net
  -1.
- `display.rs`'s `dma_write`/`dma_read`: round 3 scoped this to `surface_pixel` alone. But
  `surface_pixel` is one of dozens of callers of a shared pair of functions. That pair already
  collapses the DMA region's invariant into one hand-written assertion apiece. Migrating only
  `surface_pixel`'s own call would have duplicated that invariant (a second `MappedWindow` beside
  the untouched `dma_write`/`dma_read`) rather than collapsed it, the wrong direction for this
  milestone. So the two shared functions migrated instead: a `const WINDOW` over the whole DMA
  region (`DMA_FRAMES * 4096` bytes), with `dma_write`/`dma_read`'s bodies becoming `WINDOW.write`/
  `WINDOW.read`. That covers `surface_pixel` for free. It also bounds-checks every one of the few
  dozen virtqueue-field one-off writes in the file. Round 3's own text flagged those as "not the
  performance question and could be migrated independently"; here they came along for free rather
  than as separate scope. 2 removed, 1 added, net -1.
- `display_terminal.rs`'s `paint`: a window constructed once in `_start`. It is sized to
  `gfx::SURFACE_BYTES` in `MODE_DISPLAY` (this process maps that many bytes itself) or to
  `stride * h` in `MODE_WINDOW` (the compositor's own published geometry, the same per-client
  reasoning as `window.rs`'s). It is held in the `Wiring` struct rather than declared as a
  file-level `const`, for the same reason `window.rs`'s is not one. 1 removed, 1 added, net 0. Flat
  by block count, it is still a real reduction by criterion 2 (raw pointer arithmetic replaced by a
  typed, bounds-checked abstraction). `swish.rs`'s terminal pair and `smb_server.rs` were the same
  case.

Combined round 4: 7 `unsafe {` blocks removed, 4 added, net -3. It was measured from the diff
against this round's own base commit (`757562a3`), uncontaminated. Nothing else landed on this
branch between the base commit and this reduction, so the tree-wide census confirms it exactly.
There were 782 blocks outside `arch/` at the base commit (88 per 10,000). That matches round 3's
own final reading despite 12 blocks of unrelated tree growth landing in between, 770 to 782;
density absorbed it the same way round 1's and round 2's readings did. After: 779, exactly -3.
Density moved 88 to 87 (truncated). The line count moved by +19 net, mostly the new `SAFETY`
comments explaining each window's invariant. So the denominator barely moved this round, like
round 3's.

The ratchet, cinched a fourth time: `<!--count-at-most:unsafe-density-outside-arch-->` lowered
from 95 to 94 in the same commit (`notes/unsafe-obligations.md`, `notes/counted-claims.md`,
`notes/register-of-measures.md`), 7 points above the 87 this round reached.

The framebuffer/graphics investigation is now settled, not just narrowed. `compositor.rs`'s
per-frame hot path still carries no per-pixel `unsafe` at all (round 3's finding, unchanged). The
five genuinely per-pixel sites round 2 and round 3 identified are now four migrations and one
structural non-issue: `compositor.rs` itself needed nothing. Nothing about this cluster remains
open.

## Round 5 (2026-08-24): `tock-registers` for the fixed-layout device blocks, and the ns16550 correction narrows it to a per-file, per-architecture call

Round 3 named `console.rs`, `input.rs` and `jh7110_trng.rs` as one follow-on. Its ground was that
"`kernel/src/drivers/pl011.rs` and `kernel/src/drivers/ns16550.rs` already use
`tock_registers`... for the identical hardware." That premise was checked before migrating
anything, since a lane cannot answer "is the premise true" by reading its own brief. It turned out
to be half wrong: see [the correction in round 3](round-3.md). `ns16550.rs` was never migrated, for
exactly the reason its own module doc gives: the runtime-variable register stride
`register_structs!` cannot express. That correction changed the unit of analysis from "three
files" to "which half of which file has a compile-time-fixed layout," since `console.rs` and
`input.rs` each carry two architecture-gated halves behind one file name. They are an aarch64 PL011
driver and a riscv64 NS16550 driver.

`console.rs`'s and `input.rs`'s aarch64 halves: migrated. Both drive the PL011 at a fixed offset
table (`DR`, `FR`; `input.rs` also `IMSC`, `ICR`). That is the same fixed layout
`kernel/src/drivers/pl011.rs` already verifies at compile time for the identical hardware, with no
runtime knob analogous to the NS16550's stride. Each file gained a local `register_structs!`/
`register_bitfields!` block, private to that file. The two do not share it, matching how they
already did not share their hand-written offset constants either. Each also gained a
single-pointer-cast `unsafe` function replacing the hand-rolled `read_volatile`/`write_volatile`
pair.

`console.rs`'s and `input.rs`'s riscv64 halves: genuinely unsuitable, the same finding
`ns16550.rs` itself already made. Both hard-code the NS16550 register layout at QEMU's one-byte
stride (`THR`/`LSR` in `console.rs`; `RBR`/`IER`/`LSR` in `input.rs`). They have no `Shape`
parameter and no way to vary it at runtime today. That is not a reason to migrate them: it is a
reason not to. `ns16550.rs`'s module doc names the underlying hardware fact. This device family's
register stride is a fact the JH7110's real silicon and QEMU's emulation disagree on, and only a
runtime value (the device tree, eventually `Shape`) can resolve it. That holds for these two files
exactly as it holds for the kernel's own driver. They simply have not grown a `Shape` parameter yet
to make the disagreement visible. Encoding today's QEMU-only assumption into a `register_structs!`
block would not fix that gap. It would fossilize it behind a stronger-looking mechanism: a
compile-time-checked layout that is confidently wrong the day this driver runs against real JH7110
hardware. That is a worse failure mode than the honest hand-written offsets, which at least invite
a reader to ask "is this the right stride." Left unmigrated, on purpose, matching `ns16550.rs`'s
own precedent rather than round 3's "identical hardware" premise.

`jh7110_trng.rs`: migrated. Round 3's brief warned this might be "the one most likely to have this
problem," being real-hardware-specific code for a board this tree has not yet run against
(milestone 159 (a real hardware entropy source: the JH7110's TRNG), `NOT-STARTED`, gate HARDWARE).
So it was checked first. The check was against the crate's own sourced register file
(`crates/jh7110_entropy::regs`, transcribed from `jh7110-trng.c`) and the device-tree binding
(`starfive,jh7110-trng`, `reg = <0x1600C000 0x4000>`). The binding gives one `reg` window with no
`reg-shift`/`reg-io-width` knob, unlike the NS16550's binding. So there is no runtime-variable
aspect to this device's layout for `register_structs!` to be the wrong tool for. `r32`/`w32` were
two hand-written volatile-access functions, one `unsafe` block apiece. They collapsed into one
`regs()` pointer-cast function reused by every accessor. The register block names only the
registers this driver touches (`CTRL`, `ISTAT`, `RAND0..RAND7`). `STAT`, `MODE`, `SMODE`, `IE`,
`AUTO_RQSTS` and `AUTO_AGE` are reserved padding, the same "not otherwise used" status the crate's
own `regs` module already gives several of them. This has not run against real silicon either way
(see the crate's and the program's own module docs). The migration changes nothing about that gap,
only how the register offsets are checked.

Measured from the diff against this round's own base commit (`757562a3`): 5 `unsafe {` blocks
removed, 3 added, net -2. Per file:

- `console.rs` is flat at 1 before and 1 after. It is still a real reduction by rounds 1-3's own
  criterion 2: raw pointer arithmetic replaced by a compile-time-checked typed abstraction. That is
  the same "flat but real" case `smb_server.rs`, `fs_subtree_caretaker.rs` and `swish.rs`'s
  terminal pair were.
- `input.rs`: 2 removed, 1 added, net -1.
- `jh7110_trng.rs`: 2 removed, 1 added, net -1.

Tree-wide census, base commit versus this round's tree: 782 blocks outside `arch/` in 88,761 lines
(88 per 10,000) before, and 780 blocks in 88,812 lines (87 per 10,000) after. The line count grew
despite the block count falling. The compile-time-checked layout costs more lines in doc comments
and macro invocations than the hand-written offsets and SAFETY comments it replaced. That is the
mirror image of round 2's `asm!`-collapse, where both moved together.

*`jh7110_trng.rs` is `components/src/jh7110_entropy.rs` now. That came from milestone 175 (split
`user/`: `components/` for services, `fixtures/` for test and benchmark programs)'s split, then
calef's two 2026-09-13 rulings, `jh7110_entropy_source` and then `jh7110_entropy`; the second was
performed 2026-09-14. This round's sections spell it as it was when the blocks were counted, so the
-1 stays checkable against base commit `757562a3`. `notes/unsafe-obligations.md` spells it the
same way.*

The ratchet, cinched a fourth time: `<!--count-at-most:unsafe-density-outside-arch-->` lowered
from 95 to 94 (`notes/unsafe-obligations.md`, `notes/counted-claims.md`,
`notes/register-of-measures.md`), 7 points above the 87 this round reached. One caveat, rather than
let a merge discover it. A concurrent round-4 lane (`milestone/139-round4-graphics`) worked a
different candidate set from the same base commit. So whichever pull request landed second had to
re-measure from the merged tree rather than trust either round's before/after numbers in isolation.
That is the discipline round 2 names for tree-wide census under concurrent growth. [Round
8](round-8.md) did that re-measure.
