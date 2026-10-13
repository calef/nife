---
status: NOT-STARTED
raised: 2026-10-12
promoted_from: c-thread-locals-on-the-thread-pointer
milestone_dependencies: 836
decision_dependencies: 269
machine_requirements: none
specific_machine: none
needs_person: no
---
# 912. C thread-locals on nife's thread pointer

*(Promoted 2026-10-12 (UTC) by lane `milestone/912-c-thread-locals-on-the-thread-pointer` from the
proposal `c-thread-locals-on-the-thread-pointer`, after calef approved the lane that day. The
number is provisional until the merge queue lands it; the title and slug are drafts.)*

Raised on 2026-10-12 (UTC) by lane `milestone/836-a-c-library-stage-two-threads`. It built POSIX
threads for nife's C library and found that a C program's own thread-locals cannot be linked.

Reuse: the layouts are the ELF TLS ABI's. musl's `__init_tls` and `pthread_impl.h` show how a C
library copies the template into each thread's block and where its own per-thread record sits
beside it. Neither can be taken whole, because the block must also be `std`'s, so the copy is
written against `std`'s `thread_local/key/nife.rs`.

Not started, and held for one question to calef (below). Nothing is built until he rules.

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

## Who needs it: not fio, corrected 2026-10-12 (UTC)

The proposal said fio (milestone 833 (the same storage benchmark on nife and Linux, by porting real
fio)) needs this, citing `__thread` in `stat.h`, `gettime.c` and `verify-state.h`. Read in fio 3.40's
source (`ff930c4653ae`), that is too strong:

- `verify-state.h` has no thread-local. It has a function named `__thread_io_list_sz`, which a
  code search for `__thread` matches. `stat.h` has none either.
- `gettime.c` has the only one in a default build, `static __thread struct tv_valid
  static_tv_valid`, under `CONFIG_TLS_THREAD`. fio's `configure` sets that only when a probe
  program with a `__thread` variable compiles *and links*. On nife it does not link, so `configure`
  turns it off, and `gettime.c` falls back to `pthread_getspecific`, which milestone 836 built.
  `--disable-tls` forces the same.
