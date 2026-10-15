---
status: BUILT
raised: 2026-10-11
built: 2026-10-12
milestone_dependencies: none
decision_dependencies: 266, 282
machine_requirements: none
specific_machine: none
needs_person: no
---
# 881. The kernel benchmark file is split along its seams

*(The title and slug are provisional.)*

`kernel/src/bench.rs` is 2,110 lines at `396187b0b`. §266 (a
Rust source file stays under 2,000 lines) sets the ceiling. Milestone 840 (the scheduler file is
split along its seams) is the model. This file is compiled only under `--features bench`, and its
one entry point, `bench::run`, never returns.

The cut and every module name are calef's call. He ruled the shape on 2026-10-11 (UTC) in §282
(when a file is split, the parent becomes an index) while answering milestone 872 (the user-mode
file is split along its seams). The merge of #1988 applied it here the next day; every module name
ships provisional.

Reuse: not applicable; this moves code and adds none. The kernel already holds two benchmark
siblings under their own features, `network_bench.rs` and `storage_bench.rs`.

## What is in the file

Measured on `396187b0b` from the file's own top-level items. Ranges are approximate at the edges,
where a doc comment belongs to the item below it.

| lines | count | what |
|---|---|---|
| 1 to 63 | 63 | module docs, iteration counts, `WARMUP`, `timed` |
| 64 to 129 | 66 | `run`: every row, in a fixed order |
| 130 to 280 | 151 | `yield_switch` and the three x86_64 TSS port-map rows |
| 281 to 504 | 224 | kernel-thread IPC rows: `ipc_rtt`, `relay_rtt`, `call_reply`, `broker_rtt`, `spawn_reap` |
| 505 to 652 | 148 | `map_new` |
| 653 to 786 | 134 | `cycles_per_tick`, its per-architecture probes, `rfence_self` |
| 787 to 924 | 138 | EL0 roles, `null_syscall_el0`, `ctx_switch_el0`, `ipc_rtt_el0` |
| 925 to 1060 | 136 | milestone 134's tier A set-up and E1, `ipc_thread_scaling` |
| 1061 to 1297 | 237 | E4, `app_displacement`, and its `Racy` buffer |
| 1298 to 1451 | 154 | `sink_throughput`, `map_el0`, `spawn_el0` |
| 1452 to 1497 | 46 | `coremark_compute` |
| 1498 to 1867 | 370 | `fs_read`, `fs_throughput`, `fs_walk`, `fs_walk_bound`, `rg_search` and their readers |
| 1868 to 2110 | 243 | multi-hart throughput: `tp_*`, `tc_*`, `busy`, `smp_throughput` |

E1 and E4 are the names milestone 134 (the register of measures) gave two experiments.

The file holds no test, no Kani harness and no `cfg(feature = "bench")` of its own. Its one gate is
at `kernel/src/lib.rs:48`, on the `mod bench;` line, and every submodule inherits it. The other 76
`feature = "bench"` sites across 26 kernel files are boot-path call sites and no-op twins, and none
of them names anything inside the module but `bench::run`.

Inside, the `cfg` variants are per architecture. `CYCLE_PROBE_MEANING`, `cycle_probe_window_ticks`
and `cycle_probe_delta` each come in two or three, and so does `TCG_VIRT_CNTFRQ_HZ`. Each set moves
as one. Two feature probes, `fastpath_pad` and `ipc_stack_depth`, sit inside `run` and stay there.

## The verdict: split by family, as child modules

The tests-only axis does not apply: there are no tests. The question is whether to split at all,
and how far.

This is not one type that should stay whole. It is a list of free functions, one per row, joined
by `run`'s order and a few shared helpers: `timed`, `WARMUP`, `real_single_hart_or_skip` and
`busy`. Each family has its own constants and its own helpers. That is a seam.

The smallest move would take one family out. Moving the multi-hart set alone leaves about 1,500
lines. But the file grew 262 lines in 17 days, and it would cross 2,000 again. A split by family
gives each new row an obvious home, and it leaves `bench.rs` as the order of the run, which is what
a reader opens it for.

Child modules, not siblings like `network_bench`. A child reaches `timed` and `WARMUP` as they are,
private. As siblings they would become `pub(crate)`.

## A proposed cut

Every name here is provisional. The modules are private to `bench`.

| module, provisional | from the table | about |
|---|---|---|
| `bench` (`bench.rs`) | 1 to 129, `real_single_hart_or_skip` | 180 |
| `bench::kernel_thread` | 130 to 652 | 520 |
| `bench::cycles` | 653 to 786, 1452 to 1497 | 180 |
| `bench::user_mode` | 787 to 924, 1298 to 1451 | 290 |
| `bench::scaling` | 925 to 1297, 1868 to 2110 | 610 |
| `bench::filesystem` | 1498 to 1867 | 370 |

