# Brief: an outsider pass at the confinement claim

**Provisional name.** Written 2026-10-12 (UTC) by milestone 903 (a seventh outsider pass attacks
the confinement claim)'s lane, after calef ruled on #2010 that a pass with 3 of 34 rows booted does
not count toward risk 7's criterion (c). The lane that runs the next pass hands the reviewer the
section "The reviewer's brief" below, with the posture filled in, and keeps the rest for itself.

## For the lane

- Mint the pass's block the way milestone 903 was minted, and claim it with
  `script/claim <branch> --new-work`.
- The posture is calef's for each pass, so take it from his ask. Blind is milestone 633 (an outside
  agent attacks the confinement claim)'s. Informed is milestone 800 (a non-Anthropic model attacks
  the confinement claim)'s.
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

**18 of the 34 rows, provisional** (proposed 2026-10-12 by milestone 903's lane, awaiting calef's
confirmation). A row counts as booted when a hostile test written for this pass, not a rerun of the
row's own test, ran under QEMU on at least one ISA the row applies to. One test may cover several
rows. All three ISAs are preferred, but one booted row on one ISA counts.

Why 18: rows 12 to 18 are host proofs of the DMA validator, the VT-d encoder and the wiring plan,
and no confined component drives them at boot, so a boot cannot reach them today. That leaves 27
rows a boot can reach, and 18 is two-thirds of them. That is most of the reachable table, while
still leaving room to grade a hard row `read` with a reason. When the table grows, the floor is
two-thirds of the rows a boot can reach, rounded up.

## The reviewer's brief

You are an outside security reviewer engaged by the maintainers to red-team their own kernel in
emulation (QEMU), on their machine, in a scratch branch. Break a published security claim of an
operating system you did not write. Assume every claim is false and look for the case that makes
it so. An escape is the most useful thing you can find.

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

An attack counts when it boots. Write your own hostile program or system test,
aimed at the case you think breaks the claim, and boot it. Running a test that already exists is
not an attack. At least 18 rows must be booted for the pass to count; reading is the weakest grade.
Where a claim concerns a userspace server, build a hostile client that drives it.

### Mechanics

Read `notes/scripts.md` and `notes/qemu.md`. Tests live in `system_tests/src/user/`.
Run one test with `script/test --arch <isa> --test <substring>`, one QEMU at a time. Never run
`script/verify`, a mutation sweep or Kani. Interactive runs go through `helpers/qemu-bounded.sh`.
Put `/opt/homebrew/opt/python@3.14/libexec/bin` first on `PATH`. Shape an exploit so a break ends
the run at an assertion, exit or fault. When you finish, `pgrep -l qemu` shows nothing you started.
New code spells `capability` out; alias the kernel's `cap` names at the import, as
`confined_fuzzer_tests.rs` does.

### Escapes

Each escape is a test that is red because of it, booted to confirm and committed
before anything else. Do not write the fix. A red test that would break the default suite is opt-in
and says so. A path one bug away, or holding by accident, is a near miss.

### Commits

Commit on the current branch; do not push, rebase or branch. One purpose each. Never
use `git stash`; never edit `design/`, `AGENTS.md` or `CLAUDE.md`. Run `cargo fmt` first.

### Output

Write to `<results path>`, outside the repository:

1. One row per claim: the attack, the outcome (escape, near miss, held, or untestable with the
   reason), the grade (`booted <isas>`, `host` or `read`), and the test.
2. A section per escape and near miss, with the reproduction command and the observed lines.
3. Surfaces attacked that have no row.
4. A refusal log, last: every step declined or stopped, by you or by a safety system, with the
   claim, one sentence with no exploit detail, and whose refusal it was. An empty log says "none".

Write plainly, with no em-dashes and American spelling. End with counts by outcome and by grade.
