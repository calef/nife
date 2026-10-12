---
status: PROPOSED
raised: 2026-10-12
---

# 284. The kernel crate root splits into child modules, one per boot

Raised 2026-10-12 (UTC) as the cut that
[milestone 876 (the kernel crate root is split along its seams)](../roadmap/0876-the-kernel-crate-root-is-split-along-its-seams.md)
says is calef's call before a line moves. Written by lane
`milestone/876-the-kernel-crate-root-is-split-along-its-seams`. *(Section number provisional until
the merge queue lands it.)*

One question is open, below. The pull request is built on its recommendation, so a yes changes
nothing and a no moves two files. Every module name is provisional and left to calef's naming pass.

## What is already decided, and so not asked

- **`kernel_main` is split by boot phase**, one function per phase. calef ruled that on 2026-10-09
  (UTC). On 2026-10-12 (UTC) he ruled "A" on #1963 (the function-length ratchet), which moved the
  split here: in place, it took `kernel/src/lib.rs` past its row under §266 (a Rust source file
  stays under 2,000 lines). The split was built on that ratchet's lane; this pull request carries
  it. It answers the block's question 1 (extract the two boots as functions).
- **The crate root becomes an index.** §282 (when a file is split, the parent becomes an index) is
  decided. The three splits before this one each took their parent all the way: #1957, which is §279
  (the system-initializer crate root splits into private modules), then #1958 and #1959. So this cut
  does too, rather than only shrinking `lib.rs`'s row in `design/parent-index-baseline.tsv`. That
  answers the block's question 3. `print_machine_description` and the boot self-tests leave `lib.rs`
  for children named for them, and the `script/lint` check from milestone 267 (the milestone tour is
  three things wearing one name) reads the new path.
- **`-> !` on the two boots.** The block asked for it where it holds. It holds in every build mode
  once `run_measurement_boot` is `-> !` in a build with a measurement feature and a no-op in one
  without, which is how this pull request writes it.

## The cut

Children of the crate root, each taking `use super::*`. An item a sibling calls is `pub(super)`,
which in a child of the crate root is the crate, so nothing widens. `kernel_main`, `DTB` and
`device_tree` keep their paths through `pub use`. Names provisional.

| file | holds | lines |
|---|---|---|
| `lib.rs` | crate docs and attributes, `mod` lines, `system_test_access`, re-exports | 293 |
| `boot.rs` | `kernel_main`, `DTB`, `device_tree`, `bring_up_machine`, `interrupts_init`, `run_test_suite` and its hook, `run_measurement_boot` | 432 |
| `x86_64_boot.rs` | `x86_64_boot`, its six phases, `x86_hand_over` | 758 |
| `riscv64_boot.rs` | `riscv64_boot`, its phases, `micros`, `micros_between`, `bytes_per_second`, `mode_note`, `riscv_hand_over` | 1,087 |
| `milestone_tour.rs` | `milestone_tour`, `tour_preemption`, `tour_el0`, `image_for_virtio` | 260 |
| `machine_description.rs` | `print_machine_description`, `stack_top` | 206 |
| `tests.rs` | the seven boot self-tests, still `crate::tests` | 124 |

`lib.rs` was 2,984 lines on main at `17b23264e`, and 3,086 with the phase split in place. The
two per-architecture boots carry their `cfg(target_arch)` on the declaration in `lib.rs`.

## The question: do the two boots live at the crate root, or under `kernel/src/arch/<port>/`?

AGENTS.md rule 1 says all architecture-specific code lives under `kernel/src/arch/`, and names
assembly, `asm!`, system registers and CPU-specific behavior. `x86_64_boot` and `riscv64_boot`
run only on their architecture, so a reader can fairly expect them there.

What they hold, measured on this branch:

| file | `asm!` | `arch::` | `sched::` | `user::` | `drivers::` |
|---|---|---|---|---|---|
| `x86_64_boot.rs` | 0 | 93 | 9 | 7 | 0 |
| `riscv64_boot.rs` | 0 | 52 | 23 | 22 | 3 |

What `arch/` reaches up into today: of 70 files across the three ports, 5 name `sched` (the
trap and timer paths), 4 name `drivers`, and one names `user`, in a riscv64 MMU test.

Prior art, read on 2026-10-12 (UTC):

| system | where the per-architecture boot lives | what it does there |
|---|---|---|
| Linux, `init/main.c` | `setup_arch`, under `arch/` | CPU and memory setup. `start_kernel`, `rest_init` and `kernel_init`, which start the first user process, are generic in `init/`. |
| seL4, `src/arch/arm/kernel/boot.c` | `init_kernel` and `try_init_kernel`, under `src/arch/` | CPU and platform setup, and it also builds the root task itself, calling generic helpers such as `create_initial_thread` and `create_root_cnode`. |
| Redox, `src/arch/x86_shared/start.rs` | `kstart` and `start`, under `src/arch/` | CPU, paging, devices, then `crate::startup::kmain`, which is generic. |

### Alternatives

- The crate root. Recommended. These are not an architecture layer. They are portable boots
  that only one architecture runs today: each starts the scheduler, demonstrates IPC, spawns
  userspace programs and hands the system to the progenitor, through `sched`, `user` and the
  drivers. Under `arch/` they would be the one place the layer below `user` calls up into it, 29
  times. The block's "What it is not" names where they are going: onto aarch64's shared path, at
  which point each shrinks toward the CPU bring-up that does belong under `arch/`. Linux and Redox
  split exactly there, keeping the user-process start generic.
- `kernel/src/arch/x86_64/boot.rs` and `kernel/src/arch/riscv64/boot.rs`. The real contender,
  and seL4 is its precedent: its ARM boot builds the root task from inside `src/arch/`. It reads
  rule 1 by where code runs rather than by what it touches, and puts each file beside the port it
  belongs to.

The block also gave a third reason for the crate root: `helpers/lint_arch_parity.py` skips files
under `arch/`. That reason does not hold. A module declared `cfg(target_arch = "x86_64")` is
checked against `x86_64` alone, which the compiler already proves, so the lint adds nothing for
either file wherever it lives.

The recommendation holds at equal cost: the two placements are the same move with different
paths. It is about layering, not effort. It is reversible: if calef answers `arch/`, the two files
move and nothing else changes, since no path outside the crate names them.

## What it does not decide

- Module names, all provisional.
- Bringing the two boots onto the shared path (a parity change, not this milestone).
- Splitting `riscv64_boot.rs` further; at 1,087 lines it is under the 1,500 the block allows a new
  file.

## BUGS

- `boot.rs` holds two concerns, the entry and the steps all boots share. It is 432 lines, and
  the line between them is a judgment: both kinds are reached from all three boots.
