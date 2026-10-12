# The determinism measurement, and the rendering choices it forces

This appendix belongs to [milestone 142 (a text display good enough that people use it instead of a
GUI)](../0142-a-text-display-worth-living-in.md). It was moved out of that page on 2026-10-12 (UTC)
under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md), and the
main page is complete without it. It holds the measurements behind the page's "How the verification
survives" section, which decided the crate.

## Why the worry is real

Three parties compute the picture without talking to each other: the terminal draws it, the kernel
predicts the framebuffer pixel for pixel, and the host grades QEMU's `screendump`. A runtime
rasteriser would keep that check in principle, since it is still a pure function. In practice the
kernel-side witness would have to link an outline parser and a rasteriser into the *test* kernel
image. That is a font engine inside the thing whose smallness is the thesis, and test-only or not,
it is a bad trade for a property we can have for free.

Two versions of a rasteriser can disagree by a coverage value or two. If the rasteriser ran at
runtime, that disagreement would be a mysterious three-party mismatch. With a build-time atlas it is
a reproducibility question about a checked-in artifact: pin the generator, check in the table, and
gate that regenerating produces the same bytes. `script/vendor-verify` already has that shape for
RedoxFS ("upstream plus our recorded patches"). `crates/bitfont/src/glyphs.rs` has it for its
transcription, which was done by a script with no bits changed.

## Cross-architecture: measured twice

The worry was measured rather than argued, twice, by two investigations that did not see each
other's work.

A peer session measured cross-architecture output stability on 2026-08-20. It measured at raw `f32`
coverage rather than at the quantized byte, over 95 ASCII characters at nine sizes (fractional ones
included) across four fonts. It ran on x86_64 and aarch64, with both `std` floats and `libm`:

- `ab_glyph_rasterizer` is byte-identical across every combination.
- `fontdue` 0.9.4 is not, at default features. Its `simd` feature is on by default and compiles an
  SSE path on x86 only, and the four-wide prefix sum reorders float additions against the scalar
  accumulation. One pixel in 151,414 differed on JetBrains Mono; three in 139,454 on Source Code
  Pro. The delta was always exactly 1/255, and dimensions and metrics always matched.

A second, independent investigation reproduced this at larger scale and reached the same numbers by
a different route. It hashed 58,708 glyph renderings across fifteen fonts on raw `f32` bits.
`ab_glyph` was identical between aarch64 and x86_64. For `fontdue`, 136 of 47,166 renderings
differed (0.288%), every failure exactly one pixel by exactly one 255th: the same signature. It also
pinned the mechanism. In the four-wide prefix sum, lane 3 computes `(a3+a2) + (a1+a0)` where the
scalar path computes `((a0+a1)+a2)+a3`, and float addition is commutative but not associative.

The portability holds because of the compiler rather than the crate. Rust does not enable
floating-point contraction, so `x + dxdy * dy` cannot silently become a fused multiply-add on one
architecture and not the other. That was checked in generated assembly rather than assumed:

- zero `fmadd` or `fmla` on aarch64 at `-C opt-level=3` (35 `fmul` and 32 `fadd` kept separate);
- zero `vfmadd` on x86_64, even under `-C target-feature=+avx2,+fma`.

## Cross-version, and the one instruction this section produces

The cross-version question is the one a checked-in atlas actually depends on. The first answer
recorded here was too optimistic: a patch bump did change raw output. `ab_glyph_rasterizer` 0.1.5's
changelog says it, verbatim:

> Remove cap of `1.0` for coverage values returned by `for_each_pixel`, now `>= 1.0` means fully
> covered. This allows a minor reduction in operations / performance boost.

Measured across 0.1.1, 0.1.4, 0.1.5 and 0.1.10, the raw `f32` hash changes at 0.1.5 and the 8-bit
bitmap hash does not. Rust's float-to-integer `as` cast saturates and folds the now-uncapped values
back onto 255.

