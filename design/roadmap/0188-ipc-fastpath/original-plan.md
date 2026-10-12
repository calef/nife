# The original plan: four phases and the recommendation, as written before phase 1

An appendix of [milestone 188 (the IPC fastpath)](../0188-ipc-fastpath.md), moved out of it on
2026-10-12 (UTC) under §212 (a prose budget). The main page is complete without it: it keeps each
phase in a line, its outcome, and the recommendation as it stands now. This file keeps the phasing
and the recommendation as the block stated them before phases 1 to 3 were built on 2026-09-04. What
those phases found is in [phases-1-to-3.md](phases-1-to-3.md), and what it means for phase 4 is in
[phase-4-evidence.md](phase-4-evidence.md).

## Phasing

Each phase is a lane that can be picked up without reading the others. Phases 1 and 2 are hours,
phase 3 is a day, phase 4 is a project.

### Phase 1: measure the shape the system runs

Add `ipc_call`, `ipc_receive_cap` and `ipc_reply` to `script/fastpath-footprint`'s roots. Better,
report the SEND/RECEIVE and CALL/reply closures as two named numbers rather than merging them. They
are two shapes and averaging them would hide both, which is the reasoning the script already applies
to `ipc_fastpath` and `syscall_entry`. Re-record all three baselines with `--save`.

Correct notes/benchmarks.md's comparison section, which says *"our EL0 path issues `SEND`,
`RECEIVE`, `SEND`, `RECEIVE`"*. That is true of the `ipc_rtt_el0` benchmark and false of every real
service, which uses `CALL`. The corrected count is three syscalls to seL4's two (client `CALL`,
server `RECEIVE_CAP`, server `REPLY`), not four to two. The residual one is the `ReplyRecv` fusion
this tree does not have. That is a live open question, not a defect.

### Phase 2: make the entry figure honest

Count one 128-byte vector entry rather than the whole 2 KiB aarch64 table, and add
`exception_restore`. Record in the script beside the `ENTRY` table why each architecture's set is
what it is. Pull request #574 established that pattern for x86_64; this extends it to the other two.
Re-record. Expect aarch64's `syscall_entry` to drop to roughly 1,412 and expect nothing to have got
faster, which is the point: the commit message must say the number moved because the accounting was
wrong.

While in that file, check it against milestone 186 (derive the architecture list). The script's
`arches` default is a hardcoded `"aarch64 riscv64"`, which is the exact defect 186's worklist
exists for. Pull request #574 fixed one instance of it by hand rather than by derivation. Phase
2 is the natural moment to take whichever mechanism 186 lands; taking it here without waiting for
186 would be a second hand-fix.

### Phase 3: apply milestone 156's method to the closure

The extraction by milestone 156 (extract the rest and ratchet both ways) that took 864 bytes out of `dispatch` has never been tried on `ipc_call`,
`ipc_receive_cap`, `schedule` or `finish_switch`. Each has cold tails that a successful rendezvous
never reaches: the aborted-rendezvous returns, the capability-table-full path, and the corpse
reaping in `finish_switch`. Milestone 132 (the fast path's footprint) already had to special-case that last one as cold in the
closure walk rather than in the code. Measure first, extract second, and report the number even if
it is small, because a small number here is what decides phase 4.

### Phase 4: the hand-written fastpath

Do not start this without phase 3's number and an architect's decision. Build option 1 first (a
fast path that still calls the proved `Rendezvous` methods and skips only the scheduler and the
error plumbing), measure it, and only then consider option 2 and the equivalence proof it requires.
The two options are in [design-space.md](design-space.md).

## The recommendation, with the numbers behind it

Take phases 1 to 3. Do not commit to phase 4 yet.

The reasoning is not a preference for less work. AGENTS.md's elegance tenet explicitly refuses that
argument, and the test it gives is *would I still choose this if both options were the same amount
of work?* The answer here is still yes, for three reasons that survive the effort question.

### The premise this milestone was minted on has moved

`syscall::dispatch` was the obvious target when it was 2,024 bytes and the largest single item. At
1,160 it is a sixth of the CALL/reply closure, and the largest structurally skippable block is now
2,872 bytes of general scheduling. A fastpath aimed at the decoder would be aimed at the thing that
already shrank.

### We do not currently know what a fastpath would be measured against

The gate reports 5,788 for a shape a quarter of the programs do not run, and 7,516 for the shape they
do. And 1,892 bytes of aarch64's entry figure are never fetched. Building a second path against a
number in that state means the before-and-after cannot be trusted. A benchmark that cannot be
trusted is worse here than no benchmark, because it will be quoted.

### The payoff is unobservable on the hardware this project owns today

This must be said plainly. `script/fastpath-footprint` bounds a quantity, not a harm: nothing in
this tree models a cache, and the HVF host's L1i is several times the boards'. `script/bench`'s
icount tripwire models no cache either. The 2 to 3% `call_reply` delta in
[scoping.md](scoping.md) is exactly what it can see of a 30% footprint difference. A milestone
claiming a fastpath speedup today would be claiming a number nobody can measure. Milestone 74 (cycle
counters) on milestone 127 (the seL4 machine), with PMU cache-miss events on a 48 KB L1i, is what
turns this from arithmetic into measurement. Milestone 25 (cross-OS performance comparison) is where
the result would be published.

### Each of the first three phases stands alone

Phases 1 to 3 are each defensible on their own terms whatever phase 4 turns out to be worth. They
give a gate pointed at the right shape, an entry figure that cannot be gamed by 1,892 bytes, and the
cheap extraction applied where it has not been tried. If phase 3 gets the CALL/reply closure under
4 KiB, phase 4 is not worth building and this milestone closes having said so. That is a better
outcome than a heroic one.
