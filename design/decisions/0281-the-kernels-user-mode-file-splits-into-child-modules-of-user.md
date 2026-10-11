---
status: PROPOSED
raised: 2026-10-11
---

# 281. The kernel's user-mode file splits into child modules of `user`

Raised 2026-10-11 (UTC) as the cut that
[milestone 872 (the kernel's user-mode file is split along its seams)](../roadmap/0872-the-kernels-user-mode-file-is-split-along-its-seams.md)
says is calef's call before a line moves. Written by lane
`milestone/872-the-kernels-user-mode-file-is-split-along-its-seams`. *(Section number provisional
until the merge queue lands it.)*

It blocks the whole milestone. Nothing else waits on it.

## What the file is today

`kernel/src/user.rs` is 3,519 lines on main at `ae6b91e79`. Milestone 812 (`std::thread::spawn`
runs real threads in one address space) has merged, so the address-space registry already lives in
`kernel/src/user/address_space_registry.rs` (487 lines), behind `pub use address_space_registry::*`.
Measured from the file's own items, each range starting at its first doc comment:

| lines | count | what |
|---|---|---|
| 1 to 63 | 63 | module docs, imports, the user stack constants |
| 64 to 518 | 455 | `AddressSpace`, `Backing`, the overhead constants, `impl AddressSpace`, `ASIDS`, the registry's `mod` line |
| 519 to 570 | 52 | `impl Drop for AddressSpace` |
| 571 to 870 | 300 | `LoadError`, `load`, the timebase page, `map_segments`, `initrd`, `program` |
| 871 to 905 | 35 | `Mapping` and `Spawn` |
| 906 to 1112 | 207 | `INITRD_VA`, the per-architecture console constants, `PROGENITOR_ENTRY` through `PROGENITOR_ROLE` |
| 1113 to 1550 | 438 | `spawn_hello`, `DeviceRun`, the `run` family, `enter_frame`, the `outlaw` roles |
| 1551 to 1569 | 19 | the `x86_programs` declaration |
| 1570 to 1937 | 368 | the `x86_64` userspace demo and the two riscv64 demos |
| 1938 to 2630 | 693 | `boot_progenitor`, one function with a 56-line doc |
| 2631 to 2663 | 33 | `boot_usb_keyboard` |
| 2664 to 2872 | 209 | `mod` lines with their docs, and `term_print` |
| 2873 to 2935 | 63 | `boot_clock_page`, `VirtioBootGrant` |
| 2936 to 2963 | 28 | `VIRTIO_DMA_PHYS_OFFSET`, `write_dma_phys` |
| 2964 to 3340 | 377 | the virtio, entropy, e1000e and DesignWare starters, `boot_config_page`, `boot_screen_terminal` |
| 3341 to 3519 | 179 | `mod` lines with their docs, and `wait_for` |

Every starter (the eleven `boot_*` functions below `boot_progenitor`) has exactly one caller,
`boot_progenitor`. `write_dma_phys` is not a starter: `display_service` and `keyboard_service` call
it as `super::write_dma_phys`. The file has no tests and no Kani harness.

## The recommended cut

Child modules of `user`, private, each taking `use super::*`. `user.rs` re-exports each whole
(`pub use <child>::*`), as 812 did for the registry and §271 (the scheduler file splits into
submodules of `sched`) did for `sched`. Every `user::` path a caller uses today still resolves, so
no Rust file outside `kernel/src/user.rs` and `kernel/src/user/` changes. Every module name is
provisional and left to calef's naming pass.

| file | holds | lines, about |
|---|---|---|
| `user.rs` | module docs, `AddressSpace` and its `Drop`, the console and role constants, `write_dma_phys`, every `mod` line, `term_print`, `wait_for` | 1,240 |
| `user/loader.rs` | `LoadError` through `program` | 310 |
| `user/entry.rs` | `Mapping`, `Spawn`, `spawn_hello`, `DeviceRun`, the `run` family, `enter_frame`, the `outlaw` roles | 485 |
| `user/boot_demos.rs` | the `x86_64` and riscv64 demos | 380 |
| `user/progenitor_boot.rs` | `boot_progenitor` and the eleven starters it calls | 1,180 |

No file is over 1,500, and the parent is under the ceiling of §266 (a Rust source file stays under
2,000 lines).

## The forks

### 1. `AddressSpace` stays in the parent

The block's reason was that moving it to a child would widen its fields to `pub(super)`. That
premise is half true. The keyword changes, but who can see the fields does not: a private field
of `user` is visible to `user` and all its descendants, and a `pub(super)` field of
`user::address_space` is visible to exactly the same set. So the fork is about shape, not
encapsulation.

| | what it did |
|---|---|
| §271, for milestone 840 (the scheduler file is split along its seams) | `IpcTables`, the type every child takes, stayed in the parent `sched.rs`. |
| §272 (the grant-plan crate root splits into private modules) | `lib.rs` became an index: crate docs, `mod` lines, re-exports, byte helpers. |
| milestone 812 (`std::thread::spawn` runs real threads in one address space) | cut the registry out as a child "so it reaches `AddressSpace`'s fields as before", in its own comment. |
| Linux, recalled, not read | `mm_struct` is declared in `include/linux/mm_types.h`, apart from the ELF loader in `fs/binfmt_elf.c`. |

- A. Keep it in the parent. Recommended. The four children that reach its fields (registry,
  loader, entry, progenitor boot) do so by structure, with no annotation that reads like an
  exported surface. The module's own docs (TTBR0 and TTBR1, the higher half) are about it.
  This is the kernel precedent, §271. It holds at equal cost.
- B. Move it to `user/address_space.rs`, leaving the parent an index of about 740 lines, the 842
  shape. Its five fields and `Backing` become `pub(super)`, which a reader of the struct sees as a
  surface even though the compiler's answer is unchanged. The falsification record that patches
  `impl AddressSpace` and the comment-block row on `map_physical`'s doc would both move.

### 2. Where the starters go

The block recommended a module of their own, because milestone 811 (the boot services leave the
kernel) would then delete that one file. 811 has since been planned in order (its "The work, in
order"): one starter leaves per step, across seven steps, so no step deletes a file whole. Each
step also edits `boot_progenitor`'s grants for the slot it frees.

| | where boot-time endowment lives |
|---|---|
| seL4, `src/kernel/boot.c`, read 2026-10-11 | `create_initial_thread` beside the helpers that build what it is given: `create_root_cnode`, `provide_cap`, `create_bi_frame_cap`, `create_it_asid_pool`. |
| Linux, `init/main.c`, read 2026-10-11 | `do_basic_setup` runs `do_initcalls`; each driver's init function lives in that driver's own file. |
| this tree | `fs_service::root_directory` and `install_service::confirm` already sit in their services' files. |

- A. With `boot_progenitor`, in one module of about 1,180 lines. Recommended. They have one
  caller, and each 811 step then edits one file rather than two. Nothing gains a `pub(super)`:
  the starters and `VirtioBootGrant`'s three fields stay private to the module that uses them.
  It is seL4's shape. It holds at equal cost.
- B. A module of their own, about 480 lines, as the block proposed. Eleven functions and three
  fields gain `pub(super)`, and the module reads as 811's remaining inventory, emptying to nothing
  as 811 lands.
- C. Each beside its service, the Linux shape and this tree's two existing cases. Refused: two of
  them (`boot_clock_page`, `boot_config_page`) have no service, three share `VirtioBootGrant` and
  `boot_virtio_mmio_device`, and most of their bodies are `boot_progenitor`'s console narration
  ("network : NONE ...") rather than service wiring.

### Not asked: the per-architecture demos stay out of `kernel/src/arch/`

The block asked whether the `cfg(target_arch)` demos and console constants move under
`kernel/src/arch/`. It is not put to calef. Neither range holds `asm!`, a system register or
assembly; they call `sched` and the loader, so under `arch/` they would point the layering upward.
811's fork 1 (ruled A, 2026-10-07 UTC) already moves `x86_userspace_demo` behind a non-default
`tour` feature, so its future is decided there. This cut moves nothing out of `arch/` and adds no
architecture-specific code, so AGENTS.md rule 1 holds under it unchanged.

## What the move costs beyond moving code

- Visibility. Under 1 A and 2 A, nothing widens. The one private item that crosses a seam is
  `WINDOW_IN_OVERHEAD`, a parent constant the loader reads, which a child sees as it is. Under
  2 B the starters and `VirtioBootGrant`'s fields gain `pub(super)`; under 1 B, `AddressSpace`'s
  fields and `Backing` do.
- The ratchets. `design/file-length-baseline.tsv` drops the `user.rs` row and gains none. Of the two
  comment-block rows, `AddressSpace::map_physical`'s doc stays put under fork 1 A.
  `boot_progenitor`'s doc follows its block under the split rule of §267 (a comment states the
  constraint as it is now). That rule was added on 2026-10-11 (UTC), after the block was written.
- Falsifications. One record patches `user.rs` today
  (`user.spawn_mapping_revocation_tests.a_page_the_kernel_wired_is_unmapped_when_its_frame_is_revoked`),
  in `impl AddressSpace`. It is replayed and shown red at its own assertion.
- Citations. 32 `user.rs:NNNN` citations, all in Markdown and data files, repointed to symbols
  under §277 (cite a heading or a symbol, never a line number).
- Nothing keys on the module path. `script/fastpath-footprint`'s cold list matches
  `12AddressSpace` under any path; `script/stack-frame-check` names `disk_service`, which does not
  move.
- Proof. `script/test` on aarch64, riscv64 and x86_64 (with its AMD-Vi and OVMF legs),
  `script/fastpath-footprint`, Kani, full `script/lint` and `script/coverage`.

## What it does not decide

Module names, which are calef's naming pass. No public path, wire format, syscall or lock changes,
and no function changes behavior.
