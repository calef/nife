# What rented metal costs, for all three architectures

*Name provisional (`rented-metal`), per the rule that a lane ships a provisional name and says so.*

Why this file exists. §203 (capacity is rented rather than bought) ruled *rent, do not buy*, and
left exactly one line open: which provider, and the spend split. Nobody had acted on it. calef,
2026-09-23: *"we want to price all three architectures and lean heavily on free/trial options to keep
costs low."* This is the priced comparison that open line needs. No account was created and nothing
was spent. Every figure below carries the page it came from and the date it was read, which was
2026-09-23 for all of them. Each architecture's table and sources are in
[`rented-metal/`](rented-metal/README.md).

**What this is not.** It is not a recommendation to sign up today, and it is not a benchmark. It
prices access to machines, against the four things a machine must do before nife can use it at all.

The aarch64 offers qualify and nife cannot yet boot them, which is a different problem from a gap
in the market, with a different fix: see "aarch64" below.

---

## The four hard requirements, and the fifth column

An offer that fails any of the first four is **not cheaper, it is unusable**, and it is listed here
as failing rather than as qualifying-with-a-caveat.

1. nife must boot on it. A documented boot path and the ability to put your own kernel on the
   disk. §203 refuses closed appliances at any price for this reason, and a virtual machine that
   only boots images from a vendor library fails it just as completely as an appliance does.
2. A serial console, or an equivalent out-of-band channel, because a kernel that panics before
   userspace has nothing else to say.
3. A power or reset API, since removing the person from the loop is most of the point.
4. Multiple cores, for fatal risk 5, whose whole premise is about multicore.

**Requirement 1 is scored with four values, not two.** A requirement nobody checked is a gap in the evidence, not a failure by the offer:

| Value | Means |
|---|---|
| **yes** | the provider documents it |
| **unverified** | not checkable without an account; no accusation against the offer |
| **no (offer)** | the provider will not let you put your own kernel on the disk |
| **no (nife)** | the provider will, and **nife cannot boot what the machine hands over** |

The last value is the one the aarch64 section now turns on, and it did not exist until calef pushed
back on that section. It matters because the two "no"s imply opposite responses: `no (offer)` says
shop elsewhere, `no (nife)` says the money was never the problem.

**The fifth column is not a requirement, it is the independent variable.** calef, 2026-09-23: *"I
also think it is helpful to have multiple cloud platforms to minimize our own cloud platform
assumptions"*, and *"a nife that runs on one cloud platform but not another is also its own form of
risk."* So each row also records how a custom kernel is delivered, through what channel the console
and power are reached, and what the machine hands the kernel at entry. Where two providers of the
same architecture differ there, the difference is the thing being bought. Where they do not differ,
this file says so rather than manufacturing a diversity argument.

---

## x86_64

Commodity, and the column with real choice in it. Scaleway Elastic Metal qualifies on all four.
OVH's Rise range does too, at roughly six times the price. OVH's cheap Kimsufi range has no
console, Hetzner's console and custom image need a support request, Oracle's free x86 tier has an
eighth of a core, and Equinix Metal shut down on 30 June 2026. Whether nife boots is unverified on
every row, and much likelier than on aarch64, because the x86_64 loader already takes the ACPI RSDP
from the firmware. [`x86_64.md`](rented-metal/x86_64.md) has the table, the sources, and the case
against each offer not recommended.

### x86_64 recommendation

Scaleway Elastic Metal EM-A116X-SSD, €0.077 an hour or €27.99 a month. Four real cores, serial
console, API power actions, hourly billing so a bench evening costs under a euro, and the same
provisioning surface as the RISC-V machine in the third table, which is worth something on its own.

And a second provider for this architecture is worth buying, which is the one place this file says
that. See "What a second provider is a test of" below: OVH Rise is the second, at $64 a month, and
the reason is that its boot delivery and console channel genuinely differ from Scaleway's rather than
duplicating them.

---

## aarch64

Every aarch64 offer passes requirements 2, 3 and 4 and takes a custom image (`BOOTAA64.EFI`
exists): AWS, Azure and Google all sell Arm compute, and calef's question (2026-09-23) was *"aarch64
has no cloud options. That doesn't seem right. What about AWS?"* What blocks requirement 1 is in this
repository. Every aarch64 server cloud presents ACPI and no device tree, by Arm's Server Base Boot
Requirements. The loader writes a device tree from a machine's ACPI tables, so an ACPI-only machine
is not refused outright, and under QEMU with ACPI on the stick reaches the shell (verified against
`uefi_loader/src/arch/aarch64/mod.rs` and
[`notes/boot-stick/device-tree-handover.md`](boot-stick/device-tree-handover.md)). The kernel is
still linked for `0x4008_0000`, QEMU `virt`'s RAM, the same limit milestone 127 (the seL4
machine)'s argon port has to lift, and no cloud machine has been booted. So the `no (nife)`
verdicts below are unverified, and the experiment the recommendation names is the way to find out.
A `.metal` instance is not required: an ordinary virtualized Graviton meets all four requirements,
so the price is $0.073 an hour, not $2.32. [`aarch64.md`](rented-metal/aarch64.md) has the case
from fatal risk 9, the offers table, and the Oracle Ampere A1 experiment.

