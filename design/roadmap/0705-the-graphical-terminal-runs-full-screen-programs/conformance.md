# vttest against the candidate engines

This appendix belongs to [milestone 705 (the graphical terminal runs full-screen programs)](../0705-the-graphical-terminal-runs-full-screen-programs.md).
It holds the conformance measurement behind options A, F and G.

Measured on 2026-10-18 (UTC) on patagonia with vttest 2.7 (20251205, Homebrew). The harness was
throwaway and lives in the lane's scratch directory.

## Method

vttest runs in a pty, 80 by 24, with `TERM=xterm`. The pty's other end is an engine probe in
server mode. vttest's output goes into the engine, the engine's replies go back into the pty, and
the screen is dumped at every "Push <RETURN>" pause. The same keystroke script drives every engine:
menus 1 (cursor movement), 2 (screen features) and 8 (insert and delete), and menu 6's reports 3, 4, 5
and 7. tmux 3.8 ran the same script as a reference terminal, read with `tmux capture-pane`.

tmux turned out not to be a reliable oracle. On menu 1's autowrap screen it splits letter pairs that
vttest asks to share a row, and it places menu 2's origin-mode lines in the wrong order. Its capture
also cannot show tab runs or DEC line-drawing glyphs. So each disputed screen was graded by reading
vttest's own instruction on it ("These two lines should look the same", "The top line should be
'AB'"). Undisputed screens are the consensus.

Excluded: the 132-column screens (menu 1's second pass, two in menu 2, menu 8's second pass), because
every display here is a fixed 80-column grid. That leaves 24 screens, plus four report tests.

## Results

| engine | screens passed (of 24) | fails | reports answered |
|---|---|---|---|
| libvterm 0.3.3 | 23 | 2.15: DECSC does not save the DEC graphics charset | DSR 5 and 6, DA, DA2 |
| `alacritty_terminal` 0.26.0 | 22 | 2.7 and 2.9: soft and jump scroll down inside a 2-line region draw at the top of the screen | DSR 5 and 6, DA, DA2 |
| `vt100` 0.17.0 | 17 | 1.1, 1.3, 2.1, 2.2, 2.15, 8.3, 8.4: wrap, tabs, insert mode, delete | DSR 6 and DA only, from the probe's own callback |
| `video_terminal` (ours) | 7 | 17 screens | none |
| tmux 3.8 (reference) | 20 | 1.3, 2.11, and two the capture cannot show | DSR 5 and 6, DA, DA2 |

Nobody answered DECREQTPARM (report 7), tmux included. The DA answers differ by design: each engine
names itself differently.

How to read it. libvterm and `alacritty_terminal` are a tie within one screen. `vt100`'s replies are
the probe's ten lines, not the crate's, so its report column measures the adapter. Ours passes 7,
which is A's starting point; Part 2's census predicts its failures.

libvterm does not clear the screen on DECCOLM (132-column mode), so on 1.3 it meets vttest's
criterion with stale rows above and below. Graded on the criterion rows, it passes; a terminal that
never switches columns does not need DECCOLM.
