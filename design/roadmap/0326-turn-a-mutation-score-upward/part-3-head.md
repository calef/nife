# Part 3's head, 2026-09-20: an appendix to milestone 326

An appendix to [milestone 326 (nobody has been assigned to turn a mutation score
upward)](../0326-turn-a-mutation-score-upward.md). It is the block's record of the seven crates part
3 named: the per-crate table, the two artifacts, the deadlocks, what was found and the one exclusion
added.

## Part 3's head, 2026-09-20

The seven crates this block names are triaged, and none carries an untriaged survivor. Measured
per crate with `script/mutation -p <crate>` on the lane's own worktree, before and after; every kill
was verified by re-running the sweep and watching the mutant die, and every equivalence claim is a
mutant the second run still reports. The reasons are in
`notes/mutation-testing/new-crate-backlog.md`'s `## 2026-09-20` section, crate by crate, which is
where a reader should go to disagree with one.

| crate | before | after | killed by a test | equivalent | excluded | recorded gap |
|---|---|---|---|---|---|---|
| `work_steal_slot` | 0 | 0 | 0 | 0 | 0 | 0 |
| `memory_corruption_canary_gate` | 8 | 2 | 6 | 2 | 1 | 0 |
| `soak_page` | 7 | 0 | 7 | 0 | 0 | 0 |
| `jh7110_entropy` | 23 | 10 | 13 | 6 | 0 | 4 |
| `multicast_dns_protocol` | n/a | n/a | n/a | n/a | n/a | n/a |
| `job_mix` | 10 | 4 | 6 | 3 | 0 | 1 |
| `schedule_store` | 8 | 0 | 8 | 0 | 0 | 0 |
| total | 56 | 16 | 40 | 11 | 1 | 5 |

`before` and `after` count missed plus timeouts, which is what `script/mutation --report` lists as
the survivors themselves.

Two of the seven numbers in the table above were artifacts, and this block predicted one of them.
`work_steal_slot`'s 54.2% was its loom model, which part 1's `interleavings::` entry removes. It now
reports 14 mutants, 13 caught, 1 unviable and nothing owed. `multicast_dns_protocol` is the
census's `mdns_proto`, renamed by milestone 265 (`_proto` is a truncation). It is not in this tree
at all. Milestone 298 (retire the multicast DNS responder and its two crates) retired it and its
sibling on 2026-09-15, the day after the census measured them, so its 82 survivors close by
deletion. The other flagged crate went the opposite way: `memory_corruption_canary_gate` was
genuinely 50.0% once its loom mutants left the count, worse than the 66.7% it was flagged at.

Eight of the fifty-six were deadlocks rather than wrong answers. A loop that waits is broken by
making it never accept; a loop that gathers is broken by making it never advance. Neither returns,
so the suite's answer is to hang, and cargo-mutants can only call a suite that did not finish a
timeout. This block's own posture is that such a timeout is the tests noticing rather than missing,
and that stands. What part 3 adds is that noticing by hanging is worth converting into noticing by
failing where the crate allows it. `memory_corruption_canary_gate` now runs each test body on a
worker with a deadline (four converted). `job_mix`'s hand-rolled index became a `for` (one removed
at rung one). `jh7110_entropy`'s four were left because `Pool`'s doctest calls `take` directly, a
hang confirmed by hand-applying the mutant. A doctest can carry a deadline, with hidden `# ` lines
that `recv_timeout` a worker (#1323).

Nothing found here is a defect in shipped behavior, which is the answer to the question this
milestone exists to ask. The closest are three places where a plausible-looking mistake was
unguarded. `soak_page`'s offsets could collapse two workers onto one word with every test green.
`schedule_store`'s buffer bound could index one past a caller's array. And `job_mix`'s budget could
size a task nine pages below what `MAP` needs while a compile-time assertion beside it still passed.
All three are now checked. One real limitation was found and recorded rather than fixed:
`job_mix::order(2k)` and `order(2k + 1)` are the same permutation, in a `BUGS` section on the
function, unreachable because the kernel hands out odd seeds.

`.cargo/mutants.toml` gains one entry, `tests::`, and it is measured rather than assumed.
Cargo-mutants skips a plain `#[cfg(test)]` module but not the `#[cfg(all(test, not(loom)))]` the
five loom crates have to write. So a helper in one of those is mutated where the identical helper
elsewhere is not.
