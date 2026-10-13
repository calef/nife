---
status: PARTIAL
raised: 2026-08-18
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 139. Drive the unsafe count down, and cinch the ratchet behind it

Minted 2026-08-18 by calef, immediately after folding the unsafe census into
milestone 134: *"Can we also create a milestone to drive down the unsafe metrics and cinch up the
ratchet?"*

Milestone 134's instrument (the census and the ceiling relation in `script/lint`)
has existed and been live all along; this milestone spent it for its first real reduction, round 1 in the table below.

In brief. Reduce the hand-written `unsafe` this tree carries outside `kernel/src/arch/`, and lower
the ceiling after each reduction so the ground gained cannot be given back quietly.

This page was split on 2026-10-13 (UTC). calef ruled on 2026-09-25 (UTC) that this block is not
split, and granted it a marked exception to §212 (a prose budget) at 13,846 words; the marker had
grown to 14,318 by round 11. That ruling is superseded. calef, 2026-10-13 (UTC): "Lets launch a lane
to bring the two exceptions and 0139 under the cap safely." The exception marker is gone, and the
round-by-round record moved into appendices under [`0139-drive-down-unsafe/`](0139-drive-down-unsafe/README.md).
This page is complete without them.

## The rounds

Each round measured its reduction from the diff against its own base commit, and lowered the
`unsafe-density-outside-arch` ceiling in the same commit when the reduction was real. The full
account of each, with what was investigated and left, is in its appendix.

| Round | Date | What collapsed | Net `unsafe {` blocks | Ceiling | Appendix |
|---|---|---|---|---|---|
| 1 | 2026-08-23 | seven programs' hand-rolled volatile accessors, onto `MappedWindow` | -21 | 100 to 97 | [rounds 1 and 2](0139-drive-down-unsafe/rounds-1-and-2.md) |
| 2 | 2026-08-24 | `user_rt`'s twelve `asm!` traps into `invoke5`; nine FS page-copy loops onto `MappedWindow`; `ipc` read, nothing to collapse | -16 | 97 to 96 | [rounds 1 and 2](0139-drive-down-unsafe/rounds-1-and-2.md) |
| 3 | 2026-08-24 | `swish`, `disk_surveyor` and `net_stack` windows; device registers and framebuffer investigated | -6 | 96 to 95 | [round 3](0139-drive-down-unsafe/round-3.md) |
| 4 | 2026-08-24 | the bounds check measured, then four graphics sites migrated | -3 | 95 to 94 | [rounds 4 and 5](0139-drive-down-unsafe/rounds-4-and-5.md) |
| 5 | 2026-08-24 | the PL011 halves and `jh7110_trng.rs` onto `tock_registers`; the NS16550 halves left | -2 | the same 95 to 94 step, taken by a concurrent lane | [rounds 4 and 5](0139-drive-down-unsafe/rounds-4-and-5.md) |
| 6 | 2026-08-25 | the `user/` survey; `initrd_bytes`; `MappedWindow::as_slice` | +13 | unmoved | [round 6](0139-drive-down-unsafe/round-6.md) |
| 7 | 2026-08-26 | 122 raw `invoke` call sites behind thin wrappers | -104 | unmoved, 17 points of headroom | [round 7](0139-drive-down-unsafe/round-7.md) |
| 8 | 2026-09-01 | the kernel: 37 page-zeroing sites, 5 device-tree parses | -40 | 94 to 88 | [round 8](0139-drive-down-unsafe/round-8.md) |
| 9 | 2026-10-07 | the revocation log walk, address-space installs, code pages | -23 | 88 to 72 | [rounds 9 to 11](0139-drive-down-unsafe/rounds-9-to-11.md) |
| 10 | 2026-10-08 | the scheduler queue ownership token, `Unqueued<T>` | `sched.rs` 90 to 54 | 72 to 66 | [rounds 9 to 11](0139-drive-down-unsafe/rounds-9-to-11.md) |
| none | 2026-10-12 | nothing; the ceiling set at the tree's own value | none | 66 to 61 | [rounds 9 to 11](0139-drive-down-unsafe/rounds-9-to-11.md) |
| 11 | 2026-10-12 | question 2's renames | none | unmoved | [rounds 9 to 11](0139-drive-down-unsafe/rounds-9-to-11.md) |

