# What was built, and when: 2026-08-18 to 2026-09-19

This appendix belongs to [milestone 134 (the register of measures)](../0134-the-measurements-that-decide.md).
It was moved out on 2026-10-12 (UTC) under §212 (a prose budget), and the main page is complete
without it. It holds the dated build narrative and the corrections made along the way, in the order
they happened. The main page carries each result as a summary.

## 2026-08-18: the register, and the unsafe census folded in

notes/register-of-measures.md is the block's own subject. It holds the numbers this kernel holds
itself to (nine gated) and the numbers it merely knows (four dated, each with the command that
re-takes it). It also holds a deliberate-exclusions table, where every rejected number names the
half of the test it failed. The test it settled on is sharper than this block's first draft: a
number belongs if something depends on its value and it can move without anybody editing it. The
second half is what cuts, because it separates a measure from a decision.

Folded in at calef's direction the same day, from a separate question about unsafe: the unsafe
census, in notes/unsafe-obligations.md, and the `count-at-most` relation in `script/lint` that it
needed. The census's own result is the argument for having taken it. Outside `kernel/src/arch/` the
raw count went 171 to 747 since 2026-07-15, while the density fell from 228 to 93 per 10,000 lines
at every sample. So the tree has been getting proportionally safer and nothing was measuring it. A
single commit two days earlier, `d5a969a2`, moved that count by 94 in one change and nothing
recorded it.

The same day the block was extended at calef's direction. The first draft listed only the
experiments runnable today, on the reasoning that a measure we cannot take is not yet useful.
calef's correction: *"I'm fine defining measures that we cannot capture until the hardware. I want
to capture the measures as a milestone."* That is where tier B came from.

## A correction the building lane made to the block

The block treats "register" and "experiments" as one deliverable. They are not the same size, and
separating them is what let the register land in a day: the register is a document plus a gate, and
E1 through E4 are four benchmark harnesses. A future lane should take E2 alone, then E1, rather than
reading this block as one piece of work.

## 2026-08-22: E1, E2, E3 and E4 on the dev Mac

Built as registered instruments rather than one-off scripts: `script/fastpath-footprint --features
<name>`, `cargo xtask bench --extra-features <name>`, and `kernel/src/bench.rs`'s
`ipc_thread_scaling` and `app_displacement`. Each result is a real number rather than a placeholder.

- E2 (thread census) found 4 new threads at the SMB/FS customer-path topology.
- E1 (IPC latency against thread count) is flat at about 1,270 to 1,310 ns/iter through 16
  threads, rising 8 to 11% by 64 to 96 threads. That is a small but real knee, consistent with the
  predicted low-tens location. The dev machine's larger L1d mutes it against the 32 KB SiFive U74
  target.
- E3 (fastpath padding) roughly doubles instruction footprint on both ISAs but shows only a 2 to 3%
  latency effect on this machine. That is within run-to-run noise: no measurable cost here.
- E4 (application displacement) is 0 to 3% across 4 to 128 KiB working sets under concurrent IPC
  load, consistent with E1's own flat region.

Full detail is in `notes/register-of-measures.md`, updated the same day. Its Owed count went 12 to
8, with three honest `BUGS` entries: the sweeps were run at shipped values only, and both were
measured on a shared, noisy machine rather than the quiet single-tenant conditions earlier steps
had.

## 2026-08-23: E4's own follow-up, closed

