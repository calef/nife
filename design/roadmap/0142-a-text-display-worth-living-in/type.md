# The type: what would ship, because it will not be Menlo

This appendix belongs to [milestone 142 (a text display good enough that people use it instead of a
GUI)](../0142-a-text-display-worth-living-in.md). It was moved out of that page on 2026-10-12 (UTC)
under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md), and the
main page is complete without it. It holds the font investigation as written before
[§104 (the rich-text font is DejaVu Sans Mono, and the palette is
Solarized)](../../decisions/0104-the-font-and-the-palette.md) chose DejaVu Sans Mono on 2026-08-20.

## Menlo cannot ship, and DejaVu is its ancestor

Menlo is Apple's and cannot ship here. It is bundled with macOS and is not redistributable. This
project has already excluded three fonts on exactly this ground:

- the Kaypro II ROM, for stating no license at all;
- Linux's `font_8x16.c`, for GPL;
- Fixedsys Excelsior, for a public-domain claim that could not be read at its source.

An Apple system font is a clearer exclusion than any of them.

Its ancestry is not folklore; it is in the file. `/System/Library/Fonts/Menlo.ttc` on this
machine, read with `fontTools`, carries its own provenance in its `name` table:

| Field | Value |
|---|---|
| Copyright (id 0) | `Copyright (c) 2009 Apple Inc. Copyright (c) 2006 by Tavmjong Bah. Copyright (c) 2003 by Bitstream, Inc. All Rights Reserved.` |
| Trademark (id 7) | `Menlo is a Trademark of Apple Inc.` |
| Manufacturer (id 8) | `Bitstream` |
| Designer (id 9) | `Jim Lyles` |
| Vendor URL (id 11) | `http://www.gnome.org/contact/` |

Apple's own metadata confirms every part of the claim, and it is more specific than the folklore.
Jim Lyles designed Bitstream Vera; Tavmjong Bah is the DejaVu project; the vendor URL still points
at GNOME, which is where Vera was released. Apple's contribution is a 2009 layer on top. The
trademark and the *All Rights Reserved* are Apple's claim over their file, not over the shape of the
letters.

So DejaVu Sans Mono is not a substitute for Menlo; it is Menlo's immediate ancestor. Choosing it is
not settling for a lookalike. It is taking the same outlines from the last point in the chain where
they were given away.

## The metrics, and why the atlas is built at the 2x cell

The metrics are measured from the same file, and they replace the estimate this block was first
drafted with. Menlo is 2048 units per em. Every glyph advance is 1233 units (0.602 em, so it is
genuinely monospaced). `hhea` gives ascender 1901 and descender -483 with zero line gap; cap height
is 1493 and x-height 1120.

| "Menlo Regular 11" at | Cell | Cap height |
|---|---|---|
| 1x (11 px per em) | 6.62 x 12.80 px, so a 7x13 cell | 8.0 px |
| 2x (22 px per em) | 13.25 x 25.61 px, so a 14x26 cell | 16.0 px |

"11 point" is not a number of pixels until a density is fixed. That choice decides whether this
looks like macOS or like a 1990s X terminal. At the 1x cell the cap is eight pixels tall, which is
where unhinted rendering goes soft and where hinting engines were invented. At the 2x cell it is
sixteen, which is where unhinted anti-aliasing has looked good for fifteen years. Build the atlas at
the 2x cell.

One more number from the same read: Menlo ships four faces (Regular, Bold, Italic, Bold Italic) and
3,157 glyphs in the regular. We would ship four faces and a few hundred glyphs.

DejaVu Sans Mono 2.35 was re-measured on 2026-09-26 with `fontTools`: 2048 units per em, every
advance 1233, ascender 1901, descender -483, no line gap. Those are Menlo's numbers exactly, so the
14x26 cell stands.

## The candidates

Licenses were read at their sources rather than recalled, and file sizes taken by downloading the
releases:

| Family | License | Reserved name | Faces | Regular `.ttf` |
|---|---|---|---|---|
| DejaVu Sans Mono 2.37 | Bitstream Vera for the base, DejaVu's own changes public domain, Arev glyphs on a Vera-shaped license | "Bitstream", "Vera", "Tavmjong Bah" and "Arev" may not appear in a modified name. "DejaVu" itself is not reserved. | 4 | 340,712 B |
| JetBrains Mono 2.304 | OFL 1.1 | none declared, so OFL clause 3's renaming requirement does not bite | 4 and a variable font | 273,900 B |
| Source Code Pro 2.042R | OFL 1.1 | "Source", which is broader than it looks | many | 210,312 B `.ttf`, 131,128 B `.otf` |
| Menlo | Apple, All Rights Reserved | not applicable | 4 | not available |

Three things in that table decide more than the letterforms will.

DejaVu is the closest thing to Menlo that can be shipped, and it is the generation Apple built on.
Against that, the project has been dormant since 2016 and has no variable font, so nobody is fixing
a glyph we find wrong. Its license obliges the Bitstream and Bah notices to travel with every copy,
and forbids selling the font by itself.

JetBrains Mono has the cleanest obligations of the three: OFL 1.1 with no Reserved Font Name. That
is exactly the clause §100 (the terminal font) called the expensive one when it refused Terminus:
*"being picky about fonts means eventually fixing a glyph"*. A font with no reserved name lets us fix
one and keep the name. It is also the only actively maintained candidate. Source Code Pro reserves
the word "Source" and inherits §100's objection whole.

None of the three is on the supply-chain allow-list, and that is a fact about crates rather than
about fonts. `deny.toml` allows `MIT`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`,
`BSD-3-Clause` and `0BSD`; neither `OFL-1.1` nor the Bitstream Vera license is there. A font
arriving as a crate would therefore stop the build and need a scoped exception, the way
`libfuzzer-sys` got one for NCSA. A checked-in coverage table does not. It inherits §100's recorded
gap instead: `script/supply-chain` reads the cargo graph, and a font transcribed into a Rust table is
not in it.

## The fork as it was posed

The recommendation was a fork rather than an answer, because a font is chosen by looking. DejaVu
Sans Mono if the goal is *"what calef asked for"*, since it is Menlo's own ancestor. JetBrains Mono
if the goal is a font this tree can live with for years. `bitfont`'s specimen harness already renders
any candidate on identical sample text (`cargo run -p bitfont --example specimen`), which is how
§100 was decided. §104 then took DejaVu Sans Mono.