### aarch64 recommendation

The offers qualify; nife does not. That points at work in this repository rather than at a gap in
the market.

- The machine to buy, when nife can use it, is an ordinary virtualized Graviton instance, not a
  `.metal` one: `c7g.large` at $0.073 an hour for a two-core soak, `c7g.4xlarge` at $0.58 for
  sixteen cores. A ten-hour bench evening is $0.73 or $5.80.
- The experiment to run first costs nothing: Oracle Always Free Ampere A1, two OCPUs, with
  `BOOTAA64.EFI` on an EFI system partition, to observe what the firmware offers.
- The work that unblocks the column is a milestone, not a purchase. It was proposed here and
  promoted on 2026-09-23 as milestone 578 (an ACPI discovery path for aarch64, because every server
  machine has one). Its shape is already visible on the x86_64 side of
  the same loader, which takes the RSDP from the UEFI configuration table, and the aarch64 half would
  need the MADT for the interrupt controller and the GTDT for the timer. The linked-address limit
  above is the second half, shared with milestone 127 (the seL4 machine).
- And a second aarch64 provider is not worth buying yet, because none of them can be tested until
  that milestone lands. The diversity argument below is live for x86_64 today and deferred here.

---

## riscv64

One offer, and it is the one §203 already found: Scaleway Elastic Metal EM-RV1, a T-Head TH1520
with four cores and 16 GB. It costs €0.042 an hour or €15.99 a month, as a Labs service with a 0%
SLA and a serial console. RISE's free RISC-V GitHub Actions runners are real and cannot boot a kernel. No
other provider offers RISC-V. [`riscv64.md`](rented-metal/riscv64.md) has the table and sources.

### riscv64 recommendation

Scaleway EM-RV1, €0.042 an hour or €15.99 a month. It is the only offer that qualifies, in any
column, at any price. Buy it on its own merits and understand what it is not, below.

---

## What riscv64 cannot buy

Rental does not substitute for radon. The TH1520 is a different SoC from radon's JH7110, so
booting nife on an RV1 is a port behind Scaleway's fixed U-Boot and OpenSBI stages. Milestones 163,
239 and 493 name the JH7110 and cannot be settled by rental, nor can milestone 224 (nothing can
power-cycle radon). An RV1 soak would answer whether the soak behaves the same on a second RISC-V
implementation, which is worth its own milestone rather than redefining milestone 225 (run the soak
on radon, argon and xenon). [`riscv64.md`](rented-metal/riscv64.md) has the boot path and each
milestone.

---

## What a second provider is a test of

calef, 2026-09-23: *"a nife that runs on one cloud platform but not another is also its own form of
risk."* This is an experiment, not redundancy. Two x86_64 providers are the same silicon with
different firmware, boot delivery and tables at entry, a gap fatal risk 9 (the HAL is a fiction,
and an architecture costs a restructure rather than a port) does not cover. The hypothesis: nife
boots unchanged on both, and only image and console delivery differ. A failure would name an
assumption nife makes about what a machine hands it at entry. Finding no difference is also a
result, and it would weaken the case for the second account.

The interfaces do differ. Scaleway hands you a serial line over SSH and three REST verbs, OVH a BMC
with virtual media and serial over LAN, and Hetzner puts a human in the loop for the console. The
firmware row could not be filled in from public pages. So the argument about our tooling is
evidenced and the argument about nife's own boot assumptions is a hypothesis. The cost is three
accounts, three provisioning APIs, three sets of credentials and three console mechanisms. Two
x86_64 providers are worth it; a third is not, and aarch64 and riscv64 barely have a first.
[`a-second-provider.md`](rented-metal/a-second-provider.md) has the side-by-side table and the
costs in full.

---

## What it would cost to settle milestone 225's x86_64 and aarch64 legs

Milestone 225 (run the soak on radon, argon and xenon) is gated on hands at the boards; a rented
machine with a power API and a console removes the person. Ten hours, an eight-hour soak plus
setup, costs €0.77 on Scaleway's x86_64 metal and $0.73 on AWS `c7g.large` (or $5.80 for sixteen
cores). The money is not the constraint on either leg, and on aarch64 it never was. A rented
machine is not xenon or argon, so it is sound for the soak and unsound for anything compared
against their per-machine numbers. [`settling-milestone-225.md`](rented-metal/settling-milestone-225.md)
has the table, the corrected first-draft paragraph, and that caveat in full.

---

## The recommendation in one table

