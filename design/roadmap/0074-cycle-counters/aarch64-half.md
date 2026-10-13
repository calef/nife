# Cycle counters: the aarch64 half as built

An appendix to [milestone 74 (cycle counters)](../0074-cycle-counters.md), under §212 (a prose budget).
This appendix holds what the aarch64 lane built on 2026-09-19, the defect it found on the way, and
which path each QEMU configuration took.

## What the aarch64 half built (2026-09-19)

Lane `milestone/74-cycle-counters-aarch64`. The brief was the proposal's first version, and the
shape is the riscv64 half's: start one counter, check it moves, refuse it and say why when it does
not, print it, and add the probe line. Nothing here is on the context-switch path.

- `kernel/src/arch/aarch64/pmu.rs` (module name provisional, matching its two siblings). It runs per
  core, from `timer::init`, gated on `ID_AA64DFR0_EL1.PMUVer` exactly as milestone 228's
  `PMUSERENR_EL0` write is. It writes `PMCCFILTR_EL0` = a provisional `0`, and `PMCR_EL0` =
  `E | C | LC` (assigned, so a firmware-set divide-by-64 cannot survive). Then `PMCNTENSET_EL0` bit
  31, an `isb`, and a 100-tick read across the generic timer.
- The outcome is per core (`NoPmuV3`, `Stuck`, `Running`), which the other two halves are not. The
  brief asked for every core started, and checking each one costs microseconds.
- It also records what firmware left in `PMCCFILTR_EL0` before overwriting it. That is what seL4's
  published TX1 figures counted (the proposal, decision A, question 3).
- One boot line, every build, printed after the secondaries are up:
  `cycles : PMCCNTR_EL0 running on 4 of 4 cores (...), 6 event counters visible, PMCCFILTR_EL0 0x0
  PROVISIONAL (EL0+EL1 counted, EL2 not)`. Then `firmware left PMCCFILTR_EL0 0x0 ...`, then a line
  naming any core that disagrees with the boot core. Both dropped `PROVISIONAL` on 2026-10-07.
- `bench::cycles_per_tick` on aarch64, with its own meaning line carrying `PROVISIONAL` and a
  `CNTFRQ`/100 window. It adds a check that the window did not migrate cores. aarch64's counters are
  per core and zeroed at each core's init, so a migrated window would difference two unrelated
  numbers. `bench/baseline-aarch64.txt` is unchanged.
- The tests stopped carrying values they did not check.
  `a_granted_thread_reads_the_cycle_counter_and_an_ungranted_one_faults` now asserts the EL0 reads
  moved forward wherever the kernel reports the counter running on every online core. It always
  asserts this on x86_64 (the TSC). riscv64 is not asserted, and the test says why.
  `arch::aarch64::pmu` has four tests of its own. One reads the three registers back, so a silently
  ignored enable or a surviving `D` bit fails.

## A defect in milestone 127's EL2 drop, fixed

Milestone 127 (the seL4 machine: a Jetson TX1) owns the EL2 drop this section is about.

`boot.s` wrote `MDCR_EL2 = 0`, and `HPMN` = 0 is a reserved value without FEAT_HPMN0, which the
Cortex-A57 lacks. The new boot line showed it: `0 event counters visible` under `NIFE_EL2=1`, 6
without. `boot.s` now writes `HPMN` = `PMCR_EL0.N` read at EL2, which is Linux's `init_el2`, and
prints 6 both ways. Only event counters were affected, and nothing uses one yet.

## Which path each configuration took (all 2026-09-19, QEMU 11.1.1)

| configuration | outcome | what it proves |
|---|---|---|
| `script/test`, TCG `cortex-a72` | `Running` on 4 of 4, about 32 per tick, in steps of 1000 | the enable took and the plumbing is right; not a cycle |
| `NIFE_EL2=1`, entered at EL2 | `Running` on 4 of 4, 6 event counters (0 before the `HPMN` fix) | the EL2 drop leaves the PMU usable at EL1 on every core |
| `--cpu cortex-a57` (argon's core) | `Running` on 4 of 4 | QEMU's A57 model takes the same path |
| `--cpu cortex-a53` | `Running` on 4 of 4 | likewise |
| `--cpu cortex-a72,pmu=off` | `NoPmuV3` on every core; the grant tests skip | the absent path: nothing is written, nothing faults |
| `--cpu max` | never reached: the kernel refuses the boot on `TGran4` | unrelated pre-existing defect, recorded in `crates/machine_discovery/src/aarch64.rs`'s BUGS |
| `script/bench`, TCG `-icount` | `cycles_per_tick 16.00 (10000226 cycles over 625003 ticks at cntfrq 62500000)` | the instruction count: exactly `script/icount`'s `instructions_per_counter_tick 16`. The round ratio is the tell |
| HVF | not runnable | this QEMU refuses HVF with a GICv2, and the kernel's GIC driver is GICv2-only |

The refusal path (`Stuck`) ran on no configuration. That is the honest reading of the brief's
expectation that QEMU would mostly exercise it. Every QEMU model this tree boots drives the counter
once it is enabled. The "0 and 1000" notes/pmu.md recorded was the counter before anyone started
it, or seL4's reading of it. `Stuck` is exercised by nothing but the code review. The first machine
that can reach it is one whose secure firmware prohibits Non-secure counting.