`bench::scaling` holds E1 and E4 with the multi-hart rows because E1 reuses the multi-hart batch,
`tp_batch` and `tp_best`, and E4 uses its `busy`. The parent ends near 180 lines, and no new file
is near 1,500.

## What an architect had to rule

Answered 2026-10-12 (UTC) by the merge of #1988 under §282. The three answers: 1 the split, by
family, taken finer than this block proposed, twelve children each named for the family it holds.
2 child modules of `bench`, the parent an index, as recommended. 3 the names in the what-was-built
table ship provisional. Ruled 2026-10-15 (UTC): the cut stands, no veto, and every module name
goes to the naming queue (`script/names --unratified`) with every other provisional name, to
review and ratify there. The questions stay here as they were asked.

1. Split by family, or move one family out. The recommendation is by family, for the reason above.
   The one-family move is less work, so this recommendation is not about effort; it holds at equal
   cost.
2. Child modules of `bench`, or siblings like `network_bench`. The recommendation is children,
   because nothing widens. That holds at equal cost.
3. Every module name in the table.

No public crate API, wire format or syscall changes under any answer. `bench::run` keeps its path.

## What moving the code breaks

Found with `grep` at `396187b0b`.

- Row names are string literals. `xtask/src/bench.rs` parses `bench: <name>` and `bench-probe:`
  lines, and `bench/baseline-*.txt` is keyed on row names, so neither keys on the path. `run`'s
  order must not change, because the icount numbers depend on what ran first.
- Codegen units are partitioned by module, so a split can change inlining. That is from memory, not
  read. The module's own docs say icount numbers shift across binaries for unrelated changes. Run
  `script/bench --check` before and after; a shift past the 10% tripwire is re-saved with `--why`.
- `script/fastpath-footprint` matches mangled names under `kernel5sched` and `kernel7syscall`, not
  under `bench`. No script or helper keys on a `bench` symbol. Its numbers should not move.
- No falsification patch diffs this file. `bench/radon-2026-10-05/slot64.patch` names it in a
  comment and patches a fixture.
- Two citations of the form `bench.rs:NNNN` remain, in milestone 101 (the L4 calibration, read from
  the IPC number that pays for the trap) and milestone 811 (the boot services leave the kernel).
  Both have already drifted from the line they meant. 46 tracked files name the path in all.
- Comments in the three `pmu.rs` files and in `smp.rs` name `bench::cycles_per_tick` and
  `bench::real_single_hart_or_skip`. The first gets a new path under this cut.
- `design/file-length-baseline.tsv` lists the file at 2,110; that row goes in the same change.
  `design/comment-block-baseline.tsv` keys two rows on this path, and both blocks move.
  `bench.rs:589` (64 lines, the doc of `cycles_per_tick`) goes with its function into
  `bench/cycles.rs`. `bench.rs:1868` (42 lines, the plain comment opening the multi-hart
  throughput section) lands in `bench/scaling.rs`. The ratchet admits no row on a path the merge
  base gave none, and a block in a new file meets the 40-line cap outright. So it fails both
  unless each is cut to 40 lines in the same change. The alternative is a fix to the ratchet,
  letting a row move with its block, ruled separately.
- `notes/unsafe-obligations.md` counts `Racy`'s `unsafe impl Sync` by kind, not by file, so moving
  it changes no count. The note names `kernel/src/bench.rs`.
- `script/icount`'s comment says `feature = "icount"` appears in `bench.rs`. At `396187b0b` it does
  not; the comment is stale already.
- 6 first-parent merges in the 14 days to 2026-10-11 touched it.
- Pull request #1892 was in flight at `396187b0b` and does not touch this file at its head
  `43d8c5d`; if it changes the file before it merges, re-measure every range here.

## Done when

1. `kernel/src/bench.rs` is under 2,000 lines, and no new file is over 1,500.
2. No Rust file outside `kernel/src/bench.rs` and its new children changes, except the comments
   that name a moved function.
3. `script/bench` on aarch64, riscv64 and x86_64 prints the same rows in the same order, and
   `--check` passes, or each re-saved floor carries a `--why`.
4. `script/fastpath-footprint` is unchanged on each architecture.
5. Each new module carries a `//! Name:` block marked provisional.

## What was built