E4's original 8-pair background load sits inside E1's flat region. So the 0 to 3% it found was
expected from E1's own curve, rather than independent evidence against displacement
(`notes/register-of-measures.md`'s own BUGS said so). `app_displacement` now runs a second condition
at `SCALE_MAX_PAIRS` (48 pairs, 96 threads), the same pair count E1's own sweep tops out at.

Over three repeated runs, on the same dev Mac under HVF, the low-load condition still reads 0 to 5%
and the high-load condition reads 2 to 9%. The high-load figure is higher than the low-load one at
every working set on every run, though the two ranges overlap and neither is a clean step function.
It is a small, reproducible effect in the predicted direction rather than a decisive one. That fits
E1's own finding that this machine's larger L1d mutes the knee a 32 KB SiFive U74 would show. It
wants the same small-cache re-run E1 and E3 already want before either range is treated as more than
a first data point. Detail and raw numbers are in `notes/register-of-measures.md`.

## 2026-09-04: E3's padding was on the wrong path

A correction found on the way, which would otherwise have wasted the radon session. E3's padding
was reachable only from `sched::ipc_send`. Milestone 188 (the IPC fastpath) phase 1 (2026-09-04)
split the footprint gate into `ipc_send_receive` and `ipc_call_reply`, and established the second as
the shape services run. Measured on riscv64 the day after, `--features fastpath_pad` moved
`ipc_send_receive` to 2.10x and `ipc_call_reply` to 1.00x. E3 was padding a path nothing in this
tree uses. `sched::ipc_call` now calls `maybe_pad` as well, and both shapes pad to roughly 1.85x on
both ISAs. The experiment was correct when it was built; the thing it measures moved underneath it,
which is what a dated instrument does.

## 2026-09-04: the radon session

Six interleaved boots on a `single_hart` card. The reading is notes/footprint-perturbation.md and
the capture is `bench/radon-2026-09-04/`. Two results are stronger than the dev Mac's and one is a
defect in the experiment.

- E1 found the knee, at 16 threads: 68% up from 2 and then flat to 96, where patagonia showed 8 to
  11% with no bend.
- E4 found displacement: 5 to 8% under 96-thread IPC load against 0 to 1% at ordinary load, peaking
  at a 32 KiB working set, which is radon's L1d exactly.
- E3 found that it cannot separate footprint from code layout. The padded build is 1.49% slower on
  `call_reply` and 3.01% faster on `ipc_rtt_el0`, and dead code that is never executed has no
  mechanism for the second. That is a flaw in E3's design rather than in the session, and it was
  equally present on 2026-08-22.

A correction, recorded 2026-09-19. A Follow-on item had said the small-cache re-run of E1, E3's
latency half and E4 was runnable and not taken. It was stale from the day it was written: the
session above had already run, and the Status paragraph already said so. E1 and E4 from that session
are single-build sweeps and stand. E3 does not.

## 2026-09-03 to 2026-09-19: the counters, measure by measure

A Follow-on item said milestone 74 (cycle counters)'s counter driver was NOT-STARTED, so M5 through
M9 had no instrument (checked 2026-09-03). Both halves of 74 have since landed (riscv64 2026-09-03,
aarch64 2026-09-19 as PR #972), and milestone 309 (unhalted core cycles) added x86_64. Re-checked
against the tree on 2026-09-19, measure by measure, in notes/register-of-measures.md's "Owed" table:

- M5 has its instrument on all three ISAs: every tick row times `bench::cycles_per_tick`. radon's
  `250.00` of 2026-09-16 makes riscv64 `call_reply` about 1,256 cycles.
- M9 has its counter (`arch::pmu::cycles()` in-kernel, all three ISAs) and lacks the phase stamps.
- M6, M7 and M8 have nothing: no code on any ISA programs an event counter. M7's attribution half
  needs a data-address sampler neither the A57 nor the U74 has.

`PMCCFILTR_EL0` is provisional pending calef's decision A in
`design/roadmap/proposals/the-aarch64-half-of-74.md`, so no aarch64 cycle figure is published before
he rules. That touches M5 and M9 on aarch64, and M12 entirely (seL4's 413 and 426 are TX1 cycles).
It does not touch M6 to M8 as such, since event counters carry their own filter in
`PMEVTYPER<n>_EL0`. But that filter will raise the same question when a driver first writes it.

## 2026-09-19: the layout control (milestone 370)

`NIFE_FASTPATH_PAD` and `NIFE_FASTPATH_SHIFT` (kernel/build.rs, kernel/src/fastpath_pad.rs) size
E3's sled. They also append an unreferenced block that moves the same code and adds nothing
reachable. So E3 runs as a dose-response over four pad sizes, against four un-padded kernels that
differ only in address assignment.

- `script/fastpath-footprint --layout` proves the eight images share one instruction stream, and
  prints where each one put the hot path.
- `script/board-image` echoes both values and refuses them without the feature.
- A `fastpath_pad` bench boot prints `bench-probe: fastpath_pad units <u> shift <s>`, so a card says
  which image it is.

Built and booted under QEMU only, which proves the images boot and print and says nothing about
timing. Milestone 370 (a layout control) is the block, and it is BUILT.

## 2026-09-19: the per-IPC kernel stack depth, measured

`kernel/src/ipc_stack_depth.rs` paints a thread's own kernel stack around each IPC operation. Its
module, its `ipc_stack_depth` feature and its `ipc-stack-depth:` prefix are all provisional names.
It covers kernel threads (E1's shape) and EL0 threads (a service's), on all three ISAs under QEMU,
in the debug build `script/test` runs and the release build radon boots.

Release, per thread per round trip: about 600 bytes for kernel threads (riscv64 608 client, 576
server), and 0.5 to 1 KiB for EL0 threads. The debug build uses 2 to 4 KiB. Numbers, method, cost
and what it misses are in notes/stack-high-water.md, "Per-IPC depth"; the register carries a `dated`
row.

What it means for E1 and §96 (process kernel or event kernel): the stacks alone would not fill
radon's L1d until about 54 threads, so the knee at 8 to 16 is not capacity. It matches a set
conflict bound instead. radon's L1D is 32 KiB, 4-way, VIPT per SiFive's U74-MC manual 21G3.02.00, so
at most 8 lines sharing a page offset fit, and every stack top is page-aligned. The TCB pages would
produce the same bound. §96's performance input therefore stands as measured, and its mechanism
becomes a testable question rather than an assumption. Milestone 506 (color the kernel stacks and
take E1 again) is the test.

Two findings recorded on the way:

- notes/qemu.md: `helpers/qemu-bounded.sh` does not bound `helpers/qemu-runner-x86_64.sh`, because
  that runner does not `exec` QEMU. So the bound kills the shell and orphans the emulator (found
  when this lane's release boot left one with PPID 1).
- notes/stack-high-water.md's BUGS: the instrument copies `os_primitives_benchmarker`'s and
  `soaker`'s role numbers as local constants, as `bench.rs` and `soak.rs` already do, against
  AGENTS.md rule 7. And the per-IPC depth is `dated`, not gated.
