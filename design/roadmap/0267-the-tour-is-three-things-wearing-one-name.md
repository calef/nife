---
status: BUILT
raised: 2026-09-09
built: 2026-09-13
---
# 267. The milestone tour is three things wearing one name, and only one of them belongs in the kernel

Built 2026-09-13. Minted 2026-09-09 by calef, from one question: *"Does the milestone
tour need to be part of the build at all? Can the milestone tour just be a userspace program that we
run if we want the milestone tour?"* Built on `milestone/267-tour-split`. The split, the remainder
list and the measurement are below and all three stand.

The narrative program was deleted on 2026-09-13, by calef's ruling, on `maintainer/delete-narrator`.
The next section is what it said and why it went, and it is deliberately the first thing in this
block rather than a footnote at the end. This block turned BUILT at that deletion. The only
thing holding 267 at PARTIAL was that the program could not be typed at a prompt. Deleting the
program does not deliver that half, it dissolves it. There is no program, so there is nothing
left outstanding, and `PARTIAL` asserts remaining work that no lane could now pick up.

`REMOVED` was considered and refused. It is the right word when a milestone's code is deleted, and
some of this one's was. But three of the four things this block built are still in the tree and
enforced (`print_machine_description`, its `script/lint` check, and the kernel-only remainder
comment). A `REMOVED` row would tell a reader at a glance that the tour split was reversed. It
was not. What was reversed is the vehicle the narrative moved into, and that is a paragraph's worth
of fact rather than a status word's.

## What the narrative said, and why it is gone

The text, verbatim, as `user/src/narrator.rs` held it and as the boot printed it. It is kept
here because AGENTS.md's rule about branches applies to deleted files with nothing changed: nobody
reads a branch, and nobody reads a file that is not there. This is a well-made artifact and the
argument for this system's design in nine lines; the git history is not where an argument lives.

```
milestone 1: we are running our own code on a CPU with nothing underneath it.
milestone 2: and when it goes wrong, we get told.
           : and the machine now tells us what it is, instead of us guessing.
milestone 3: and we know which parts of it are ours to give away.
milestone 4: and nothing writable is executable, and Vec works again.
milestone 5: and the machine can now interrupt us. we are preemptible.
milestone 6: and a thread that refuses to yield gets preempted anyway.
milestone 7: and now it runs a binary it did not compile, unprivileged.
           : and that binary can talk to a server it can only name, not reach.

  every line above was printed by a program at EL0, through a console driver that is
  also a program at EL0, and the kernel does not contain a line of code that puts a
  user's bytes on the wire.
```

And the observation that made this milestone happen, which should outlive the program it
produced. That closing claim used to be printed by the kernel, on the program's behalf. It was
the one line of the tour not demonstrated by the thing saying it: a `println!` at EL1 asserting that
no kernel code puts a user's bytes on the wire. Noticing that a record can be in the wrong mouth is
what 267 is about, and it is a reusable way to read this tree. The fix held for four days and is
worth keeping as a habit rather than as a program.

Why calef ruled it deleted, and none of this is "267 was wrong". Each reason is about what the
program became, not about the argument that created it.

- Nothing verified it. No test asserted those lines ever reached the console. If it had silently
  stopped printing, nothing in this tree would have gone red. That makes it the shape of record
  AGENTS.md's ladder puts at rung zero: true only while somebody happens to notice.
- The text was frozen at milestones 1 through 11, and there are 267. No gate compared it with
  `design/roadmap/`. The program's own header defended the freeze (*"the story stops early on
  purpose: it is the argument for the design, not a changelog"*), which is a fair defense and is
  still a boot that prints an argument where a reader may reasonably expect a status.
- It existed on aarch64 only. riscv64 and x86_64 each halt in their own arch tour before the
  shared path it belonged to, so two of three supported architectures never printed a word of it.
  Rule 5 (parity is a gate) is not violated by a demonstration, but a demonstration one architecture
  in three can see is a weak demonstration.
- The demonstration is redundant with the system existing. At milestone 11, *"a userspace
  program printed to the screen"* was remarkable. At 267, with a shell, a compositor, a filesystem
  and a network stack all printing through userspace, it is ordinary. The claim stays true; it
  stopped needing a dedicated program to say it.

What went with it: `user/src/narrator.rs`, its `[[bin]]` stanza in `user/Cargo.toml`, its
`("narrator", "narrator")` entry in `xtask`'s `initrd_aarch64` archive table, the spawn block in
`kernel/src/main.rs`, and `kernel/src/user/console_service.rs`'s `spawn_client`, which had exactly
one caller and was that block.

What deliberately did not go with it: `console_service::start` and `components/src/console.rs`.
Deleting those is a larger change than was authorized. It removes infrastructure rather than
a demonstration, and the question it raises is in the Follow-on section below with what the compiler
says about it.

## The measurement, first, because it changed what this milestone is about

The compile-time exclusion was not buying what its name suggests. The tour costs bytes on aarch64
only, what costs them is the kernel-only demonstrations rather than the narrative, and moving the
narrative out made the kernel slightly larger. The consequences for the feature pair are in
`design/roadmap/proposals/two-boot-mode-features-for-one-decision.md`. Nothing was retired here.
None of `Stage::Tour`'s consumers observe the thing this milestone moved, and none was touched.

