# The `uefi-boot` two-core rate at `-smp 2`

*(Written by the lane executing milestone 412 (`script/test`'s UEFI leg asserts two cores online,
and the second one does not always start), 2026-10-16 (14:23 to 14:45 UTC). Name provisional: the
lane mints it, the name-ratification queue owns it. The block this serves is
[design/roadmap/0412-the-uefi-boot-gate-asserts-two-cores-that-do-not-always-start.md](../design/roadmap/0412-the-uefi-boot-gate-asserts-two-cores-that-do-not-always-start.md);
the measurement recipe it followed is notes/load-sensitive-assertions.md's, and the gradient
precedent is [the `uefi-test` exit rate](uefi-test-exit-rate.md).)*

A lane landing milestone 294 (the index is generated) saw `smp: cpu 1 did not start (firmware
returned -1)` once in three `script/test` runs on 2026-09-14, and filed the proposal that became
this block. The UEFI tour's gate asserts `smp: 2 core(s) online` unconditionally, so that sighting
was a red gate nobody had caused. The tree has since root-caused
the defect under that line. `cpu_start` re-read the online count after the STARTUP IPIs, so a core
that checked in during the settle delay was counted absent. The count is read once now, fixed
2026-09-19, measured 40 of 40 at `-smp 4` after. The fix's own campaign saw the first secondary
fail too, so the old "third or later" bound in `arch::x86_64::ap_boot`'s BUGS was never a
boundary. The fix commit names the UEFI gate's one-in-three as this defect reaching two cores.
What the block still owed was the dedicated `cargo xtask uefi-boot` rate on the tree as it now is.
This page is that measurement, and the gate question it decides.

## The measurement

Forty-two counted `cargo xtask uefi-boot` boots on patagonia, 8 cores, Homebrew QEMU 11.1.2 with
its edk2 OVMF, sequential, one warm-up boot first (not counted; it carried the cold cargo caches
and passed). Every boot ran at the gate's own setting, `NIFE_SMP=2`, bounded by the runner's 90 s
limit and stopped by the screen watch once the shell had answered. Each row records the 1/5/15
minute load average before and after the run, the wall-clock seconds, and the outcome. Window:
14:24 to 14:45 UTC, 2026-10-16, in two batches with a seven minute gap between them while the
machine's load was identified.

**Zero failures.** Every one of the 42 boots printed `smp: 2 core(s) in the madt`, the
`init-sipi-sipi` trampoline line, `smp: 2 core(s) online`, and the gate's success line. Not one
printed `did not start`; every exit was 0. Runs took 12 to 24 s. By the rule of three, zero in 42
bounds the rate below 1 in 14 at 95% confidence.

**The machine was never idle, and that is a recorded condition, not a cleanup.** The 1-minute load
average ran 6.63 to 23.76 across the window and never reached a quiet band. Three sources were
visible. Patagonia's own desktop load was one. A sibling lane gated an aarch64 suite in the
worktree beside this one, and its QEMU appeared mid-window; lanes cannot see each other. A macOS
Spotlight indexing storm (a dozen `mdworker` processes) covered the last twenty minutes. The load
recipe's one-spinner-per-core batch was deliberately not run: the tree's rule is never to induce
load while other lanes are gating here. So this window is a load-gradient batch, and its top leg
is the storm's load rather than an induced one.

