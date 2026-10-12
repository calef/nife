---
status: PROPOSED
raised: 2026-10-12
milestone_dependencies: 876
decision_dependencies: 19, 284
machine_requirements: none
specific_machine: none
needs_person: no
---
# riscv64 and x86_64 join the shared boot path

Raised 2026-10-12 (UTC) by lane `milestone/876-the-kernel-crate-root-is-split-along-its-seams`,
when calef ruled on §284 (the kernel crate root splits into child modules, one per boot): "We should
treat them as temporary." Title and slug are drafts.

Reuse: not applicable; this moves and deletes kernel code and adds none.

## The problem

`kernel_main` in `kernel/src/boot.rs` hands `x86_64` and riscv64 to their own boots,
`kernel/src/x86_64_boot.rs` and `kernel/src/riscv64_boot.rs`, and only aarch64 continues through
`bring_up_machine` and the shared path after it. Each port's boot is a chain of phase functions
that brings up its own traps, memory, scheduler and interrupts, runs its own demonstrations, and
ends at its own hand-over (`x86_hand_over`, `riscv_hand_over`). That is two copies of the boot's
portable middle, written per port because the arch layer beneath the shared path was not built for
either port when its boot was written.

calef's ruling makes both files temporary: each is its port's route until it joins the shared
path, and the parity work shrinks it toward the CPU bring-up that belongs under `kernel/src/arch/`.
Nothing yet tracks that work.

## What it would take

Measured at the head of milestone 876 (the kernel crate root is split along its seams):
`x86_64_boot.rs` holds six phase functions and `x86_hand_over`; `riscv64_boot.rs` holds nine, four
timing helpers and `riscv_hand_over`. A lane would, per port:

1. Sort each phase into what `bring_up_machine` already does portably, what is CPU bring-up and
   goes under `arch/<port>/`, and what is a demonstration the milestone tour already carries.
2. Make the shared path take the port, so `kernel_main` stops branching on the architecture.
3. Delete the port's boot file once it is empty, and the two hand-overs once the shared path's
   hand-over serves all three.

DECISIONS §19 (architectural parity is a tenet) is the reason. The riscv64 boot's middle stages
are the only proof this tree has of them on real silicon (notes/visionfive2.md), so a lane keeps
every transcript line `board_console` reads, or moves its claim into the shared path first.

## Done when

1. `kernel/src/x86_64_boot.rs` and `kernel/src/riscv64_boot.rs` are gone, or hold only what
   `arch/<port>/` cannot.
2. `kernel_main` has no `cfg(target_arch)` branch to a per-port boot.
3. `script/test`, `script/swish-check` and the board's `board_console` stages pass on all three.