- [evidence.md](0267-the-tour-is-three-things-wearing-one-name/evidence.md) holds the measurement
  and its tables, every consumer of `Reached(Tour)`, and what the boot printed before and after
  the deletion.

## The three things, which want opposite treatment

Boot output was one undifferentiated stream, compiled in or out as a unit. It is three audiences.

The machine description is diagnostics and belongs on every boot, never compiled out.
Paging, ISA, firmware, cores, ACPI tables, PCI ECAM, timers, memory map. xenon proved this is not
decoration. At first light there was no serial console this project could read, so the boot output
*was* the transcript, read off a photograph. It is what diagnosed both the local-APIC collision
and the PCI BAR window landing in RAM. A port is verified by reading these lines.

The milestone narrative is a demonstration. *"milestone 1: we are running our own code on a CPU
with nothing underneath it."*

And a genuine kernel remainder that cannot move anywhere, because it demonstrates facts only
kernel code can establish.

## What was built

1. The machine description is `print_machine_description`, called unconditionally from
`kernel_main`, with byte-identical output. It was not gated on `shell` or `initboot` before either,
which is the one thing this block's original text got wrong: the `cfg` at what was line 1540 sat
*after* the description and governed only the tour. What was true is the rest of the diagnosis. The
boundary between the two audiences was a reader's inference about where a line happened to sit, and
a line moved a few rows down would have changed audience with nothing saying so.

`script/lint` now fails if a `feature = "..."` cfg appears anywhere inside that function. The
signature keeps `#[cfg(not(any(test, feature = "bench")))]` and the check deliberately does not look
at it. A `bench` boot diverges into `bench::run` before this point and a `test` boot exits through
semihosting, and neither is a boot anybody reads to bring up a board.

2. The narrative was `the narrator program`, a program at EL0, spawned as the console server's
client, and it is deleted (2026-09-13, the section above). The console server was moved to the
top of the tour to feed it, which kept the transcript in the order it always read. The `hello`
printing client it used to feed went at the same time: running two programs to demonstrate one thing
was one program too many.

This closed a gap nobody had named, and that is the part of item 2 that survives the deletion. The kernel used to print *"a userspace program printed to the screen, and the kernel
does not contain a line of code that puts a user's bytes on the wire"* on the program's behalf. For
four days the narrator said it, through a driver at EL0 that holds the UART, from an address space
that could not reach the device at all. The default boot now prints no narrative at all. The BUGS
entry below records how that reading was refused in September and ruled for in the end. It was
rewritten rather than deleted, because what changed is the answer and not the fact that the
question was live.

3. The remainder is listed in one comment at the top of the tour block, with the privilege each
entry needs:

| survivor | why it cannot leave |
|---|---|
| two threads that never yield | `sched::spawn`, `sched::preemptions()`, `timer::spin_for`. An EL0 spinner proves only that EL0 is preempted; these run *inside* the kernel and are still taken off the CPU, which is the stronger claim |
| the virtio and PCIe block demos | the kernel enumerates the bus and mints registers, a DMA page and an interrupt into a driver's world, then reads the report with `sched::ipc_recv`. The granter cannot be the grantee |
| the outlaw | `&raw const USER_FAULTS` is the address of a kernel static, handed to a program that faults reading it and increments the counter it reached for. A program cannot name that address |
| the memory-region demo | it prints `memory::stats().used` before and after a process spends its own budget, and the claim is that the number did not move. That number is the kernel's own frame accounting and is not exposed to EL0 |

## What must not break, and did not

- The machine description prints on every boot, on all three architectures. It is now harder to
  break than it was: it is a function with a name and a gate rather than the first half of a block.
- `xtask`'s archive manifest. `narrator` was packed into `initrd_aarch64`'s table, aarch64 only:
  the other two boots halt before the shared path, so packing it there would be archive bytes
  nothing can reach. The entry went with the program on 2026-09-13.
- `script/swish-check` runs the real boot on both legs. Green.

## BUGS

- The default boot now prints no milestone narrative, and this block argued both sides before it
  got there. Its "proof" paragraph asked for *"a boot that prints the machine description and
  nothing else"*; its BUGS section argued back in the same breath that `script/server` is what a
  stranger runs first and *"run this other program" is a worse default than "it prints"*, and in
  September the second argument won. calef ruled for the first on 2026-09-13, and the route
  there was not the argument being re-run: it was the reasons under "What the narrative said, and
  why it is gone". A reader meeting an empty-feeling
  boot should know the trade was made deliberately, twice, in opposite directions, with the reasons
  recorded both times.
- `narrator` was never ratified and now never will be. Names in this tree are an architect's,
  and this one shipped provisional under the rule that lets a lane ship rather than wait. `tour` had
  been refused because this tree spends the word on the boot path itself (`Stage::Tour`,
  `script/board-console`). The name is gone with the program. The refusal of `tour` stands on its
  own, and is recorded here so the sweep that deleted the program does not also delete the
  reasoning that would be re-derived by the next person who wants the word.
