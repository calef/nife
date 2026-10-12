---
status: PROPOSED
raised: 2026-10-12
milestone_dependencies: 836
decision_dependencies: 269
machine_requirements: none
specific_machine: none
needs_person: no
---
# C thread-locals (`__thread`) on nife's thread pointer

Raised on 2026-10-12 (UTC) by lane `milestone/836-a-c-library-stage-two-threads`, which built POSIX
threads for nife's C library and found that a C program's own thread-locals cannot be linked.

Reuse: the layouts are the ELF TLS ABI's, and musl's and relibc's `__init_tls` and dynamic-linker
code show how a C library copies the template into each thread's block. Neither can be taken
whole, because the block must also be `std`'s, so the copy is written against `std`'s
`thread_local/key/nife.rs`.

## What is missing

A C variable declared `__thread` or `_Thread_local` is ELF thread-local storage. The compiler
addresses it as an offset from the thread pointer (`TPIDR_EL0`, `tp`, the `FS` base), and the
linker lays the variables out in a `PT_TLS` segment that a C library copies into a block for each
thread. On nife the thread pointer already names `std`'s own per-thread block
(`patches/std-nife/overlay/std/src/sys/thread_local/key/nife.rs`, under §269 (how threads share a
process) fork 4), and the userspace link script has no `PT_TLS` segment. So a C program with one
thread-local does not link:

```
rust-lld: error: ...(tls.o) has an STT_TLS symbol but doesn't have a PT_TLS segment
```

That is the right failure (nothing is silently overwritten), and `c_library/README.md`'s `BUGS`
records it. relibc's own thread-locals were moved onto `std`'s `thread_local!` by milestone 836 (a C
library, stage 2: threads), so the library itself needs none of this.

## Who needs it

fio, milestone 833 (the same storage benchmark on nife and Linux, by porting real fio). Its sources
use `__thread` (`stat.h`, `gettime.c`, `verify-state.h`, found by GitHub code search on
2026-10-12). Other ported C will too.

## What it would take

- A `PT_TLS` segment in the userspace link script, and the template's address and sizes known to
  the process at start (from its own program headers, which the loader maps, or a symbol the link
  script defines).
- One layout per architecture that holds both `std`'s block and the ELF block at the offsets each
  ABI fixes: variant I on aarch64 (a 16-byte control block at the pointer, the data after it) and
  riscv64 (the data at the pointer), variant II on x86_64 (the data below the pointer, whose first
  word points at itself, which `std`'s block already does).
- `std`'s `Thread::new` and the main thread's setup copying the template into each new block.

This changes what the thread pointer points at, which `std`, the kernel's `CONFIGURE` and
`SET_THREAD_POINTER` all agree on today, so it is a §269 fork 4 question for calef before it is
code.

## Index row

C's own thread-locals, `__thread` and `_Thread_local`, on the thread pointer `std` already
switches, so fio and the C programs after it link.
