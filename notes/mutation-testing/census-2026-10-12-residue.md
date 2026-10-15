# The 2026-10-12 census's residue, and the recorded gaps

Lane maintainer/mutation-triage-2026w42-residue, 2026-10-14 (UTC), for fatal risk 3 (the tests do
not test). Name provisional. [inflow-2026-10-12-triage](inflow-2026-10-12-triage.md) holds the
method and triaged both inflow lists. This note takes what those lists never named: every survivor
key of the 2026-10-12 census,
[38252397727](https://github.com/nifeos/nife/actions/runs/38252397727), with no row in
`notes/project-metrics/mutation-triage.csv`. It also tries each `gap` row once.

Every crate was re-run with `script/mutation -p <crate> --re <the functions>`, scoped to the keys,
on the lane's worktree (macOS, base `deff22e9f`), one crate at a time, before the new tests and
again after them. Each row's disposition is what the second run showed. Its reason cites the ledger
whose argument it repeats.

## The residue

Joining the census's survivor list against the triage record on `(crate, function, mutation)` gives
61 keys covering 102 mutants. Every one now has a row.

| crate | keys | mutants | killed | equivalent | gap |
|---|---|---|---|---|---|
| `documentation` | 19 | 28 | 1 | 18 | 0 |
| `machine_discovery` | 12 | 17 | 2 | 8 | 2 |
| `video_terminal` | 10 | 12 | 0 | 10 | 0 |
| `abi` | 8 | 8 | 7 | 1 | 0 |
| `pci` | 5 | 11 | 0 | 5 | 0 |
| `swish` | 3 | 4 | 0 | 3 | 0 |
| `sealed_pair` | 2 | 2 | 2 | 0 | 0 |
| `filesystem_protocol` | 1 | 19 | 0 | 1 | 0 |
| `uefi_loader` | 1 | 1 | 1 | 0 | 0 |
| total | 61 | 102 | 13 | 46 | 2 |

Most of the residue was already argued in prose and had no row. The equivalents in `documentation`,
`video_terminal`, `machine_discovery` and `filesystem_protocol` are the shapes
[part two](census-2026-10-03-triage-part-2.md) names. Those in `swish` and `pci` are
[part four](census-2026-10-03-triage-part-4.md)'s. The re-run still reports each one.

- `abi`'s eight are the old names of `CycleMeaning::is_core_cycles` and `admitted`. The inflow
  triage keyed them under the new names and said the census would still list the old ones. They
  get rows under both, with the same dispositions.
- `sealed_pair::hex_string` (2, killed). Its only test found `hex_string`'s output inside the
  refusal text, which any spelling passes. `a_digest_is_printed_as_shasum_prints_it` pins the hex
  of SHA-256 of `abc`.
- The compile-time table loops in `machine_discovery` (`< with ==` and `< with >`, killed). A
  `const _` block that walks a table can only fail the build, and the build only ever sees the
  good table. So a mutant that stops the walk passes. Each walk is now a `const fn`, which
  `aarch64::codes_are_distinct` and `riscv64::ids_are_sequential` name, and a test feeds it a
  table it must refuse.
- `uefi_loader`'s `with_initrd` is `with_initial_ramdisk`. Under the new name it is caught.
- `documentation`'s `flush_table` `> with >=` (killed). Its survivor was `vis > *width`, which a
  split of `flush_table` moved into `table_widths`. The key now names `c > 0`, which is caught.
  `table_widths`'s new key gets the old equivalence.

## The recorded gaps

16 rows were `gap`. 9 are now killed and 7 stay.

| crate | keys | killed | still a gap |
|---|---|---|---|
| `board_console` | 3 | 0 | 3 |
| `uefi_loader` | 2 | 2 | 0 |
| `filesystem_protocol` | 2 | 2 | 0 |
| `address_space_identifier` | 2 | 0 | 2 |
| `designware_ethernet` | 2 | 2 | 0 |
| `walk_pricing` | 1 | 1 | 0 |
| `documentation` | 1 | 1 | 0 |
| `job_mix` | 1 | 1 | 0 |
| `network_time_protocol` | 1 | 0 | 1 |
| `direct_memory_access_validator` | 1 | 0 | 1 |
| total | 16 | 9 | 7 |

- `uefi_loader::CMDLINE_LEN` (11 mutants, two keys). The row called it deliberate slack. There is
  none: the only room is the screen token's own rounded-up `MAX_LEN` and the NUL. The longest-line
  test now asserts exactly that sum, so changing any `+` breaks it.
- `filesystem_protocol`'s `verb::TABLE` order walk. It is the `const` shape above, now
  `in_opcode_order`, and a test refuses two rows swapped.
- `designware_ethernet::coherence::prepare`. The read-back buffer had its own length expression,
  which `sim::Memory`'s write-allocate cache hid. It is now a copy of the poison array, so the two
  lengths cannot differ, and that mutant no longer exists.
- `walk_pricing::price`. Both medians take `middle()`, which a test checks on values a test
  controls.
- `documentation`'s `take_table_row` `-= with /=` was a hang in the trailing-blank trim loop. The
  trim is now an `rposition`, so there is no step to lose.
- `job_mix::order`'s `seed | 1`. `order(1)` is the order the kernel's first task walks in every
  recorded run, and it is now pinned. `seed ^ 1` starts the generator at 0 and fails it.

## Also corrected

`filesystem_protocol`'s `- with +` and `- with /` rows said `killed`. The re-run reported both, in
`fixture::walk::RG_MATCHES`, which no test pinned. It is now 138, one line per small file. The rows
say so.

## BUGS

- Seven gaps stay. `board_console`'s three need a real tty and a host `/dev`.
  `address_space_identifier`'s two are a release-only guard behind two `debug_assert!`s of the same
  condition, which `notes/mutation-testing.md` calls equivalent under the harness. They stay `gap`,
  because that is the weaker claim. `network_time_protocol`'s `PROVED_NANOS` and
  `direct_memory_access_validator`'s `ChainMem::racing` sit in an inline `#[cfg(kani)] mod
  verification`, which the `verification::` exclusion cannot reach. `machine_discovery`'s two new
  gaps are `x86_64`'s `N`, the same shape.
- What would close those four: the module in its own file, which `.cargo/mutants.toml`'s
  `**/src/verification.rs` glob already excludes. §282 (when a file is split, the parent becomes an
  index) blocks that move today. `helpers/parent_index.py` exempts a `tests` or `proofs` child and
  not a `verification` one. Renaming the modules to `proofs` would rename every harness its
  falsification patches and `notes/confinement-claims.md` cite. Exempting `verification` in the
  check is the smaller change. It is a change to a gate, so it is proposed here rather than made.
- The 547 rows marked `killed` whose keys the 2026-10-12 census still lists were not re-run. Most
  are the inflow triage's kills, written after that census. Run
  [38654094250](https://github.com/nifeos/nife/actions/runs/38654094250) will show whether any of
  them still survives.
