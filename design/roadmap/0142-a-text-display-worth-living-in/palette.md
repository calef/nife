# Solarized Dark Higher Contrast, against milestone 141's three properties

This appendix belongs to [milestone 142 (a text display good enough that people use it instead of a
GUI)](../0142-a-text-display-worth-living-in.md). It was moved out of that page on 2026-10-12 (UTC)
under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md), and the
main page is complete without it. It holds the palette investigation that [§104 (the rich-text font
is DejaVu Sans Mono, and the palette is Solarized)](../../decisions/0104-the-font-and-the-palette.md)
ruled on, choosing canonical Solarized Dark.

## The name

calef named Solarized Dark Higher Contrast, and the first finding is about the name.

It is not Ethan Schoonover's, and it is not an iTerm2 built-in. iTerm2's `ColorPresets.plist`
holds eleven presets, "Solarized Dark" among them, and no Higher Contrast variant. The palette
under that name traces to a 2011 GitHub gist by `heisters`, titled *"Solarized High Contrast Dark
theme for iTerm2"*. It reaches most people through `mbadolato/iTerm2-Color-Schemes`; the two
files are byte-for-byte the same palette under two names.

## A different palette, not a contrast adjustment

All sixteen ANSI values differ from canonical Solarized Dark, as do the background, foreground,
bold and cursor colors. What it discards is Solarized's structural idea, and that is the difference
calef would actually feel:

| ANSI slot | Canonical Solarized Dark | Higher Contrast |
|---|---|---|
| 10 bright green | `#586e75` base01, a gray | `#51EF84` a green |
| 11 bright yellow | `#657b83` base00, a gray | `#B27E28` a yellow |
| 12 bright blue | `#839496` base0, a gray | `#178EC8` a blue |
| 14 bright cyan | `#93a1a1` base1, a gray | `#00B39E` a cyan |

Canonical Solarized spends half its ANSI table on a grayscale ramp on purpose, so only eight of
sixteen slots hold a hue. That is a deliberate and unusual choice. It means every program that uses
a bright color for emphasis (`ls --color`, a diff, a linter) gets a gray. The Higher Contrast
variant is a conventional sixteen-color terminal palette sitting on a Solarized-ish dark ground.
That is very likely why it exists and why someone would prefer it. Neither is wrong; they are
different things, and the record should not call the second one Solarized without saying so.

## The three properties, over both

Milestone 141 (a palette worth looking at, and a gate that lets it be one) defines three
properties. The check is over exactly the right sixteen values in both cases, because Solarized's
published ANSI mapping uses each palette entry exactly once.

| Property | Canonical Solarized Dark | Higher Contrast | The palette shipping at minting |
|---|---|---|---|
| 1. every entry has three distinct channel values | fails on one entry: ANSI 14 `#93a1a1` (`a1` twice) | fails on one entry: ANSI 2 `#6CBE6C` (`6C` twice) | fails on all sixteen |
| 2. no two entries are channel permutations | passes | passes | fails |
| 3. no two entries saturate `0xff` in the same channel | passes, and no entry has any channel at `0xff` at all | passes, same | fails on all three channels |

So both pass, each after a one-unit nudge to a single channel of a single color. `#93a1a1` becomes
`#93a1a0`; `#6CBE6C` becomes `#6CBE6B`. Neither is perceptible, both are inside the noise of the
display, and each is a one-character edit.

Two things beyond the verdict. Both candidates are strictly better test instruments than what
shipped, which failed all three properties. Both are also better on property 3 in a way 141 did not
anticipate: neither has a single channel at `0xff` anywhere, where the old palette had twelve. And
milestone 141's gate does not choose between them; it says both are admissible. The choice was
taste and the grayscale-ramp question above, and it was an architect's.