So the instruction the atlas takes from this is one line: quantize to `u8` at the boundary and never
persist raw `f32`. That makes the version-to-version risk mostly disappear, and it costs nothing,
because 8-bit coverage is what the table holds anyway. It is the difference between a table that
survived a patch bump by luck and one that survives it by construction.

With that in place, the measured picture is:

- `ab_glyph` 0.2.15 through 0.2.32 bit-identical;
- `ab_glyph_rasterizer` 0.1.5 through 0.1.10 bit-identical;
- `fontdue`'s scalar path byte-identical 0.6.4 through 0.9.4.

Neither project promises any of it, and the page's `BUGS` says so.

And pin 0.1.10 specifically. `ab_glyph_rasterizer` 0.1.4 through 0.1.8 panic with an index out of
bounds on some in-bounds-adjacent geometry, which a randomized harness hit immediately. Fixed in
0.1.9.

## A trap in `fontdue`

One more trap, in the crate this block does not recommend, recorded so nobody rediscovers it.
`fontdue`'s `FontSettings::scale` (default 40.0) feeds the curve linearization tolerance at font
*load* time. So the same glyph at the same pixel size renders differently depending on a number set
somewhere else entirely. `'a'` at 32 px gives byte sums of 32,613, 32,682 and 32,643 at scale 40,
100 and 12. A picture that depends on a load-time setting is precisely the shape this tree's
three-party check exists to refuse.

## Why one pixel in 150,000 decides the crate

One pixel in 150,000 is a catastrophe for this tree specifically. The kernel's witness and the
host's checker compare every pixel, and a single mismatch is a failed test. A rasteriser that
disagrees between aarch64 and riscv64 breaks the three-party agreement outright. Pinning the version
is not enough for `fontdue`. It would need `default-features = false` forced at every call site,
which is a rung-four mechanism (a note somebody has to remember) guarding the property the whole
check rests on.

So the crate is decided by determinism rather than by dependency count, and the two agree:
`ttf-parser` plus `ab_glyph_rasterizer`. It is also the smallest graph on offer and the one already
proven to build on both ISAs. A build-time atlas softens this further. The divergence would then be
between the machine that regenerated the table and the machine that checked it, rather than between
two target architectures. Softening it is not a reason to choose the crate that has it.

## Two rendering choices that are verification choices

Both should be made deliberately:

- Grayscale anti-aliasing, not subpixel (ClearType-style) anti-aliasing. Subpixel rendering triples
  the horizontal resolution by lighting a panel's red, green and blue stripes independently. So it
  depends on that panel's subpixel order, which a framebuffer does not know and QEMU's `screendump`
  cannot represent. It also produces colored fringes that a pixel-exact check would have to model.
  Grayscale keeps the picture a property of the pixels rather than of somebody's monitor, and it is
  where Apple ended up as well.
- Blend in linear light, not in sRGB. This is the difference between "anti-aliased" and "good", and
  it is the thing nobody mentions. Lerping coverage in sRGB space makes light-on-dark text look
  bolder and dark-on-light text look thinner than the outline says. Solarized Dark is light text on
  a dark ground, which is exactly the case that goes wrong. The fix is a 256-entry sRGB-to-linear
  table and a reverse table, all integer, no float at runtime, and perhaps fifty lines.

## What anti-aliasing retires from milestone 141

Milestone 141 (a palette worth looking at) could not have known this, because it was written for a
bitmap terminal. Its palette check exists so that a corrupted pixel is a detectably wrong color
rather than a different legal one. With anti-aliasing there is no such thing as an off-palette
pixel. Every edge pixel is already a blend, so the legal set is not sixteen values; it is every
value between each pair. The properties do not become wrong. They become decorative for this
terminal, and the pixel-exact comparison (always the stronger check) carries the whole load. 141
should still land, because the bitmap terminal is what ships in between and because the palette it
admits is the palette this one inherits. (141 landed on 2026-09-29.)
