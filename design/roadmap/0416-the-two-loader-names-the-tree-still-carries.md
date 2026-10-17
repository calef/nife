---
status: PARTIAL
raised: 2026-09-15
promoted_from: the-two-loader-names-the-tree-still-carries
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 416. Two loader names the tree still carries, and neither one exists

Promoted from the proposal `the-two-loader-names-the-tree-still-carries`. `boot_progenitor` is
ratified (calef, 2026-09-15), so nothing here waits on a naming decision.

Two function names survive in prose and code comments after the functions themselves are gone.
`riscv_shell_boot` has one successor: `boot_progenitor`, which milestone 166 (unifying the
per-architecture progenitor loaders) made the one boot loader. `spawn_init` has two, and the site
decides. The boot endowment it describes is `boot_progenitor`'s. The test-role endowment (an
untyped in slot 0, a report endpoint in slot 1, the archive read-only at `INITIAL_RAMDISK_VA`) is
`spawn_hello`'s. Neither `fn` exists anywhere in the tree.

## The rules the sweep applies

Mechanical in the edits, and the judgment is the whole job:

- Enumerate first, then apply through an assertion-checked list that aborts when a line is not what
  was enumerated. Milestone 166's lane caught an occurrence wrapping across two lines this way,
  which a tree-wide replace would have silently mangled.
- A `BUILT` roadmap block is an account of what happened and keeps the old name where it narrates
  history; it moves only where it makes a present-tense claim about the system as it is now.
- A measurement table keeps the name it was measured under. `notes/frames.md` and `notes/net.md`
  count runs made under the old names, and sweeping them would make a true measurement cite a
  function that never produced it.
- **A quotation never moves**, whoever said it.
- Where an old name is kept deliberately, say so in a clause ("then `riscv_shell_boot`, renamed
  `boot_progenitor` at milestone 166") so a reader does not grep for a vanished name.

## What the lane's phase did

Swept 2026-10-15 against 8fad33799f, the enumeration applied through the assertion-checked list:
12 sites rewritten across `kernel/`, `crates/`, `system_tests/`, `script/` and `notes/`. Re-counted
at execution: 41 occurrences of `riscv_shell_boot` across 21 files and 27 of `spawn_init` across
18, down from the 48 and 32 the filing counted, because lanes landing in between had already taken
some. The judgments a reader should know:

- Boot sites name `boot_progenitor`: the console hand-off (`kernel/src/console.rs`), the ramdisk
  mapping contract (`crates/user_mode_runtime/src/initial_ramdisk.rs`), and the trusted-init and
  line-discipline notes.
- Test-endowment sites name `spawn_hello`, which grants exactly that endowment today: the five
  `system_tests` spawn doc comments, and the x86_64 IRQ note about `INIT_TEST_SGI`, whose enabler
  is `spawn_hello`, not the boot.
- `kernel/src/cap.rs`'s `DeviceFrame` doc needed a rewrite, not a rename: "a riscv build without
  `--features shell` never mints one" stopped being true when milestone 268 (every architecture
  boots the same way) routed the default riscv boot through `boot_progenitor`. It now names
  today's constructors.
- Two deliberate keeps, each with successor and milestone named: `script/lint`'s parity note (it
  explains why the gate covers both ISAs) and `boot_progenitor`'s own provenance clause.

## The design/ citations, which are a maintainer's

A developer lane does not edit `design/` outside its own block, and 0266's BUGS assigned these
citations to a maintainer, who applied them 2026-10-16 through an assertion-checked list against
98bea98a7. Re-counted there: 43 lines in `design/` carried the two names before the sweep (27 lines
of `riscv_shell_boot`, 16 of `spawn_init`, some on both), 36 of them outside this block. 18 lines
were edited (15 moved or rewritten, 3 kept with a successor clause: 0177 lines 49, 72 and 74), and the
rest keep, as judged:

| file | in force |
|---|---|
| `decisions/0120` | open: calef's call whether the entropy refusal keeps its day's names or gains the successor in a clause |
| `roadmap/0049/bugs-found.md`, `0166`, `0301`, `0266` line 130, `0295` line 42 | keep: past-tense accounts, the rename's own account, and a proposal as filed |
| `roadmap/0161`/`processes-and-userspace.md` | names `spawn_hello`, the slot-2 `UART_PHYS` grant |
| `roadmap/0167` | names `boot_progenitor` and `spawn_hello` |
| `roadmap/0177` | the reachable-boot lines name `boot_progenitor`; the slot-count lines keep as old-layout measurements, with a clause naming the successor and the table's 32 slots today |
| `roadmap/0182` | drops the dead half |
| `roadmap/0267`/`evidence.md`, `0268`/`second-pass.md`, `0296`, `0295` lines 155 and 160 | name `boot_progenitor` |
| `roadmap/0289` | names `boot_progenitor`; line 27's `#[cfg(feature = "shell")]` claim now says it held when the block was written and that milestone 268 made the default boot reach it |
| `roadmap/0406` | names `boot_progenitor`; the shell build hands over through `riscv_hand_over` and the boot thread exits the scheduler before the tour, so "halts" was wrong too |

Reuse: none exists (a prose sweep of two dead names; nothing to take or adapt).

## Follow-on

- **Outstanding.** `decisions/0120`'s two citations, held for calef's ruling (needs-architect on the
  pull request that applied the rest). Outside `design/` and the generated census, the two names
  leave exactly the two deliberate keeps (`script/lint`'s parity note and `boot_progenitor`'s
  provenance clause), each carrying its successor and milestone.

## BUGS

- `notes/dependency-census/dependencies.tsv` is generated and quotes block text at its census
  dates; sweeping it by hand would forge the snapshots. Its next regeneration reads whatever its
  sources say that day.
- `kernel/src/cap.rs`'s `#[cfg_attr(all(target_arch = "riscv64", not(feature = "shell")),
  allow(dead_code))]` on `DeviceFrame` reaches a wider set of builds than the fact it was written
  for. The default riscv boot mints one since milestone 268, and what still mints none is the
  workload builds (bench, soak, job mix and kin) that never reach a hand-off. Narrowing it wants
  its own gate-proven change, so this prose sweep left it.

## Index row

Two function names survived in prose and code comments after the functions themselves were gone.
`riscv_shell_boot`'s one successor is `boot_progenitor` (milestone 166 unified the boot loaders).
`spawn_init`'s sites split between `boot_progenitor` and `spawn_hello` by which endowment they
describe. Swept everywhere, `design/` included, with past-tense accounts and quotations keeping the
old name beside a clause naming the successor. Only `decisions/0120`'s two citations wait, on
calef.
