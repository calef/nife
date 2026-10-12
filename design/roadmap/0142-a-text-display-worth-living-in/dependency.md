# The dependency: §46's test, the measured graphs, and the prior art

This appendix belongs to [milestone 142 (a text display good enough that people use it instead of a
GUI)](../0142-a-text-display-worth-living-in.md). It was moved out of that page on 2026-10-12 (UTC)
under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md), and the
main page is complete without it. It holds the dependency analysis that [§166 (the rasteriser dependency, and whether the glyph atlas ships one face or four)](../../decisions/0166-the-rasteriser-and-how-many-faces.md)
ruled on 2026-10-05 (UTC): rasteriser
A, `ttf-parser` 0.25.1 plus `ab_glyph_rasterizer` 0.1.10.

## §46's test, in order

[§46 (thin primitives or whole subsystems; we write everything in
between)](../../decisions/0046-dependency-rule.md) runs its test in order. The atlas makes the answer
unusually clean because it splits the work in two.

### 1. Is it on the verification path?

*The runtime is; the generator is not, and that is the whole point.* What gets verified is the
table, by regenerating it and comparing bytes, the same shape `script/vendor-verify` uses for
RedoxFS. The code that produced the table is not in the kernel, not in the terminal, and not in any
proof harness. So §46's strongest reason to write a thing does not apply to it at all: you cannot
restructure someone else's crate to make a model checker tractable. A runtime rasteriser would land
squarely on the verification path and this answer would reverse.

### 2 and 4. Is the spec the whole of correctness?

Here the two halves separate, in opposite directions:

- A TrueType and OpenType outline parser is a whole subsystem we would never write. The format is
  decades deep in vendor quirks. Correctness is won by having been fed thousands of real fonts
  rather than by reading the specification. That is rule 4's crypto shape exactly: take it.
- A scanline coverage rasteriser is not. Filling a closed path of quadratic and cubic beziers and
  reporting per-pixel coverage is a few hundred lines of published, fully specified geometry. It has
  an obvious oracle (compare against a supersampled reference) and no adversary. That is the
  in-between this tree writes.

So the shape §46 recommends is: take `ttf-parser` for the outlines, write the coverage rasteriser,
and run both on the host. That is "thin primitives or whole subsystems, nothing in between" applied
honestly rather than as a slogan. The tree gets a rasteriser it can reason about, attached to a
parser it never has to.

## Two mechanical facts a lane will hit

- A build-time dependency is not outside the supply-chain gate. One `deny.toml` is applied to every
  workspace in the tree, and its own header gives the reason: *"a licence we would refuse in the
  kernel is not acceptable in the host tooling either."* So a generator in `xtask` or `tools/` is
  checked by the same advisory and license policy as anything that boots. What it does buy is that
  it is not in the shipped artifact. That is a real reduction in exposure, not a reduction in
  scrutiny.
- The license allow-list is five entries (`MIT`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`,
  `BSD-3-Clause`, `0BSD`) plus one scoped exception for `libfuzzer-sys`. It is an allow-list on
  purpose, *"because the failure we care about is a licence nobody looked at"*. A chosen font
  arriving as a crate under the OFL would stop the build and need an explicit scoped exception. A
  checked-in table sidesteps that, and inherits §100 (the terminal font)'s recorded gap instead:
  `script/supply-chain` reads the cargo graph, and a font transcribed into a Rust table is not in it.

## The measured graphs

A peer session measured these on this machine on 2026-08-20 with `cargo tree` and `cargo build`,
rather than reading them off crates.io. Transitive dependencies exclude the crate itself, at default
features:

| Crate | Deps | In `no_std` | Needs `alloc` | License |
|---|---|---|---|---|
| `ttf-parser` 0.25.1 | 0 | 2 (`core_maths`, `libm`) | no | MIT OR Apache-2.0 |
| `ab_glyph_rasterizer` 0.1.10 | 0 | 1 (`libm`) | yes | Apache-2.0 only |
| `ab_glyph` 0.2.32 | 3 | 5 | yes | Apache-2.0 only |
| `fontdue` 0.9.4 | 7 | 7 | yes | MIT / Apache-2.0 / Zlib |
| `swash` 0.2.10 | 12, one a proc-macro | 14 | yes | Apache-2.0 OR MIT |
| `cosmic-text` 0.19.0 | 39 | 30 | yes | MIT OR Apache-2.0 |

The two-crate option is real and the APIs already fit. `ttf-parser` gives
`Face::outline_glyph(GlyphId, &mut dyn OutlineBuilder)` against a trait of
`move_to`/`line_to`/`quad_to`/`curve_to`/`close`. `ab_glyph_rasterizer` accepts exactly
`draw_line`/`draw_quad`/`draw_cubic` and yields `for_each_pixel(|index, alpha: f32|)`. So the pair
is two crates, zero transitive dependencies on the host, and no shaping stack at all. That is a
smaller graph than writing the rasteriser and taking only the parser. It is close enough to the §46
recommendation above that the honest thing is to say so: the parser is the whole subsystem, and
`ab_glyph_rasterizer` at 0 dependencies is a thin primitive rather than the in-between. Take both.

And it builds on both architectures already, which turns a recommendation into a fact. The same
peer session compiled the minimal combination, skipping `ab_glyph` and `owned_ttf_parser` entirely.
That is four crates in total (`ttf-parser`, `ab_glyph_rasterizer`, `libm` shared between them, and
`core_maths`), `no_std` plus `alloc`. It built clean for `aarch64-unknown-none-softfloat` and
`riscv64imac-unknown-none-elf`. [§19 (architectural parity is a
tenet)](../../decisions/0019-architectural-parity.md) makes parity a gate rather than an aspiration,
and the cheapest option on the table is the one already proven to build on both. That also matters
for the option this block does *not* recommend: if a runtime rasteriser is ever wanted after all,
this is the one that could be.

## Two license facts

`ab_glyph_rasterizer` is Apache-2.0 only, where nearly everything else in this tree is dual MIT or
Apache-2.0. Apache-2.0 is on the allow-list, so the gate is satisfied. But a dual-licensed project
taking an Apache-only dependency narrows what its downstream can do. Two things soften it:

- the minimal combination drops `ab_glyph` and `owned_ttf_parser`, which were the other two
  Apache-only crates, leaving `ttf-parser` dual and one crate not;
- a build-time atlas keeps even that one out of the shipped graph.

And `cosmic-text`'s tree contains `self_cell`, which is Apache-2.0 OR GPL-2.0-only, the only
GPL-adjacent crate in any of these graphs. Its tree also carries duplicate majors of `read-fonts`,
`font-types` and `skrifa`. That is rung three's problem, and it should meet it knowingly.

## Prior art

- `cosmic-text`, and why it is rung three's rather than this one's. It solves shaping, bidirectional
  text, fallback and proportional layout. A monospace grid needs none of them, and its dependency
  graph is priced above.
- Every terminal emulator ships an atlas. kitty, Alacritty and WezTerm all rasterize a glyph once
  and cache it in a texture, precisely because a terminal draws the same few hundred pictures
  forever. The move this block proposes is theirs with the cache moved from runtime to build time.
  That is available to us and not to them, because our repertoire and size are fixed at build time
  and theirs are not.
- macOS is the prior art for buying legibility with density rather than with grid-fitting and
  subpixel tricks. That is the trade the atlas makes when it picks a 2x cell.
- `bitfont` is the prior art inside this tree. The structure it proved (a table, a pure function, a
  transcription script that changed no bits) is the structure increment three keeps.
