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

Raised on 2026-10-12 (UTC) by lane `milestone/836-a-c-library-stage-two-threads`. It built POSIX
threads for nife's C library and found that a C program's own thread-locals cannot be linked.

Reuse: the layouts are the ELF TLS ABI's. musl's and relibc's `__init_tls` show how a C library
copies the template into each thread's block. Neither can be taken whole, because the block must
also be `std`'s, so the copy is written against `std`'s `thread_local/key/nife.rs`.

## What is missing

A C variable declared `__thread` or `_Thread_local` is ELF thread-local storage. The compiler
addresses it as an offset from the thread pointer (`TPIDR_EL0`, `tp`, the `FS` base). The linker
lays the variables out in a `PT_TLS` segment, and a C library copies that into a block for each
thread.

On nife the thread pointer already names `std`'s own per-thread block. That is
`patches/std-nife/overlay/std/src/sys/thread_local/key/nife.rs`, under §269 (how threads share a
process) fork 4. The userspace link script has no `PT_TLS` segment. So a C program with one
thread-local does not link:

```
rust-lld: error: ...(tls.o) has an STT_TLS symbol but doesn't have a PT_TLS segment
```

That is the right failure, since nothing is silently overwritten, and `c_library/README.md`'s
`BUGS` records it. Milestone 836 (a C library, stage 2: threads) moved relibc's own thread-locals
onto `std`'s `thread_local!`, so the library itself needs none of this.

## Who needs it

fio, milestone 833 (the same storage benchmark on nife and Linux, by porting real fio). Its sources
use `__thread` in `stat.h`, `gettime.c` and `verify-state.h` (GitHub code search, 2026-10-12).
Other ported C will too.

## What it would take

- A `PT_TLS` segment in the userspace link script. The process must learn the template's address
  and sizes at start, from its own program headers or from symbols the link script defines.
- One layout per architecture that holds both `std`'s block and the ELF block where each ABI puts
  it. aarch64 and riscv64 use variant I: the data follows the pointer (after a 16-byte control
  block on aarch64). x86_64 uses variant II: the data sits below the pointer, whose first word
  points at itself, as `std`'s block already does.
- `std`'s `Thread::new` and the main thread's setup copying the template into each new block.

This changes what the thread pointer points at. `std`, the kernel's `CONFIGURE` and
`SET_THREAD_POINTER` all agree on that today, so it is a §269 fork 4 question for calef before it is
code.

## Index row

C's own thread-locals, `__thread` and `_Thread_local`, on the thread pointer `std` already
switches, so fio and the C programs after it link.
