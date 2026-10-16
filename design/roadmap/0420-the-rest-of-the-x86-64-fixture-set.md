---
status: PARTIAL
raised: 2026-09-16
promoted_from: the-rest-of-the-x86-64-fixture-set
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 420. x86_64 has one fixture of five and no NIC, GPU, keyboard or RNG, so its skip count is a device list

**The title is a statement about the past** (milestone 303 (x86_64's FS service has a server and
no disk it can find)'s block makes the same move): since
2026-10-16 (UTC) the runner attaches all five disk fixtures, the NIC and an RNG, and the residue
is the display half, which milestone 161 (the x86_64 kernel port)'s own note already said wants
work beyond attaching hardware. Promoted from the proposal `the-rest-of-the-x86-64-fixture-set`,
filed 2026-09-16 by milestone 303's lane (x86_64's FS disk), which attached the RedoxFS image and
deliberately stopped there so that one milestone proved one thing. Milestone 215 (a PCI
function's interrupt reaches nothing on x86_64, so no userspace driver)'s block listed the same
set without numbering it. *(Number provisional until the merge queue lands it.)*

## What landed

`helpers/qemu-runner-x86_64.sh` attaches all five disk fixtures the other two runners build, each
one `virtio-blk-pci` function in command-line order, so the ordinals `virtio::find_block_device_n`
counts stay nifefs=0, redoxfs=1, crash=2, gpt=3, blank=4 on this machine exactly as the mmio slots
carry them on the other two. A `virtio-rng-pci` rides behind the same `iommu_platform=on` the
disks use, under the `NIFE_RNG` every test leg already exports. `cargo xtask test`'s x86_64 leg
and its AMD-Vi leg build the three new fixtures, and the leg exports `NIFE_NET` itself, so a
single-architecture run boots the machine a full run always leaked in. The device-by-device
outcome, which is this milestone's deliverable:

- **GPT disk** (ordinal 3): `the_disk_surveyor_reads_a_table_gptfdisk_wrote` compiled and skipped
  on every x86_64 boot; it now runs, asserting the `q35` shape (every block device a PCI function,
  none mmio) and the table's contents.
- **Blank disk** (ordinal 4): `the_write_half_needs_a_disk_and_an_entropy_rendezvous_and_holds_nothing_else`
  skipped twice over, no disk and no entropy; it now runs, taking its entropy rendezvous on the
  PCIe transport, which is the bus `q35` has.
- **RNG**: `a_client_obtains_unpredictable_bytes_from_a_virtio_rng_over_pcie` now runs on x86_64
  and asserts the device sits behind the IOMMU (VT-d, or AMD-Vi on that leg). The mmio entropy
  arm, and every wiring that asks for mmio by name (credentials, NTP, login), still skips here:
  `q35` has no virtio-mmio bus, so that arm's skip line is a machine fact, not a missing device.
- **NIC**: the proposal named `virtio-net-pci`, but the virtio-net tests' driver is not this
  port's NIC; milestone 494 (a driver for the network card a PC actually has) made the `e1000e`
  the card a PC has, and the runner has attached it under `NIFE_NET` since. What was missing was
  the export: the four `net_confinement_tests` and four `e1000e_tests` skipped on every
  `--arch x86_64` run because only a full run leaked the variable in. They now run on every x86_64
  leg.
- **Crash disk** (ordinal 2): attached, and its fixture built, but nothing witnesses it on this
  ISA. Both crash-consistency twins are `cfg`-gated to their own ISAs, so no x86_64 test skips
  for want of this disk and none runs with it. The end-of-run host check keeps the crash image out
  of the x86_64-only arm for that reason: a check whose subject did not happen is worse than no
  check.

## BUGS

- The crash disk is attached but unwitnessed on x86_64: the crash-consistency test has no x86_64
  arm, so the fixture's only job on this machine is keeping the ordinals aligned with the other
  two runners'. A future x86_64 crash arm inherits both the attachment and this gap.
- The mmio skip lines (`no virtio-rng device on the mmio bus (NIFE_RNG not set?)`) are misleading
  on this one machine: no value of `NIFE_RNG` can attach an mmio device to `q35`. True on the
  other two, where setting it works.

## Follow-on

- **Outstanding.** The GPU and the keyboard. The work's own bound was attachment without breaking
  the suite, and on this port that amount is zero: the keyboard test's keystroke comes from the
  host monitor's `sendkey`, which only `cargo xtask`'s scanout referee provides, and the x86_64
  leg runs no referee, so an attached keyboard turns that test's skip into a bounded failure; the
  scanout half wants milestone 161's beyond-attachment work (console, input, keyboard_driver).
  Checked by reading the two tests' host halves in `system_tests/src/user/display_tests.rs`
  against `xtask/src/suite.rs`'s x86_64 leg, which uses no referee. `helpers/qemu-runner-x86_64.sh`'s
  header says device by device what is here and what is not. Recorded 2026-10-16.
- **Recorded.** The crash disk attached but unwitnessed on x86_64: this block's `BUGS`, and
  `kernel/src/user/fs_service/crash.rs` is where an x86_64 arm starts (both twins are `cfg`-gated
  to their own ISAs).
- **Recorded.** The mmio skip lines that mislead on `q35`: this block's `BUGS`, and
  `system_tests/src/user/entropy_tests.rs` is where the lines live.

## Index row

`helpers/qemu-runner-x86_64.sh` attaches all five disk fixtures the other two runners build (each
one `virtio-blk-pci` function, ordinals nifefs=0 through blank=4, the numbers `fs_service::crash`
and `disk_service` name), the NVMe controller, an `e1000e` NIC under `NIFE_NET` and a
`virtio-rng-pci` under `NIFE_RNG`, so the GPT surveyor, the blank-disk write half, the PCIe
entropy test and the `e1000e` tests run on every x86_64 leg instead of skipping for want of a
device QEMU emulates on `q35`. The mmio entropy arm still skips here, and that is a machine fact:
`q35` has no virtio-mmio bus. What remains is the display half: the GPU and keyboard want
milestone 161's work beyond attaching hardware, and a keyboard with no referee turns a skip into
a failure, so neither is attached.
