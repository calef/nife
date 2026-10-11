# The counter rate in userspace: a closed limitation, and why it stayed open

An appendix of [ISA discovery](../isa-discovery.md), moved from its `BUGS` on 2026-10-11 (UTC)
under §212 (a prose budget). The entry was closed on 2026-09-21. The parent keeps a one-line
record of it; this file keeps the reasoning that kept it open and the lesson.

## What was wrong

`user_mode_runtime::cntfrq` returned 10 MHz on RISC-V until 2026-09-21.

This entry used to say that userspace *cannot* learn the rate. It said closing it was an ABI
addition, an aux-vector entry at process start, the way Linux passes `AT_HWCAP`. So it was filed as
a design fork rather than a fix.

## Why the reasoning was wrong

The premise was right and the conclusion was not. Userspace cannot *read* the device tree, but the
kernel can *hand it a page*. That is not an ABI addition, because the mechanism already existed and
shipped on `x86_64`: `counter_frequency_protocol`, from milestone 161 (the x86_64 kernel port). The
fix was to widen a `cfg`, not to design a handoff.

## The lesson

A limitation recorded with a proposed fix attached stops being re-examined. This one went two
months without anybody asking whether the tree already solved it somewhere else. What made it
urgent was radon, whose rate is 4 MHz, so the "honest gap" was a silent 2.5x on the one board that
matters.
