# Cycle counters: the corrections this block carries

An appendix to [milestone 74 (cycle counters)](../0074-cycle-counters.md), under §212 (a prose budget).
This appendix holds three things the block once said wrongly, kept rather than silently corrected:
two misquotations of `notes/benchmarks.md`, a parity heading, and a wrong mechanism for x86_64.

## The quotation, wrong twice

This file has quoted the `notes/benchmarks.md` paragraph wrongly twice. That is why the quote on the
page carries an attribution line a gate can check.

The first version quoted the retracted arithmetic as the current record: *"At ~3.2 GHz, 705 ns is
~2,200 cycles round trip... we are 4 to 7 times heavier"*. Milestone 101 (the L4 calibration) had already re-measured it
and found three errors. The replacement written on 2026-08-04 was a paraphrase presented as a
quotation. It read "At ~3.2 GHz, 350 ns is roughly 960 to 1,420 cycles round trip", and those words
appear in no note.

It also put back the one assumption the correction had removed. The note says in as many words that
the old paragraph's 3.2 GHz "is not this machine". The 960-to-1,420 range is 350 ns against the M3's
two clocks, 2.75 GHz on an E-core and 4.05 GHz on a P-core, and nothing pins the vCPU thread to
either. At 3.2 GHz, 350 ns is ~1,120 cycles, a single number rather than a range. So the sentence
attached a correct range to the clock that had just been rejected, and read as sober while doing it.

The mechanism rather than the apology: a prose block quote of another document is a citation no
gate resolves. `script/citations` checks one only when it carries a `-- path` attribution
line. It is the work of milestone 97 (citations that name what they cite). Neither wrong version had one, so both passed every gate in the tree, twice. They
did so in the file notes/citations.md already uses as its worked example. The binding form costs
one line and is rung two of CLAUDE.md's ladder instead of rung four.

## The parity heading

The parity heading read "Parity makes this two ISAs, not one" until 2026-09-16. That was worse than
wrong: it asserted completeness over a gap. It was true when written on 2026-08-03, when x86_64 was
declared and not started. x86_64 now boots, enumerates PCI, runs `std` and runs unmodified
`ripgrep`. So a reader met a parity claim covering two of three architectures, with nothing saying
which one was missing or why. The wording of §19 (architectural parity is a tenet) is that a capability ships everywhere or a scope note
records the gap and the plan. This block had the gap and no note. It has one now, on the page.

## The wrong mechanism for x86_64

The parity section named the wrong mechanism for x86_64. It is kept here because it is the
reasoning most likely to be re-derived. It said "x86_64 already reads `rdtsc` in
`arch/x86_64/timer.rs` for its own calibration", implying the probe could be built on that read. It
cannot: `arch::x86_64::timer::now()` is `rdtsc`, so such a probe divides one counter by itself and
prints an exact `1.00` on every part. Milestone 309 (unhalted core cycles on `x86_64`) built it on `IA32_PERF_FIXED_CTR1` (unhalted
core cycles) instead, and its block has the argument.
