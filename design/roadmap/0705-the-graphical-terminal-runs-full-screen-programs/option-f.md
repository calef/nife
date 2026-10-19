# Option F, measured: `display_terminal` as a `std` program on a full Rust engine

This appendix belongs to [milestone 705 (the graphical terminal runs full-screen programs)](../0705-the-graphical-terminal-runs-full-screen-programs.md).
It holds option F's measurements and its seven-question entry.

Every number here was measured on 2026-10-18 (UTC) on patagonia, against `main` at `948806b32`, unless
it is marked *from memory*. The probes were throwaway programs in the lane's scratch directory and are
not committed.

## What F is

`display_terminal` stops being a `no_std` program with a static `Vt` and becomes a `std` program
(§22 (Rust `std` on the native ABI, the Hermit way)). Its grid, modes and replies come from a full
Rust terminal model taken from crates.io. Drawing stays ours: the engine's cells go through the same
bitmap font and the same cell-to-pixel rule `Vt::pixel` applies today.

## The probe

One feature script, written from Part 2's list, run against each engine through a thin adapter.
It has fifteen checks: the alternate screen (`?1049`), DECSTBM, reverse index inside a region,
DSR 6, DA, DECAWM off, `CSI @ P X`, `CSI L M`, `ESC 7`/`ESC 8`, `CSI S T`, `CSI G d`, DECTCEM and
DECCKM state, DECOM, resize, and settable tab stops. Each check states the exact rows or bytes it
expects. The same probe was built for all three nife `std` targets with the patched farm, and
run on the host.

## Results

| | `alacritty_terminal` 0.26.0 | `vt100` 0.17.0 | `termwiz` 0.23.3 |
|---|---|---|---|
| What it is | a terminal model: grid, modes, scrollback, damage | a terminal model: grid, modes, scrollback | a parser and a surface; nothing applies one to the other |
| Builds for the three nife targets, unmodified | no: `polling` 3.11 refuses the OS, through `event_loop` | yes | no: see below |
| With its pty and event loop cut out | yes, and links, on all three | not needed | not tried |
| Part 2 checks passed (host) | 15 of 15 | 13 of 15: no DECAWM, tab stops fixed at eight | none apply |
| Replies (DSR, DA) | its own, as `Event::PtyWrite` | none; the probe wrote them in a callback | none |
| Damage tracking | yes (`LineDamageBounds`) | no; a caller diffs two cloned screens | n/a |
| License | Apache-2.0 | MIT | MIT |
| Downloads; last release | 2.1M; 2026-04-06 | 14.9M; 2026-10-18 | 27.6M; 2025-03-20 |
| `cargo deny` against `deny.toml` | advisories, bans, licenses ok | advisories, bans, licenses ok | not run |
| Under QEMU on nife, three architectures | 15 of 15; 4.3 MiB peak heap | 13 of 15; 0.25 MiB | n/a |

The cut removes `src/tty/` and `src/event_loop.rs` (2,087 lines) and three dependencies (`home`,
`libc`, `polling`). Only `lib.rs` named them, so the cut is two lines of `lib.rs` and the manifest.
The `cfg(unix)` dependencies (`rustix`, `signal-hook`) never applied, because nife's target is not
unix. The crate keeps 9,650 lines.

`termwiz` fails three ways. `stacker` (through `pest`) calls `libc::mmap`, which `libc` does not
bind for nife. `psm`'s build script fails on riscv64. And `sha2` 0.10 hits an LLVM error under
x86_64 softfloat. It also needs `getrandom`'s custom backend. It is not an engine, so it is out
whatever happens to its build.

`vt100` 0.17.0 was released on the day of this survey; 0.16.2 (2025-07-12) is the prior release.

## Dependencies the base graph would gain

Counted over the nife target's graph (`cargo tree` with the target spec), less what `Cargo.lock`
already holds.

- `vt100`: five new crates, `vt100`, `vte`, `arrayvec`, `itoa`, `unicode-width`.
- The `alacritty_terminal` cut: ten new crates, `alacritty_terminal`, `vte`, `arrayvec`, `base64`,
  `cursor-icon`, `lock_api`, `parking_lot`, `parking_lot_core`, `scopeguard`, `unicode-width`. Eight
  more are already in the lock: `aho-corasick`, `bitflags`, `cfg-if`, `log`, `memchr`,
  `regex-automata`, `regex-syntax`, `smallvec`.

## Size