Built 2026-10-12 (UTC) on `f38ed4aa2`, in pull request #1988, by lane
`maintainer/last-three-over-2000` under §282, in the same sweep that took 879's file, before an
881 lane was cut. `kernel/src/bench.rs` went from 2,110 lines to 75. Every name below is
provisional.

| file | lines | holds |
|---|---|---|
| `bench.rs` | 75 | the index: module docs, every `mod` line, the re-imports |
| `bench/timing.rs` | 31 | the iteration counts, `WARMUP`, `timed` |
| `bench/sequence.rs` | 75 | `run`, every row in the fixed order a baseline is read in |
| `bench/switch.rs` | 161 | `yield_switch` and the three x86_64 TSS port-map rows |
| `bench/ipc.rs` | 214 | the kernel-thread IPC rows: `ipc_rtt`, `relay_rtt`, `call_reply`, `broker_rtt` |
| `bench/spawn_map.rs` | 152 | `spawn_reap`, `map_new`, `rfence_self` |
| `bench/cycle_probe.rs` | 163 | `cycles_per_tick`, the per-architecture probes, `TCG_VIRT_CNTFRQ_HZ` |
| `bench/el0.rs` | 317 | the EL0 roles, `null_syscall_el0`, `ctx_switch_el0`, `ipc_rtt_el0`, `sink_throughput`, `map_el0`, `spawn_el0` |
| `bench/compute.rs` | 25 | `coremark_compute` |
| `bench/thread_scaling.rs` | 144 | milestone 134's tier A set-up and E1's `ipc_thread_scaling`, `real_single_hart_or_skip` |
| `bench/app_displacement.rs` | 238 | E4, `app_displacement`, and its `Racy` buffer |
| `bench/filesystem.rs` | 419 | `fs_read`, `fs_throughput`, `fs_walk`, `fs_walk_bound`, `rg_search` and their readers |
| `bench/smp_throughput.rs` | 250 | `tp_batch`, `tp_best`, `busy`, the `tc_*` pair, `smp_throughput` |

The done-when list, checked on `d5d232015`:

1. The parent is 75 lines and the largest child (`filesystem.rs`) is 419. The file-length row is
   gone from `design/file-length-baseline.tsv`, removed in the split; the baseline holds no rows
   at all now.
2. Code moved only: #1988's line-multiset check against the old file found nothing changed beyond
   `pub(super)`, the module headers and the index lines. `bench::run` keeps its path. The only
   Rust files outside `bench` the split touched are two doc-pointer files the exception covers
   (`crates/job_mix/src/lib.rs`, `fixtures/src/job_mix_task.rs`). The lane that recorded this
   block re-pointed the comments the section above names, in the three `pmu.rs` files and
   `smp.rs`, under the same exception.
3. `script/bench --check` passed on aarch64, riscv64 and x86_64 in #1988, within the 10%
   tripwire, and the nightly floor restamps since carry no row change. The rows are string
   literals that moved whole, and the order lives in `bench/sequence.rs`.
4. `script/fastpath-footprint` is unchanged on aarch64 and x86_64. On riscv64 the floor was
   re-recorded in #1988 with the reason, on the fs_service leg's codegen-unit partition and not
   this file's, by milestone 872's precedent; `bench/fastpath-riscv64.txt` carries the why.
5. All twelve new modules carry a `//! Name:` block marked provisional.

## Follow-on

- **Recorded.** The two `bench.rs:NNNN` citations this block found already drifted stand in other
  milestones' blocks, which this lane may not edit. 0101 cites `ipc_rtt_el0` by line in
  `bench.rs`; the row lives in `kernel/src/bench/el0.rs` now. 0811 cites `fs_service` by line in
  `bench.rs`, the file rows; they live in `kernel/src/bench/filesystem.rs` now. The milestone 74
  (cycle counters) row in `notes/dependency-census/dependencies.tsv` quotes
  `bench::cycles_per_tick` inside a dated census, and the register of measures' two M5 rows name
  it the old way. Whether a dated record gets repointed is a maintainer's call, the same one
  292's census needs. Listed on pull request #2204 for a maintainer lane.
- **Decision.** Ruled 2026-10-15 (UTC): the cut stands, and the names go to the naming queue with
  every other provisional name, per
  `design/decisions/0282-when-a-file-is-split-the-parent-becomes-an-index.md`.

## Index row

`kernel/src/bench.rs` held every benchmark row in one 2,110-line file, a list of free functions
joined only by the order `run` called them. It is now a 75-line index of twelve child modules
under `kernel/src/bench/`, each named for the family it holds, as §282 rules. Every module name is
provisional.