| run | start (UTC) | load before | s | load after | outcome |
|---|---|---|---|---|---|
| 001 | 14:24:25Z | 7.71 7.40 7.21 | 14 | 7.14 7.29 7.17 | pass |
| 002 | 14:24:39Z | 7.14 7.29 7.17 | 14 | 8.47 7.58 7.28 | pass |
| 003 | 14:24:53Z | 8.47 7.58 7.28 | 13 | 7.19 7.34 7.20 | pass |
| 004 | 14:25:06Z | 7.19 7.34 7.20 | 12 | 6.63 7.22 7.16 | pass |
| 005 | 14:25:18Z | 6.63 7.22 7.16 | 14 | 7.57 7.40 7.22 | pass |
| 006 | 14:25:32Z | 7.57 7.40 7.22 | 13 | 7.25 7.33 7.20 | pass |
| 007 | 14:25:45Z | 7.25 7.33 7.20 | 13 | 6.76 7.22 7.16 | pass |
| 008 | 14:25:58Z | 6.76 7.22 7.16 | 21 | 10.28 7.93 7.42 | pass |
| 009 | 14:26:19Z | 10.28 7.93 7.42 | 13 | 10.46 8.11 7.49 | pass |
| 010 | 14:26:32Z | 10.46 8.11 7.49 | 13 | 9.47 7.97 7.45 | pass |
| 011 | 14:26:45Z | 9.47 7.97 7.45 | 13 | 8.25 7.78 7.39 | pass |
| 012 | 14:26:58Z | 8.25 7.78 7.39 | 12 | 7.48 7.63 7.34 | pass |
| 013 | 14:27:10Z | 7.48 7.63 7.34 | 14 | 7.43 7.61 7.33 | pass |
| 014 | 14:27:24Z | 7.43 7.61 7.33 | 12 | 6.97 7.49 7.30 | pass |
| 015 | 14:27:36Z | 6.97 7.49 7.30 | 19 | 11.38 8.52 7.67 | pass |
| 016 | 14:27:55Z | 11.38 8.52 7.67 | 17 | 12.54 8.94 7.84 | pass |
| 017 | 14:28:12Z | 12.54 8.94 7.84 | 12 | 11.07 8.75 7.78 | pass |
| 018 | 14:28:25Z | 11.07 8.75 7.78 | 13 | 10.13 8.66 7.77 | pass |
| 019 | 14:28:38Z | 10.13 8.66 7.77 | 13 | 9.45 8.58 7.75 | pass |
| 020 | 14:28:51Z | 9.45 8.58 7.75 | 13 | 8.76 8.46 7.72 | pass |
| 021 | 14:29:04Z | 8.76 8.46 7.72 | 13 | 8.62 8.44 7.72 | pass |
| 022 | 14:29:17Z | 8.62 8.44 7.72 | 14 | 8.12 8.33 7.70 | pass |
| 023 | 14:29:31Z | 8.12 8.33 7.70 | 13 | 7.92 8.29 7.69 | pass |
| 024 | 14:29:44Z | 7.92 8.29 7.69 | 24 | 9.08 8.48 7.77 | pass |
| 025 | 14:30:08Z | 9.08 8.48 7.77 | 18 | 12.19 9.23 8.06 | pass |
| 026 | 14:30:26Z | 12.19 9.23 8.06 | 17 | 12.29 9.40 8.14 | pass |
| 027 | 14:30:43Z | 12.29 9.40 8.14 | 15 | 12.62 9.59 8.23 | pass |
| 028 | 14:30:58Z | 12.62 9.59 8.23 | 19 | 17.79 10.99 8.76 | pass |
| 029 | 14:31:17Z | 17.79 10.99 8.76 | 18 | 21.25 12.22 9.25 | pass |
| 030 | 14:31:35Z | 21.25 12.22 9.25 | 19 | 23.26 13.09 9.62 | pass |
| 031 | 14:42:01Z | 19.64 18.31 14.34 | 21 | 23.76 19.34 14.80 | pass |
| 032 | 14:42:22Z | 23.76 19.34 14.80 | 13 | 22.53 19.29 14.86 | pass |
| 033 | 14:42:35Z | 22.53 19.29 14.86 | 16 | 21.70 19.28 14.94 | pass |
| 034 | 14:42:51Z | 21.70 19.28 14.94 | 14 | 20.78 19.20 14.98 | pass |
| 035 | 14:43:05Z | 20.78 19.20 14.98 | 15 | 21.39 19.37 15.09 | pass |
| 036 | 14:43:20Z | 21.39 19.37 15.09 | 15 | 19.96 19.16 15.09 | pass |
| 037 | 14:43:35Z | 19.96 19.16 15.09 | 13 | 18.50 18.87 15.06 | pass |
| 038 | 14:43:48Z | 18.50 18.87 15.06 | 13 | 20.33 19.27 15.27 | pass |
| 039 | 14:44:01Z | 20.33 19.27 15.27 | 13 | 20.45 19.34 15.34 | pass |
| 040 | 14:44:14Z | 20.45 19.34 15.34 | 12 | 18.95 19.07 15.31 | pass |
| 041 | 14:44:26Z | 18.95 19.07 15.31 | 17 | 21.75 19.73 15.62 | pass |
| 042 | 14:44:43Z | 21.75 19.73 15.62 | 15 | 23.55 20.20 15.85 | pass |

