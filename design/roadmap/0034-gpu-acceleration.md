---
status: NOT-STARTED
raised: 2026-07-30
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 34. GPU acceleration via virtio-gpu 3D (the display ladder's rung four)

The block prices it as a mountain and says it reopens the parked competitor question. That question
is decided, not merely open. [DECISIONS
§131](../decisions/0131-hold-at-rung-two.md) (calef, 2026-08-26): hold at the display ladder's rung
two until something useful is built and proven on text mode. This is not an unresolved fork blocking
a lane. It is a resolved one whose answer is "not yet, and not a lane's to pick up," and it
reopens on the terms §131 names rather than on a further decision here.

**In brief.** The Venus path: Vulkan commands serialized over the virtio-gpu device, arriving on the
§18 (the PCIe transport) PCIe transport, so the guest gets real GPU acceleration without owning a
hardware driver. Needs
the 3D context and command-submission side of virtio-gpu that rung one deliberately left alone.
(Rung one sets up no cursor queue and no 3D context, keeping the §23 (multi-queue DMA confinement)
two-queue ceiling untouched.)
Needs the confinement story extended to command-carried backing addresses. That is the residual gap
of DECISIONS §30 (the DMA boundary is proved for
descriptors): those are the addresses the descriptor validator structurally cannot see, and today
only an IOMMU stops them. And needs something to consume it, which is what would give `wgpu` a real target

Why it matters. how every VM gets a GPU without a hardware driver, and the honest ceiling on the
display ladder. Rung five (a bare-metal driver for the VisionFive 2's BXE-4-32 3D core) is struck as
a Linux-scale multi-year effort that proves nothing this does not. A mountain, priced as such, and
it reopens the parked competitor question the ladder's governance note names as the architect's call

## Index row

how every VM gets a GPU without a hardware driver. Held by DECISIONS §131: text mode first.
