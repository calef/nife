# The truncated echo, week 2026W42 (name provisional)

A swish-check leg that stops reading input partway through a typed line. Companion to
[the swish-check CI flake, measured](swish-check-flake.md); every date is UTC.

## The two sightings

Both are aarch64, and both stop on a line longer than the PL011's 16-byte receive FIFO.

| when | where | line typed | echo stopped at | bytes echoed |
|---|---|---|---|---|
| 2026-10-12 17:15 | CI job 114922119493 (run 38282228136), third boot | `graphical_terminal` | `graphical_termin` | 16 of 19 |
| 2026-10-13 16:00 | patagonia, a local `script/swish-check --arch aarch64` on #2060's branch, host swapping | `xargs caps rm globmany/m-*.txt` | `xargs caps rm globmany/m` | 24 of 31 |

Each was green on the next run. Nothing after the cut ever reached the shell: no more echo and no
Enter, for 30 s in the local run and 120 s in CI. No other failed swish-check job from
2026-10-01 to 2026-10-13 has this shape.

## The cause: a receive interrupt cleared while a byte waits

`components/src/input.rs` is the UART's receive driver. Its loop was wait, drain the FIFO, clear the
interrupt at the device, acknowledge. QEMU's PL011 (11.1.2, `hw/char/pl011.c`) raises its receive
interrupt only when the FIFO goes from empty to one byte, because `read_trigger` is fixed at 1. A
write to `ICR` lowers it whatever the FIFO holds.

A line longer than 16 bytes reaches the FIFO in two pieces. The second piece arrives only after the
guest has read bytes out (`qemu_chr_fe_accept_input`), so it races the driver. If it lands after the
drain's last empty check and before the clear, the clear wipes its interrupt. Every later byte
lands on a non-empty FIFO and raises nothing, so the driver waits forever with input sitting in
the device. Host load widens the window, since a descheduled vCPU sits inside it longer.

The fix clears the interrupt before draining. A byte that arrives after the drain's last check then
leaves the interrupt raised, and the next wait returns at once. The riscv64 and x86_64 legs drive a
16550. QEMU recomputes its receive interrupt from what the FIFO holds, and the driver never clears
it by a write, so neither leg has this window.

## Ruled out

- The line editor. It redraws the whole line when a queued kernel line is printed, but a redraw
  repeats bytes and cannot drop the ones that never arrived. No Enter was ever processed.
- The harness's timing. The echo wait is 30 s against a 0.11 s median line, and the input
  never resumed at all. Not a load-sensitive assertion; the register in
  [load-sensitive-assertions.md](load-sensitive-assertions.md) does not take it.
- The console path dropping output. Input stopped, not output: the command never ran.

## The proof

`script/swish-check --arch aarch64 --input-race` builds the driver with `input_race_probe`, which
holds 200 ms after every drain, and types Enter 50 ms after each line. Enter then always arrives
inside the hold. One boot each on patagonia, 2026-10-13:

| driver loop | load average at start | result |
|---|---|---|
| drain, hold, clear, acknowledge | 26.8 | red on the first line: `echo hello world \| wc` echoed, Enter never delivered |
| clear, drain, hold, acknowledge | 26.9 | green: 156 lines in 47.8 s, every one echoed and answered |

## BUGS

- The probe boots aarch64 only, in CI's `swish-check` job and `script/ci-build swish-check`. The
  riscv64 and x86_64 drivers share the loop but not the window, so a probe there proves nothing.
- On real PL011 silicon the receive interrupt fires at a FIFO trigger level, and fewer bytes than
  that raise only the receive-timeout interrupt, which this driver does not arm. QEMU fires at one
  byte, so nothing here tests it. No aarch64 board in this tree has a PL011 console today.
