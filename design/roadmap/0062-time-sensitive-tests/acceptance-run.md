# The 2026-08-17 acceptance run: an appendix to milestone 62

An appendix to [milestone 62 (tests that assert on time)](../0062-time-sensitive-tests.md), kept
apart under §212 (a prose budget). It is the block's account of its first acceptance run, which did
not pass, and of the kernel bug that run found.

## The acceptance run, 2026-08-17: 45 runs under load, 36 green

This is the evidence this block asks for, taken by `script/repeat-under-load` (one busy loop per host core,
the full suite, the load average sampled every ten seconds). It ran on an eight-core Mac at a one-minute
load average between 26.1 and 63.0, over 108 minutes. The full per-run table, the diagnosis of
every red, and the honest limits are in notes/load-sensitive-assertions.md. Nine of forty-five runs
went red, and the shape of those nine is the result rather than the count:

- Eight were two assertions, twice each, on both ISAs. `ticks_arrive_at_the_configured_rate`'s
  eight-attempt retry budget, and `the_handler_keeps_up_when_no_lock_is_held`'s taxonomy cut. Both
  were already named in that note's BUGS section as residuals that only the icount instrument can
  close, and neither had ever been observed before this run; one of the two entries called itself
  "rarer by orders" and has been corrected in place. That instrument now exists, which is what turns
  these eight reds from a wait into a decision. The perfect ISA symmetry is what says these are
  properties of the assertions rather than of either architecture.
- The ninth was a real kernel bug: a double free of a frame during `DESTROY`'s reclaim of a
  region whose resident was blocked in `recv` (riscv64, one occurrence). Recorded in the BUGS section
  of notes/object-revocation.md. *It got its own lane, and it is fixed: PR #316* (2026-08-18).
  It was not riscv64-only, which is the correction worth carrying rather than the fix: the
  ownership defect reproduces deterministically on aarch64, and riscv64 is merely where the timing
  exposed it. Two ingredients were needed. `user_aspace_create` builds an address space inside a
  region the caller already holds an `Untyped` to, where `AddressSpace::new` carves its own. Both
  stored a bare `u64` whose `Drop` called `untyped::destroy` unconditionally. That is two names for one
  run of memory, with a pin argument that holds only for a drop under `SCHED`, which
  `sched::finish_switch` releases before dropping. And `untyped::destroy` released the `REGIONS`
  lock between deciding and removing the slot, so two callers could both pass the refusal check.

  So this run's ninth red is gone and the eight timing reds are the whole of it, which makes
  this block a pure disposition question with no correctness bug behind it. That is a better
  outcome than it looks: the run's own conclusion was that "the one red worth reading arrived
  wearing the same color as eight that were not", and the eight are now dispositioned and the one
  is fixed.

Nothing was widened to make this green, and the run is the argument for not widening. The retry
budget's implicit second claim is already asserted properly by the sibling test, which passed in the
same log a line before the panic.

The most useful thing the run measured was not the load average. Runs sharing the host with
another lane's emulator went red 6 times in 17; runs with only their own went red 3 times in 28. Run
42 passed at the highest peak load in the table (63.0) and run 11 failed near the lowest (33.8). A
competing emulator predicts these failures where a load average does not.

