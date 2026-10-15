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
# 879. The kernel's file-service wiring is split along its seams

*(The title and slug are provisional.)*

`kernel/src/user/fs_service.rs` is 2,258 lines at `396187b0b`. §266 (a Rust source file stays
under 2,000 lines) sets the ceiling. Milestone 840 (the scheduler file is split along its seams)
is the model for this block.

Its row in `design/file-length-baseline.tsv` still says 2,407. The file shrank twice in two days
without the row being banked, which §266's ratchet allows. Milestone 835 (a C library, stage 1:
files, clock and memory) moved the shared `std` spawn into `fs_service/std_spawn.rs` (2,407 to
2,343). Milestone 809 (the package client becomes a program) moved `install_for_test` into
`fs_service/test_install.rs` (2,343 to 2,258). Each carved off just enough to pass the ratchet,
and the next feature here will hit the same wall.

The file is a child module of `kernel/src/user.rs`, which milestone 872 (the kernel's user-mode
file is split along its seams) covers. The two are independent, with the two caveats under "What
moving the code breaks".

The cut and every module name are calef's call. He ruled the shape on 2026-10-11 (UTC) in §282
(when a file is split, the parent becomes an index), while ruling milestone 872's fork, and the
merge of #1988 applied it here the next day; every module name ships provisional.

Reuse: not applicable; this moves code and adds none. The precedent is in this file's own
directory: `std_spawn` and `test_install` are the two child modules already split out of it.

## What is in the file

Measured on `396187b0b` from the file's own top-level items. Ranges are approximate at the edges,
where a doc comment belongs to the item below it.

| lines | count | what |
|---|---|---|
| 1 to 656 | 656 | constants, the file and block channels, stack poisoning, client windows, `ensure`, `wire_servers`, the NVMe arm, `spawn_block_server`, `spawn_fs_server` |
| 657 to 797 | 141 | the crash test's own service on its own disk: `CrashRun`, `start_crash`, `recover_crash` |
| 798 to 893 | 96 | `spawn_fs_client` and `start` |
| 894 to 928 | 35 | `Grant`, the per-file grant's value |
| 929 to 988 | 60 | `wait_for_service`, `wait_for_caretaker` |
| 989 to 1096 | 108 | `start_granted`, `DirGrant` |
| 1097 to 1165 | 69 | the server, `mkfs` and block-driver images, and their refusal sentences |
| 1166 to 1188 | 23 | `root_directory`, the one entry point the interactive boot uses |
| 1189 to 1348 | 160 | `narrow_dir`, `narrow_dir_held`, `spawn_caretaker`, `start_granted_dir` |
| 1349 to 1475 | 127 | the set grant: `SetGrant`, `start_granted_set` |
| 1476 to 1588 | 113 | the shared-frame witness, two live clients on one service |
| 1589 to 1686 | 98 | the file sink and the file source |
| 1687 to 2010 | 324 | every `start_std*`, `NarrowedStd`, `BoundStd`, `bind_subtree`, the `std_spawn` declaration |
| 2011 to 2145 | 135 | two directory grants to one process |
| 2146 to 2258 | 113 | the root-file helpers tests use, and the `test_install` declaration |

There are no tests in the file: no `#[test]` or `#[test_case]` and no `mod tests`. The tests that
drive it live in 21 files under `system_tests/src/user/`. Of the file's items, 35 carry
`allow(dead_code)` outside `test` and `system_tests`. They are rigs only the suite calls.

## The verdict: split by feature; there are no tests to move

The tests-only axis of milestone 846 (the file-server core moves its tests out) is not available.
The question is only which features leave.

Each block in the table after line 656 is a rig with its own helpers, built by one milestone. The
crash service, the caretaker grants, the `std` spawns, the sink, the witness: each takes the
service from `ensure` and adds one shape of client. That is a by-feature seam, and every one of
them can be a child module that keeps reaching the parent's private statics. Nothing widens.

What stays is the service itself: the channels, the windows, the two servers, the client spawn,
the readiness drain and the images. That is also the part milestone 811 (the boot services leave
the kernel: the kernel starts only the progenitor) moves out of the kernel. It leaves the test-only
rigs where they are. After this split, 811 takes most of the parent and touches no child.

The smaller step is to move the 324 lines of `std` spawns into `std_spawn.rs` and stop. That
leaves the file at about 1,934, under the ceiling by 66 lines. Two lanes in a row already did the
smallest step here, so this block recommends the full cut once.

## A proposed cut

Every name here is provisional. The new modules are private, and `fs_service.rs` re-exports their
`pub` items, so every `kernel::user::fs_service::` path in use today still resolves.