Load values are the 1/5/15 minute averages `sysctl -n vm.loadavg` reports. The storm's onset is
visible mid-table: runs 024 and 028 to 030 rode it before the second batch named it. Duration
moved 12 to 24 s across the whole gradient, so the 90 s bound and the 180 s screen-watch deadline
were nowhere in sight.

## The verdict

The rate on today's tree is zero in 42, across a load band that includes the regime the standard
load recipe targets (about 22 on 8 cores). The block's either/or resolves to the first horn: the
"third or later" bound was the wrong record, the one-in-three sighting was the count re-read
reaching two cores, and there is no second failure mode in this sample. One in three and zero in
42 sit together once the dates are read: the sighting is 2026-09-14, the fix is 2026-09-19, and
every boot here is five weeks past both.

**The gate question never opened.** The block's step 3 weighed weakening or retrying the
assertion, and priced a gate that fails one run in three as worse than either. With the defect
fixed and the rate at zero at this sample size, the assertion stays exactly as it is. Two cores
online is what proves the trampoline page the loader asks for by name was usable under firmware.
The measurement is the evidence that the assertion costs no flakes. No architect ask was filed,
because there was nothing left to decide.

## What this lane corrected

The fix removed the defect but left four records reasoning from the old bound, and this lane's
commit corrects each in place:

- `xtask/src/uefi.rs`, the `NIFE_SMP=2` comment: it still said the tour does not touch a two-core
  defect that "fails one of its tests about half the time", citing `ap_boot`'s BUGS #3, a section
  the comment-cap cleanup deleted on 2026-10-13. It now states the split as it is: the tour is
  where the second core is gated, and the suite leg's one-core default is unmeasured-since, not a
  dodge.
- `xtask/src/uefi.rs`, the `uefi-test` leg's comment: it still said
  `every_secondary_runs_scheduled_work` fails about half the time at two cores and that the PVH
  runner defaults to one because of it. Milestone 316 (which core booted) fixed that defect, 19 of
  19 across its two-core campaign, and the PVH runner has defaulted to two since 2026-09-23.
- `xtask/src/swish_check/boot.rs`: it still gave `ap_boot`'s BUGS #3 as the reason its OVMF leg
  boots one core.
- `helpers/qemu-runner-x86_64.sh`: it still pointed at `ap_boot`'s BUGS section as the
  authoritative account of what is open at three cores and above. The pointer now names the module
  doc that carries it (three or more cores unmeasured on silicon; xenon not yet asked).

## BUGS

- One host, one QEMU (Homebrew 11.1.2), one afternoon, and no true-idle band: the window's minimum
  1-minute load was 6.63. The induced-load leg was refused on the tree's own rule, so the loaded
  half of this gradient is ambient and storm load, not a controlled recipe. A quiet-machine batch
  would tighten what this page can say.
- Zero in 42 bounds the rate below 1 in 14 at 95% confidence, and no tighter. A rate of 1 in 50
  would not have been seen here. The assertion's real protection is that the defect beneath it was
  root-caused and removed, not this sampling.
- The rate is about the tour boot only. The suite leg (`uefi-test`) still boots one core under
  OVMF, and the whole suite at two cores there is unmeasured since the fixes. That is the residual
  the corrected comments carry, and it wants a small lane of its own: flip the leg to two, measure,
  and keep whichever default the measurement earns.

## See also

- [design/roadmap/0412-the-uefi-boot-gate-asserts-two-cores-that-do-not-always-start.md](../design/roadmap/0412-the-uefi-boot-gate-asserts-two-cores-that-do-not-always-start.md):
  the block this page serves
- [notes/load-sensitive-assertions.md](load-sensitive-assertions.md): the measurement recipe, the
  host-load line, and the register of the assertion family
- [notes/uefi-test-exit-rate.md](uefi-test-exit-rate.md): the sibling measurement, and the
  load-gradient precedent this page followed
- [design/roadmap/0316-x86-smp-two-cores.md](../design/roadmap/0316-x86-smp-two-cores.md): the
  boot-core-identity fix, and its 26 two-core boots with zero missing cores
