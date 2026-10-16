---
status: BUILT
raised: 2026-09-19
built: 2026-10-16
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 508. `script/lint` does not build the UEFI binary, so lint-clean code can fail `script/test`

*(Promoted from the proposal `lint-does-not-build-the-uefi-binary`, filed 2026-09-19. Found by
milestone 243 (a machine with no serial port has no way to say anything, and no gate can read it)'s
lane. That lane re-applied its loader banner and watched `script/lint` pass. `script/test`'s x86_64
leg then failed to compile the same code. The number is provisional until the merge queue lands
it.)*

## The stage

`script/lint` now compiles `uefi_loader`'s binary, which `required-features = ["uefi"]` kept out of
every pass and every ordinary build. Four configurations, each printing its own `==>` line, in the
clippy half beside the other target passes:

| target | configuration | what it becomes |
|---|---|---|
| `x86_64-unknown-uefi` | `--features uefi` | `BOOTX64.EFI` (PE/COFF) |
| `x86_64-unknown-uefi` | `--features uefi,screen_hold` | `uefi-boot`'s screen-gate build |
| `aarch64-unknown-uefi` | `--features uefi` | `BOOTAA64.EFI` (PE/COFF) |
| `riscv64imac-unknown-none-elf` | `--features uefi` | `BOOTRISCV64.EFI`, a static PIE the stick converts with `crates/portable_executable` |

Each pass needs a kernel to embed. The stage builds the three kernels the stick builds first, by
the same invocations, so the artifacts are shared. Each pass gets its own architecture's kernel by
absolute path. The archive variable stays unset. That is the loader's tour build, and it keeps the
measured-boot seal out of a lint gate. riscv64 has no UEFI target in rustc at all. Its pass
compiles the target the stick converts from, and takes none of the stick's link arguments.

The first run found three warnings the host passes cannot see: a nine-argument `hand_over` in each
of the three arch arms, an explicit-counter loop in `console.rs`, and a broken doc-list indentation
in `placement.rs`. That is the milestone's thesis confirmed on arrival. Code only `script/test`
compiled was code no gate held.

## The measurement

Both timed 2026-10-16 (UTC) on patagonia with other lanes working. The load average is recorded
with each number rather than averaged away.

- Warm, everything fresh in the worktree's own `target/`: 7 s, load 4.9 falling to 4.6.
- Cold, an empty `CARGO_TARGET_DIR`: 12 s, 213 MB written, load 4.4 rising to 5.8.

The cost is the three kernel builds. Warm, those are freshness checks. A lane that touched kernel
code pays the incremental rebuild instead. At seconds, the plain always-build shape stands, beside
the three kernel passes the script already runs. The proposal's cheaper shapes, a path-triggered
stage and a move to `script/ci-build`, are not taken: nothing here needed them.

## What lint compiles, what test compiles

Non-host binaries only. The host workspace (the crates, xtask, the RedoxFS tools, fuzz) is
clippy'd, rustdoc'd and tested by both sides.

| binary | `script/lint` | `script/test` |
|---|---|---|
| kernel, aarch64-unknown-none-softfloat (bin, features, test target) | yes | yes (suite, screen boot) |
| kernel, riscv64imac-unknown-none-elf (bin per feature) | yes | yes (suite, screen boot) |
| kernel, x86_64-unknown-none (bin per feature) | yes | yes (suite) |
| user programs, aarch64 and x86_64 (`components`, `fixtures`, `user_mode_runtime`) | yes | yes (the archives) |
| user programs, riscv64 | no, and the pass is red today (see BUGS) | yes (the riscv64 archive) |
| `uefi_loader` bin, the four configurations above | yes, this milestone | x86_64 only, through `uefi-boot`/`uefi-test`; aarch64 and riscv64 only in `cargo xtask stick` |
| `redoxfs_server` and `mkfs` for x86_64-unknown-none | no (host lint only) | yes (the x86_64 archive) |
| the `std` farm (`targets/*-unknown-nife.json`) | no | yes (`std-exerciser`) |
| `system_tests` | aarch64 only | all three, plus the UEFI test images |

## Exit

`script/lint` fails when any of the four loader configurations stops compiling clean for its
target, before a lane needs an emulator boot to find out. Measured cost: 7 s warm, 12 s cold.

Reuse: the tree's own clippy-stage shape, the written-out feature loops above the new stanza;
xtask's own kernel-build invocations, unchanged; and the loader build script's existing contract.
No new dependency.

## BUGS

- A compile is not a boot. The stage proves that the four configurations type-check with `-D
  warnings`. The loader's behavior is still proved by the OVMF boots in `script/test` and by the
  stick's boots. Nothing here changes that.
- The riscv64 pass checks the ELF compile, not the boot file. The stick's RUSTFLAGS are link
  arguments and a check build links none of them. The PE conversion in
  `crates/portable_executable` runs only in `cargo xtask stick`, and its logic is host-tested.
- The table's holes are real, and one is red today. A riscv64 userspace pass
  (`cargo clippy -p components -p fixtures -p user_mode_runtime --target
  riscv64imac-unknown-none-elf`) fails with two missing-backtick doc warnings in the doc comment
  over `child_stub.rs`'s `SEND_THEN_EXIT`, code gated to riscv64 that nothing in
  lint compiles. Until a lane fixes those two warnings and adds the stanza, a user-program
  regression on riscv64 is found by `script/test`'s archive build. `redoxfs_server`'s bare-metal
  build, the std farm and the kernel test target off aarch64 are in the same class: compiled by
  `script/test`, not by lint.
- The stage builds three kernels it only embeds as bytes. On the runs where they rebuild, their
  warnings print in lint's output. The x86_64 kernel's dead-code warnings are the expected ones,
  and the x86_64 kernel pass above carries the argument for `-A dead_code`.

## Follow-on

- **Recorded.** The riscv64 userspace gap and its two warnings, in this block's `BUGS`; the
  `redoxfs_server`, std-farm and kernel-test-target holes beside it.
- **Recorded.** A compile is not a boot, in this block's `BUGS`; the emulator boots remain the
  proof.

## Index row

`script/lint` compiles the three UEFI boot files and `uefi-boot`'s screen-hold build with `-D
warnings`, so lint-clean code can no longer fail `script/test`'s x86_64 leg on the loader; the
stage costs 7 s warm and 12 s cold, measured.
