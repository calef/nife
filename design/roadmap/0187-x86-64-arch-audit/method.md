# The first arch audit's method, and how the scale was counted

An appendix to [milestone 187 (read the x86_64 arch tree through the lens the first arch audit
used)](../0187-x86-64-arch-audit.md). It holds what `notes/arch-audit.md` actually did, which is the
whole scope of this milestone, and the method behind the page's line counts.

## What the existing audit actually did, because "on its own terms" is the whole scope

`notes/arch-audit.md` is not a review. Reading it as "somebody read the arch tree and looked for
bugs" is what would make this milestone unpickable, so the shape is worth stating precisely.

It took exactly one bug class, stated generally before any code was read. That note's heading is
*The bug class, stated generally*. The class is reproduced here without an attribution line, because
it is a block quote there too. `script/citations` cannot match a quote of a quote past the source's
own `>` prefixes.

> Kernel code that stages state in single-copy hardware registers, or in any per-CPU location that
> the exception path itself clobbers, across more than one instruction, while an exception,
> interrupt, or preemption can land in the middle.

It asked four fixed questions of every candidate, and the fourth is the one that does the work:

- (a) what is the window;
- (b) what can land in it;
- (c) what state is corrupted if something does;
- (d) is it reachable, or is it a window that exists in the instruction stream that nothing can ever
  land in.

Two of the three findings turned entirely on (d). One of them (the RISC-V `sstatus` mask) was
reported as a *record* defect rather than a code defect. The code was correct, and the comment
claiming it was correct "by construction" was false on that ISA.

It recorded the cleared candidates too, at length, and that section is longer than the findings
section. The stated reason is that "we looked and it is fine" is the other half of an audit, and
that each cleared candidate is a place a future change could break something. A pass that reports
three findings and nothing else has thrown away most of what it produced.

It established structural facts that bound the search, and these are the cheapest thing in the
whole method. It found two on the ISAs it read. There is exactly one `eret` and exactly one `sret`
in the kernel, both inside the restore sequence, so there is no second way out of the kernel to
audit. And there are exactly two places that fabricate a trap frame, one per ISA. Establishing those
took minutes and made the rest of the read finite.

It named its own general lesson, which is the finding that outlived the three bugs: when the
same fix is applied to two architectures, check whether it is load-bearing for the same reason on
both. Finding 1 was a complete fix on aarch64 and a partial one on RISC-V, and the symmetry of the
two patches is what concealed it. Finding 3 was the same shape from the other side: one ISA quietly
getting a guarantee from its hardware (the GIC's write-1-to-set enables) that the other does not.

What it deliberately did not cover. It read `kernel/src/arch/` plus the code on the other side of
the seam that the class reaches into. That code is `drivers/gic.rs`, `drivers/plic.rs`, `sched.rs`'s
switch path, `cpu.rs`, `smp.rs`'s `secondary_main`, and `user.rs`'s trap-frame placement, read only.
It did not read for memory ordering, for integer overflow, for time-of-check-to-time-of-use, or for
anything else. Those became the lenses of milestone 43 (a second security audit, with a different lens) precisely because this one did not take them. It
closes by saying so: an audit by reading finds what the reader thinks to look for. The bug that
prompted it was found by a failing test rather than by any of the earlier readings of that same
file.

## How the scale was counted

How this was counted, since "lines" is ambiguous and the number should reproduce:

```sh
$ find kernel/src/arch/x86_64 -type f -exec cat {} + | wc -l
```

Every file in the directory, `.rs` and `.s` alike, counting physical lines including blanks and
comments. That is not SLOC, and for an audit by reading it is the right measure. A reader reads the
comments, and on this tree the comments are where the hardware constraints are written down.

The check that this is also the method the original note used: run the same command over
`kernel/src/arch/aarch64` and `kernel/src/arch/riscv64` at commit `8334d0ba`. That is the commit
that added `notes/arch-audit.md`, and the command gives 6,202 lines. The note's own words are "the
whole arch tree is about 6,200 lines". The figure reproduces, so the page's comparison is like for
like.
