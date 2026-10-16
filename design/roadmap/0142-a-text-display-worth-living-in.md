---
status: PARTIAL
raised: 2026-08-20
milestone_dependencies: 141
decision_dependencies: 166
machine_requirements: none
specific_machine: none
needs_person: no
---
# 142. A text display good enough that people use it instead of a GUI

Minted 2026-08-20 by calef, on seeing the Kaypro-style font land:
*"I would like to explore a really awesome text display and not a grainy one like we get with
Kaypro II... the idea would be to deliver a text display so good that people would use it outside
of a GUI."*

At minting the terminal was 18 columns by 8 rows of a hand-drawn 7x8 bitmap on a 128x64 screen. The
ask is a display somebody would choose over a window manager. That is four independent axes: the
surface, the terminal, the type and the color. The font is the one everybody names and the smallest
one. This block sequences them, prices the type, and reports one finding that makes the expensive
axis cheap: the rasteriser belongs at build time.

## Where it stands

Both decisions are taken. [§104 (the rich-text font is DejaVu Sans Mono, and the palette is
Solarized)](../decisions/0104-the-font-and-the-palette.md) chose the family and the palette on
2026-08-20, though this block's gate did not say so until 2026-09-19. [§166 (the rasteriser dependency, and whether the glyph atlas ships one face or four)](../decisions/0166-the-rasteriser-and-how-many-faces.md)
took the crate, which §46 (thin
primitives or whole subsystems) makes a decision, and the face count. Milestone 435 (a decision
nobody wrote down)'s lane wrote it up, and calef ruled on 2026-10-05 (UTC).

Increments one and two are built (`milestone/142-terminal-size`). They are also [journey
1](../journeys/01-login-to-kilo.md)'s step 6 (calef, 2026-08-27: "I want a full size usable
terminal"), and need nothing from milestone 177 (wire the graphical terminal stack into the real
interactive boot). The scanout was first grown to 1280x720, giving 182x90 at the shipping 7x8 cell,
double any terminal anyone runs. On 2026-08-27 it was retargeted to 924x344 for a 132x43 grid, the
VT100 "wide mode" size. When the atlas widens the cell, the grid shrinks with it.

