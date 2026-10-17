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

The title is a statement about the past, as milestone 303 (x86_64's FS service has a server and
no disk it can find)'s block says of its own. Since 2026-10-16 (UTC) the runner attaches all five
disk fixtures, the NIC and an RNG. The residue is the display half, which
milestone 161 (the x86_64 kernel port)'s own note already said wants work beyond attaching
hardware. Promoted from the proposal `the-rest-of-the-x86-64-fixture-set`, filed 2026-09-16 by
milestone 303's lane (x86_64's FS disk). That lane attached the RedoxFS image and deliberately
stopped there, so that one milestone proved one thing. Milestone 215 (a PCI function's interrupt
reaches nothing on x86_64, so no userspace driver) listed the same set first. It numbered none of
them. *(Number provisional until the merge queue lands it.)*

## What landed

`helpers/qemu-runner-x86_64.sh` attaches all five disk fixtures the other two runners build, each
one `virtio-blk-pci` function in command-line order. The ordinals `virtio::find_block_device_n`
counts stay nifefs=0, redoxfs=1, crash=2, gpt=3, blank=4, exactly as the mmio slots carry them on
the other two. A `virtio-rng-pci` rides behind the same `iommu_platform=on` the disks use, under
the `NIFE_RNG` every test leg already exports. `cargo xtask test`'s x86_64 leg and its AMD-Vi leg
build the three new fixtures. The leg exports `NIFE_NET` itself, so a single-architecture run
boots the machine a full run always leaked in. The device-by-device outcome, which is this
milestone's deliverable:

- GPT disk (ordinal 3): `the_disk_surveyor_reads_a_table_gptfdisk_wrote` compiled and skipped on
  every x86_64 boot. It now runs, asserting the `q35` shape (every block device a PCI function,
  none mmio) and the table's contents. Running it exposed the one kernel gap the fixture set hid.
  `blk::SIZE` over virtio-pci had always answered 0: the config read (mmio offset 0x100) had no
  pci arm. The transport now resolves the `DEVICE_CFG` vendor capability and answers it. The
  surveyor's size read is true on both buses, and the gpu's config becomes reachable the same way
  when the display half lands.
- Blank disk (ordinal 4):
  `the_write_half_needs_a_disk_and_an_entropy_rendezvous_and_holds_nothing_else` skipped twice
  over, no disk and no entropy. It now runs. Its entropy rendezvous rides the PCIe transport,
  which is the bus `q35` has.
- RNG: `a_client_obtains_unpredictable_bytes_from_a_virtio_rng_over_pcie` now runs on x86_64. It
  asserts the device sits behind the IOMMU (VT-d, or AMD-Vi on that leg). The mmio entropy arm
  still skips, and every wiring that asks for mmio by name (credentials, NTP, login) with it.
  `q35` has no virtio-mmio bus. That arm's skip line is a machine fact, not a missing device.
- NIC: the proposal named `virtio-net-pci`, but the virtio-net tests' driver is not this port's
  NIC. Milestone 494 (a driver for the network card a PC actually has) made the `e1000e` the card
  a PC has. The runner has attached it under `NIFE_NET` since that milestone. What was missing was
  the export. The four `net_confinement_tests` and four `e1000e_tests` skipped on every
  `--arch x86_64` run; only a full run leaked the variable in. They now run on every x86_64 leg.
- Crash disk (ordinal 2): attached, and its fixture built, but nothing witnesses it on this ISA.
  Both crash-consistency twins are `cfg`-gated to their own ISAs. No x86_64 test skips for want of
  this disk, and none runs with it. The end-of-run host check keeps the crash image out of the
  x86_64-only arm for that reason. A check whose subject did not happen is worse than no check.

## BUGS

- The crash disk is attached but unwitnessed on x86_64. The crash-consistency test has no x86_64
  arm. The fixture's only job on this machine is keeping the ordinals aligned with the other two
  runners'. A future x86_64 crash arm inherits both the attachment and this gap.
- The mmio skip lines (`no virtio-rng device on the mmio bus (NIFE_RNG not set?)`) mislead on this
  one machine. No value of `NIFE_RNG` can attach an mmio device to `q35`. The lines are true on
  the other two, where setting it works.

## Follow-on

- **Outstanding.** The GPU and the keyboard. The work's own bound was attachment without breaking
  the suite, and on this port that amount is zero. The keyboard test's keystroke comes from the
  host monitor's `sendkey`, which only `cargo xtask`'s scanout referee provides, and the x86_64
  leg runs no referee. An attached keyboard therefore turns that test's skip into a bounded
  failure. The scanout half wants milestone 161's beyond-attachment work (console, input,
  keyboard_driver). Checked by reading the two tests' host halves in
  `system_tests/src/user/display_tests.rs`. The x86_64 leg of `xtask/src/suite.rs` runs no
  referee. `helpers/qemu-runner-x86_64.sh`'s header says device by device what is here and what is
  not. Recorded 2026-10-16.
- **Recorded.** The crash disk attached but unwitnessed on x86_64. This block's `BUGS` carries it,
  and `kernel/src/user/fs_service/crash.rs` is where an x86_64 arm starts (both twins are
  `cfg`-gated to their own ISAs).
- **Recorded.** The mmio skip lines that mislead on `q35`. This block's `BUGS` carries it, and
  `system_tests/src/user/entropy_tests.rs` is where the lines live.
- **Recorded.** The firmware path still proves none of the new fixtures.
  `helpers/qemu-uefi-x86_64.sh` attaches the two-disk set and no RNG. Under OVMF the crash, GPT,
  blank and entropy tests skip on their own lines. Its NIC is unconditional, so the net tests do
  run there. Its own `BUGS` carries the note. Extending it is a `-device`-line-sized piece with no
  open question in it.

## Index row

`helpers/qemu-runner-x86_64.sh` attaches all five disk fixtures the other two runners build, each
one `virtio-blk-pci` function in command-line order. The ordinals are nifefs=0 through blank=4,
the numbers `fs_service::crash` and `disk_service` name. It also carries the NVMe controller, an
`e1000e` NIC under `NIFE_NET` and a `virtio-rng-pci` under `NIFE_RNG`. The GPT surveyor, the
blank-disk write half, the PCIe entropy test and the `e1000e` tests therefore run on every x86_64
leg. They no longer skip for want of a device QEMU emulates on `q35`. The mmio entropy arm still
skips here, and that is a machine fact: `q35` has no virtio-mmio bus. What remains is the display
half. The GPU and keyboard want milestone 161's work beyond attaching hardware. A keyboard with no
referee turns a skip into a failure, so neither is attached.
