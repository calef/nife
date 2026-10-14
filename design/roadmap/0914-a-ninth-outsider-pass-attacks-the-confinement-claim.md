---
status: PARTIAL
raised: 2026-10-13
milestone_dependencies: 824
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 914. A ninth outsider pass attacks the confinement claim

*(Minted 2026-10-13 (UTC) by lane `maintainer/outsider-passes-eight-and-nine`, modeled on
milestone 913 (an eighth outsider pass attacks the confinement claim), on calef's ruling recorded
there. The
number 914 is provisional; other lanes are minting nearby, so expect renumbering at merge. Title,
slug and every name below are drafts.)*

Reuse: the brief is `briefs/confinement-outsider-pass.md` and the launch is
`briefs/confinement-outsider-pass-launch.md` (name provisional), so this pass compares row for row
with 913. The isolation is milestone 633 (an outside agent attacks the confinement claim)'s: blind.

## Index row

The ninth outsider pass at risk 7's confinement claim, by Claude Fable in Claude's cloud runner,
against the same pinned commit as milestone 913's GLM 5.3 pass. Held to the 27-of-34 booted-row
floor, it is the Anthropic half of criterion (c)'s two consecutive clean passes.

## Why

Criterion (c) needs two consecutive clean passes, at least one non-Anthropic or human. calef ruled
on 2026-10-13 (UTC) that 913 (GLM 5.3) and this pass (Fable) are the two, both attacking one
pinned commit. 913's block has the ruling whole: the pinned commit, why running concurrently still
counts as consecutive, and the reset rule.

## The attack

- Reviewer: a fresh Claude Fable agent in Claude's cloud runner, started by the maintainer with
  the one prompt `briefs/confinement-outsider-pass-launch.md` assembles. The pass note records
  the model id the session reports.
- Posture: blind, milestone 633's. The claims table is inline in the prompt, since the runner
  shares no file system with the lane. Nothing from `lab/` and no earlier pass's findings go into
  the prompt or the runner's environment.
- The commit: the first main commit after #2035 lands, which brings milestone 824 (a retype
  mints no right its budget lacks) to main. It is the same commit as 913's, pinned at launch and
  named in the pass note.
- Floor and counting: the brief's, unchanged. 27 of 34 rows booted; rows 12 to 18 host proofs;
  rows 1 to 3 carried from pass 7 with written acceptance.
- Timing: the runner shares no host with the lanes on patagonia, so contention there does not
  touch this pass. It has no HVF, which costs nothing, since `script/test --arch <isa>` runs every
  leg under TCG on either host. The reviewer records `uname -m`, `nproc` and each leg's wall time.

## The lane's role

The same as 913's: brief, collect, reproduce, record. The lane assembles the prompt and has the
maintainer start the session. When the session pushes, it fetches the branch and reruns every
booted row's test on patagonia, one ISA at a time, at the pinned commit. Each escape is reproduced
by test or boot before it is recorded. It does not attack and does not hint.

## Before launch: a setup-only dry run

Nothing else in this tree shows a Claude cloud runner building or booting nife, so the setup
steps from the launch brief ran alone first, on 2026-10-13 (UTC) between 19:03 and 19:21, in a
Claude cloud runner (lane branch `claude/vibrant-meitner-pkabyk`). It is green, so the pass may
launch.

- Pinned commit: `643e0bef86a626d3be9e77e5369ee4c73e47d814`, the merge of #2035 that brings
  milestone 824 to main. The dry run used a detached worktree at that commit.
- Host: Ubuntu 24.04.5, `uname -m` x86_64, `nproc` 4, 15 GiB of memory, no swap, 30 GB free on
  `/` before the run and 28 GB after it. `rustup` and `clang` came preinstalled.
- `script/ci-qemu`: failed in 0.1 s on its first command, `apt-get install libslirp0`, with
  "Unable to locate package", because the container starts with empty apt lists. After one
  `apt-get update` (4 s) it built QEMU 11.1.2 cold in 7 min 30 s, and `script/qemu-check` passed.
  `download.qemu.org` is reachable.
- `NIFE_SKIP_LINT_TOOLS=1 script/bootstrap`: passed in 47 s. It installed the
  `nightly-2026-10-12` toolchain and ShellCheck.
- `script/qemu-check`: passed in 0.1 s. QEMU 11.1.2 matches the pin.
- `script/test --arch <isa> --test seventh_pass`, each leg under TCG, one at a time: aarch64 passed
  in 5 min 34 s, including the cold build; riscv64 in 1 min 6 s; x86_64 in 1 min 44 s. Each ran 2 of
  2 tests, with no watchdog or timeout. No QEMU was left running.

## The first attempt: served by Opus, below the floor

