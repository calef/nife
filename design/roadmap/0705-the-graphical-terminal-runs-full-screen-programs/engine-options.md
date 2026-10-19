# Options A to E, in full

This appendix belongs to [milestone 705 (the graphical terminal runs full-screen programs)](../0705-the-graphical-terminal-runs-full-screen-programs.md).
It holds the full seven-question entries for options A to E. Option F has [its own appendix](option-f.md).
The shared answers to questions 2 and 4 are in the main document's Part 4.

## A. Extend `video_terminal`

The list in Part 2. Estimate: 500 to 900 lines of engine code plus about as many of tests. This
is an estimate by analogy, not a measurement. On 2026-10-18 the engine is 1,738 lines across six
files (`vt.rs` 1,033), plus `keymap.rs` (572) and `script.rs` (352), and `display_terminal.rs` is
547. A second grid and a region-aware scroll touch `line_feed`, the damage record and the scrollback
ring, which is where the risk is, not the sequence count.

1. Considered instead: B, C, F and G, each under its own entry, and D for the ordering.
2. See the shared answer in the main document.
3. Prior art: xterm's control-sequence document and `vttest` define the behavior (not read here; named
   from memory). Every real engine in section C or B is an existence proof.
4. See the shared answer in the main document.
5. Cost: lines above, no new dependency, no new toolchain, host-testable in milliseconds. The release
   `display_terminal` is 18 KB of code and 717 KB of `.data` (the static grid) on aarch64, measured
   2026-10-18; A adds to the code and, with a second grid, to the data.
6. Fully reversible. No wire format changes, and the engine is behind `display_terminal`.
7. Against B to E, yes at equal cost: it keeps the three-witness proof (`pixel(x, y)`) with no
   rebuild, and it is the only option with no dependency in the shipping graph. Against F and G, no:
   at equal cost an engine that passes 22 or 23 of vttest's 24 screens owns every program's output
   better than ours at 7. A's remaining argument is fewer moving parts.

## B. libghostty-vt as a second engine behind the C seam

Everything below was read from `ghostty-org/ghostty` `main` on 2026-10-02, except where marked.

- *Maturity.* `include/ghostty/vt.h` opens "WARNING: This is an incomplete, work-in-progress API. It is
  not yet stable and is definitely going to change." `lib_vt.zig` says the behavior is stable
  (extracted from a shipped terminal) and the API is not. The tree's own note already says to pin.
- *Toolchain.* `build.zig.zon` requires Zig 0.16.0 or later. Zig is not installed here (Homebrew has
  0.16.0), so the build-time cost is unmeasured. The source it compiles is large:
  `src/terminal` holds 164 Zig files totaling 6.2 MiB, tests included, line count not taken.
- *Freestanding.* Not "no allocation" (section above). It also needs a `std.Io` (`TinyIo` is the
  small one), and an entropy callback on targets with no `getrandom`
  (`GHOSTTY_SYS_OPT_RANDOM_SECURE`, `vt_sys.h`; whether the terminal path exercises it is not
  checked). Whether the Zig 0.16 freestanding build links without libc symbols is the first thing
  a lane would have to find out; I did not try.
- *License.* MIT (GitHub license field, and the repository `LICENSE`).
- *What it buys.* Alternate screen (modes 47, 1047, 1049), device attributes, cursor report, size
  report, mouse and key encoders, reflow and scrollback, per `vt.h`, `vt_modes.h` and
  `vt_terminal.h`.
- *Cells versus pixels.* Its C ABI gives cells through a render-state API (`render.h`, 1,104 lines of
  header). `Vt::pixel(x, y)` is a pure function of our own grid. The rebuild is the expected-picture
  definition, as `notes/glyphs/libghostty-vt.md` says: either it moves to the Zig side or it is rewritten over
  the cell layout and checked against `Vt` on the same scripts. That check, two engines graded against
  each other, is the milestone 23 (a capability-routed component OS with live replacement)
  demonstration the existing note argues for.

1. Considered instead: A, which avoids the dependency and loses on completeness and on the milestone 23
   demonstration; C, which is a parser only (below); and F, which reaches the same completeness in
   Rust without a toolchain.
2. See the shared answer.
3. Prior art read: the headers above. Ghostty's conformance history against `vttest` is claimed in
   `notes/glyphs/libghostty-vt.md`; I did not verify it.
4. See the shared answer, including the stale "no allocation" line.
5. Cost: a Zig 0.16 toolchain in the build (unmeasured), a pinned vendored tree under §18's policy, an
   allocator for the component, a cell-based expected-picture check (not estimated), and shim code
   (not estimated).
6. Hard to reverse once programs rely on its behavior; easy to reverse before. A dependency in the
   shipping graph is the irreversible category in `CLAUDE.md`.
7. Mostly yes. It is the option this tree already prefers for its milestone 23 claim, and the
   answer is less about effort than about whether the claim is worth a toolchain.