- `engines/ime.c` has the other, and that engine (DDN's IME) is not built without its library.

So fio builds without this milestone, at the cost of one `pthread_getspecific` per `fio_gettime`
call when the CPU clock is the source, against a Linux build that uses `__thread`. That is a few
nanoseconds against microseconds per I/O, and it counts against nife, not for it: a caveat on
milestone 833's numbers. This milestone no longer gates fatal risk 6. It is for the next C
program that has no such fallback, and no program on the roadmap is yet known to be one.

## What it would take

- A `PT_TLS` segment in the userspace link script (`crates/user_mode_runtime/link.ld`), with
  `.tdata` and `.tbss` in the `data` segment. The process learns the template's address and sizes
  from symbols the link script defines, since nife passes no auxiliary vector.
- One layout per architecture that holds both `std`'s block and the ELF block where each ABI puts
  it. Which layout is the question below.
- `std`'s `Thread::new` and the main thread's setup copying the template (the file bytes, then
  zeros for `.tbss`) into each new block.
- The main thread's block made eagerly at start when the program has a `PT_TLS` segment, instead
  of on first use: C code may touch a thread-local before `std` has. A program with no segment
  keeps today's lazy path.
- A test on all three architectures: a C program with an initialized `__thread`, a `.tbss` one and
  a 64-byte-aligned one, read from the main thread and two spawned ones, each seeing its own copy.

The kernel changes nothing. `CONFIGURE` and `SET_THREAD_POINTER` store a user-half value and
install it at each switch; neither reads what it names (`kernel/src/sched/configure.rs`,
`is_thread_pointer`). The proposal said the kernel agrees with `std` on what the pointer names.
It does not, which narrows the question to `std`.

## What the toolchain already fixes

Measured 2026-10-12 (UTC) with Homebrew clang 23.1.3 and the pinned nightly's `rust-lld`, linking
three thread-locals (8 bytes initialized, 40 of `.tbss`, 8 aligned to 64) statically on each target
with the C library's own flags:

| | variant | first variable's address | what the code also reads |
|---|---|---|---|
| aarch64 | I | pointer + 0x40 (16 bytes reserved, rounded up to the 64-byte alignment) | nothing |
| riscv64 | I | pointer + 0 | nothing |
| `x86_64` | II | pointer - 0xc0 (the block ends at the pointer) | `fs:[0]`, which must hold the pointer itself |

Every C object compiled for nife bakes these offsets in, and nife cannot choose them: the
processor-specific ABIs and LLVM do. That part is irreversible in the sense that matters, and it
is not a fork. Three facts follow:

- On aarch64 and riscv64, nothing the compiler emits reads below the pointer.
- On `x86_64`, nothing it emits reads above the pointer except the self-pointer at `fs:[0]`,
  which `std`'s block already has as its first field.
- The pointer must be aligned to the segment's alignment on every architecture.

## Open questions for calef

1. Where does `std`'s block sit relative to the ELF block? §269 fork 4 ruled one thread-pointer
ABI, set by the kernel. This keeps that contract and decides what the value names. Only `std` reads
`std`'s part, and `std` is rebuilt with every program, so whichever option is taken can be changed
later by rebuilding; the part other code acts on is the table above, which is not a choice.

- **(a) One allocation per thread, `std`'s block on the side the ABI leaves free.** On aarch64 and
  riscv64 the block ends at the pointer and the ELF block starts above it: `[std block][pointer][16
  reserved bytes on aarch64][ELF data]`. On `x86_64` the ELF data ends at the pointer and `std`'s
  block starts there, `this` first, as today. `std` finds its block at the pointer minus a constant
  on the first two and through `fs:[0]` on the third. Cost: no extra instruction, since the
  constant folds into the load; the hot fields (the allocation cache) go at the end of the block,
  nearest the pointer, inside riscv64's 2 KiB signed immediate and aarch64's 256-byte unscaled one.
  Each spawn copies the template, a few bytes for most programs. musl does exactly this (its
  `struct pthread` sits below the pointer under `TLS_ABOVE_TP` on aarch64 and riscv64, at it on
  `x86_64`; read 2026-10-12). Fuchsia does the same with its runtime's words: the stack guard and
  unsafe stack pointer are at -0x10 and -0x8 on aarch64 and riscv64, +0x10 and +0x18 on `x86_64`
  (`zircon/tls.h`, read 2026-10-12).
- **(b) A pointer to `std`'s block in a word beside the ELF block.** On aarch64, in the 16 reserved
  bytes; on riscv64, which reserves none, at the pointer minus 8. `std`'s block stays a separate
  allocation. relibc does this on aarch64 (the word at the pointer names its control block, which
  sits after the TLS; `src/ld_so/tcb.rs`, read 2026-10-12). Cost: one more dependent load on every
  `std` thread-local access, the allocation cache included, and two allocations a thread. It loses
  to (a) on every count except keeping the block's allocation unchanged.
- **(c) The pointer names only what the ABI defines, and `std`'s block becomes an ELF
  thread-local.** The target specs gain `has-thread-local`, every `thread_local!` in `std` and in
  programs becomes a native one at a fixed offset, and the key table goes away with its 250-key
  limit (that file's `BUGS`). sel4runtime does this: its per-thread state, such as the IPC buffer
  pointer, is itself a `__thread` variable, and the area at the pointer is only the ABI's
  (`src/env.c`, read 2026-10-12). Rust's `std` on Linux reaches its thread data the same way. Cost:
  `std`'s thread-local backend changes from keys to native, which 812 chose against, and a
  destructor list replaces the key table's. Access to a `thread_local!` gets cheaper (one
  pointer-relative load against a block lookup, a bounds assertion and a slot load); the
  allocation cache stays one load either way.
- **(d) Change nothing; compile C with `-femulated-tls`.** Each access calls compiler-rt's
  `__emutls_get_address`, which sits on `pthread_key`. No layout question at all. Cost: a call
  where (a) has three instructions (measured above on all three targets), a flag every port must
  carry, and any object compiled without it still fails to link. Android used it before API level
  29 and OpenBSD for years (recalled, not read). It is a workaround, and refused for that reason
  unless calef wants nothing on the pointer to change.
- **(e) A second register for `std`.** aarch64 has `TPIDRRO_EL0`; riscv64 and `x86_64` have no
  free one (`GS` is the kernel's, `notes/thread-pointer.md`). Refused on parity, §19 (architectural parity is a tenet).

Recommendation: (a). It changes nothing the kernel or any test of the contract sees, costs
nothing on the paths that are hot today, and is musl's and Fuchsia's shape. Asked whether (a)
would still win if (c) cost the same: not entirely. At equal cost, (c) is the better end state for
Rust's own thread-locals, faster and with no key limit. Preferring (a) here is about scope, and so
partly about effort: (c) also replaces `std`'s thread-local backend, which wants its own
measurement. (a) does not foreclose (c), because (c) changes only where `std` keeps its own data,
which (a) leaves private to `std`.

If calef says no to all of (a) to (c), C thread-locals stay unlinkable, `BUGS` keeps saying so,
and (d) remains available per program.

## Index row

C's own thread-locals, `__thread` and `_Thread_local`, on the thread pointer `std` already
switches, so C programs that need them link. fio does not need it after all: its `configure`
falls back to `pthread_getspecific`.
