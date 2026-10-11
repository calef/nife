# Rented riscv64: the one offer, and what rental cannot buy

An appendix to [`notes/rented-metal.md`](../rented-metal.md), which holds the four requirements,
the recommendation for each architecture and BUGS. This file holds the riscv64 table, the free
RISC-V runners that do not qualify, and why no rental substitutes for radon.

One offer, and it is the one §203 (capacity is rented rather than bought) already found.

| Provider | Offer | Cores | RAM | Per hour | Per month | Free or trial | 1 boot | 2 console | 3 power API | 4 cores | Source, read 2026-09-23 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Scaleway | Elastic Metal EM-RV1 (T-Head TH1520, C910, RV64GC, 1.85 GHz) | 4C/4T | 16 GB | €0.042 | €15.99 | none | yes | yes | yes | yes | [RV1](https://www.scaleway.com/en/elastic-metal-rv1/), [Labs](https://labs.scaleway.com/en/em-rv1/) |
| RISE | RISC-V GitHub Actions runners | 1 job per node | - | free | free | free, no allowlist | **no** | no | no | - | [announcement](https://riseproject.dev/2026/03/24/announcing-the-rise-risc-v-runners-free-native-risc-v-ci-on-github/) |
| AWS, Azure, Hetzner, OVH | - | - | - | - | - | - | **nothing offered** | - | - | - | [survey](https://en.wikipedia.org/wiki/RISC-V_ecosystem) |

§203's figures are confirmed. The RV1 page gives the SoC, four cores at 1.85 GHz, 16 GB LPDDR4,
128 GB eMMC, 100 Mbit/s networking, and €0.042 an hour or €15.99 a month excluding VAT, for
`EM-RV1-C4M16S128-A`. The Labs page confirms the Labs status and is blunter than §203 was: *"EM-RV1s
are considered as 'Labs' services, with a service level agreement (SLA) contractually defined in our
special conditions for BETA services"*, and the SLA is 0%. Custom operating systems are supported
on the record: *"Access to the server's serial console is available for installing the most exotic
operating systems."*

The free RISC-V offer is real and does not qualify. RISE runs free native RISC-V GitHub Actions
runners for any open source project on GitHub, with no approval process and no allowlist. They run
on Scaleway EM-RV1 hardware, one job per node. It is genuinely excellent. It is also a Kubernetes
pod on a shared RISC-V node, so it cannot boot a kernel. It fails requirement 1 and with it 2 and 3.
It is worth knowing about for a different reason. If nife ever needs a RISC-V *build* or
*cross-check* leg in CI, this is free hardware for it. That is a separate proposal from this file's
subject.

## What riscv64 cannot buy

Rental does not substitute for radon, and §203 already said so. The TH1520 is a different SoC
from radon's JH7110, so booting nife on an RV1 is a port, with its own UART address, its own device
tree, and whatever its firmware hands over. That makes it a second RISC-V implementation, which
is a good thing to have and a different thing from a spare radon.

The RV1's boot path is documented and is the concrete form of that port. Scaleway's own account:
the early stages are *"the full boot process (which cannot be modified by the customer)"*, a first
stage U-Boot SPL fixed in eMMC *"which ensures a stable and immutable boot"*, then U-Boot Proper,
and only then the customer-controlled final stage, *"again U-Boot Proper, but configured
differently, which loads OpenSBI and the Linux kernel."* So nife arrives on this machine as
something U-Boot loads with OpenSBI underneath it, which is the same shape radon uses and a
different set of addresses and tables.

Three milestones name radon's JH7110 specifically and cannot be settled by rental at any price.
They are milestone 163 (the JH7110's PCIe root complex), milestone 239 (radon's device tree does not describe
the TRNG), and milestone 493 (a disk-file job mix needs a disk radon can drive). An RV1 has no PCIe
root complex to drive, a different device tree, and 128 GB of eMMC rather than radon's storage. It
also has no remote power cycle of radon, which is milestone 224 (nothing can power-cycle radon)'s
subject, and rented metal does nothing for that either.

And the RV1 will not settle milestone 225 (run the soak on radon, argon and xenon)'s RISC-V leg
as written, because that milestone names radon. It would settle a *different* and arguably more
interesting question: whether the soak behaves the same on a second RISC-V implementation. That is
worth proposing as its own milestone rather than quietly redefining 225's.