## C. `vte` (the Rust crate)

Read from the 0.15.0 source (crates.io, `Apache-2.0 OR MIT`, 57 KB).

- *What it is.* A parser: "The state machine doesn't assign meaning to the parsed data and is thus not
  itself sufficient for writing a terminal emulator" (its `README.md`). You implement `Perform` and
  receive `print`, `execute`, `csi_dispatch`, `esc_dispatch`, `osc_dispatch` and `hook`. The parser is
  `src/lib.rs`, 832 lines before its tests, plus `params.rs` (144).
- *What 0.15.0 adds.* An optional `ansi` feature (2,016 lines before tests) with a `Handler` trait
  that already names `set_scrolling_region`, `device_status`, `identify_terminal` and a
  `PrivateMode` enum including `SwapScreenAndSetRestoreCursor` (1049). It is still stateless: the
  grid, the regions, the second screen and the replies stay ours. It also pulls `alloc`, `log`,
  `bitflags` and `cursor-icon`, and `display_terminal`, being `no_std`, has no allocator.
- *What it would save.* The sequence-recognition half of `csi` (`lib.rs:1393-1442`): the parameter and
  intermediate-byte handling that this engine already has and tests. It saves none of the state the
  Part 2 list needs. The roadmap's phrase "much less complete" (milestone 29) understates this: it
  is not an engine at all.
- *Dependencies.* The parser needs `arrayvec` (not in `Cargo.lock`) and `memchr` (already there).

1. Considered instead: A. It is A with a different front half.
2. See the shared answer.
3. Prior art read: `ansi.rs` says it "was originally part of the `alacritty_terminal` crate", so the
   state half lives in a different crate.
4. See the shared answer.
5. Cost: the lines above, two dependencies, and the engine work of A on top.
6. Reversible before use; a dependency in the shipping graph once used.
7. No. At equal cost A wins over C, since C adds a dependency to save a parser that is already written
   and tested here. C is listed so that a refusal, if it comes, has its reason on record.

## D. A then B, or B then A

The two are not exclusive: B behind the seam does not remove A, and `notes/glyphs.md` already
recommends keeping `video_terminal` as the reference the foreign engine is checked against.

- **A then B.** The full-screen goal is met without a toolchain, and B arrives later for its own
  reason (milestone 23). The cost is that the expected-picture check is not rebuilt until B, and A's
  region and alternate-screen code becomes the reference B is graded against, which is a useful
  property and not an accident.
- **B then A.** Full-screen programs come from B's completeness first. The cost is that the goal waits
  on the Zig toolchain and the cells-based check, and A is then written afterwards as a second engine
  for no customer reason. I see no argument for this order other than the milestone 23 claim.

1. Considered instead: A alone, B alone. Already above.
2. to 4. See the shared answers in the main document.
5. Cost: the sum, spread over two lanes; the first delivers the goal.
6. As reversible as each half.
7. For A then B, yes at equal cost, because the first step delivers the measure. For B then A, the
   ordering is effort-neutral and I would not choose it.

## E. Replace `video_terminal` with libghostty-vt

B without keeping A's engine: libghostty-vt becomes the only VT engine behind `display_terminal`, and
`video_terminal` is retired (or kept only as a host-side test fixture). Facts about the library are
those in B; this entry is about what losing the second engine costs and buys.

- *Against.*
  - `video_terminal`'s `pixel(x, y)` carries the three-witness check: the component draws, the kernel
    test predicts, and xtask grades the scanout. libghostty-vt gives cells, so the check has to be
    rebuilt on cells with a small renderer we own.
  - Losing the second engine loses differential testing. §37 (text as a value) and `notes/glyphs.md`
    call that comparison the stronger milestone 23 (a capability-routed component OS with live
    replacement) demonstration.
  - As the only engine, every upstream API break ("definitely going to change") breaks the display
    path. That path then needs the Zig toolchain and a supplied allocator just to work.
- *For.*
  - No 500 to 900 lines of our own engine code to keep up.
  - No double maintenance, and no triage of disagreements between two engines.
  - Immediate completeness: DEC modes, resize, grapheme clustering, mouse, and a `vttest` history
    (claimed in `notes/glyphs.md`, not verified here).
- *What would make E more attractive.* An upstream API declared stable.

1. Considered instead: A (keeps the second engine, costs the lines), B and D (keep both engines).
2. to 4. See the shared answers in the main document.
5. Cost: B's toolchain, vendoring and allocator costs, plus the cell-based check, minus the engine
   lines of A. Not measured beyond what B records.
6. Harder to reverse than B: once `video_terminal` is gone, bringing it back is a rewrite, and the
   display path depends on a library whose API is declared unstable.
7. Not decided here. At equal cost I would still keep two engines, because the differential check is
   the claim; E wins only if that claim is not worth the second engine's upkeep.