On 2026-09-26 lane `milestone/142-text-display` built increment five's widening and the navigation
cluster (home, end, insert, delete, page up and down, with xterm's shift forms). Scrollback from the
keyboard needs a new opcode, because `line_editor` sits in front of `display_terminal`;
[milestone 668 (scrollback from the keyboard)](0668-scrollback-from-the-keyboard.md) holds it. Increment six
is built (checked 2026-10-03, UTC). Increments three and four have not started, and nothing blocks
them. [Sequencing](#sequencing) has each increment's state.

The working behind each section below is in [its appendices](0142-a-text-display-worth-living-in/README.md),
each linked where it applies. This page is complete without them.

## Where this sits, and it is not a rung

[The display ladder](../display-ladder.md) climbs toward compositing and acceleration: rung three
is real applications on iced's software renderer with `cosmic-text`, rung four is a GPU. This
milestone points the other way: calef's ask is a system good enough that you would not open a GUI.
That is the more interesting claim for a microkernel, because a terminal is a component the whole
confinement story already covers, and a desktop is not.

So this is a sibling of rung three that rung three then consumes, not a step on the ladder. Rung
three would buy good type expensively, as a side effect of a UI toolkit: `cosmic-text` does shaping,
bidirectional text, fallback and proportional layout, and a monospace terminal needs none of those.

## What "would use it outside a GUI" actually requires

calef named color, type and rich text. The axis he did not name decides whether anybody would live
in it, and it needs no rasteriser: 80 columns by 24 rows as an absolute floor, scrollback, UTF-8,
and arrow keys. At minting the terminal had none of them (notes/glyphs.md recorded each as an honest
limit), and that was more than half the work. So the block's main recommendation is to make it a
terminal first, then make it beautiful. A gorgeous 18x8 window is a demo; a plain wide window with
scrollback is a place you can work.

## The scanout

128x64 was chosen as a test instrument. `gfx_proto::WIDTH` records that a square surface hides a
stride bug, a transposition and an x/y swap. That argument constrains the ratio, not the size, so it
survives growth. §102 (a Frame names a run of pages) made a big surface reachable: a `PageFrame` naming a run turns hundreds of
capabilities and `MAP` calls into one of each. The real price is the harness, whose referee compares
every pixel of a `screendump` every 100 ms (see Follow-on). The arithmetic and the build detail are
in [`scanout.md`](0142-a-text-display-worth-living-in/scanout.md).

## The type

Menlo is Apple's and cannot ship here. Its own `name` table shows it is Bitstream Vera by way of
DejaVu, with a 2009 Apple layer on top. So DejaVu Sans Mono is not a lookalike; it is Menlo's
immediate ancestor, the last point in the chain where the outlines were given away. §104 chose it
over JetBrains Mono, which has the cleanest license (OFL 1.1, no reserved name).

Menlo Regular 11 at 2x density is a 14x26 cell with a 16-pixel cap height. At 1x the cap is eight
pixels, where unhinted rendering goes soft. Build the atlas at the 2x cell. The candidates' licenses
and the metrics are in
[`type.md`](0142-a-text-display-worth-living-in/type.md).

## The palette

"Solarized Dark Higher Contrast" is not Ethan Schoonover's and not an iTerm2 built-in; it is a 2011
gist, and a different palette: canonical Solarized spends half its ANSI table on a gray ramp, and
the variant puts hues there. Both pass milestone 141 (a palette worth looking at)'s three properties after a one-unit nudge to a
single channel. §104 chose canonical Solarized Dark. The working is in [`palette.md`](0142-a-text-display-worth-living-in/palette.md).

## Rich text

At minting a cell was two bytes. A display somebody would live in needs 24-bit color, a `char`,
real bold and italic, underline styles and more: roughly 16 bytes a cell. Real bold reverses the
recorded *"bold is bright"*, whose reason (an 8x8 bold face is a smudge) expires at an anti-aliased
cell. So "rich" means four faces, and §166 ruled four, with italic as DejaVu's Oblique. Increment
five's built half and its limits are in
[`rich-text.md`](0142-a-text-display-worth-living-in/rich-text.md).

## How the verification survives, which it must

Three parties compute today's picture without talking to each other: the terminal draws it, the
kernel predicts the framebuffer pixel for pixel through the direct map, and the host grades QEMU's
`screendump`. The host checker's negative control is a screen with one letter changed, an `o` for a
zero. That structure is why text on this screen is proved rather than plausible.

The atlas keeps it exactly, which is the strongest argument for the atlas. A coverage table is a
`static` and a lookup, as pure as `bitfont::glyph`, so all three parties keep running the same
function. A runtime rasteriser would put a font engine into the test kernel image.

Determinism was measured twice, by two investigations that did not see each other's work.
`ab_glyph_rasterizer` is bit-identical across aarch64 and x86_64. `fontdue` at default features is
not: its x86-only SIMD path reorders float additions, and about one pixel in 150,000 differs by
1/255. In a tree whose checker fails on a single pixel, that decides the crate. Three instructions
follow, and increment three takes them as decided:

- Use `ttf-parser` plus `ab_glyph_rasterizer`, pinned at 0.1.10 (0.1.4 through 0.1.8 panic).
- Quantize coverage to `u8` at the boundary and never persist raw `f32`. A patch bump (0.1.5)
  changed raw output, and the `u8` cast is what absorbs it.
- Use grayscale anti-aliasing, not subpixel, because subpixel output depends on a panel QEMU cannot
  describe.

Blending belongs in linear light, which matters most for light text on a dark ground. Anti-aliasing
also makes milestone 141's properties decorative for this terminal, since every edge pixel is a
legal blend; the pixel-exact comparison carries the load. The numbers, the mechanism and the traps
are in [`determinism.md`](0142-a-text-display-worth-living-in/determinism.md).

## The dependency, which is §46's question and has a clean answer

The atlas splits the work. The generator is not on the verification path; the table it produces is,
checked by regenerating it. An outline parser is a whole subsystem this tree would never write, so
take `ttf-parser`. `ab_glyph_rasterizer` has zero dependencies and is a thin primitive, so take it
too. The pair has no transitive dependencies, and it built clean for aarch64 and riscv64 under
`no_std` plus `alloc`. A build-time tool is still inside the supply-chain gate. The rasteriser is
Apache-2.0 only, where the tree is mostly dual MIT or Apache-2.0, and the build-time atlas keeps it
out of the shipped graph. §166 ruled this pair. The test, the table and the prior art are in
[`dependency.md`](0142-a-text-display-worth-living-in/dependency.md).

## Sequencing

Six increments. The first two needed no decision from calef and no dependency at all, and they were
the larger half of "would use it outside a GUI".

1. Grow the surface. BUILT. §102 built, the scanout grown to 1280x720 and then retargeted on
   2026-08-27 to 924x344. The harness cost was paid, not dodged.
2. Make the grid a terminal. BUILT. Scrollback, UTF-8 in the VT engine, and the arrow keys, on a
   132x43 grid at today's 7x8 bitmap font.
3. The atlas and the host-side generator. NOT-STARTED. A tool that turns an outline font into a coverage table, a
   checked-in table, and a gate that regenerating reproduces it byte for byte. The crate pair, the
   `u8` boundary and grayscale coverage are decided and should not be re-litigated in the lane. It
   must also regrow the scanout (see Follow-on).
4. Blending, in linear light, with the sRGB tables. NOT-STARTED on purpose, since it has no consumer
   until the atlas exists. Small, and it is what makes the atlas look like type rather than gray
   mush.
5. Rich attributes. PARTIAL. Widen `Attr` and `Cell` for truecolor, real weights and underline
   styles, and ship the bold, italic and bold-italic faces. The widening is built; the faces wait
   on the atlas, and *bold is bright* stands until they land.
6. The palette. Built: milestone 141's gate exists and the Solarized Dark table passes it.

A seventh, listed and not recommended: a glyph service, a component serving rendered glyphs so a
program that draws text needs no filesystem authority and no rasteriser. It is the
capability-shaped answer, but it buys nothing this milestone needs (the atlas is smaller, faster and
verifiable) and everything rung three needs (arbitrary sizes, proportional faces, fallback). Propose
it as its own milestone when rung three is live.

## What is an architect's

Nothing blocks increment three now. The family was §104's (2026-08-20) and the face count §166's
(2026-10-05). Still calef's, and blocking nothing:

- Names. The generator tool, the atlas crate and any glyph service are all provisional.
- Whether the terminal ever gains a configurable palette. Milestone 141 flags that as larger than it
  sounds, because the palette is a `const` three parties agree on.
- Whether a nudged palette is still called Solarized (141's block).

## BUGS

- The 14x26 cell was measured from Menlo, and DejaVu Sans Mono was re-measured against it only at
  version 2.35 (matplotlib's copy on patagonia), not the 2.37 this block priced.
- Nothing here has been measured on the machine. The harness cost, the table sizes and the cell
  arithmetic are all computed from constants in the tree. The soft-float claim is read from two
  target JSONs and from what `compiler_builtins` provides. No rasteriser has been run on either
  target, which is the measurement §46's rule 1 would want before a dependency were taken.
- The atlas assumes the repertoire stays small. Full Unicode coverage is not a bigger table; it is
  a different design, back to a runtime rasteriser or a glyph service. The block says "a terminal
  does not need that", and a person running `cat` on a file of Devanagari would disagree.
- Every determinism number here is measured behavior, not a documented guarantee. Both
  investigations searched both projects' READMEs, docs and changelogs for any statement of output
  stability and found none. `ab_glyph_rasterizer` 0.1.5 already changed raw output in a patch bump.
  The regenerate-and-compare gate turns the assumption into a check, which is why increment three
  includes it.
- Neither project's CI tests more than one platform; both are `ubuntu-latest` only, so
  `ab_glyph`'s cross-architecture exactness is a property of its code that nothing upstream
  defends. ab-glyph issue 121 is open and uncommented: accumulated float error leaves tiny nonzero
  coverage outside the glyph, visible as shadows when text is drawn repeatedly. The reporter asks
  for better rounding, and if that is fixed, output changes.
- The x86 AVX2 path was verified statically, not run. Rosetta does not expose AVX2, so the absence
  of fused multiply-add was read out of generated assembly. Rosetta is also one machine, not native
  silicon. Neither limit touches the two bare-metal targets: the minimal crate set built clean for
  `aarch64-unknown-none-softfloat` and `riscv64imac-unknown-none-elf` on 2026-08-20, though the
  determinism runs were on the x86_64 and aarch64 hosts, not on either target (see above).
- This milestone enlarges the supply-chain gap §100 (the terminal font) recorded.
  `script/supply-chain` reads the cargo graph, so a font transcribed into a Rust table is invisible
  to it. Today that is a kilobyte of public-domain bitmap; the atlas would be hundreds of kilobytes
  under a third party's license. The register in `vendor/README.md` is where it belongs, written at
  the same time as the table.
- "Rich text" is read here as terminal attributes, not as a document model. If calef meant
  proportional type, embedded images, or anything a terminal is not, the right answer is rung three.

## Follow-on

- **Decision.** The font family was chosen on 2026-08-20:
  `design/decisions/0104-the-font-and-the-palette.md` picks DejaVu Sans Mono, Menlo's own ancestor.
- **Decision.** The palette was chosen in the same call, canonical Solarized Dark, in
  `design/decisions/0104-the-font-and-the-palette.md`, after this block's finding that "Higher
  Contrast" is a different palette rather than a contrast adjustment.
- **Done.** §102 is built and consumed: the page-frame object in `kernel/src/cap.rs` carries a
  count that is the run's length in pages, and `crates/graphics_protocol` sizes the scanout at
  924x344 over 311 frames.
- **Done.** The screendump cost was measured and settled: the referee poll in `xtask/src/main.rs`
  records the 39x growth, that both suites still finish in normal time, and calef's call to leave
  the 100 ms cadence alone until something is measurably slow.
- **Milestone 668.** Scrollback is still not reachable from a keyboard, and it needs an architect:
  the route has a new opcode in it. `design/roadmap/0668-scrollback-from-the-keyboard.md` has the
  work, the four options and the recommendation. The keys (`CSI 5;2~`, `CSI 6;2~`) are sent since
  2026-09-26.
- **Outstanding.** Increment three has not started: nothing in `Cargo.lock` mentions a TrueType
  parser or a glyph rasteriser, there is no generator under `tools/` or `xtask`, and no checked-in
  coverage table or regenerate-and-compare gate exists. Checked 2026-09-03.
- **Outstanding.** Increment five is partial: its widening is built (truecolor, the 256-color
  table, underline, strikethrough, dim, concealed; 2026-09-26). Real weights and italic need faces
  and wait on the atlas, so "bold is bright" stands. Underline styles beyond one line (SGR `4:3`
  and the rest of the colon forms) and the underline color (SGR 58) are not built: the parser
  swallows `:` whole, recorded in notes/glyphs.md.
- **Done.** Whether the atlas ships one face or four: four, with italic as DejaVu's Oblique
  (calef, 2026-10-05 UTC, per §166). §104 chose the family and said nothing about faces.
- **Done.** The palette increment: milestone 141 is BUILT 2026-09-29, and the one-unit nudge §104
  flagged is in `crates/video_terminal/src/lib.rs` (entry 14, `0x93a1a0`). Whether a nudged
  palette is still called Solarized is calef's (141's block).
- **Done.** The chosen face is re-measured (2026-09-26, `fontTools`, DejaVu Sans Mono 2.35): 2048
  units per em, every advance 1233, ascender 1901, descender -483, no line gap. Those are Menlo's
  numbers exactly, so the 14x26 cell stands.
- **Outstanding.** At 14x26 the 924x344 scanout is a 66x13 grid, under the 80x24 floor. Increment
  3 must regrow the scanout: 1120x624 (683 page frames) for 80x24, or 1848x1118 (2,018) to keep
  132x43.
- **Recorded.** A font transcribed into a Rust table stays invisible to `script/supply-chain`,
  which reads the cargo graph, and `vendor/README.md` still carries no entry for the current font
  or a future atlas.

## Index row

Minted by calef on 2026-08-20, on seeing the Kaypro-style font land: he wants Solarized Dark
Higher Contrast and Menlo Regular 11, and the sentence that is the milestone is *"a text display
so good that people would use it outside of a GUI."* Four axes, and the font is the smallest: the
surface, the terminal, the type and the color. The finding that makes the expensive axis cheap is
that a monospace grid puts every glyph on an integer cell boundary. So the rasteriser belongs at
build time and the runtime keeps a coverage table exactly as `bitfont` keeps a bit table, which is
what preserves the three-party pixel-exact proof. Increments one and two (the scanout, first grown
to 1280x720 via DECISIONS §102 then retargeted 2026-08-27 to 924x344 for a 132x43 grid at the
shipping 7x8 cell, and a real terminal: scrollback, UTF-8, arrow keys) are built. Increment five
is partial (truecolour and the line renditions built 2026-09-26; the faces are ruled and wait on the
atlas). The navigation cluster is in the keymap, and scrollback from the keyboard is a proposal because its
route needs a new opcode. Six, the palette, is built (the gate of milestone 141, a palette worth
looking at). Three and four remain NOT-STARTED; the ruling of 2026-10-05 UTC and milestone 141 is
built, so nothing blocks them.
