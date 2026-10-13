---
experiment_status: RUN
experiment_run: 2026-08-31
---
# 1. Only software written for nife runs on nife

*Risk 1 of [the nine](README.md). The status vocabulary, the rule an entry meets and the running order are there.*

The claim, stated so it can fail: the platform runs hand-written Rust and nothing else, so every
piece of software anyone wants has to be rewritten. It is the most dangerous entry because it is
structural. Optimization cannot fix "nothing runs here". A system in this state is a research
demonstrator forever, which is not what DECISIONS §14 (a verified-Rust capability microkernel that
runs real workloads) claims.

**The experiment:** milestone 121 (`ripgrep`: enumeration as a capability), for its real dependency
tree, its filesystem walk and its threads.

GREEN since 2026-10-12 (UTC), ruled by calef once all three gaps below met his conditions. Asked
whether it goes green now or stays amber until milestone 810 (`ripgrep` is packaged in basalt and
installed with `jig`), he answered: *"Track packaging separately. We already do."* (recorded on #2016).
Packaging is tracked by milestones 810 (`ripgrep` is packaged in basalt) and 897 (SQLite is packaged
in basalt), and by a decision in flight (#2023) that foreign programs reach nife through `jig` from
basalt.

Earlier verdicts: amber from 2026-10-06 (green on all three architectures from 2026-09-16): a stranger's program
builds and runs here unmodified, and has never yet done its job. The blocker was not what anyone
predicted. Unmodified `ripgrep` 14.1.1, forty transitive crates, zero
patches, and three byte-identical transcripts from three separately built binaries
([`notes/ripgrep-on-nife.md`](../../notes/ripgrep-on-nife.md)). What stopped it was the missing
argument vector, milestone 205 (how a foreign program is told what to do). Correction, 2026-09-27:
205 is BUILT, and what keeps `ripgrep` from the prompt now is the 256 KiB image ceiling (#1399).

Correction, 2026-10-06 (UTC), from `notes/ripgrep-on-nife.md`'s own `BUGS` and
`system_tests/src/user/ripgrep_tests.rs`: `rg` has never searched. Of the three things it was chosen
to exercise, it reached one, the dependency tree. It stopped at argument parsing, so its walk never
ran and its threads were never asked for. The walk was run instead by an in-tree `std` walker
(`crates/walk_pricing`), which is our code. Milestones 205 and 206 are BUILT, yet the harness still
starts `rg` with no arguments, so nothing has yet tried a search. When one is tried, PR #1777
measured `rg --threads 1 --no-mmap` over this tree peaking at 3.0 MiB on macOS, against a `std`
heap of about 1 MiB.

Correction, 2026-10-07 (UTC), from milestone 595 (the shell runs a `std` program)'s block and
`script/swish-check` on aarch64, riscv64 and x86_64 (lane `milestone/595-rg-at-prompt`): `rg` has
now searched, from the prompt. Unmodified `ripgrep` 14.1.1, run by its path and unvouched, was
told `rg needle docs ../hay/secret.txt` and printed `docs/n.txt:find the needle here`, the one
match in the directory the word `docs` granted. It could not open `../hay/secret.txt`, which holds
the needle too: the caretaker refused the `..`. The transcript is byte-identical on the three
architectures. The 256 KiB ceiling named above was raised to 4 MiB by milestone 595 on 2026-09-27,
and the two last steps were a manifest note linked into `rg` and a disk copy without its symbol
table. Two limits on what this shows: the lines run only where nife's ripgrep build script was run,
never in CI (§46 (thin primitives or whole subsystems)), and one file in one directory is not the
walk `ripgrep` was chosen to exercise. That walk is measured in the kernel harness,
`notes/ripgrep-on-nife.md`.

Added 2026-10-07 (UTC), from milestone 121 (`ripgrep`: enumeration as a capability)'s block and
`script/swish-check` on aarch64, riscv64 and x86_64 (lane `milestone/121-rg-walk`, #1845): `rg` has
now walked a real tree at the prompt. In `/search`, holding a copy of this repository's `crates/`
(661 files, 275 directories, 8,202,706 bytes, nested five deep), the gate typed
`/installed/rg BUGS crates` with no flags. On each architecture `rg` printed 349 lines from 136
files, and the gate compared them line for line with what `rg` 14.1.1 built for the host printed
over the same copy: no line missing, none extra. The walk took 14.9 s on aarch64 (HVF), 40.8 s on
riscv64 and 345.8 s on x86_64 (both TCG). The same limits as above still hold: it runs only where
nife's ripgrep build script was run, never in CI, and `rg` sits on the data disk rather than being
installed by `jig`. Its peak memory at the prompt is unmeasured; it finished within the `std` heap.

Ruled 2026-10-08 (UTC), on the evidence above. calef: *"I would keep it amber and we prioritize
the path to green."* The verdict stayed amber. The path to green was three gaps, each with an owner:

1. Nothing re-checked the claim. Owner: lane `milestone/121-rg-ci` (#1853). Its `swish-check` rows
   build `rg` in CI, type the walk on every pull request, and refuse to skip it there.
2. One program is one data point. A second foreign program, unrelated to `ripgrep` and ideally not
   written in Rust, is untested. Closed 2026-10-12 (UTC), calef: *"835's speedtest1 and ioping
   count as risk 1's second foreign program."* Milestone 835 (a C library, stage 1: files, clock and memory) runs both
   unmodified (#1896).
3. Threads were never exercised. nife answers `available_parallelism()` with `Ok(1)`, and §105
   declines `std::thread::spawn`, so no foreign program here has run a second thread. Closed
   2026-10-12 (UTC) by #2016 (below), with calef's ruling above. Fact, 2026-10-12: milestone 812 (`std::thread::spawn` runs
   real threads in one address space) is BUILT (#1892), and 836 (a C library, stage 2: threads) is
   the C half, claimed today. calef, the same day, on what closes this gap: *"the path to green on
   risk 1 requires a foreign program running a second thread."* A thread library alone does not.
   Fact, 2026-10-12 (UTC), lane milestone/121-rg-threads: unmodified `ripgrep` 14.1.1 ran its
   parallel walker on five threads, its main thread and four workers, on aarch64, riscv64 and
   x86_64 at `-smp 4`, both with no thread flag and with `--threads 4`. The count is the kernel's,
   read from the process after it ended, not the program's. Each run printed exactly the tree's 138
   match lines, which are the host `rg`'s lines once both are sorted. CI's `rg-threads` rows re-check
   it on every pull request and fail at one thread
   ([`notes/ripgrep-on-nife.md`](../../notes/ripgrep-on-nife.md)). The parallel walk needs the
   `ignore` patch described below; `ripgrep`'s own source is untouched. Under §216 (fatal-risk
   facts are correctable, and verdicts are the architect's) the verdict is calef's, quoted above.

DECISIONS §105 (`std::thread::spawn` stays declined, until a customer needs it) was never reached,
and that reverses the premise. `ripgrep` asks `available_parallelism()` rather than assuming it, and
nife answers `Ok(1)` honestly. A platform answering `Unsupported` there would have failed this
program.

The caveat. The structural fear is retired. The one published argument that speaks to this says it
goes badly: clean-slate kernels have *"significantly fewer features than Linux ... impeding
adoption"*, risk 8's paper ([`notes/incremental-path.md`](../../notes/incremental-path.md)).
[Appendix](somebody-elses-software.md).

2026-10-08 (UTC): threads are to be built (§263 (threads are built), milestone 812
(`std::thread::spawn` runs real threads in one address space)), so `ripgrep`'s threads become
reachable; `fork` is refused and spawn supported (§264 (`fork` is declined for good), milestone 172
(a capability-native subprocess primitive)).

2026-10-10 (UTC): milestone 812 is built. `std::thread::spawn` runs real threads on all three
architectures, and unmodified `rayon` from crates.io runs its parallel sum on every core. That
moves Rust software that uses threads, `ripgrep`'s parallel walk among it, from refused to
reachable. It does nothing for C software, which still waits on a C library with threads (milestone
836 (a C library, stage 2: threads)), so the risk's status is unchanged. The "zero patches" above
no longer holds for the whole tree: with threads, `rg` walks in parallel, and its `ignore`
dependency needed a ten-line build-time patch for that on nife (calef, #1892). `ripgrep`'s own
source is still untouched, and the patch is nife's to maintain until upstream fixes the bug
([`notes/ripgrep-on-nife.md`](../../notes/ripgrep-on-nife.md)).

Fact, 2026-10-10 (UTC), from milestone 835 (a C library, stage 1: files, clock and memory)'s block
and #1896: two programs that are neither Rust nor related to `ripgrep` now run unmodified here.
SQLite 3.50.4's `speedtest1` and ioping 1.3 run under QEMU on all three architectures, and
`speedtest1` prints the same verification hash as on macOS. Like `rg` before #1853, they run only
where their helpers were run, never in CI. Whether this closes gap 2 is calef's, under §216
(fatal-risk facts are correctable, and verdicts are the architect's), so the verdict stays AMBER.

Fact, 2026-10-12 (UTC): every foreign program this page counts runs today from a binary a
`helpers/build-*.sh` script built, not from a package. That is `rg`, `speedtest1` and ioping on
`main`, and hackbench once #2009 (milestone 836 (a C library, stage 2: threads)) lands. calef ruled
the same day that every program reaches nife that way: §297 (software reaches a nife system as a
package `jig` installs from basalt's index, the kernel included). Their packaging is milestone 810 (`ripgrep` is packaged in basalt and installed with
`jig`), milestone 897 (SQLite is packaged in basalt, and its tests run in basalt's package build),
milestone 906 (ioping is packaged in basalt, and its test installs it with `jig`) and milestone 907
(hackbench is packaged in basalt, and its test installs it with `jig`). The verdict is unchanged.

## Benchmarks that inform this risk

As of 2026-10-08 (UTC), under §262 (nife is measured with the field's standard benchmarks).
Each is to run unmodified on nife and on Linux, and none has produced a number yet. No port waits
on a C library: milestone 835 (#1896) built one from relibc, as §265 (a C library started from
relibc) ruled, and `speedtest1` and ioping already run on nife.

- [Milestone 831 (SQLite's speedtest1 on nife and Linux)](../roadmap/0831-sqlite-speedtest1-on-nife-and-linux.md). Runs on nife since 835; the Linux row is open.
- [Milestone 833 (the same storage benchmark on nife and Linux, by porting real fio)](../roadmap/0833-the-same-storage-benchmark-on-nife-and-linux.md). Waits on threads too.
- [Milestone 834 (ioping on nife and Linux)](../roadmap/0834-ioping-on-nife-and-linux.md). Runs on nife since 835; the Linux row is open.
