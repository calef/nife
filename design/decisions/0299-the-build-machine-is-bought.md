---
status: DECIDED
raised: 2026-10-15
decided: 2026-10-15
ratified_by: calef
---

# 299. The build machine is bought, because buying repaid renting within a year

calef, 2026-10-15 (UTC). This section amends the ruling of
[§203 (capacity is rented rather than bought)](0203-capacity-is-rented-not-bought.md) for one
machine. *(Number provisional until the merge queue lands it.)*

## The ruling

In his words: *"We ordered a new workstation for nife development. It will be called galapagos. It
is a HP Z8 G4, 2x Xeon Platinum 8160 (24-core, 2.1 to 3.7 GHz each) with 192 GB of RAM. We will
move anything we use cordoba for in nife onto a container on this build machine."*

And why: *"We priced out the rental and the buy paid for itself in less than a year."*

And what it is for: *"galapagos' main load is going to be builds. patagonia cannot keep up and is
our bottleneck now that our token utilization is more optimized."*

galapagos is owned, and its main load is builds. It also takes over every role nife gives cordoba
today: the gateway
for open-model lanes, the lab update index, the KVM host for x86 benchmarks, and the second x86_64
machine for falsifying results. Those roles run in a container on it. Renting still covers what one
owned machine cannot: elastic capacity, and bare metal of another architecture or SoC, such as
§215 (the second RISC-V machine is a rented Scaleway Elastic Metal RV1).

## Why this does not contradict §203's reasoning

§203 found that inference alone does not repay hardware, and that holds; galapagos is not bought
for inference. §203 also found that the ceiling was patagonia's 8 cores and 16 GB, which set
`VERIFY_JOBS` at 4 and the lane count at three or four. That ceiling now binds: with token use
optimized, patagonia's builds are the bottleneck. galapagos is bought against it, which renting was
meant to lift, and calef's pricing found buying cheaper within a year.

## What it does not buy

- x86_64 gets hardware virtualization on galapagos. aarch64 and riscv64 under QEMU there are fully
  emulated, with nothing like patagonia's HVF, so a run moves only after it is measured faster.
- Skylake-SP adds cores, not x86 feature coverage beyond xenon's: no 5-level paging and no CET.
- cordoba keeps its other services. Each nife role leaves it through a written spec to the
  homelab agent once its replacement runs on galapagos.

## What is not decided

Whether anything that gates a merge runs on galapagos. A self-hosted runner on a public repository
runs pull-request code, and §203 sets the condition any runner meets: restricted to this
repository's own branches, with outside pull requests on hosted runners. That is calef's ruling to
make when galapagos is on the bench.
