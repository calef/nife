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
# 880. The x86-64 MMU file is split along its seams

*(The title and slug are provisional.)*

`kernel/src/arch/x86_64/mmu.rs` is the x86-64 memory management unit (MMU) module, and it is 2,192
lines at `396187b0b`. It is not growing fast. §266 (a Rust source file stays under 2,000 lines) sets the ceiling. Milestone 840 (the
scheduler file is split along its seams) is the model. Everything stays under `kernel/src/arch/`,
since this is architecture code.

Pull request #1892, for milestone 812 (`std::thread::spawn` runs real threads in one address
space), was in flight when this was measured, and at its head it does not touch this file. If it
changes this file before it merges, the line ranges below must be measured again.

The cut and every module name are calef's call. He ruled the shape on 2026-10-11 (UTC) in §282
(when a file is split, the parent becomes an index), while ruling milestone 872 (the kernel's
user-mode file is split along its seams) and its fork. The merge of #1988 applied it here the
next day; every module name ships provisional.

Reuse: not applicable; this moves code and adds none.

## What is in the file

Measured on `396187b0b` from the file's own top-level items and section banners. Ranges are
approximate at the edges, where a doc comment belongs to the item below it.

| lines | count | what |
|---|---|---|
| 1 to 125 | 125 | module docs, six BUGS entries, imports, `Format` |
| 126 to 307 | 182 | the two bases, `phys_to_virt`, fixed device addresses, `print_summary` |
| 308 to 475 | 168 | the kernel map: `init`, `init_secondary`, `map_page`, `unmap_page`, `flush_tlb` |
| 476 to 668 | 193 | the TLB shootdown by NMI, and the port-grant revocation that rides on it |
| 669 to 689 | 21 | `translate`, `is_mapped` |
| 690 to 1231 | 542 | the firmware memory map: `Claim`, the fill ceiling, the MMIO hole, `memory_mapped_io_window`, the direct-map and firmware claims |
| 1232 to 1297 | 66 | `MapFailure` and `first_conflict`, which name what a refused mapping hit |
| 1298 to 1515 | 218 | `map_everything`, `direct_map`, `map_range`, `verify` |
| 1516 to 1588 | 73 | linker symbols, `phys_to_ptr` |
| 1589 to 1867 | 279 | the user address spaces |
| 1868 to 1927 | 60 | the device windows |
| 1928 to 2192 | 265 | `mod map_tests`, 8 `#[test_case]` functions |

The two sibling ports have one file each and no child modules.

| file | lines | tests and proofs |
|---|---|---|
| `kernel/src/arch/aarch64/mmu.rs` | 1,442 | `mod tests`, 12 `#[test_case]` |
| `kernel/src/arch/riscv64/mmu.rs` | 1,922 | `mod tests`, 14 `#[test_case]`; `mod proofs` under Kani |

All three share one spine in the same order: layout constants, `init` and `init_secondary`,
`map_everything` and `verify`, linker symbols, the user address spaces, `map_page` and
`flush_tlb`. Two parts of the x86 file have no counterpart in either sibling. The shootdown is
software here. aarch64 broadcasts its TLB invalidation in hardware, and riscv64 asks the SBI
firmware to do it through a helper in `arch/riscv64/mod.rs`. The firmware memory map exists because a PC describes its memory
in a UEFI map and its devices in ACPI, where the other two read one device tree.

## The verdict: move the two x86-only parts out, with their tests

Moving only the tests out leaves 1,927 lines, 73 under the ceiling. It passes, and the next
feature puts the file back over. It also misses the point: all 8 tests test the firmware memory
map or the failure report built from it. That is one self-contained part with its own helpers.

The by-feature cut takes out the two parts the siblings do not have. The firmware memory map
leaves with its 8 tests. The shootdown leaves on its own. What stays is the spine, about 1,130
lines, which a reader can hold beside `aarch64/mmu.rs` and `riscv64/mmu.rs` and compare section
by section. That parity reading is worth more here than the line count, since §19 (architectural
parity is a tenet) makes the three ports a matched set.

A child module reaches its parent's private items, and the parent reaches the child's
`pub(super)` items. `map_everything` needs `Claim`, `direct_map_claims` and `MapFailure` from the
child, and nothing widens past `mmu`.

## A proposed cut

Every name here is provisional.

| module, provisional | from the first table | about |
|---|---|---|
| `mmu` (`mmu.rs`) | 1 to 475, 669 to 689, 1298 to 1927, `mod` and `pub use` lines | 1,135 |
| `mmu::shootdown` | 476 to 668 | 195 |
| `mmu::firmware_map` | 690 to 1297, and `map_tests` as its `mod tests` | 875 |

`mmu.rs` stays a file beside an `mmu/` directory, the shape `kernel/src/user.rs` has beside
`kernel/src/user/`.

