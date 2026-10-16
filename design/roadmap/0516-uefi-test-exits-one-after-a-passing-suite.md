---
status: BUILT
built: 2026-10-16
raised: 2026-09-19
promoted_from: uefi-test-exits-one-after-a-passing-suite
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 516. `uefi-test` can exit 1 after its own suite has passed, and nobody has taken a rate

*(Number provisional until the merge queue lands it.)* Promoted from the
proposal `uefi-test-exits-one-after-a-passing-suite`, filed 2026-09-19, on calef's instruction of
2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own, unedited
except for this paragraph: the argument is its author's and promotion is not the moment to improve
it. Written by milestone 117 (the stranger test)'s run 6 lane, from the stranger's red `script/test` and the operator's
green re-run.

A lane can start today; it needs the dev Mac and nothing else.

**In brief.** On 2026-09-19 a stranger's `cargo xtask test` passed every leg on all three ISAs and
then failed at the last one: under OVMF the kernel suite printed `test result: ok. 215 passed, 71
skipped` and `uefi-test` reported `qemu exited Some(1), not 3`. A re-run of `cargo xtask uefi-test`
an hour later, same tree, same machine, same Homebrew QEMU 11.1.1, passed. Both runs printed the
same VT-d fault lines, which milestone 215 established are the escape test's own expected faults,
so they are not the cause, and they are the only thing on screen that looks like one.

## What it would take

Take a rate before anything else, the way notes/load-sensitive-assertions.md did for the timer
assertions: twenty runs of `cargo xtask uefi-test`, with the host load average beside each, on an
idle machine and on a loaded one. Then find out what path exits 1 after the harness has printed its
summary (a late fault, a triple fault on the way to `isa-debug-exit`, the firmware's own exit), and
make the failure message say which, since the one a reader gets today invites a QEMU-version theory
the evidence does not support. Say at the failure that the VT-d lines are expected.

Reversible, and on the customer path only by way of trust: a gate that goes red for a reason nobody
can name costs every contributor who meets it an hour, which is what it cost run 6.

## What the lane found and did, 2026-10-16 (UTC)

The rate, on patagonia (8 cores, Homebrew QEMU 11.1.2): twenty `cargo xtask uefi-test` runs,
zero red, every run printing both suites' green summaries and exiting 0. The block asked for
idle and loaded batches; the window delivered a load gradient instead (1-minute average 2.71 to
5.63). The lane executing milestone 412 (`script/test`'s UEFI leg asserts two cores online) was
measuring `uefi-boot` rates on this machine at the same time, lanes cannot see each other, and
forcing load would have poisoned both rates. The
per-run load columns and the concurrency caveat are in
[notes/uefi-test-exit-rate.md](../../notes/uefi-test-exit-rate.md). By the rule of three, zero in
twenty bounds the rate below 1 in 7 at 95% confidence: consistent with the single 2026-09-19
sighting, and the finding is the bound, not an absolution. The message work below stands on the
code reading, not on a reproduced failure.

What exits 1 after the summary, now stated where the reader meets it: `isa-debug-exit` can only
produce odd statuses. A 1 is either the guest's failure write, whose only in-tree writer past the
green summary is the panic arm (which prints `[PANIC]` first), or QEMU's own error exit, whose
reason streams to the harness's stderr. A 0 is never the device: the `NIFE_UEFI_TIMEOUT` bound
lands there, because QEMU exits 0 on the SIGTERM that ends it (measured 2026-10-15). The failure
line now names the boot, decodes the status against that protocol, and carries the elapsed
seconds. It opens by saying the run's VT-d lines are the escape test's own expected faults, which
milestone 215 (a PCI function's interrupt reaches nothing on x86_64) established.
`exit_verdict_lines` in `xtask/src/uefi.rs` holds
the classifier, unit-tested; notes/uefi-test-exit-rate.md holds the transcript-marker table that
attributes any guest-side failure write.

Measurement by-product, fixed in the lane: a bare `cargo xtask uefi-test` had been
deterministically red since 857654808 (2026-10-04). Only `test()` set `CONFINEMENT_ATTACKERS`,
so the system_tests image packed an archive without the DMA-escape role
and failed by name on every bare run. Three of the rate attempt's first four runs died there,
before the block's own defect could be observed. `uefi_test()` now sets the switch itself; it
never stages a shipped image.

## Follow-on

- **Recorded.** The rate's two residuals live in `notes/uefi-test-exit-rate.md`'s BUGS: the
  idle-machine batch is unmeasured (the window's 1-minute load never dropped below 2.71), and
  the exit-path classifier's readings are unit-tested, not nature-tested, until a real red boot
  exercises a path.

## Index row

the rate a passing suite exits 1 anyway, and a failure message that names its exit path