`notes/unsafe-obligations/ceilings.md` carries the density after each step and the reasoning
behind each headroom.

## What is still open

Exit criterion, ratified by calef on 2026-10-08 (UTC) on #1842: this block is BUILT when every
item under this heading and every Follow-on entry marked Outstanding is done, split out with a
recorded owner, or refused with a reason. His words: "Yes, although my thought we would do those
things rather than create new milestones." So the two items left are built under milestone 139
itself, in follow-up lanes, and the block stays PARTIAL until both land.

### The rulings on round 9's three questions (calef, 2026-10-08 UTC, #1842)

Question 1, the scheduler's queue handoff: "C". An ownership token, `Unqueued<T>` (non-`Clone`),
replaces `NonNull<T>` in `intrusive_fifo` and `inter_process_communication`. Only a pop, a removal
or thread creation mints one, and the running thread's token lives in its core's current slot. The
crates and their harnesses go first, on the host; then `sched.rs`, under the full suite and the
icount benchmarks. Round 9's measurement of the surface: 21 blocks in `sched.rs` hand a thread
pointer to a queue (6 run-queue pushes, 2 inbox pushes, 13 `Rendezvous` operations); the crates
carry 7 `unsafe fn`s and 15 Kani harnesses between them. Options A (record it), B (a safe wrapper,
which this block refuses) and D (Kani over the unchanged blocks) were not chosen.

Question 2, the seventeen provisional names from rounds 6 and 7:

| Was | Ruling | calef's words |
|---|---|---|
| `initrd::initrd_bytes` | `user_mode_runtime::initial_ramdisk`, with `initial_ramdisk::bytes(len)` | "Yes" |
| `MappedWindow::as_slice`, `as_mut_slice` | ratified as is | "as_slice / as_mut_slice" |
| `retype_page_frame` | `retype_into_page_frame` | "retype_into_page_frame and retype_into_kernel_object" |
| `retype_object` | `retype_into_kernel_object` | as above, superseding a same-day `retype_kernel_object` |
| `split_region` | `split_memory_region` | "Ratify split_memory_region, ..." |
| `destroy_region` | `destroy_memory_region` | same ruling |
| `map_region_page` | `map_memory_region_page` | same ruling |
| `revoke_frame` | `revoke_page_frame` | same ruling |
| `map_into` | `address_space_map_into` | "Ratify retype_kernel_object address_space_map_into." |
| `tcb_cap_insert` | `thread_control_block_capability_insert` | first ruling |
| `tcb_configure` | `thread_control_block_configure` | first ruling |
| `tcb_start` | `thread_control_block_start` | first ruling |
| `irq_wait` | `interrupt_wait`, and `abi::irq` becomes `abi::interrupt` | "Ratify those three." |
| `irq_ack` | `interrupt_acknowledge` | same ruling |
| `send_cap` | `send_capability` | first ruling |
| `is_granted` | ratified as is | first ruling |
| the `virtio` module's four wrappers | `virtio::read_register`, `write_register`, `setup_queue`, `notify`; `abi::virtio::READ_REG`/`WRITE_REG` become `READ_REGISTER`/`WRITE_REGISTER` | "Yes" |

Two reasons came with them. `initrd` is a ramdisk (a nifefs image left in RAM), so the foreign
spelling stays only where it arrives: the device tree's `linux,initrd-start` and QEMU's `-initrd`.
The program side is swept, and `design/naming.md` gains a line saying the foreign spelling stops at
the boundary. And "IRQ" is said aloud only among kernel and embedded developers; a newcomer says
"interrupt", per §154 (the acronym test is whether the phrase is spoken). Hardware-owned spellings at the
boundary keep theirs. The three names ratified as is carry their `Name: ratified` lines at the
definition since round 9's records commit; the rest stay provisional until the rename lane.

