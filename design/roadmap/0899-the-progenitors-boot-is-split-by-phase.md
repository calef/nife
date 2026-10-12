---
status: NOT-STARTED
raised: 2026-10-11
promoted_from: the-progenitors-boot-is-split-by-phase
milestone_dependencies: 843
decision_dependencies: 266, 279
machine_requirements: none
specific_machine: none
needs_person: no
---
# 899. The progenitor's boot is split by phase

*(Promoted from the proposal pile on 2026-10-12 (UTC) by lane
`maintainer/promote-proposals`; number provisional until the merge queue lands it. Title
and slug are drafts.)*

Raised 2026-10-11 (UTC) by lane `milestone/843-the-system-initializer-crate-root-is-split-along-its-seams`
as that milestone's named follow-on. Title and slug are drafts.

Milestone 843 (the system-initializer crate root is split along its seams) moves
`system_initializer::boot` into a file of its own unchanged. That meets the ceiling of §266 (a Rust
source file stays under 2,000 lines), but `boot` is still one body of 1,700 lines. A reader editing
the shell's step still works with locals bound while the archive was read. calef ruled on
2026-10-09 (UTC) that the kernel's own boot sequence, `kernel_main`, is split by boot phase, one
function per phase
([the function-length ratchet proposal](0888-a-ratchet-on-function-length.md), open question 4). This is
the same split for the progenitor's half of booting.

Reuse: not applicable; this restructures code and adds none.

## The phases, measured

Measured on main at `ae6b91e79`, before 843's move, from `boot`'s top-level statements. Each phase is
named by the comment that opens it, since the move changes every line number. Counts are physical
lines, comments included. "Carried" counts the top-level bindings made in that phase and read in a
later one.

| phase | opens with | lines | carried |
|---|---|---|---|
| signature and doc | "Build the interactive system and become its spawn service" | 14 | |
| read the archive | "The archive the kernel mapped read-only" | 229 | 25 |
| entropy | "Read before anything below retypes" | 159 | 3 |
| network stack | "The network stack (milestone 590 (provisional)" | 37 | 1 |
| steps 1 and 2: console, line discipline, terminal | "The two components that have to exist before the progenitor can say anything" | 224 | 5 |
| the refusal | "Refuse the system if a required component is not the one that was measured" | 48 | 3 |
| step 3: input, the USB keyboard, the spawn channel | "3. The keystroke source" | 85 | 3 |
| step 4: the shell | "4. The shell" | 229 | 4 |
| step 5: the sink adapter | "5. The terminal's sink adapter" | 40 | 1 |
| the login stack and the run-unvouched endpoint | "Milestone 49's login stack" (milestone 49 (users and attribution)) | 358 | 2 |
| the giveaway and what the progenitor says | "Give the construction budget away" | 188 | 4 |
| step 6: the undertaker, then the spawn loop | "6. The undertaker" | 89 | |

70 top-level bindings, 51 of them read in a later phase. 25 of those 51 come from reading the
archive: each component's measured image, in one pass so a program is hashed once. That pass is
the natural first function, returning one struct of verdicts.

`spawn_service` is the same shape at 1,026 lines: 30 lines of setup, then one `loop` whose body is
about 970 lines handling one request. Its seams are the request's own stages (decode the wiring,
receive the grants and frames, carve the job's memory, build and endow the child, answer). It
belongs in the same milestone or the next.

## What makes it hard

- Retype order is load-bearing. Every object `boot` makes lands in the first free slot, and the
  entropy phase's comment records a slot collision found by bisection, not reasoning. A phase
  function must not reorder a retype across a phase boundary.
- The progenitor's table peaks inside the login phase, "one slot from the wall". Moving a
  capability's lifetime into or out of a function can move that peak.
- There are no host tests. `script/swish-check` on aarch64, riscv64 and x86_64, which boots the
  real progenitor and types at the prompt, is the only proof, with `script/test` on all three.
- Stack. The progenitor's stack is measured (`kernel/src/progenitor_stack.rs`); a phase function
  kept out of line changes frame sizes, so the headroom is re-measured.

## Done when

1. `boot` is a sequence of calls, one per phase above, and no phase function is over 300 lines.
2. The retype order is unchanged. The lane shows that by tracing the slot each object lands in, on
   one boot per architecture, before and after.
3. `script/swish-check` and `script/test` pass on aarch64, riscv64 and x86_64, and both
   `swish_check` falsification patches still fail their lines.
4. If the function-length ratchet has landed, `boot`'s row is banked down or removed.

## Index row

The progenitor's `boot`, one body of about 1,700 lines, splits into one function per boot phase,
the split calef ruled for the kernel's own `kernel_main`.
