# Brief: an outsider pass at the confinement claim

**Provisional name.** Written 2026-10-12 (UTC) by milestone 903 (a seventh outsider pass attacks
the confinement claim)'s lane, after calef ruled on #2010 that a pass with 3 of 34 rows booted does
not count toward risk 7's criterion (c). The lane that runs the next pass hands the reviewer the
section "The reviewer's brief" below, with the posture filled in, and keeps the rest for itself.

## For the lane

- Mint the pass's block the way milestone 903 was minted, and claim it with
  `script/claim <branch> --new-work`.
- Build the reviewer's one prompt with [the launch brief](confinement-outsider-pass-launch.md),
  which pins the commit and holds the per-runner sections (the Mac, Claude's cloud runner).
- The posture is calef's for each pass, so take it from his ask. Blind is milestone 633 (an outside
  agent attacks the confinement claim)'s. Informed is milestone 800 (a non-Anthropic model attacks
  the confinement claim)'s.
- Criterion (c) needs two consecutive clean passes, and at least one must come from a
  non-Anthropic model or a human. An Anthropic pass can only be the other one.
- Blind: copy the claims table alone (`notes/confinement-claims.md` from `## The claims` to the end
  of the table) to a scratch file and give the reviewer that path, never the note. Pass 7's first
  reviewer read past the table, and the attempt was discounted.
- Spawn a fresh agent. The lane briefs, collects, reproduces each claimed escape by test or boot,
  and records. It does not attack and it does not hint.
- Reproduce every booted row's test yourself, one ISA's QEMU at a time. A timeout under load is
  inconclusive, not a pass.
- Before recording, count the rows graded `booted`. Below the floor, the pass is recorded in full
  but does not count toward criterion (c), and the note says so.

## The booted-row floor

**Every row a boot can reach: 27 of the 34** (calef, 2026-10-12 UTC, on #2010: "27 seems right").
Rows 12 to 18 are host proofs of the DMA validator, the VT-d encoder and the wiring plan. No
confined component drives them at boot, so they are exempt and graded `host`. When the table
grows, the floor is every row a boot can reach.

A row counts as booted when a hostile test written for this pass ran under QEMU on at least one ISA
the row applies to. A rerun of the row's own cited test does not count. One test may cover several
rows. All three ISAs are preferred.

Rows 1 to 3 may be carried from pass 7. They count as booted when the reviewer reads pass 7's
attack tests and, for each, either accepts it in writing (what it attacks and why that is enough)
or strengthens it and boots the result.

## The reviewer's brief

### Context

This is the nife project's own kernel. Its maintainers authorized this pass. Every attack test runs
only under QEMU, on the maintainers' machine or a cloud runner they started, in a scratch branch.
Its purpose is to falsify nife's published confinement claims before anyone else does: defensive
verification of the project's own code.

### Your job

You are an outside security reviewer. Break a published security claim of an operating system you
did not write. Assume every claim is false and look for the case that makes it so. An escape is the
most useful thing you can find.

nife is a capability microkernel in Rust for aarch64, riscv64 and x86_64. Its confinement claim: a
confined component cannot reach an object it was not granted. The numbered claims are in the table
at `<path>`. Work in `<worktree>` only.

### Posture

Blind: do not open `notes/confinement-claims.md` or `notes/confinement-claims/`,
`design/audit-reports/`, `design/fatal-risks/`, `design/roadmap/`, any note whose name contains
`outsider`, `falsif`, `adversarial`, `breaking` or `audit`, git history, or GitHub. Source, comments,
`BUGS` sections, `design/decisions/` and how-to notes are fair game; record any hint you meet.
Informed: you have the whole tree and its history, and you report a re-found fixed escape apart, as
a re-discovery.

### What counts

An attack counts when it boots. Write your own hostile program or system test, aimed at the case
you think breaks the claim, and boot it. Running a test that already exists is not an attack. Every
row except 12 to 18 must be booted for the pass to count: 27 of 34. Reading is the weakest grade.
Where a claim concerns a userspace server, build a hostile client that drives it.

Rows 1 to 3 have attack tests from an earlier pass in `system_tests/src/user/`:
`seventh_pass_delegation_tests.rs` (rows 1 and 2) and `seventh_pass_retype_tests.rs` (row 3). Read
each. Either accept it in writing, saying what it attacks and why that is enough, or strengthen it
and boot the result. Either way the row counts as booted.

### Mechanics

Read `notes/scripts.md` and `notes/qemu.md`. The two test files above are a working harness to copy:

- Layout: one module per attack family under `system_tests/src/user/`, declared in
  `system_tests/src/user.rs` under `#[cfg(test)]` with a doc comment saying which claims it
  attacks. Each test drives the real dispatcher through a local `call` helper over
  `syscall::invoke`.
- Names: new code spells `capability` out. Alias the kernel's `cap` names at the import, as those
  files do (`rendezvous_cap as rendezvous_capability`, a `const SEND_CAPABILITY`), because
  `script/lint` refuses new `cap` identifiers.
- Running: `script/test --arch aarch64 --test <module>`, then `riscv64`, then `x86_64`, one at a
  time and never in parallel.

Never run `script/verify`, a mutation sweep or Kani. Interactive runs go through
`helpers/qemu-bounded.sh`. Put `/opt/homebrew/opt/python@3.14/libexec/bin` first on `PATH`. Shape
an exploit so a break ends the run at an assertion, exit or fault. When you finish,
`pgrep -l qemu` shows nothing you started.

### Escapes

Each escape is a test that is red because of it, booted to confirm and committed before anything
else. Do not write the fix. A red test that would break the default suite is opt-in and says so. A
path one bug away, or holding by accident, is a near miss.

### Commits

Commit on the current branch; do not push, rebase or branch. One purpose each. Never use
`git stash`; never edit `design/`, `AGENTS.md` or `CLAUDE.md`. Run `cargo fmt` first.

### Output

Write to `<results path>`, outside the repository:

1. One row per claim: the attack, the outcome (escape, near miss, held, or untestable with the
   reason), the grade (`booted <isas>`, `host` or `read`), and the test.
2. The rows you attacked hardest, and why you chose them. Name the rows you only touched lightly
   too, so a shallow pass shows in your own report.
3. A section per escape and near miss, with the reproduction command and the observed lines.
4. Surfaces attacked that have no row.
5. A refusal log, last. List every step you declined, and every stop by a safety system or
   classifier, including one before any tool ran. Give the claim, one sentence with no exploit
   detail, and whose refusal it was. An empty log says "none".

Write plainly, with no em-dashes and American spelling. End with counts by outcome and by grade.