One pass ran on 2026-10-13 (UTC), its commits between 20:38 and 22:36, in cloud session
`session_0198ZNJVhj6z8jdsXyKHPH64` at the pinned commit. It does not count toward criterion (c).
[`notes/confinement-outsider-pass-9.md`](../../notes/confinement-outsider-pass-9.md) records it in
full.

- The session was configured for `claude-fable-5-1` and served by `claude-opus-4-8`, so it is not
  the Fable pass this milestone names.
- It booted 20 of the 27 bootable rows by the brief's counting: rows 19, 20, 24, 25 and 34 were
  reruns of existing tests, and rows 31 and 32 were graded untestable.
- No escape. One near miss, claim 17, already milestone 799 (the shadow descriptor is published in
  two stores).
- Its tests are on main as `system_tests/src/user/ninth_pass_*`, and every one holds on patagonia
  on all three ISAs.

## Done means

- The dry run recorded, then every bootable row booted, or the pass recorded in full and marked
  as not counting, as the brief says.
- Every escape is a failing test committed before any fix.
- A pass-9 note with the per-row table, the host facts and the refusal log, indexed in
  `notes/README/` and its area page.
- Risk 7's appendix cites the pass through the maintainer under §216 (fatal-risk facts are
  correctable, and verdicts are the architect's). Moving the color stays calef's.

## BUGS

What a cloud runner needs that patagonia has, checked against the dry run above.

- Verified, and worked around. The runner starts with empty apt lists, and `script/ci-qemu` at
  the pinned commit runs `apt-get install libslirp0` before any `apt-get update`, so it fails at
  once. `script/ci-qemu` on main updates and retries when that install fails. The pinned commit
  predates that, so the launch prompt runs `apt-get update` first.
- Verified. QEMU 11.1.2 is built, not installed, because Ubuntu 24.04's QEMU 8.2 lacks
  `riscv-iommu-pci`. The runner's allowlist permits `download.qemu.org`, and the cold build takes
  about 7.5 minutes on 4 cores.
- Verified for what ran. `static.rust-lang.org`, the Ubuntu apt mirrors and the cargo registry
  all answer. `sh.rustup.rs` was not exercised, since `rustup` was preinstalled. Bootstrap does not
  install `ovmf` or `ipxe-qemu` once the built QEMU is on `PATH`; the x86_64 leg boots with the
  built QEMU's own `share/qemu/edk2-x86_64-code.fd`, which `helpers/qemu-uefi-x86_64.sh` finds.
- Still open, narrowed. The runner is x86_64, and CI runs test legs only on arm64 Linux. The
  `seventh_pass` legs passed on all three ISAs here without a watchdog, but the other rows are
  unmeasured on this host. The brief grades a timeout inconclusive, and the lane's rerun on
  patagonia settles it.
- Verified harmless. The brief's `/opt/homebrew/...` `PATH` line does not exist on Linux and is
  skipped. The runner's `python3` is 3.13, new enough for `tomllib`, and `/usr/share/OVMF` is
  absent, which costs nothing because of the firmware above.
- Still open. Results cannot stay outside the repository, since the container is discarded
  with its session. The prompt has the report committed last at the repository root and the
  branch pushed once. A pushed branch is public, so any escape is public from that push, no worse
  than every escape test this project already commits in public before its fix. A push from the
  runner works.
- Still open. Blind to history is honor only. The runner's clone is shallow (553 commits, `main`
  and its own branch), but `git fetch` reaches every branch, including 913's once it pushes. The
  prompt forbids history and GitHub; nothing enforces it.
- Disk verified, session length still open. After three legs, `target/` is 4.9 GB, the
  toolchains 0.9 GB and the QEMU build 0.6 GB, against 28 GB still free. A pass runs for hours,
  and the runner's session limit is unknown.

Minimal setup, in the order the launch brief's prompt runs it:

    apt-get update
    script/ci-qemu
    NIFE_SKIP_LINT_TOOLS=1 script/bootstrap
    script/qemu-check
    uname -m; nproc; df -h .
    script/test --arch aarch64 --test seventh_pass

## Follow-on

- **Outstanding.** The Fable pass itself: a fresh session that is served by Fable, at the pinned
  commit, held to the 27-of-34 floor. The prompt is unchanged; check the served model before
  counting.
- **Done.** Every row pass 9 booted reproduces on patagonia, one ISA at a time, at the pinned
  commit and on main: 13 of 13 tests on aarch64 and riscv64, 18 of 18 on x86_64. Its tests are on
  main as `ninth_pass_*`, each reviewer commit carrying `Co-Authored-By: Claude Opus 4.8`, and the
  runner's branch is gone. The pass note has the per-ISA table.
- **Milestone 198.** A human review, or a public bounty once a stranger can install nife, is the
  stronger form and waits on milestone 198 (a package manager, and the trivial install that makes a
  second customer possible).
