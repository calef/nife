# Phase 4's evidence: what phases 1 to 3 and E3 on radon say about a hand-written fastpath

An appendix of [milestone 188 (the IPC fastpath)](../0188-ipc-fastpath.md), moved out of it on
2026-10-12 (UTC) under §212 (a prose budget). The main page is complete without it: it keeps the
verdict, the headline figures and the recommendation. This file keeps the argument for and against
phase 4 after phases 1 to 3, and the E3 run on radon with its table. Both were written on 2026-09-04.

## Phases 1 to 3 did not close the gap and were never going to

The shape the system runs is `ipc_call_reply`, at 5,936 to 8,122 bytes against a 4 KiB target. That
is 48% to 103% over, after the cheap method has been applied and after the accounting was made
honest. The block's own closing condition was *"if phase 3 gets the CALL/reply closure under 4 KiB,
phase 4 is not worth building"*. It did not, by a wide margin, on any architecture.

## Three things changed in phase 4's favor, and one against it

For it. The number a fastpath would be measured against can now be trusted, which was the block's
second reason to wait and is discharged. The gate reports the shape services run, aarch64's entry
figure is no longer gameable by 1,892 bytes, and the cold classification is derived from the code.
The scheduler bypass is still the largest skippable block: `schedule` + `finish_switch` + `wake` +
`switch_to` is 2,380 bytes of the 7,028, 34%, and phase 3 shrank it by only 256. And the riscv64 half of
milestone 74 (cycle counters) landed the same day, so radon can read real cycles through the SBI PMU extension. The
block had priced that as waiting on the silicon of milestone 127 (the seL4 machine).

Against it. Radon is not sufficient on its own, and the honest version of that must be said.
`kernel/src/arch/riscv64/pmu.rs` gives cycles. A cycle count on the U74 with a 32 KB L1i would show
*whether the round trip got faster*. It would not show *why*. Liedtke's claim is specifically about
the application's working set being evicted, which is a displacement measurement rather than a
latency one. The E4 experiment of milestone 134 (the register of measures) already measured 0 to 3% application displacement under IPC load and
is the instrument that would have to move. So radon turns "no observation at all" into "a latency
observation with no attribution", which is progress and is not the experiment.

## The recommendation, unchanged in direction and better supported

Do not start phase 4 on footprint arithmetic alone. Start it when radon can show a *latency*
difference on a padded build. That is milestone 134's E3 experiment run on silicon rather than under
icount, and it is a day of work rather than a project. If E3-on-radon shows nothing, phase 4 is a
standing verification obligation bought for a number nobody can observe, and this milestone should
close having said so. If it shows something, phase 4's option 1 is aimed at 34% of the closure and is
the one to build first. Option 1 is a fast path that still calls the proved `Rendezvous` methods and
skips only the scheduler and the error plumbing.

## E3 ran on radon, 2026-09-04, and it did not decide this

The paragraph above named the measurement that would settle phase 4, and the measurement came back
confounded rather than positive or null. The capture is in notes/footprint-perturbation.md: six
boots, interleaved, on a `single_hart` card where the placement lottery cannot appear.

| row | unpadded (3 boots) | padded (3 boots) | delta |
|---|---|---|---|
| `ipc_rtt` | 4259 · 4259 · 4259 | 4311 · 4310 · 4310 | +1.21% |
| `call_reply` | 5015 · 5013 · 5013 | 5088 · 5089 · 5088 | +1.49% |
| `ipc_rtt_el0` | 128606 · 128626 · 128615 | 124958 · 124391 · 124903 | -3.01% |

Every separation is non-overlapping, with a within-condition spread of 0 to 2 units against gaps of
74 and 3,865. On the arithmetic this is the row-1 outcome of the block's own routing table and
phase 4 is justified.

### It is not, because the third row is impossible

The padded build is 3% *faster* on the round trip that crosses EL0, and the padding is never
executed. It is reached through `core::hint::black_box` on a branch that is never taken. Resident
dead code has no mechanism by which to make anything faster. What moved is the layout, and the
layout effect is 193 ns where the footprint effect it is sitting on top of is 19 ns.

So the +1.49% on `call_reply` is a real difference between two binaries and is not evidence about
two footprints. E3 varies a Cargo feature, and a Cargo feature that inserts a symbol changes the
address of everything after it. This is the Mytkowicz result (*Producing Wrong Data Without Doing
Anything Obviously Wrong*, ASPLOS 2009) reaching our own bench data. It was equally true of the
2026-08-22 patagonia run; the small cache is what made it visible rather than what caused it.

### Phase 4 therefore stays exactly where it was

The reason is now sharper than "no evidence". It is that the instrument built to produce the evidence
cannot separate the effect from an artifact an order of magnitude larger. A second IPC path is a
permanent verification obligation, and 19 ns under a 193 ns artifact is not what anyone should buy
one with.

### What would decide it is a layout control

It is cheap: run `fastpath_pad` at several sizes rather than as a boolean, because footprint predicts
a monotonic dose response and layout does not. Written up as
[milestone 370 (a layout control for the perturbation experiments)](../0370-a-layout-control-for-the-perturbation-experiments.md).

### One thing the session did settle, in phase 4's disfavor but not against it

The board can now resolve these differences at all: `bench: cycles_per_tick 250.00` from milestone
74's riscv64 PMU, and a boot-to-boot spread of 0 to 2 units on a four-figure count. The
methodological complaint nobody could answer on patagonia, that a small percentage might be a timer
artifact, is answered. The measurement is precise. It is the attribution that is missing.