Code bytes (`llvm-size` text), release builds. The probes use `std_exerciser`'s profile (`opt-level
"s"`, debuginfo stripped).

| ELF | aarch64 | riscv64 | x86_64 |
|---|---|---|---|
| `display_terminal` today (`no_std`) | 18,326 | 17,850 | 19,390 |
| a `std` program that prints one line | 56,908 | 51,328 | 52,057 |
| the `vt100` probe | 111,896 | 105,446 | 107,801 |
| the `alacritty_terminal` probe | 232,160 | 229,192 | 236,856 |

So the engine costs about 55 KB (`vt100`) or about 180 KB (`alacritty_terminal`) of code over the
`std` floor. This is not apples-to-apples. The probes carry probe code and not the component, and
today's 18 KB carries both the component and the engine. Read F's component as roughly 250 KB of
code, against 18 KB.

Data moves rather than grows. Today's ELF carries 717,440 bytes of `.data`, the static `Vt` (its
grid and its 300-row scrollback). Under F the grid is on the heap. An `alacritty_terminal` cell is 24
bytes. Its default history is 10,000 rows, about 19 MB at 80 columns, so the component must cap it.
Even capped, its row storage grows in 1,000-row steps (`MAX_CACHE_SIZE` in `grid/storage.rs`), a
2 MiB allocation at 80 columns (the spike below has the totals).

## What F needs beyond the engine

Read in the tree:

- Capability slots. `display_terminal` takes slots 0 to 5 (`REPORT`, `PRESENT`, `TERM`, `BUDGET`,
  `SURFACE_FRAME`, `OUT_PAGE_FRAME`). A `std` program's slots 0 to 6 are reserved (`notes/std.md`), so
  they move, and so do their five spawners: `start_display_terminal` in `system_initializer`'s
  `children.rs`, the kernel's `display_service.rs`, `compositor_service.rs` and `device_starters.rs`,
  and the firmware-screen test in `system_tests`' `display_tests.rs`.
- Raw syscalls from a `std` program. `jig` already does it, through `user_mode_runtime`.
- The renderer. `Vt::pixel`'s cell-to-pixel half becomes a function of a cell and the cursor, so
  both engines' cells draw through it. One probe finding: `alacritty_terminal` stores `\t` in the cell
  where a tab started, which the renderer draws as a blank.

## The three witnesses, rebuilt

Today the component draws with `Vt`. The kernel predicts the scanout with `Vt`
(`display_service.rs`, `assert_screen_is`), and `xtask` grades it with `Vt` on the host. The kernel
cannot link a `std` engine. So under F the kernel's witness keeps `video_terminal`, and the check
becomes differential: two engines, drawing through one renderer, must agree pixel for pixel on the
scripted screen. That is the comparison §37 (text as a value) values, without option B's toolchain.

The limit is honest. The kernel can predict only what `video_terminal` implements. The features only
F's engine has (the alternate screen, regions, replies) are graded by Part 1's host harness against
goldens from a reference terminal, not by the kernel. `video_terminal` survives as the oracle and
stops being the engine on screen.

## The spike

The probe ran on nife itself, under QEMU, on all three architectures (`script/test --test`). It was
packed as a `std_exerciser` program on a local branch that was never pushed. The verdicts match the
host's: `alacritty_terminal` 15 of 15, `vt100` 13 of 15. Peak heap was 1,091 pages (4.3 MiB) for
`alacritty_terminal` and 65 pages for `vt100`. Nothing returned `Unsupported`.

Two findings on the way. One `alacritty_terminal` terminal at 80x24 holds 4.1 MB, measured on the
host with a counting allocator, even with its history capped at 300 rows. Today's static grid is 717
KB. And a probe that held two terminals at once ran out of the test boot's frame pool, which gave
any process about 7 MB on aarch64, 6.5 MB on riscv64 and 5.2 MB on x86_64.

No full-screen program ran. None exists on nife (Part 1), so the spike shows the engine, not `vim`.

## The seven questions

1. Considered instead: A, which adds no dependency and keeps the kernel's witness over every feature,
   at 500 to 900 lines we write and keep. B and E, which need Zig and an unstable C API. C, a parser
   only. Within F, `vt100` builds unmodified and is a third the size, and loses on DECAWM (procps `top`
   toggles it, per the census), tab stops, damage and replies. `termwiz` does not build and is not an
   engine.
2. The tree's analogous cases. `jig` is a `std` program making raw syscalls. Milestone 784 (reuse what the tree wrote) holds that
   needing `std` is not a reason to refuse a crate. RedoxFS is vendored because it needs a patch
   (§34 (RedoxFS is the primary filesystem, on three conditions)).
3. Prior art read: both crates' source at the versions above. That Alacritty itself runs this model
   is *from memory*; its conformance against `vttest` was not checked.
4. Is the premise true? Partly. "No `std`, no allocator" was this program's inherited choice. A `std`
   process has a heap (slot 0) and, since milestone 812 (`std::thread::spawn` runs real threads in one address space), threads. What is still true is that the
   kernel's witness is `no_std`, which is why `video_terminal` stays.
5. Cost: the sizes and graphs above, measured. Lane work is estimated, not measured: the `std` port,
   the slot moves in five spawn sites, the renderer split, the vendored cut and the host harness.
6. Reversible as code until a program relies on the engine. The dependency is §46 (thin primitives or whole subsystems)'s expensive
   category, and a vendored cut has to follow upstream. Nobody has acted on it.
7. Yes. At equal cost I would still choose F on `alacritty_terminal`: conformance is the point, and
   the kernel's oracle stays ours. F has more moving parts than A (a `std` port, slot moves, a
   vendored cut), and that is A's real argument; it is about elegance, not effort.
