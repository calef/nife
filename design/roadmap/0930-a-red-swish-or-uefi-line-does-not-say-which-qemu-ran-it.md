---
status: NOT-STARTED
promoted_from: a-red-swish-or-uefi-line-does-not-say-which-qemu-ran-it
raised: 2026-10-16
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 930. A red swish-check or UEFI line does not say which QEMU ran it

Raised by milestone 414's lane (a red post-run check does not say which emulator produced it) on
2026-10-16, while putting the emulator version on the kernel legs' banner and on every red
post-run check line. The same hole sits one harness out, and both sites belong to other lanes'
code, so this lane recorded the work instead of doing it.

The measured fact behind milestone 414 is not confined to the post-run set: apt's QEMU 8.2.2 and
the pinned 11.0.2 fail disjoint sets of host-driven checks. Two places a reader meets that and
still gets no version:

- A `swish-check` failure names emulator behavior directly. Its sendkey arm says "the host's
  `sendkey` is not reaching the device", which is the exact check the milestone's table shows
  flipping between versions, and the USB-keyboard boot drives the same monitor. The failure line
  carries no version; neither does the banner above it.
- A `uefi-test` failure line classifies the exit status against the isa-debug-exit protocol
  (milestone 516 ("`uefi-test` can exit 1 after its own suite has passed, and nobody has taken a
  rate")) but not the version. notes/uefi-test-exit-rate.md exists because a QEMU-version theory
  survived an hour unrefuted.

## What would close it

`host::qemu_version` (milestone 414) already reads the version the way the runner resolves the
binary. One string beside each failure line, and in the two harnesses' banners, closes it; nothing
new learns how.

Reuse: everything. The version reader, the banner pattern and the red-line shape are all in the
tree from milestone 414 (`xtask/src/host.rs`, the `scanout`/`inbound` failure lines). This work
only carries existing strings to two more output sites; nothing is written and nothing is taken
on.

## Done means

A red swish-check line and a red UEFI line each carry the version, and the banners above them do
too, as the kernel legs' now do.

## Index row

A red `swish-check` or `uefi-test` failure line carries no QEMU version, though apt's 8.2.2 and the pinned 11.0.2 fail disjoint sets of checks. Milestone 414's version reader needs carrying to those two harnesses' banners and failure lines.