| Architecture | Buy | Price | Why |
|---|---|---|---|
| x86_64 | Scaleway Elastic Metal EM-A116X-SSD | €0.077/hr, €27.99/mo | the only cheap row that passes all four; hourly billing suits a bench evening |
| x86_64, second | OVHcloud Rise RISE-1 | $64/mo | a genuinely different boot delivery and console channel, which is the experiment, not redundancy |
| aarch64 | **buy nothing yet.** AWS `c7g.large` is the machine when nife can use it; Oracle Always Free is the experiment now | $0.073/hr, and €0 | the offers qualify and nife does not: every aarch64 cloud is ACPI-only and nife's aarch64 path needs a device tree |
| riscv64 | Scaleway Elastic Metal EM-RV1 | €0.042/hr, €15.99/mo | the only qualifying RISC-V rental anywhere, and it is a second implementation rather than a spare radon |

Which free and trial tiers genuinely qualify: none of them, and for two different reasons. RISE's
RISC-V runners are free, real, and cannot boot a kernel, which is the offer's limit. Oracle's Always
Free x86 micro shapes fail on cores, which is also the offer's limit. Oracle's Always Free Ampere
A1 passes all four as an offer and fails on nife's side. That makes it the best free thing on this
page: it costs nothing to run the observation that would show what still stands in the way. calef asked to lean
hard on free and trial options; the
honest report is that the cheap paid options are cheaper than they look and the free ones do not
do this job. A soak on Scaleway metal costs less per evening than a cup of coffee.

---

## BUGS

- **Hetzner's own product pages do not render prices to a fetcher.** The AX matrix, the AX42 page and
  the cloud page all returned specifications with the price fields empty. The €97.30 for AX42-1 comes
  from Hetzner's price adjustment document, which is a Hetzner page but is a change notice rather
  than a price list, and it is dated *15 June 2026*. The CAX cloud prices in the aarch64 table come
  from third-party aggregators and disagree with each other by nearly 2x; they are marked with a
  range for that reason and should not be quoted.
- Every AWS hourly price here is third-party. $0.073, $0.58, $2.32 and $3.83 for us-east-1 are
  from an aggregator, not from AWS's own pricing page, which did not render figures to a fetcher.
  Treat all four as indicative, and note they are Linux on-demand rates in one region.
- **Azure and Google Arm prices were not read at all.** Both providers' Arm series are in the aarch64
  table on their specifications, with the price column blank, because neither pricing page was
  fetched. That is a gap rather than a finding, and it does not currently matter, because the column
  is blocked on nife rather than on price. It would matter the day nife boots one of them. (The loader's ACPI path landed on 2026-09-23; the
  link-address limit has not moved.)
- The ACPI-only claim about aarch64 clouds is sourced three ways and verified on none of them by
  observation. AWS's boot-mode page shows arm64 resolving to UEFI, Azure's Arm series is
  Generation 2, and Google's image-import requirements say the boot disk must support ACPI. Arm's
  SBBR is the reason behind all three. The strongest single piece of evidence is a third-party
  port of another OS to Graviton reporting ACPI with no timer node and having to parse the GTDT, and
  a third-party report is weaker than a vendor statement. None of this is the same as booting
  `BOOTAA64.EFI` on one of these machines and reading the console, which is the experiment this
  file recommends precisely because the claim deserves it. If it turns out some provider does offer a
  device tree through `DEVICE_TREE_GUID`, this section is wrong and that would be good news.
- This file's negatives lean confident. A claim that no offer qualifies, or that an image does not
  exist, is checkable in this repository; check it before relying on it.
- **OVH's API coverage for reboot and netboot was not confirmed.** The IPMI guide documents the
  console and does not cover API-level power control, and the API documentation index did not render.
  The Rise row scores requirement 3 yes on the basis of IPMI, which is out-of-band power control by
  definition, not on a documented REST endpoint.
- The Scaleway serial console was confirmed for the RV1 explicitly and for Elastic Metal generally
  from the documentation index, where `Use the serial console` appears as a how-to. Two candidate
  how-to URLs both 404'd, so the per-offer availability of the serial console on the x86 Elastic
  Metal ranges rests on the index and on the Labs page's account, not on a page read end to end.
- Every price excludes VAT, egress, storage and setup. Scaleway's figures are stated excluding
  VAT. The Elastic Metal and RV1 prices are for the machine; network transfer beyond whatever is
  included is not priced here, and the RV1's 100 Mbit/s link is a specification rather than a quota
  that was checked. OVH's figures are shown in USD on a page that also advertises them as special
  offers, so a renewal price may differ from a first-term price. AWS's figure excludes storage, the
  EBS volume the AMI needs, and egress.
- Prices move, and this column moved recently. Hetzner repriced its whole dedicated line on
  15 June 2026, and Equinix Metal's shutdown on 30 June 2026 removed a provider §203 had
  listed. Anything here is worth re-reading before money is spent, and the read date on every row is
  2026-09-23 for exactly that reason.
- The firmware row of the fifth-column table is empty for all three x86_64 providers, which is the
  single most load-bearing gap in this file. What a machine hands a kernel at entry is not published
  by any of them, so the risk-9-shaped argument for a second provider is a hypothesis this file could
  not test from a browser.
- No provider was contacted and no account was created. Everything here is from public pages.