- A program that prints and nothing that checks it is the shape to watch for, not a fixed
  defect. The narrative was unverified for its whole life, on both sides of the move: nothing
  asserted the lines reached the console when it was twenty `println!`s at EL1 either. It is worth
  a reader knowing that the tour's remaining demonstrations below are in the same position, with
  the exception of the ones `script/swish-check` boots.
- The move made the kernel 664 bytes bigger, and the deletion has not been re-measured.
  Measured on 2026-09-09
  ([evidence.md](0267-the-tour-is-three-things-wearing-one-name/evidence.md)). The move bought the
  split, the self-demonstrating claim, and a program. It did not buy bytes. A reader should not infer that it did from the fact that a
  milestone about the cost of the tour exists. The 2026-09-13 deletion removed the spawn block,
  `spawn_client` and one archive entry, so the direction is certainly downward. But no number in this block was re-taken, and none should be quoted as if it had been. `script/fastpath-footprint`
  is the method if somebody wants it. It was not run because the deletion's correctness does not
  depend on the answer and an unverified number in a table of verified ones is worse than a gap.
- The interleaving hazard grew and then went away again, and the underlying one did not. The
  kernel's own UART driver and the userspace console server write the same device with nothing
  arbitrating, which `script/swish-check`'s BUGS section documents at length and
  milestone 230 (`script/shell-check` is red on `main`, on both architectures) proposes fixing. This milestone put fifteen more userspace-printed lines on the default boot; the
  deletion took all fifteen back, along with the `timer::spin_for` window that had been keeping the
  two writers apart. The unarbitrated device is unchanged, because it was never the narrative's doing. The boot-time console server still holds the UART's registers on a tour boot. It simply has
  nothing to print through them now.
- Preemption counts are still not proposed for exposure. A supervisor or benchmark might want
  them one day, and the shape is already established by §139 and milestones 229/237: a per-thread
  grant, not an ambient fact. Nothing needs it today, so nothing is built. This is why the spinner
  demo is on the kernel-only list rather than being a program.

## Follow-on

- **Milestone 394.** The boot-time console server now comes up with no client at all: `start` is
  still called on a tour boot and `rustc` reports every field of its `Console` handle as never read.
  Whether it should keep being started, and what it was infrastructure for, is an architect's rather
  than a lane's, and 394 carries the three options with their real costs. Numbered on 2026-09-19 by
  milestone 433's drain of the pile.
- **Refused.** "A narrative program the shell cannot start" is moot and its proposal file is
  retired. It asked for one of three ways to let `swish` start `narrator`, all of which presupposed
  a narrator. With the program deleted, the missing half of "a program you can run on purpose" is
  not unfinished work but a question about a thing that no longer exists. The proposal's one
  durable finding is not lost: that this tree has two ways for a program to
  receive an output channel (the console server's raw shared-page-plus-two-endpoints protocol, and
  `crates/byte_sink_proto`'s sink), and that a program written against one cannot be started by the
  other. That is recorded here rather than left in a deleted file, because it is true of every
  program and was only ever illustrated by this one.
- **Milestone 296.** Whether `initboot` and `shell` should collapse to one feature, or stop being a
  compile-time switch at all, now that the measurement exists: 7.3% on one architecture, zero on
  x86_64, and something else entirely on riscv64. Not acted on here, on purpose.
  `design/roadmap/proposals/two-boot-mode-features-for-one-decision.md` was this lane's file and is
  the block milestone 296 was promoted from. Answered: the switch is kept on that 7.3%, and it is
  one feature, `shell`. `initboot` was deleted after a build of each was found to carry the same
  3,109 symbols at the same total size.
- **Milestone 296.** `design/roadmap/proposals/what-the-boot-path-is-called.md` is **not** retired by
  this milestone, which is the opposite of what this block originally predicted. Checked by reading
  it: it chooses between `progenitor-boot` and `handoff` for the boot mode, and both features still
  exist and still mean the same thing, so the question it holds is untouched. What changed is that
  there is now a fourth candidate, because the feature is better described as naming the absence of
  the demonstrations than as naming a boot path. (Carried here as `**Proposed.**` rather than
  `**Outstanding.**` because this block is BUILT and the work is nobody's.) It was retired by
  milestone 296, by a fifth answer none of the four candidates could be: calef deleted the thing
  the proposal was naming.
- **Recorded.** The kernel's UART driver and the userspace console server write the same device
  unarbitrated. The limitation is `script/swish-check`'s and milestone 230's, and it is not this
  milestone's doing in either direction; the BUGS section above says how the hazard came and went.
  Also recorded beside the code in `kernel/src/main.rs`.

## Index row

calef, 2026-09-09. The machine description is diagnostics and must always print, xenon proved that
with no serial console; the narrative is a demo that could be a program; the remainder needs
sched::spawn and a kernel static. It also dissolves the shell/initboot feature pair, which mean
the same thing to the kernel. The narrative program was deleted 2026-09-13 on calef's ruling and
the block keeps its text; the split, the lint check and the remainder list stand
