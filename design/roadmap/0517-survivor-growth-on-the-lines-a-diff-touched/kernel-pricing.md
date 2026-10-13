# What it would take to put `kernel/**` and `components/**` in the corpus

An appendix to [milestone 517 (what fraction of survivor growth arrives on lines a pull request
touched)](../0517-survivor-growth-on-the-lines-a-diff-touched.md). The main page is complete without
it, and carries the verdict this working leads to. This is the pricing behind that verdict.

## The question

This is the third objection of milestone 438 (would a diff-scoped mutation check have caught the
55) and the one that decides whether an inflow gate can carry
risk 3 at all: a kernel change passes by construction today, because `.cargo/mutants.toml` excludes
both trees. The honest answer is that it is mechanically possible and nobody has to invent anything.
The price is between two and three orders of magnitude.

The corpus would roughly double. `cargo mutants --no-config --list` generates 7,529 mutants
for `kernel` and 3,482 for `components`, against a census corpus of about 9,300 viable mutants
today.

## The mechanism already exists and is not the problem

`.cargo/config.toml` sets a `runner` for each bare-metal target, so `cargo test -p kernel --target
aarch64-unknown-none-softfloat` boots QEMU and runs the suite; cargo-mutants takes
`-C --target=...` for exactly this. The exclusion's stated reason, that these are "crates whose
lines cannot execute on the host", is true and is about the *host*, not about testability.

## The price is the test command, and it is a wall-clock problem

It is not a harness problem. The kernel suite is one QEMU boot that runs every test.
`xtask/src/suite.rs` records the aarch64 leg at about 53 seconds for 312 tests, and
`kernel/build.rs` records a kernel relink at about 2.3 seconds. So a kernel mutant costs about 55
seconds, against the fraction of a second a host crate's mutant costs.

| | mutants | at ~55s each, serial | per architecture, 8 shards |
|---|---|---|---|
| `kernel` | 7,529 | 115 hours | 14 hours |
| `components` | 3,482 | 53 hours | 7 hours |
| both | 11,011 | 168 hours | 21 hours |

§19 (architectural parity is a tenet) makes it a gate, so multiply by three architectures. That is
about 500 hours of runner time per census, against the 52 minutes the whole host corpus costs today.
A 21-hour shard also exceeds GitHub's per-job ceiling, so the sharding would have to be roughly
quadrupled before the run is even expressible.

## Three things make it worse, and one makes it better

- A kernel mutant that hangs the boot costs the timeout, not the suite. Mutating a scheduler or
  an MMU path does not fail a test, it wedges the machine, and the suite has no way to fail fast. On
  the host corpus, timeouts are already the dominant cost of the one parser change milestone 438
  measured.
- `components` has no tests of its own: zero `#[test]` and zero `#[cfg(test)]` in
  `components/src`. What proves it is `kernel/src/user/tests.rs`, 3,195 lines of kernel-side tests
  that boot the whole system and run the programs. cargo-mutants can express that
  (`--test-package kernel`), so the mutant is testable, but every components mutant pays the full
  system boot too.
- The kernel test leg is not `cargo test`, it is `cargo xtask test`, and the difference is
  scaffolding. `suite.rs` builds the std exerciser, every user program, the initrd, the RedoxFS
  images, a GPT disk, a blank disk and an NVMe image. It sets `NIFE_GPU`, `NIFE_KEYBOARD`,
  `NIFE_RNG` and `NIFE_NVME` before the boot. Several tests assert those devices are present
  rather than skipping. In a cargo-mutants build directory none of that exists, so the unmutated
  baseline fails and the run refuses to start. Making the runner self-sufficient under a mutation
  flag is real work, and it is the smallest piece of this.
- The one thing in its favor: `--in-diff` makes the wall clock scale with the diff rather than
  the corpus, so a *diff-scoped* kernel check is affordable where a kernel census is not. A pull
  request touching ten kernel lines is a handful of mutants at 55 seconds, which is minutes. The
  expensive thing is the standing corpus, not the derivative.
