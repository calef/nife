# What fio needs from nife's C library

*Name: provisional (a lane ships names provisionally; an architect names things). Written
2026-10-12 (UTC) by lane `lane/fio-c-library-needs` for milestone 833 (the same storage benchmark
on nife and Linux, by porting real fio), so the port starts from a known list instead of a first
compile.*

The source is fio 3.40 (`ff930c4653ae`, the release the Linux side of fatal risk 6 ran), compiled
for `aarch64-unknown-nife` against `c_library` and `vendor/relibc/include` with
`helpers/c-library-cflags.sh`. Nothing was linked into a program and nothing ran on nife. riscv64
and `x86_64` were not compiled; the C library is the same on all three, so the list should hold.

## How the list was made

fio's `configure` decides features by compiling and linking small probes, so it was run with a
compiler wrapper that tells it the truth about nife. The wrapper compiles with the C library's
flags. It refuses a link in three cases: a symbol is undefined in both the objects and `c_library`,
an object carries a thread-local, or the link asks for a library other than `pthread`, `m`, `rt`,
`dl` or `c`. The defined set (1,972 names) came from `llvm-nm` over `c_library`'s build of a trivial
program through `helpers/build-c-program.sh`. A stand-in `uname` answered `nife`, and a throwaway
`os/os-nife.h` gave fio an OS layer. Missing headers were stubbed with Linux declarations, round by
round, until every object compiled; then the undefined symbols were collected.

## Thread-locals: not needed

fio's one thread-local in a default build is in `gettime.c`, under `CONFIG_TLS_THREAD`.
`configure` sets that only when its `__thread` probe links, and on nife it does not, so fio uses
`pthread_getspecific` instead. C thread-locals on nife's thread pointer, proposed in pull request
#2036, are not on fio's path. The cost is one key lookup per `fio_gettime` when the CPU clock is the source, which a
Linux build does not pay. It counts against nife, and it belongs in 833's caveats.

## What is missing

Two bugs in the C library's headers, which any program can hit:

- `stdlib.h` uses `__noreturn` and `__deprecated` without including `features.h`. A file whose
  first include is `stdlib.h` fails to compile (about 90 errors across fio). `-include features.h`
  works around it.
- `sys/select.h` defines `fd_set` but does not declare `select`.

Headers that do not exist:

| header | what fio uses | on the benchmark's path? |
|---|---|---|
| `sys/resource.h` | `struct rusage` is a field of `thread_data` (`fio.h`); `getrusage`, `RUSAGE_SELF` | yes: every file needs the type |
| `libgen.h` | `basename` | yes |
| `sys/ipc.h`, `sys/shm.h` | included unconditionally by `init.c` and `memory.c` | the headers only, with `--disable-shm` |
| `dlfcn.h` | `dlopen` and its family, for external engines | no; stubs that fail are enough |
| `syslog.h` | `openlog`, `syslog`, `closelog` | no; server mode only |
| `netdb.h`, `netinet/tcp.h` | `getaddrinfo` and its family | no; client, server and network engine only |

Two macros are missing from headers that exist: `O_SYNC` and `POSIX_MADV_DONTNEED`.

Functions undefined at link, with the headers stubbed: `basename`, `getrusage` and `select` on the
path. Off it: `closelog`, `dlclose`, `dlerror`, `dlopen`, `dlsym`, `freeaddrinfo`, `gai_strerror`,
`getaddrinfo`, `openlog` and `syslog`. fio's other 228 external references are all defined.
`getrusage` may answer zeros. `select` is the helper thread's timed wait on an in-process pipe,
which nife's pipes support.

Some are defined but refuse on nife, and none of them is on the path. They are `fork` and `waitpid` (process jobs and
server mode), `socket`, `bind` and `connect` (the network engine and client mode), `execvp`, `system`
and `popen` (the `exec` engine and `exec_prerun`), and file-backed or cross-process `mmap`.

## How fio would be configured

`--cpu=aarch64 --disable-native --disable-shm --disable-tls`. `--disable-shm` makes `thread=1` the
default. With it a job is `pthread_create` and `pthread_detach`, never `fork`; the reaper skips
`waitpid` for thread jobs, and `smalloc` takes anonymous memory rather than System V segments.
`--disable-tls` pins what `configure` finds anyway. It detects `clock_gettime` with
`CLOCK_MONOTONIC`, `pread`, `preadv` and `pwritev`, `posix_fallocate`, `fdatasync` and `pipe2`, and
finds no affinity, `posix_fadvise`, timerfd, libaio or `io_uring`.

## What the port still owns

These are the port's work and 833's questions, not the C library's:

- An `os/os-nife.h` in fio, about 20 lines for discovery: a new `FIO_OS` value, `os_phys_mem`,
  `OS_MAP_ANON`, the byte-swap macros and `FIO_USE_GENERIC_INIT_RANDOM_STATE`. Its real work is
  `blockdev_size` and `blockdev_invalidate_cache` for the confined NVMe view, which is 833's open
  `ioengine` question.
- The clock source. aarch64 reads `cntvct_el0` and riscv64 `rdtime`, both readable at user level on
  nife. `x86_64` reads `rdtsc`, which §139 (who may read the cycle counter, and by what authority)
  governs, so check it before choosing `clocksource=cpu` there.
