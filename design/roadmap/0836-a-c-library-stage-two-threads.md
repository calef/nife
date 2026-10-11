---
status: NOT-STARTED
raised: 2026-10-08
milestone_dependencies: 835, 812
decision_dependencies: 265
machine_requirements: none
specific_machine: none
needs_person: no
---
# 836. A C library, stage 2: threads

*(Minted 2026-10-08 (UTC) by lane/c-library from calef's ruling the same day; number provisional
until the merge queue lands it.)*

Stage 2 of §265 (a C library started from relibc): `pthread_create`, `pthread_join`, mutexes,
condition variables, thread-local storage for more than one thread, and the thread-safe `stdio`
locking relibc already has. It rides on milestone 812 (`std::thread::spawn` runs real threads in one
address space), which PR #1856 rules is to be built.

## First consumers

hackbench (827) in its thread mode, schbench (830), iperf3 (828) and the storage
benchmark (833), all on PR #1854.

Reuse: relibc's `src/pthread/` and `src/sync/` (MIT), seeded as stage 1 seeded the headers.

## What it builds

relibc's `src/pthread/` and `src/sync/`, seeded as stage 1 seeded the headers, over two platform
calls: thread creation (relibc's `rlct_clone`) and a wait-on-address pair (its `futex_wait` and
`futex_wake`). Milestone 812 provides that pair, so `src/sync/` is kept nearly whole: corrected
2026-10-11 (UTC), since 812 (#1892) built `abi::address_space::WAIT` and `WAKE`, an address-keyed
futex (`notes/futex.md`). It has no timeout yet, which a timed wait such as
`pthread_cond_timedwait` will meet.
`TPIDR_EL0`, `tp` and the x86_64 FS base become per-thread state, which is 812's to switch.

## Done when

hackbench's thread mode and schbench build unmodified and run on all three architectures.

## Index row

POSIX threads for nife's C library, on milestone 812's threads, so the scheduling benchmarks can
run as their authors wrote them.
