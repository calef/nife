---
status: NOT-STARTED
raised: 2026-10-20
milestone_dependencies: 53
decision_dependencies: none
machine_requirements: riscv64 silicon with two DesignWare Ethernet QoS ports; a cable a person moves between them
specific_machine: radon (the only machine nife runs on with a second JH7110 port, and the one whose second jack found nothing on 2026-10-20)
needs_person: yes
---
# 940. radon's second Ethernet port: the driver brings up `ethernet@16040000` as well

*(Number, title and slug provisional. The choice between the options below is calef's and is
asked on the pull request that added this block.)*

calef asked for this on 2026-10-20 (UTC), at the bench: "Lets create a milestones to get jack #1
also working." A writing-only lane wrote it the same day and built nothing.

## Why

radon, a VisionFive 2 v1.3B, has two RJ45 jacks. Each is a Synopsys DesignWare Ethernet QoS
controller behind its own Motorcomm YT8531: `gmac0` at `0x1603_0000` and `gmac1` at
`0x1604_0000`. Milestone 53 (the board's own peripherals: network and storage on real silicon)
drives only the first. Its first bench runs, on 2026-10-20 (UTC), on branch
`milestone/53-radon-first-runs` (`bench/radon-2026-10-20/network-bench-{1,2,3}.log`):

- U-Boot 2021.10 links on either jack.
- With the cable in `gmac0`'s jack, `network_bench` reached a 1000 Mbit/s link, a DHCP lease and
  `coherence : Coherent`. The PHY read `id 0x4f51e91b`, 1800 mV I/O, settings from `Fallback`.
- With the cable in `gmac1`'s jack, nife printed `link : NONE after 5003 ms`. That is expected
  and not a fault: nothing in the tree touches `0x1604_0000`.

So a person who plugs radon in by the jack nearest to hand has a one-in-two chance of no network,
and nothing on the console says to try the other one. The firmware has no such limit.

## What the tree has

Everything port-specific is written for `gmac0`, and the shape of a second port is already in the
sources this tree read.

- The port. `designware_ethernet::jh7110::discover` returns the first node matching either
  compatible, which is `gmac0` in both trees. `GMAC1_BASE` exists in
  `crates/designware_ethernet/src/jh7110.rs` and nothing reads it. `DeviceTreeBlob`'s lookups
  answer the first match; the node name, `ethernet@16040000`, is the same in both trees, so
  `node_reg` and `node_prop` by name reach the second.
- The clocks and resets. `crates/jh7110_clock_and_reset` has `GMAC0_SYS_BRING_UP` and
  `GMAC0_AON_BRING_UP`, split across two domains. `gmac1` lives wholly in SYS. Mainline's
  `jh7110.dtsi` (torvalds/linux `f47d223627f8`, read 2026-10-20) names its clocks `GMAC1_AXI` (98),
  `GMAC1_AHB` (97), `GMAC1_PTP` (102), `GMAC1_TX_INV` (106) and `GMAC1_GTXC` (107), and its resets
  `SYSRST_GMAC1_AXI` (66) and `SYSRST_GMAC1_AHB` (67). The v1.3B board file sets `GMAC1_TX` (105)'s
  parent to `GMAC1_RMII_RTX` (101) beside `starfive,tx-use-rgmii-clk`, the same arrangement
  `gmac0` has in AON. The `SYS` domain's arithmetic already covers reset 66 and clock 107.
- The interface select. `GMAC0_SYSCON_OFFSET` and `select_rgmii` write bits 18 to 20 of the
  AON syscon's word `0xc`. `gmac1`'s is in the other syscon: mainline's `starfive,syscon =
  <&sys_syscon 0x90 0x2>`, bits 2 to 4 of word `0x90` at `0x1303_0000`. That window is
  `SYS_SYSCON_BASE` in the clock crate, mapped by step 6c-prime of `kernel/src/arch/riscv64/mmu.rs`
  only when the PMIC plan exists. The vendor U-Boot's board code for `gmac1`'s select was not read.
- The PHY settings. `vendor_phy` looks up the tree's first `ethernet-phy@0`, and the fallback
  is `Settings::FALLBACK_GMAC0`. `gmac1`'s values differ, and both trees agree on them
  (`crates/designware_ethernet/tests/fixtures/jh7110-gmac-{vendor,mainline}.dts`): receive delay
  `0x2` (300 ps), transmit delay 0, transmit clock not inverted at 1000 Mbit/s. `gmac0`'s are
  `0xa`, `0xa` and inverted. On silicon `gmac0` read `Fallback`, so radon's own tree did not yield
  the vendor spelling; a `gmac1` brought up the same way today would get `gmac0`'s delays. The two
  trees also disagree on inverting at 10 Mbit/s, for both ports.
- The PHY address. Mainline's `phy1` is `ethernet-phy@1` with `reg = <0>`, on `gmac1`'s own
  MDIO bus. The vendor node has no `reg`, and `vendor_phy` reads a missing `reg` as 0, which agrees.
  `Discovered::phy_address`'s doc comment says a unit address is the second choice; the code does
  not do that, and for `gmac1` it would be wrong.
- One port, one process. `kernel/src/designware_ethernet.rs` keeps one `REGION` and one
  `LAST_REPORT`. `crates/designware_ethernet/src/process.rs` puts the DMA region at the address
  virtio and `e1000e` use, because "a `net_stack` drives one NIC, never two". The progenitor holds
  one kernel-built `Stack` in slots 29 and 30 (`grant_kernel_network_stack` in
  `kernel/src/user/progenitor_boot.rs`), filled by the first of `e1000e` and this driver.
- The bench. `kernel/src/network_bench.rs` prints one port's account and says `gmac0
  interface` in its syscon line.

## The fork: which port, or both, and how one is chosen

The driver half is the same in every option: a per-port table, both ports discoverable, either one
able to come up. The options differ in what the system does with two.

| | what the boot does | what it costs beyond the driver |
|---|---|---|
| A. The port with a link (recommended) | both PHYs autonegotiate; the first to link within the existing 5 s gets the one `net_stack`, and `gmac0` wins a tie | nothing outside the driver and the bench; one DMA region still serves |
| B. Both at once | two controllers and two DMA regions, under two `net_stack` processes or one stack with two interfaces | a second `Stack` in the progenitor and a socket contract that says which interface a socket uses; neither exists |
| C. Chosen by configuration | a boot argument or the tree's `ethernet0`/`ethernet1` alias names the port | a person must know which jack the cable is in, which is the failure on 2026-10-20 |

**Recommendation: A.** It is what "jack #1 also working" means at a desk: a cable in either jack
gets a lease, with nothing to configure. B is how Linux and the BSDs present two ports (one
interface each, policy in userspace; recalled, not read), and it is the right end state for a
router or a machine on two networks. Its cost is not the driver. It is a contract question no
workload on radon asks yet: which network a socket reaches. That question belongs to its own
milestone with its own ruling, and A's per-port table is what B would build on. Would A still win
at equal cost? Yes. B's extra part is an unsettled contract, not more code, and C fails the case
that prompted this.

If calef rules B, this block's exit criteria 4 and 5 become "both ports lease at once" and the
milestone gains a dependency on a socket-contract decision that is unwritten. If C, criterion 5
becomes "the named port is used, and the other is reported idle".

Reuse: OpenBSD's `if_dwqe_fdt.c` (`dwqe_setup_jh7110`), already adapted for `gmac0`, picks the
port by the controller's address, so its `gmac1` path is the one to adapt (its handling of the
second port's syscon is to be read, not assumed). Linux's `dwmac-starfive.c` and mainline's device tree (GPL) are
read for facts only, as milestone 53 does. No existing code chooses between two NICs by link in
this tree; the policy is a loop over the two PHYs' status registers.

## Exit criteria

1. `crates/designware_ethernet` describes each port as data: its base, its clock and reset plans,
   its syscon window, offset and shift, and its PHY fallback. `gmac0`'s entry is today's constants.
   `gmac1`'s cites mainline at `f47d223627f8` and the fixtures above. No function in the crate
   names `gmac0` in its body.
2. Host tests run against the existing vendor and mainline fixtures. Discovery finds both ports in
   each tree. `gmac1`'s PHY settings read receive `0x2`, transmit 0 and no inversion at 1000 Mbit/s
   from either spelling, and the vendor lookup reads the port's own child node. `gmac1`'s
   interface select changes bits 2 to 4 of its word and nothing else.
3. `crates/jh7110_clock_and_reset` carries `gmac1`'s plan, and a host test shows its reset steps
   land on reset numbers 66 and 67 of the SYS domain and its parent select on clock 105.
4. The kernel maps and brings up both ports' control planes when the tree names both, with a
   report per port. The SYS syscon window is mapped whenever a JH7110 Ethernet port is described,
   not only when the PMIC plan exists.
5. The boot does what calef rules. Under A: the port with a link within 5 s gets `net_stack`,
   `gmac0` if both, and the console names which port was chosen and that the other had no link.
6. `network_bench` prints one block of lines per port (`nic`, `syscon`, `phy`, `link`), the
   syscon line naming the port it read, and runs the coherence probe on the port it hands out.
7. **On radon, with a person moving the cable.** With the cable in `gmac1`'s jack, the bench
   prints a `link` line and a `dhcp : leased` line for `0x16040000`. `verdict
   LEASED-AND-MEASURED` waits on 53's receive stall. With the cable in `gmac0`'s jack, the run
   matches 53's. Both logs are committed under `bench/radon-<date>/`, each recording its port's
   PHY id, I/O voltage and settings source.
8. `notes/designware-ethernet.md` says the cable may go in either jack, its runbook covers both,
   and its BUGS loses "`gmac0` only".

## BUGS

- **The TCP transfer stall on radon's receive side is milestone 53's**, seen on `gmac0`. This
  milestone does not fix it, and criterion 7 accepts a lease without a measured transfer for that
  reason.
- **A cable moved after boot is not followed.** Under A the port is chosen once. Re-choosing on
  link loss needs the interrupt-driven link state that 53's polled driver does not have.
- **`gmac1`'s interface select in the vendor U-Boot is unread.** The offset and shift come from
  mainline alone; the bench's syscon line before and after is the check.
- **Why `gmac0` read `Fallback` on silicon is unexplained.** It matters more here, since a wrong
  fallback for `gmac1` is a link that resolves and frames that do not move. Criterion 7 records the
  source per port. Finding why radon's tree lacks the vendor properties is 53's.

## Index row

radon has two Ethernet jacks and nife drives only `gmac0`'s, so a cable in the other one, `gmac1`
at `0x1604_0000`, gets no network although the firmware links on it. Describe each port as data,
give `gmac1` its SYS-domain clocks, SYS-syscon interface select and its own PHY delays, and have
the boot hand `net_stack` the port that has a link.
