---
status: BUILT
raised: 2026-09-14
built: 2026-10-16
promoted_from: the-uefi-boot-gate-asserts-two-cores-that-do-not-always-start
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 412. `script/test`'s UEFI leg asserts two cores online, and the second one does not always start

Promoted from the proposal
`the-uefi-boot-gate-asserts-two-cores-that-do-not-always-start`, filed 2026-09-14.
*(Number provisional until the merge queue lands it.)*

## The gate, and its measured rate

`cargo xtask uefi-boot` still asserts `smp: 2 core(s) online` unconditionally, at the `NIFE_SMP=2`
its own runner comment explains. The assertion stays. The dedicated measurement this block asked
for found zero failures in 42 boots on 2026-10-16, across a 6.63 to 23.76 host-load gradient:
every boot brought both cores online under real firmware, and none printed the `did not start`
line. The per-run rows, the machine's condition and the caveats are in
[notes/uefi-boot-two-core-rate.md](../../notes/uefi-boot-two-core-rate.md).

## Which record was wrong

The bound was. `arch::x86_64::ap_boot`'s BUGS section bounded the "did not start" flake at a third
or later secondary, and the tour's comment reasoned from it. The boundary was never real. The
defect was `cpu_start` re-reading the online count after the STARTUP IPIs, so a core that checked
in during the settle delay was counted absent at any position, and the count is read once now
(fixed 2026-09-19). That is the tree's own account, and the fix names the UEFI gate's one-in-three
as this defect reaching two cores. Four comments that still reasoned from the deleted bound are
corrected in this milestone's commit: the `NIFE_SMP=2` comment and the `uefi-test` leg's comment,
both in `xtask/src/uefi.rs`; `xtask/src/swish_check/boot.rs`'s; and the pointer to the deleted
BUGS section in `helpers/qemu-runner-x86_64.sh`.

## What the gate asserts, and why it is untouched

Two cores online is the boot-path proof that the trampoline page the loader asks for by name was
usable under real firmware, which is what the assertion's own comment says it is for. The block
weighed weakening or retrying the assertion against a gate known to fail one run in three. With
the defect fixed and the rate at zero at this sample size, the trade buys nothing, and the fork
never reached an architect: there was nothing left to decide.

## Follow-on

- **Recorded.** The suite under OVMF still boots one core (`uefi-test` keeps the runner's default),
  and the whole suite at two cores there is unmeasured since the fixes. It is carried where a
  reader meets it, in the `uefi-test` leg's own comment in `xtask/src/uefi.rs`, and in
  [notes/uefi-boot-two-core-rate.md](../../notes/uefi-boot-two-core-rate.md)'s BUGS.
- **Recorded.** An idle-machine batch of the rate, and an induced-load leg (one spinner per core),
  are both still unmeasured. This window never dropped below a 1-minute load of 6.63 and refused
  the load recipe on the tree's own rule. The same note's BUGS carries both.
- **Recorded.** xenon has still not been asked to bring all four cores online; `smp: 4 core(s)
  online` there would settle the silicon question `ap_boot`'s module doc carries.

## Index row

`cargo xtask uefi-boot` asserts `smp: 2 core(s) online` unconditionally at its own `-smp 2`. The
one-in-three `cpu 1 did not start` sighting that raised this block was `cpu_start`'s count
re-read, which reached two cores as well as three and was fixed on 2026-09-19. The dedicated
measurement is 0 of 42 boots across a 6.63 to 23.76 host-load gradient
([notes/uefi-boot-two-core-rate.md](../../notes/uefi-boot-two-core-rate.md)). The assertion
stays exactly as it is and no architect ask was filed. The comments that still reasoned from the
deleted "third or later" bound state the corrected record. The suite leg under OVMF (`uefi-test`)
still boots one core, and two cores there is unmeasured since the fixes.