## What an architect had to rule

Answered 2026-10-12 (UTC) by the merge of #1988 under §282. The shootdown is a child of `mmu`,
as recommended. `MapFailure` and `first_conflict` went with the claim enumeration they walk, into
`claims.rs`, one seam over from `map_everything` in `fine_map.rs`, which is the recommendation's
own reason. The names in the what-was-built table ship provisional. The questions stay here as
they were asked.

1. Whether the shootdown is a child of `mmu` or a sibling, `arch/x86_64/shootdown.rs`. The
   recommendation is a child. Its whole design argument, why it is an NMI and not an ordinary
   IPI, is that callers already hold `KERNEL_MMU` with interrupts masked. That lock is private to
   `mmu`. As a sibling, the argument would live in one file and the lock in another. The
   counter-argument is that `revoke_port_grant_others`, called from `segments.rs`, is not memory
   management. The reason for a child holds at equal cost.
2. Whether `MapFailure` and `first_conflict` go with the firmware map or stay with
   `map_everything`. The recommendation is the firmware map: `first_conflict` walks the same claim
   enumeration `map_everything` maps from, and one test constructs a `MapFailure`.
3. Every module name in the second table.

No syscall, public path or behavior changes under any answer. Splitting the sibling ports is not
part of this milestone. `riscv64/mmu.rs` is 78 lines under the ceiling and may want its own block.

## What moving the code breaks

Found with `grep` at `396187b0b`. A lane has to carry each one.

- Portable code reaches this file as `arch::mmu::*`. `kernel/src/lib.rs` calls
  `arch::mmu::memory_mapped_io_window`; `exceptions.rs` calls `mmu::serve_shootdown_nmi`; and
  `segments.rs` calls `mmu::revoke_port_grant_others`, which is `pub(super)`. A `pub use` in
  `mmu.rs` keeps every path. The `pub(super)` item needs `pub(in crate::arch::x86_64)` in the
  child, which names the same reach and widens nothing.
- One falsification patch diffs this file:
  `system_tests/falsifications/user.tests.a_user_program_cannot_read_a_kernel_address.patch`.
  Its hunk is in `map_everything`, which stays, but at line 1348, which moves. It is regenerated
  and shown to fail again. Its file name follows the test's module path, not this file's, so the
  name stays.
- No Kani harness is in this file. No falsification under `kernel/falsifications/` names it, and
  no `Falsification:` record is in it.
- No script keys on a mangled symbol from this module. `script/fastpath-footprint`,
  `script/stack-frame-check` and `script/icount` do not name it. `script/lint` names
  `crate::arch::mmu::phys_to_virt` for the riscv64 proof module, and that item stays.
- The 8 tests change names, from `mmu::map_tests::*` to `mmu::firmware_map::tests::*`. Only
  bench transcripts from 2026-09-17 quote the old names, and they are history.
