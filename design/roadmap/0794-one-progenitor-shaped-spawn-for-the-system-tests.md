---
status: PARTIAL
promoted_from: one-progenitor-shaped-spawn-for-the-system-tests
raised: 2026-10-05
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 794. One progenitor-shaped spawn for the system tests

Raised by the build of §185 (what carries the claim that userspace composes a process from an
authority you can count on one hand). The tree holds five copies of this sequence, because each is
private to its test module.

## The duplication

Five harnesses in `system_tests/src/user/` start a program the way the kernel starts the
progenitor. Each one does these steps:

- Lay out the program's ELF in a hand-built space.
- Map a stack.
- Map the timebase page on x86_64 and riscv64.
- Map the whole archive read-only at `INITIAL_RAMDISK_VA`.
- Insert a memory region in slot 0 and a report line in slot 1.
- Configure, and start with the archive's length in `x1`.

The five are `authority_tests::spawn_tree`, `c_seam_tests::spawn_confiner`, `live_swap_tests::spawn_swapper`,
`timetable_tests::spawn_timetable` (a variant that copies a narrowed archive into fresh pages
rather than mapping the real one) and `process_composition_service::wire`. Each sizes the space
with the same arithmetic. That includes the `log_pages_for(initial_ramdisk_pages)` term, which had to be
added to every copy when `map_physical` began recording.

`run` cannot serve them. It takes the extra pages as a slice, and the suite has no allocator
to build a slice of a few thousand mappings.

## The proposal

One `pub(super)` function in `system_tests/src/user.rs` (or beside it), taking the program's name,
its budget in pages, the region capability's rights and the report rendezvous, and returning once
the program is started. The five call sites shrink to one line each. The next change to the
progenitor's endowment lands in one place. The kernel's own `spawn_hello` is a sixth copy, in the
kernel crate. Folding it in is a separate question, because it also measures the program and routes
interrupts.

The cost is a touch of five files in the test-wiring hotspot. That is why the §185 lane did not do
it. The lane for milestone 95 (an unmap primitive, and the mappings init never lets go) was
open in the same files.

Reuse: the helper would be the reuse. `kernel::user::run` was considered and does not fit
(above); `spawn_hello` measures and routes interrupts, which no test harness wants. Searched
`system_tests/src/user/` and `kernel/src/user.rs` for `INITIAL_RAMDISK_VA`.

## What is blocked

Nothing. The copies agree today; the risk is the next edit that reaches four of them.

## Scope

2026-10-16 (UTC): built as `test_programs::spawn_progenitor_shaped` (name provisional). The report
rendezvous is made by the caller and passed in, because `live_swap_tests` carves its notification
and timer from the same region the report line comes from; the helper takes both capabilities'
rights. Four of the five call sites are one call each, and the `log_pages_for(initial_ramdisk_pages)`
term is in one place. The helper carries two beyond-the-floor parameters for the operator:
capabilities past slot 1, and one notification bound between configure and start.
`timetable_tests::spawn_timetable_with` stayed whole. Its slots are its own four-slot contract
(output, budget, child report, deaths), its archive is a narrowed copy it builds rather than the
initial ramdisk, and its stack, optional mappings and start arguments are its own. A helper
parameterized to cover all that would be that function wearing this one's name. The kernel's
`spawn_hello` remains the sixth copy, out of scope here.

## Follow-on

- **Outstanding.** `timetable_tests::spawn_timetable_with` is still its own spawn site, the one of
  the five this block did not fold, for the scope paragraph's reason. Checked with the grep the
  block was raised with (`grep -rn INITIAL_RAMDISK_VA system_tests/src/user system_tests/src/user.rs`):
  five hits at the raise, one now, and this is it.
- **Recorded.** `system_tests/src/user/test_programs.rs`: the kernel's `spawn_hello` remains the
  sixth copy, and the helper's own doc says why it stays one (it measures the program and routes
  the boot interrupts, which no test harness wants).

## Index row

Five system-test harnesses lay out a program the way the kernel starts the progenitor, each with its own copy of the stack, timebase, archive and log-page arithmetic. One shared spawn removes the copies and the drift between them.