| module, provisional | from the table | about |
|---|---|---|
| `fs_service` (`fs_service.rs`) | 1 to 656, 798 to 893, 929 to 988, 1097 to 1188, re-exports | 950 |
| `fs_service::crash` | 657 to 797 | 140 |
| `fs_service::grant` | 894 to 928, 989 to 1096, 1189 to 1475, 2011 to 2145 | 570 |
| `fs_service::std_spawn` (exists, 128) | 1687 to 2010 | 450 |
| `fs_service::shared_frame_witness` | 1476 to 1588 | 115 |
| `fs_service::file_sink` | 1589 to 1686 | 100 |
| `fs_service::test_install` (exists, 98) | 2146 to 2258 | 210 |

The parent ends near 950 lines, far under 840's standard of 3,000, and no new file passes 1,500.
`spawn_caretaker` has callers only in `grant`, so it moves there and stays private. The `std`
spawns call `narrow_dir_held`, which is `pub` and resolves through the parent's re-export.

## What an architect had to rule

Answered 2026-10-12 (UTC) by the merge of #1988 under §282. The four answers: 1 the full cut,
all seventeen modules; 2 private children re-exported whole, so no `fs_service::` path changed;
3 the witness and the sink stand alone; 4 the names in the what-was-built table ship provisional.
The questions stay here as they were asked.

1. The full cut, or only the `std` move. The recommendation is the full cut. The smaller step
   leaves 66 lines of headroom in a file that grew 2,132 to 2,407 between 2026-09-27 and
   2026-10-07. That reason is about how soon the work recurs, which is close to an effort
   argument; at equal cost the full cut still wins because it matches 811's seam.
2. Private modules with re-exports, or public modules. The recommendation is private: 35 Rust
   files in `kernel` and `system_tests` use `fs_service::` paths, and none of them changes. Public modules would
   add a second path to every item. That reason holds at equal cost.
3. Whether the 100-line witness and sink modules stand alone or join another. The recommendation is
   alone, since each is one milestone's rig with nothing shared.
4. Every module name in the table.

No public crate API, wire format or syscall changes under any answer here.

## What it is not

A file split, not a gating change. Gating the test-only modules behind `cfg(feature =
"system_tests")` would drop code from the production build. That changes what ships and is not
this milestone.

## What moving the code breaks

Found with `grep` at `396187b0b`. A lane has to carry each one.

- No falsification patch under `kernel/falsifications/`, `system_tests/falsifications/` or any
  other `falsifications` directory diffs this file. One,
  `user.spawn_mapping_revocation_tests.a_page_the_kernel_wired_is_unmapped_when_its_frame_is_revoked.patch`,
  names `fs_service::spawn_fs_server` in its prose and diffs `revoke.rs` and `user.rs`. Those file
  names carry a test's module path, but no test lives here, so none is renamed.
- No Kani harness is in this file, and nothing in `.github/` names it.
- `script/fastpath-footprint` matches mangled names under `kernel5sched` and `kernel7syscall` only.
  `script/stack-frame-check` lists no `kernel::user::fs_service` symbol. A moved function's
  symbol path changes, and neither tool keys on any of them.
- `helpers/cap_abbreviation.py` has no allowance keyed on this path.
- `design/file-length-baseline.tsv` row 27 must be removed in the same change. It sits next to
  `kernel/src/user.rs`'s row 26, which milestone 872 removes or lowers. If both pull requests are
  open at once, the second to merge rebases over a one-line conflict.
- `fs_service.rs` opens with `use super::*`, so it sees whatever `user.rs` names. If milestone 872
  moves an item this file uses into a private module of `user` without re-exporting it, this file
  stops compiling. That is 872's to keep working; this split adds no new reach into `user`.
