---
status: BUILT
raised: 2026-10-12
built: 2026-10-15
promoted_from: the-swish-check-script-splits-by-topic-before-it-crosses-the-ceiling
milestone_dependencies: 847
decision_dependencies: 266, 280
machine_requirements: none
specific_machine: none
needs_person: no
---
# 900. The swish-check script splits by topic before it crosses the ceiling

*(The title and slug are provisional.)*

Milestone 847 (the swish-check harness is split along its seams) gave the script table its own
module.

`xtask/src/swish_check/script.rs` holds the rows `script/swish-check` types. It starts at 1,287
lines. `SWISH_CHECK_SCRIPT` alone was 405 lines on 2026-09-23. It was 1,004 on 2026-10-04 and 1,099
on 2026-10-11. At last week's rate the file reaches the ceiling in about eight weeks. At the 18-day
rate it takes three. The ceiling is §266 (a Rust source file stays under 2,000 lines).

The work is to cut the table into topic files under `script/`. Candidates are pipes and files,
navigation, packages, the network and `std` programs. The leg still types one slice, assembled
from them in the same order, so no transcript changes. §280 (the swish-check harness splits into
submodules, with its script in one) gave the table its own module for this. The split stays
inside `script/`.

Assembling one `&[Line]` from several `const` slices is the only new code. How to do it is this
milestone's choice.

Reuse: not applicable; this moves rows and adds only the assembly.

Done when every file under `script/` is under 1,500 lines. `script/swish-check` must pass on
aarch64, riscv64 and x86_64, typing the same lines in the same order.

## What was built

Built 2026-10-15 (UTC). The table's 1,116 rows moved verbatim into fourteen topic files under
`script/`, each one contiguous run of the table, so the cut cannot reorder anything. §282 (when
a file is split the parent becomes an index) then took the rest of the old parent. `Line` and
its constructors went into `line.rs`; the second boot's table went into `after_reboot.rs` with
its `FROM_BASALT` skip list. Every name below is provisional.

| file | lines | holds |
|---|---:|---|
| `script.rs` | 89 | the index: uses, re-exports, the `mod` list, the keyboard-boot table, the marked assembly |
| `script/line.rs` | 83 | `Line`, `line`, `keys`, `JOB_DID_NOT_RUN`, `interrupted_at_prompt` |
| `script/after_reboot.rs` | 367 | the second boot's table, `HACKBENCH_LINE`, `IOPING_LINE`, `FROM_BASALT` |
| `script/pipes_files.rs` | 123 | pipes, redirection, `wc`, the `mdr` viewer and the doc store |
| `script/clock_configuration.rs` | 53 | `date`, `printenv`, and their `caps` previews |
| `script/navigation.rs` | 32 | Tab completion, `^C` at the prompt |
| `script/process_tools.rs` | 130 | `ps`, `pgrep`, `top`, `free`, `vmstat`, `slabtop`, `uptime` |
| `script/installer.rs` | 149 | the installer, its refusals, generation 1, the unvouched run |
| `script/std_programs.rs` | 119 | `std_echo`, `std_grep`, the `rg` walk |
| `script/packages.rs` | 203 | fetching, versions, bare names |
| `script/owner.rs` | 61 | suspend and resume, vouch, rollback |
| `script/redirection.rs` | 111 | `uuid`, `>`, `<`, `2>`, `time` |
| `script/directories.rs` | 81 | `rm`, `xargs` |
| `script/words.rs` | 79 | quoting, sequencing, the job budget |
| `script/network.rs` | 66 | the echo client, the witness, `caps reboot` |
| `script/interrupts.rs` | 37 | the heeder and the ignorer |
| `script/std_exerciser.rs` | 64 | the progenitor-spawned `std` program |

The assembly is the only new code: `SWISH_CHECK_SCRIPT` is a `LazyLock<Vec<Line>>` that
concatenates the topic slices in table order (`Line` gained `#[derive(Clone)]` for `concat`),
so the leg still types one slice. The fourteen slices in assembly order reproduce the single
table's rows byte for byte, checked against the pre-split file at the lane's base commit, and
`cargo xtask`'s host tests pass unchanged, the script's own among them. `script/swish-check` on
aarch64, riscv64 and x86_64 typing the same lines in the same order is this branch's CI gate.

## Follow-on

- **Recorded.** The new file names are provisional; each module doc under
  `xtask/src/swish_check/script/` says so, for calef's name-ratification pass.

## Index row

The swish-check script table is cut into topic files before it reaches the 2,000-line ceiling,
assembled into the same slice so no transcript changes.
