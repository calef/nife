---
status: NOT-STARTED
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

Nothing in this tree shows a Claude cloud runner building or booting nife, so the first session
runs only the setup steps from the launch brief and reports their output and times. The pass is
not launched until that run is green and its numbers are in this block.

## Done means

- The dry run recorded, then every bootable row booted, or the pass recorded in full and marked
  as not counting, as the brief says.
- Every escape is a failing test committed before any fix.
- A pass-9 note with the per-row table, the host facts and the refusal log, indexed in
  `notes/README/` and its area page.
- Risk 7's appendix cites the pass through the maintainer under §216 (fatal-risk facts are
  correctable, and verdicts are the architect's). Moving the color stays calef's.

## BUGS

What a cloud runner needs that patagonia has, read from `script/bootstrap`, `script/ci-qemu` and
CI's `test-shard` job. CI runs this same Linux path green on `ubuntu-24.04-arm`. Nothing here was
run in a Claude cloud runner, so each line is unverified until the dry run.

- QEMU 11.1.2 is built, not installed. Ubuntu 24.04 ships QEMU 8.2, which lacks
  `riscv-iommu-pci`, so the riscv64 leg cannot boot on it. `script/ci-qemu` builds the pinned
  version into `~/.cache/nife-qemu`. That takes sudo, `apt-get` and a download from
  `download.qemu.org`, and 10 to 20 minutes cold. If the runner's network allowlist omits that
  host, the build fails at its first step, and the script has no knob for another source. Setup:
  allow `download.qemu.org` in the runner's network settings. Failing that, build the same
  version from a source the runner allows, with `script/ci-qemu`'s configure line, into
  `~/.cache/nife-qemu`; `helpers/qemu-path.sh` puts it on `PATH` when its version matches.
- The rest of the network. `script/bootstrap` needs `sh.rustup.rs` and
  `static.rust-lang.org` for the pinned `nightly-2026-10-12`. It also needs the apt mirrors for
  `ovmf` and `ipxe-qemu`, and the cargo registry for the first build.
- Host architecture is unmeasured on x86_64. CI runs the test legs only on arm64 Linux, and
  says moving a leg to an x86_64 host "changes the emulator under the suite and is unmeasured".
  If the runner is x86_64, a watchdog may fire where patagonia passes. The brief already grades a
  timeout inconclusive, and the lane's own rerun on patagonia settles it.
- Homebrew paths. The brief's `/opt/homebrew/opt/python@3.14/libexec/bin` does not exist on
  Linux and is skipped. Ubuntu 24.04's `python3` is 3.12, new enough for `tomllib`. The OVMF
  search in `helpers/qemu-uefi-x86_64.sh` already tries `/usr/share/OVMF/OVMF_CODE_4M.fd`.
- Results cannot stay outside the repository. The brief says to write the report outside the
  repository and not to push, and a cloud container is discarded with its session. The prompt
  overrides both: the report is committed last, at the repository root, and the session pushes
  its branch once, when the report is done. A pushed branch is public, so any escape is
  public from that push. That is no worse than every escape test this project already commits
  in public before its fix.
- Blind to history is honor only. The runner clones the repository with its history and every
  branch, including 913's if it has pushed. The prompt forbids history and GitHub, as on the Mac;
  nothing enforces it.
- Disk and session length. `target/` reaches about 2.2 GB on patagonia, before rustup and the
  QEMU build. A pass runs for hours. Neither limit is known for the runner.

Minimal setup, in the order the launch brief's prompt runs it:

    script/ci-qemu
    NIFE_SKIP_LINT_TOOLS=1 script/bootstrap
    script/qemu-check
    uname -m; nproc; df -h .
    script/test --arch aarch64 --test seventh_pass

## Follow-on

- **Milestone 198.** A human review, or a public bounty once a stranger can install nife, is the
  stronger form and waits on milestone 198 (a package manager, and the trivial install that makes a
  second customer possible).