- Seven lines in five files cite `fs_service.rs:NNNN`. The two in
  `design/roadmap/0811-boot-services-leave-the-kernel.md` and the ones in milestones 292 and 849
  are best replaced with function names. Those in the dated audit report and in
  `notes/swish-check-flake.md` describe the file as it was and stay.
  **Done.** Repointed by lane `maintainer/stale-line-citations` (#2206).
- 28 tracked files outside this block name the path. `notes/fs-server.md` and
  `notes/dir-capability.md` point a reader at items that will move.
- `helpers/qemu-runner-aarch64.sh` and `helpers/qemu-runner-x86_64.sh` name the file in comments,
  and `fs_service::wire_servers` stays in the parent.
- 8 first-parent merges in the 14 days to 2026-10-11 touched it. That is cool enough to take any
  time.

Pull request #1892 (milestone 812 (`std::thread::spawn` runs real threads in one address space))
does not touch this file at its current head, but if it changes the file before it merges, every
line range above must be re-measured.

## Done when

1. `kernel/src/user/fs_service.rs` is under 1,500 lines, and no new file is over 1,500.
2. No Rust file outside `kernel/src/user/fs_service.rs` and `kernel/src/user/fs_service/` changes,
   and no item's visibility widens.
3. `script/test` passes on aarch64, riscv64 and x86_64 with no test body changed.
4. The row for this file is gone from `design/file-length-baseline.tsv`.
5. The citations listed above that are kept current name a function, not a line.
6. Each new module carries a `//! Name:` block marked provisional.

## What was built

Built 2026-10-12 (UTC) on `7b8b1df11`, in pull request #1988, by lane
`maintainer/last-three-over-2000` under §282. No 879 lane was ever cut: the ruling that answered
this block's fork covered the work, and the sweep took the file with it.
`kernel/src/user/fs_service.rs` went from 2,258 lines to 63. Every name below is provisional.

| file | lines | holds |
|---|---|---|
| `fs_service.rs` | 63 | the index: module docs, every `mod` line, the re-exports |
| `fs_service/wiring.rs` | 391 | wiring the service: the block and FS servers, `wire_servers`, the readiness sentinels, the NVMe arm, `spawn_fs_client` and `start` |
| `fs_service/std_start.rs` | 333 | every `start_std*`, `NarrowedStd`, `BoundStd`, `bind_subtree` |
| `fs_service/client.rs` | 167 | spawning a client of the service, and the waits that order it after the servers and caretakers it depends on |
| `fs_service/directory_grant.rs` | 230 | the per-directory grant: `narrow_dir`, `narrow_dir_held`, `spawn_caretaker`, `start_granted_dir`, `root_directory` |
| `fs_service/crash.rs` | 150 | milestone 37 (prove RedoxFS's crash consistency)'s crash service: `CrashRun`, `start_crash`, `recover_crash` |
| `fs_service/two_dirs.rs` | 145 | two directory grants to one process |
| `fs_service/set_grant.rs` | 140 | the set grant: `SetGrant`, `start_granted_set` |
| `fs_service/std_spawn.rs` | 128 | the shared `std` spawn (child since milestone 835) |
| `fs_service/shared_frame_witness.rs` | 125 | the shared-frame witness, two live clients on one service |
| `fs_service/stack_watermark.rs` | 120 | the FS server's stack size, the poison its pages are filled with, and the high-water read |
| `fs_service/test_files.rs` | 118 | test-only reads and writes at the file service's root |
| `fs_service/file_grant.rs` | 115 | `Grant`, `start_granted` |
| `fs_service/file_sink.rs` | 109 | the file sink and the file source |
| `fs_service/channel.rs` | 102 | the file and block channels, and where every party maps them |
| `fs_service/test_install.rs` | 98 | `install_for_test` (child since milestone 809) |
| `fs_service/client_windows.rs` | 97 | the client windows, one file channel per live client |
| `fs_service/images.rs` | 80 | the server, `mkfs` and block-driver images, and their refusal sentences |

The done-when list, checked on `0f22bc9a4`:

1. The parent is 63 lines and the largest child (`wiring.rs`) is 391. The file-length row is gone
   from `design/file-length-baseline.tsv`, removed in the same change; the baseline holds no rows
   at all now.
2. Code moved only: the split's line-multiset check against the old file found nothing changed
   beyond `pub(super)`, the module headers and the index. An item a sibling reaches became
   `pub(super)`, the same set of modules that saw it before. The six Rust files outside the block
   that changed are the doc pointers done-when 5 asks for. No `fs_service::` path changed.
3. `script/test` passed on all three ISAs in #1988's CI (the aarch64 and riscv64 shard in 11m14s,
   the x86_64 shard in 8m57s), with no test body changed.
4. See 1.
5. The doc pointers outside `design/` name the child file now: the three caretaker components,
   the filesystem protocol's fixture, `fixtures/src/fs_test_client.rs` and
   `notes/dir-capability.md`. The line citations inside three other milestones' blocks still
   stand; see Follow-on.
6. All fifteen new modules carry a `//! Name:` block marked provisional. `std_spawn.rs` and
   `test_install.rs` are not new; they carry the milestones (835, 809) that cut them.

## Follow-on

- **Recorded.** The `fs_service.rs:NNNN` citations this block said are kept current stand in
  three other milestones' blocks, which this lane may not edit. 0811 cites the images, the NVMe
  arm and the crash pair by line; those live in `fs_service/images.rs`, `fs_service/wiring.rs`
  and `fs_service/crash.rs` now. 292's role-constant census cites constants that exist nowhere
  in the tree, and its other two sites are gone with them, so that whole table is a dated record
  now. 849 cites the `rg` spawn inside `start_std_full`, which lives in `fs_service/std_start.rs`
  now. Listed on pull request #2199 for a maintainer lane.
- **Decision.** Every module name is provisional and is calef's to rule, per
  `design/decisions/0282-when-a-file-is-split-the-parent-becomes-an-index.md`.

## Index row

`kernel/src/user/fs_service.rs` held the file service and a dozen test rigs built on it, 2,258
lines with no tests of its own. It is now a 63-line index of seventeen child modules under
`kernel/src/user/fs_service/`, each named for what it holds, as §282 rules. Every module name is
provisional.
