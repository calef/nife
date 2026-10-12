# What monit 5.35.2's source needs, and why `fork` is not needed here

An appendix of [milestone 905](../0905-monit-is-packaged-in-basalt-and-restarts-a-nife-service.md)
(monit is packaged in basalt, installed with `jig`, and restarts a nife service it watches), written
2026-10-12 (UTC). It is a separate file because the block would otherwise pass §212 (a prose
budget)'s cap.

## What monit needs

Read from the monit 5.35.2 release tarball (mmonit.com/monit/dist/, the newest on 2026-10-12 UTC).

- Process statistics go through a per-OS layer, `src/process/sysdep_<OS>.c`, eight functions
  behind `process_sysdep.h`, from `init_processtree_sysdep` to `Sysdep_getMainThread`. A twin layer,
  `src/device/sysdep_<OS>.c`, does filesystems. Both exist for AIX, Darwin, DragonFly, FreeBSD,
  Linux, NetBSD, OpenBSD and Solaris, plus an `UNKNOWN` stub.
- Linux's layer reads `/proc`: `glob("/proc/[0-9]*")`, then per process `stat`, `status`,
  `cmdline` and the rest; system-wide `/proc/meminfo`, `/proc/stat` and `/proc/loadavg`; mounts
  from `/proc/self/mounts` and usage from `statvfs`.
- Porting is by adding a layer. `configure.ac` (and libmonit's) switch on the build host's `uname`
  and compile `sysdep_@ARCH@.c`; anything unrecognized warns "Architecture not supported" and gets
  `UNKNOWN`. No porting document ships, but a new pair of `sysdep` files plus a `uname` case in two
  `configure.ac` files is the shape the tree is built for. `UNKNOWN` alone is
  not enough: its `Sysdep_getMainThread` returns 0, so every pidfile check reads as "not running".
- Starting a program: `libmonit/src/system/Command.c`'s `Command_execute` calls plain `fork`, then
  `setsid`, `dup2`, closes descriptors to 256, `setgid`/`setgroups`/`setuid`, `umask` and `execve`,
  with no shell. A comment above it rejects `posix_spawn` by name (below). Exit status is collected only by a `SIGCHLD` handler
  that loops `waitpid`; `Process_exitStatus` says "Trust SIGCHLD handler to set status".
- Existence: a pidfile read by `Util_getPid`, or a regex over every command line; then `getpgid`
  and `Sysdep_getMainThread`.
- Signals: `SIGTERM` stops, `SIGHUP` reloads, `SIGUSR1` wakes; `monit reload` and `monit quit`
  signal the daemon found by pidfile. `monit status`,
  `summary` and the start and stop actions are HTTP requests to the daemon's own web server, over
  TCP to `localhost` or a Unix socket. `daemonize` forks twice; `-I` runs in the foreground.
- Threads: pthreads are required. One thread runs the web server, one request at a time over
  `poll`; one sends heartbeats to M/Monit. Nothing else.
- Sockets: listening TCP and Unix sockets, `getaddrinfo`, `poll` (never `select`), `getifaddrs`,
  raw ICMP for `ping`, SMTP for alerts, and 36 protocol tests in `src/protocols/`.
- Files: `stat` (the control file must be owned by `geteuid()`), bundled MD5 and SHA-1,
  `regcomp`, `syslog`, `getpwnam`, `crypt`, `glob`.
- Build: autotools, libmonit as a configured subdirectory. OpenSSL, PAM and zlib are on by default,
  each turned off by a `--without-` flag.

## Why upstream's objections to `posix_spawn` do not hold on nife

The comment above `Command_execute` (`libmonit/src/system/Command.c`, lines 838 to 871) gives six
reasons for `fork`. Each assumes a Unix child that inherits its parent's state and must shed it.
A nife child inherits nothing it is not handed, so each step `fork` exists to run has nothing to do.

- `setsid`, to leave the controlling terminal. A child holds a terminal only if spawn grants one;
  there is no session to leave. `setsid` answers `EPERM` in `c_library` today.
- `setuid` and `setgid`. nife has no users: `geteuid` is `NOBODY`. Authority is the capabilities
  the spawn request grants, so changing privilege means granting less, which `posix_spawn` can say.
- Closing every descriptor. A child's capability table starts empty apart from its grants, so
  close-all is the default rather than a step.
- `chdir`. A process stands in the directory it was granted, and `chdir` answers `ENOSYS`. The
  service's directory is a grant in the spawn request: the file actions of milestone 838 (a C library: `posix_spawn`, and no fork).
- `umask`. It is C-library state with no kernel counterpart; `c_library` answers `0o022` and
  enforces no mask. The child starts with its own.
- The `clone` and `vfork` limits. No code runs in a nife child before its image starts. Spawn is a
  request to the spawner, under §219 (how the shell names an installed program to the spawner).
  Milestone 172 (a capability-native subprocess primitive) builds the child from an image. The
  `getpwuid` and `getgrouplist` calls the comment worries about have no users to look up.