- Citations of x86's `mmu.rs:NNNN`, told apart from the other two by path or context: 12 in 8
  files. Eight are history and stay: panic transcripts quoting `mmu.rs:325` and `mmu.rs:349`,
  a built milestone's "what was wrong" snippet, and a dated audit's evidence table. Four are live
  pointers in blocks not yet started, and already off by drift. Milestone 716 (x86_64 names a
  kernel stack overflow the way aarch64 and riscv64 do) cites `:1498`, `:1503` and `:691`.
  Milestone 187 (read the x86_64 arch tree through the lens the first arch audit used) cites
  `:529`. Each gets a function name in place of a number.
  **Done.** Repointed by lane `maintainer/stale-line-citations` (#2206).
- Comments elsewhere name `mmu::shoot_down_others` in `instructions.rs`, `irq.rs`, `ap_boot.rs`,
  `exceptions.rs` and `helpers/qemu-runner-x86_64.sh`. They still read true through the re-export,
  but each should name the new module.
- 13 tracked files name the path besides the patch and two baseline files.
  `design/file-length-baseline.tsv` has a row at 2,192, removed in the same change.
  `design/comment-block-baseline.tsv` holds the module header (`mmu.rs:1`, 116 lines), and the
  header stays.
- Codegen units are partitioned by module, so the move could change what is inlined across
  `flush_tlb` and the shootdown. That is from memory, not read. Measure with `script/icount` on
  the x86_64 leg.
- 5 first-parent merges in the 14 days to 2026-10-11 touched it.

## Done when

1. `mmu.rs` is under 1,300 lines, and no new file is over 1,500.
2. Every `arch::mmu` path callers use today still resolves, so no Rust file outside
   `kernel/src/arch/x86_64/` changes.
3. `script/test` passes on aarch64, riscv64 and x86_64, and the 8 moved tests pass with no body
   changed.
4. The falsification patch applies and its test fails.
5. `script/icount` is unchanged on x86_64, or each difference is explained.
6. Each new module carries a `//! Name:` block marked provisional.

## What was built

Built 2026-10-12 (UTC), in pull request #1988, by lane `maintainer/last-three-over-2000` under
§282. No 880 lane was ever cut: the ruling that answered this block's fork covered the work, and
the sweep took the file with it. `kernel/src/arch/x86_64/mmu.rs` went from 2,192 lines to an
index. Every name below is provisional.

| file | lines | holds |
|---|---|---|
| `mmu.rs` | 81 | the index: module docs, the `Format` alias, every `mod` line with its re-export |
| `mmu/fine_map.rs` | 308 | `map_everything`, `direct_map`, `map_range`, `verify` |
| `mmu/user_space.rs` | 304 | `CR3`, the kernel half, the per-space map, unmap and translate |
| `mmu/claims.rs` | 303 | `Claim`, `MapFailure`, `first_conflict`, the claim enumerations |
| `mmu/map_tests.rs` | 265 | the eight tests, moved with their indent alone; the path `mmu::map_tests` is the same |
| `mmu/memory_mapped_io_window.rs` | 264 | the fill ceiling, the MMIO hole, the BAR window and their refusal sentences |
| `mmu/shootdown.rs` | 261 | the NMI shootdown and `revoke_port_grant_others` |
| `mmu/kernel_map.rs` | 246 | `init`, `init_secondary`, the live tables under `KERNEL_MMU` |
| `mmu/layout.rs` | 156 | the two bases, `phys_to_virt`, the fixed device addresses, `print_summary` |
| `mmu/image_bounds.rs` | 76 | the linker symbols, `phys_to_ptr` |
| `mmu/device_windows.rs` | 70 | the device windows |

The done-when list, checked on `0f22bc9a4`:

1. The parent is 81 lines and the largest child (`fine_map.rs`) is 308. The file-length row is
   gone from `design/file-length-baseline.tsv`, removed in the same change; the baseline holds no
   rows at all now.
2. Code moved only: the split's line-multiset check against the old file found nothing changed
   beyond `pub(super)`, the module headers and the index. `revoke_port_grant_others` became
   `pub(in crate::arch::x86_64)`, the same reach it had. The split commit changes no Rust file
   outside `kernel/src/arch/x86_64/`, so every `arch::mmu` path callers use still resolves.
3. `script/test` passed on all three ISAs in #1988's CI (the aarch64 and riscv64 shard in 11m14s,
   the x86_64 shard in 8m57s). This lane diffed the moved tests against the pre-split module:
   byte-identical from the first `use` once dedented, eight `#[test_case]` on each side.
4. The falsification record is now
   `user.el0_boundary_tests.a_user_program_cannot_read_a_kernel_address.patch` (the test's module
   path changed when `user.rs` was split, and the record's name follows the test, so the name
   changed with it). Its x86_64 hunk patches `mmu/fine_map.rs`. This lane applied it and replayed
   the test red on x86_64: panic at the test's headline assertion in
   `system_tests/src/user/el0_boundary_tests.rs`, "the user
   program read a kernel address and was NOT stopped". The replay is dated in the record.
5. CI's icount tripwire passed on the split's head: `script/bench --check` on all three ISAs and
   the instruction-count instrument, in the one `bench (icount regression tripwire)` job. No
   x86_64 difference was recorded.
6. The nine new modules carry a `//! Name:` block marked provisional. `map_tests.rs` is not new;
   it carries the move note and its date.

## Follow-on

- **Recorded.** The live `mmu.rs:NNNN` citations this block named are stale now, and stand in
  blocks this lane may not edit. 716's guard-page asserts at `:1498` and `:1503` are the `verify`
  asserts in `mmu/fine_map.rs`, and its `:691` is `serve_shootdown_nmi` in `mmu/shootdown.rs`.
  187's `:529` is the swapgs-window comment beside `serve_shootdown_nmi`. 320 heads a snippet
  with a `mmu.rs` line comment over `PCI_ECAM_BUSES`, which lives in `mmu/layout.rs`. Each gets a
  function name in place of a number. Listed on pull request #2201 for a maintainer lane.
- **Decision.** Every module name is provisional and is calef's to rule, per
  `design/decisions/0282-when-a-file-is-split-the-parent-becomes-an-index.md`.

## Index row

`kernel/src/arch/x86_64/mmu.rs` was 2,192 lines, over §266's ceiling, with two parts no sibling
port has: a software TLB shootdown by NMI, and the firmware memory map the direct map is built
from. It is now an 81-line index of nine child modules and its tests under
`kernel/src/arch/x86_64/mmu/`, each named for what it holds, as §282 rules; the shootdown and the
claims stand alone, as this block proposed. Every module name is provisional.