Question 3, round 6's slice accessors: "Keep. We don't want to penalize reuse." The +12 raw blocks
are the price of one invariant at six declarations instead of eighteen call sites.

### What remains, under this milestone

1. The ownership token, as question 1 rules it. Built by round 10 (2026-10-08 UTC, #1852),
   with one deviation for calef to confirm: the token on `Thread::own_token`, not a per-core
   current slot.
2. The renames, as question 2 rules them. Built by round 11 (2026-10-12 UTC, #1998).

## The measurement it starts from

Taken on `main` 2026-08-18 and to be re-taken by 134 rather than trusted here:

| | count |
|---|---|
| `unsafe { }` blocks, ours, `vendor/` excluded | 893 |
| `unsafe fn` | 53 |
| `unsafe impl` | 28 |
| `// SAFETY:` comments | 885 |

By location: `kernel/src/arch/` 139, kernel outside `arch/` 203, `user/` 285, `ipc` 44, `user_heap`
41, `user_rt` 31.

`arch/` is not a target and this block will not accept a reduction there. Rule 1 says
architecture-specific code lives under `arch/`, and unsafe is what architecture is made of. Driving
that number down means either writing the assembly wrong or moving it somewhere it does not belong,
and both are worse than the number.

## What counts as a reduction, which is the whole design of this block

The metric is gameable and the obvious way to game it is invisible. Moving three `unsafe` blocks
into one helper function reduces the count by two and reduces the risk by nothing: the same
invariants are asserted by the same code in a different place. A milestone that rewarded that would
make the tree worse while the graph went the right way, and the graph would be the reason.

**So the test is not the token count, it is the number of distinct invariants asserted by hand.**

§94 is the worked example and it is what a real reduction looks like. The trap instruction was
inlined at 48 sites in 7 variants across 58 panic handlers, each one a hand-written assertion of
the same invariant. Lifting it into `user_rt::trap()` left one. That is 47 chances to write it
differently, removed. §94 states the general form: *a per-binary item whose body is copied verbatim
into every binary is not per-binary; only its declaration is,* and copying it is asserting the same
invariant N times by hand, which §61 says a `// SAFETY:` comment must never be.

And §94 found the cost of the copy by counting the copies: one of the 58 handlers was different.
`terminal_sink_caretaker` called `exit()` and never trapped, so a panic there reported `EVENT_EXIT`
where every other program reports `EVENT_FAULT`. **A supervisor would have been told a panicking
program finished cleanly.** That is the argument for this milestone in one incident: the duplication
was not only ugly, it was already wrong in one place and nobody knew.

So a reduction qualifies when it does one of these:

- Collapses N hand-written assertions of one invariant into one, the §94 shape. Best available,
  and the only one that reliably reduces risk rather than moving it.
- Replaces raw pointer arithmetic with a typed abstraction whose invariant the compiler or Kani
  holds, so the assertion stops being a comment. Rung one on the ladder.
- Deletes unsafe that was never needed, which is the cheapest and rarest.

It does not qualify when it merely relocates unsafe, wraps it in a function whose safety argument is
the same argument, or hides it behind a macro.

## The ratchet

After each reduction, **lower the ceiling to the new count in the same commit.** A ceiling left above
the true number is not a ratchet, it is a budget, and a budget gets spent by whoever arrives next
without knowing it was won.

## Why it matters

The verified-Rust claim in this project's own thesis is measured here. DECISIONS §14 calls this a
verified-Rust capability microkernel; `unsafe` is where that claim is suspended, and 203 suspensions
outside the architecture layer is the honest size of the gap. Every one of them is a place where the
proofs and the type system are standing aside and a person's comment is the whole argument.

## BUGS

- This block sets no target number, and no ceiling has ever fired on honest work: across ten
  rounds the density stayed 5 to 17 points under the ceiling, until 2026-10-12 set it at the
  tree. `script/lint` has had three checks deleted for only ever rejecting
  legitimate work, and a too-tight ceiling would be the fourth. Round 6 bounded `user/`'s floor
  at 160 to 260 blocks depending on the `invoke` cluster; round 7 resolved it to 162, essentially
  all of the cluster being wrappable.
- `user/`'s 285 (now 162) is explained: 122 of the 123 raw `invoke(...)` calls are behind thin
  wrappers (round 7), one stays raw with its reason at the call site, and the rest is device
  registers, `asm!` entry stubs, deliberate-fault test programs and one-off writes, each
  investigated and either migrated or deliberately left in rounds 1-6.
- `sched.rs` was the kernel's largest share, 21 of its blocks handing a thread to a queue under a
  fact the caller established. Round 10 replaced those with `Unqueued<T>`, option C.
- A reduction can be real and still not show in the count, and round 6 is the sharpest example
  on record of the inverse: a real reduction (eighteen independently worded page-sharing
  invariants collapsed to six canonical declarations) that shows as a raw block-count increase
  (+12), because unlike round 1's `r8`/`w8` cluster, the "hand back a whole slice" call shape has no
  runtime check to add that would let the call site drop `unsafe` the way a bounds check did. Do
  not let the number alone decide which work is worth doing, in either direction: it under-credits
  round 6's collapse and it would over-credit a Kani-proved block that stays exactly where it is.

## Follow-on

- **Done.** `kernel/src/revoke.rs`'s log-page walk: round 9, onto `revoke::chain`. Ten blocks and
  an `unsafe fn` became one block.
- **Done.** `kernel/src/user/tests.rs`, now `system_tests/src/user/tests.rs`: round 9, 15 blocks
  to 7, with the seven left named and argued in round 9's section
  ([rounds-9-to-11.md](0139-drive-down-unsafe/rounds-9-to-11.md)).
- **Done.** `crates/abi`'s `START` doc comment: round 9.
- **Done.** The ratchet's scope: round 9 gated the other `unsafe impl`s (`unsafe-trait-claims`)
  and recorded why `unsafe fn` gets no ceiling.
- **Done.** The renames calef ruled on 2026-10-08 (UTC), and his ruling B on #1998 (2026-10-12
  UTC) for how far the `initial_ramdisk` sweep reaches: round 11.
- **Done.** Round 6's slice accessors stay: calef, 2026-10-08 (UTC), "Keep. We don't want to
  penalize reuse." `as_slice`, `as_mut_slice` and `is_granted` are ratified as is.
- **Done.** The scheduler's ownership token (`Unqueued<T>`), option C: round 10 (#1852). calef's
  confirmation of `Thread::own_token` over a per-core slot is pending.
- **Done.** Round 5's caveat that whichever concurrent lane landed second would find the ceiling
  arithmetic stale was discharged by round 8, which re-measured from the merged tree.
- **Done.** The `tock_registers` follow-on round 3 named was taken in round 5.
- **Done.** The framebuffer and graphics cluster is settled: round 4 measured the bounds check and
  migrated all four sites round 3 named.
- **Done.** "What is still open" no longer contradicts BUGS about the `user/` survey; round 9
  rewrote it.
- **Recorded.** `crates/inter_process_communication`'s three production blocks each assert a
  different fact under a different caller contract; nothing further is indicated there.
- **Recorded.** The NS16550 halves of the console and input programs stay hand-written, because
  the register stride is a runtime fact no register-layout macro can express.
- **Recorded.** `hello.rs`, `flaky.rs`, `outlaw.rs`, `memory_grant_depleter.rs` and `swapper.rs`
  are deliberately not candidates.
- **Recorded.** No target number, by design. The ceiling stands at 61, the tree's own value on
  2026-10-12 (UTC), with no headroom.
- **Refused.** `kernel/src/arch/` is not a target and this block will not accept a reduction there.
  It is 346 blocks today.

## Index row

First real reduction: seven programs' hand-rolled shared-page volatile-access functions collapsed
into `user_rt::mapped_window::MappedWindow`, net -21 unsafe blocks (32 removed, 11 added),
bounds-checked at every access instead of trusted by hand. Density 93.4 to 90.8; the `unsafe-density-outside-arch` ceiling cinched from 100 to 97 in the same commit. `user/`'s broader
unsafe survey and several named candidate clusters remain.
