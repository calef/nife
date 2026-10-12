# The bench procedure for option A, in order

An appendix to [milestone 192 (a keyboard on real silicon)](../0192-keyboard-on-real-silicon.md).
It holds the bench procedure option A's lane wrote on 2026-09-02, for whoever runs option A on a
board.

It cannot be run yet, and the reason is milestone 157 (U-Boot's `simple-framebuffer` handoff) rather
than this milestone. Option A is framebuffer output plus serial input, and the framebuffer half is
[157](../0157-uboot-framebuffer-handoff.md) (U-Boot's `simple-framebuffer` handoff), which is
NOT-STARTED. Until a board can put a pixel on a monitor there is nothing for a serial keystroke to
echo onto. Written now so that 157's lane inherits it rather than rederiving it.

Needs radon powered on (the Kasa plug), a serial console attached before power, a microSD card, and
a monitor on the board's HDMI. `notes/visionfive2.md` carries the board facts and the
failure-triage ladder.

1. Follow milestone 159's steps 1 to 3 verbatim (`script/board-image`, copy all three files from
   one run, DIP switches to QSPI, serial at 115200 8N1, interrupt U-Boot, the five `load`/`booti`
   lines). Nothing about this milestone changes the boot recipe.
2. Read the boot line `keystrokes:`, which is the whole of what this lane added and the first
   thing to check. (Corrected 2026-10-12, UTC: no boot prints this line since milestone 600 (the
   graphical terminal stack is built in userspace) removed it on 2026-09-26, and since milestone
   632 (graphics on demand) graphics start from the prompt, not at boot. The table is the line as
   this lane shipped it, and this step needs rewriting before a bench run.)

   | Line | What it means |
   |---|---|
   | `keystrokes: Serial (graphical boot)` | What a board should print. The framebuffer driver came up, the display terminal came up, and the UART is the keystroke source. |
   | `keystrokes: Keyboard (graphical boot)` | A virtio-input device was found, which on a board means something is wrong with device discovery, not that a keyboard works. |
   | no `keystrokes:` line at all | The graphical stack reported absent and the boot fell back to the plain `console`/`input` pair. Either no framebuffer was found (157's problem) or one of `gpu_driver`, `display_terminal`, `input` is missing from the archive. |

3. Look at the monitor. A prompt on the screen is 157's claim, not this one; record it either
   way, because everything below depends on it.
4. Type a character on the serial console. It must echo on the monitor, not on the serial
   line: `input` forwards it to `line_editor`, which echoes through `display_terminal`. If it
   echoes on the serial console instead, the boot took the plain fallback and step 2 said so.
5. Type `echo hello` and press return. `hello` on the monitor is option A complete on a real
   machine: a person at a laptop typing across a null-modem cable into a system whose output is on
   its own screen.
6. Failure modes, in the order they are worth checking. No echo at all and no `keystrokes:`
   line means the fallback (step 2). No echo with a `Serial` line means one of two things. Either
   the UART receive interrupt is not routed, or the display path hung. The first is the same
   PLIC-source hazard `uart_irq_and_source`'s own doc records for the JH7110, where the QEMU
   constant armed the wrong source. The second is how the second-`FLUSH` hang of milestone 177
   (wire the graphical terminal stack into the real interactive boot) looked before it was
   root-caused and fixed on 2026-09-19: a character echoing once and then nothing.
