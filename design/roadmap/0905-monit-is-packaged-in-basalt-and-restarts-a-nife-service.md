---
status: NOT-STARTED
raised: 2026-10-12
milestone_dependencies: 801, 809, 810, 836, 837, 838, 849, 897
decision_dependencies: 135, 223, 264, 265, unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 905. monit is packaged in basalt, installed with `jig`, and restarts a nife service it watches

*(Minted 2026-10-12 (UTC) by lane `maintainer/mint-monit-milestone` at calef's request, directly
rather than through the proposal pile, as milestones 836 to 838 were minted from his ruling. The
number is provisional until the merge queue lands it; the title and slug are drafts.)*

calef, 2026-10-12 (UTC): *"monit is really useful. Lets add a milestone for porting it to nife."*
And the same day: *"That milestone should require it as a package delivered via jig through
basalt."*

**In brief.** Port monit, the C process and service supervisor from Tildeslash (mmonit.com,
AGPL-3.0). Ship it the way milestone 810 (`ripgrep` is packaged in basalt and installed with
`jig`) ships `rg`. That means a recipe in basalt and a package build against the C library at
basalt's pinned nife commit. Then publication to basalt's index, and `jig install monit` at the
prompt. The program
does not run unmodified. Its process layer has no nife backend, and it starts every program with
`fork`, which §264 (`fork` is declined for good, and spawn is the supported way to start a program)
refuses. So this milestone is a port with two named changes, and it says so.

## What monit needs, read from its source

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

## What each need maps onto in nife

| monit needs | nife today | owner |
|---|---|---|
| A C library | relibc-seeded `c_library/`, files, clock and memory: BUILT | milestone 835 (a C library, stage 1: files, clock and memory) |
| pthreads | stage 2, BUILT on its branch (#2009) and in the merge queue | milestone 836 (a C library, stage 2: threads) |
| TCP and Unix sockets, `poll`, `getaddrinfo` | stage 3 promises TCP, `poll` and literal-address `getaddrinfo`; no Unix sockets | milestone 837 (a C library, stage 3: sockets) |
| Starting a service | `fork` returns `ENOSYS` (`c_library/src/platform/nife.rs`) and always will | §264; milestone 838 (a C library: `posix_spawn`, and no fork) on milestone 172 (a capability-native subprocess primitive) |
| Process ids | `getpid` is 0, `kill` names only the caller, and `getpgid` answers 0 for any number, so every pid looks alive | unwritten (question 3) |
| `SIGCHLD`, `SIGHUP`, `SIGTERM` | none delivered; `sigaction` records a handler and nothing fires it | §265 (a C library started from relibc, whose Rust platform layer holds the capabilities), "What this does not decide" |
| A process table | `endpoint::SURVEY` over one supervision domain, records `STATE` and `PLACEMENT`; no command line, CPU time or memory | §223 (the process view is the supervision domain); milestone 126 (the `procps` package, now `process-tools`) |
| Ending a service | no `kill`; ending a child is its region holder's | milestone 455 (`kill`, `pkill`, `skill` and `snice`: the signaling stratum of `procps`), REFUSED |
| Learning of a death | the kernel's death message to the spawner's supervision endpoint | §26 (the fault endpoint: thread death becomes a message a supervisor holds) |
| Grants (files, network, a domain to watch) | `grant_plan::Manifest` (`domain`, `network`, a directory) in an ELF note | milestone 849 (a package declares what it needs at run time, apart from what it links) for a foreign package |
| Uid checks on the control file | `geteuid` and `st_uid` are both `NOBODY`, so the check passes | none needed |

The sysdep a port writes reads `SURVEY` where Linux's reads `/proc`. Over the domain monit is
handed, it can answer existence and the main thread now, CPU placement as a bonus, and nothing
about memory, CPU time or command lines until the survey grows records for them. Those are kernel
surface, an architect's call, and not in this milestone. So this port's process checks are
existence and restart; `cpu` and `memory` rules report "not available" through the
`available_statistics` path monit already has.

## Where monit overlaps nife's supervision, and what it adds

The overlap is the mechanism. nife already turns a death into a message (§26) and keeps restart
policy in userspace. `sub_server_supervisor` restarts `flaky` per label, from milestone 22 (trusted
init: verify it, and shrink what a broken one can do). monit's restart, done right on nife, is the same
shape: monit spawns the service, so the service's death reaches monit, and monit spawns it again.

What monit adds is everything above the mechanism, and today nife has none of it on a booted
system. Milestone 854 (the init package holds only what init does) found that nothing on today's
boot starts `root_supervisor` or `sub_server_supervisor`; only tests do. So monit would be the
first restart policy a customer can configure. It brings a declarative file of services, start and
stop programs, timeouts and retry limits. Its health checks look past "is it alive": a port that
answers, a protocol that speaks, a file's checksum or age, a filesystem's free space. And it has
alerts by mail, a status page and a CLI. None of that should be rebuilt in the kernel or the progenitor.

monit's model of a stranger's process is a pid read from a file, the ambient naming §223 refuses.
On nife it sees only the services it spawned or a domain it was granted: a narrower monit, on
purpose.

## The modification question

§262 (nife is measured with the field's standard benchmarks, run unmodified, against Linux) sets
the rule for benchmarks: the program's source is upstream's at a pinned version, and the layer
under it is ours. Risk 1 uses the same word. monit is not a benchmark, but the standard is the one
this tree applies to foreign software, and monit cannot meet it.

- A `sysdep_NIFE.c` pair is a modification by that wording: it is new source compiled into the
  program. It is also the seam upstream built for exactly this, the change least likely to rot and
  the one upstream could take. The alternative that leaves monit untouched is a C library serving a
  `/proc`-shaped directory over the granted domain, so a Linux-host build's `sysdep_LINUX.c` runs
  as is. That puts a compatibility filesystem in the C library to avoid one file in monit.
- A nife-only `Command_execute` on `posix_spawn` (milestone 838) is a patch. It is selected the
  way monit selects a `sysdep`, so `fork` stays the path on every other system. It is §264's stated
  consequence for a C program that forks. It also stops trusting `SIGCHLD` and collects status with
  `waitpid(WNOHANG)` on its poll, or every start program waits out its timeout.
- What does not work and is not patched: `monit reload` and `monit quit` (signals), `ping` (raw
  sockets), PAM, and `getifaddrs` link checks. Each goes in the block's `BUGS` when built.

So monit reaches nife as a port with two named changes, the ripgrep precedent: `rg`'s own source
untouched, a ten-line patch in a dependency, and the patch nife's to maintain (#1892).

Ruled by calef, 2026-10-12 (UTC), on question 1 (#2022): *"monit 1 of 4, ship it patched as you
recommend."* basalt's recipe carries both changes, both are offered upstream as a platform port,
and monit is recorded as a port with two named changes.

### Why upstream's objections to `posix_spawn` do not hold on nife

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
  service's directory is a grant in the spawn request, milestone 838's file actions.
- `umask`. It is C-library state with no kernel counterpart; `c_library` answers `0o022` and
  enforces no mask. The child starts with its own.
- The `clone` and `vfork` limits. No code runs in a nife child before its image starts. Spawn is a
  request to the spawner, under §219 (how the shell names an installed program to the spawner).
  Milestone 172 (a capability-native subprocess primitive) builds the child from an image. The
  `getpwuid` and `getgrouplist` calls the comment worries about have no users to look up.

## Delivery: a package from basalt

calef's second sentence decides the route. The deliverable is not a binary on a data disk and not a
base-image program (§241 (a threadbare base: the boot slot holds the kernel and what boots and
repairs, and every other program is a package)). It is a package `jig` installs.

1. basalt carries a recipe naming monit 5.35.2's tarball by SHA-256, with the `sysdep` pair and
   the `Command.c` patch as files beside it. Its configure line is `--without-pam`, `--without-ssl`
   (question 4), `--without-zlib` unless basalt packages zlib, and the `ARCH` the patch's
   `configure.ac` case selects. A download that does not match fails the build (§195 (a reviewed
   recipe vouches for a package, and the machine's owner may overrule it)).
2. basalt builds it for aarch64, riscv64 and x86_64 against `c_library` at the pinned nife commit,
   with `helpers/c-library-cflags.sh`'s flags. Milestone 897 (SQLite is packaged in basalt, and its
   tests run in basalt's package build) builds that path for the first C package; this is the
   second.
3. The manifest note, for a foreign program, is milestone 849's: a directory for `monitrc` and its
   state, the network for its web server and checks, the spawn request, and `ENUMERATE` on the
   domain its services are born under.
4. One archive per architecture, kind `optional`, under §197 (a package is one archive file,
   named and vouched for by its recipe). It is published to basalt's index, under
   §250 (an image names its distribution's package index, at `basalt.nifeos.org`, and a package's
   bytes may live anywhere). Milestone 810 builds basalt's first publish job.
5. `jig install monit` installs it. Its `install <name>` is built by milestone 809 (the package
   client becomes a program, `jig`, with the verbs an index needs). It fetches the index by name
   over HTTPS, whose QEMU half is built by milestone 801 (packages over the internet: a host name,
   HTTPS and the distribution's index). If monit is served from the opt-in copyleft repository (below), the
   test first runs `jig add-repository`, milestone 809's unbuilt item 7.

The order across repositories: the nife pieces (`posix_spawn`, sockets, process ids, the manifest
support) land first through nife's queue. basalt then bumps its pin in a separate pull request
citing the landed commit (`nifeos/nife@<sha>`) and adds the recipe; that pull request records the
dependency.

## The license, as facts

- monit's `COPYING` is the stock GNU AGPL v3. Each source file adds an exception permitting
  linking with OpenSSL. libmonit carries MIT-licensed parts from Hanson's CII library.
- Statically linking `c_library` (MIT, from relibc) into monit is permitted by MIT; the linked
  binary is distributed under the AGPL.
- §135 (running GPL software is aggregation, the capability boundary is what makes it so, and
  packages are how it arrives) was amended on 2026-09-27. Copyleft never enters the image or the
  `base` kind. It arrives only in `optional` packages from a repository the user opts into through
  basalt. The distributor publishes corresponding source beside each binary and ships the license
  text in the package. Whether that repository lives under nifeos or a sibling
  organization is still open under milestone 198 (a package manager).
- Shipping the binary obliges basalt, under the AGPL's section 6, to offer the corresponding
  source: the tarball, the `sysdep` pair, the patch and the build scripts.
- Section 13 applies to a modified version, and this one is modified. Whoever runs it must
  prominently offer users who interact with it over a network (its web UI is that interaction)
  the source of that version. Upstream's footer links only to mmonit.com.

## Done when

On aarch64, riscv64 and x86_64 under QEMU, in `script/swish-check`, from a nife at basalt's pinned
commit:

1. `jig install monit` fetches monit from basalt's index over HTTPS, and the archive's digest
   matches the recipe.
2. monit's daemon runs with a `monitrc` declaring one service: a fixture that listens on a port and
   faults when told to over it, since nothing can `kill` it (milestone 455). monit starts it.
3. `monit status` typed at the prompt prints the service as `OK` with a pid that names it.
4. The test tells the fixture to fault. Within two monit cycles the daemon reports it gone,
   starts it again, and `monit status` at the prompt shows it `OK` under a new pid.
5. The recipe's tarball checksum, altered, fails basalt's build.

Every change to monit's source is listed in the recipe and in this block. A lane that finds a third
change it needs stops and asks rather than adding it.

## Which fatal risk it informs

Risk 1 (`1-only-software-written-for-nife.md`): only software written for nife runs on nife. monit
is a stranger's C daemon that uses threads, sockets, process creation and a process table. That is
more of the POSIX surface than speedtest1 or ioping touch. It informs the risk honestly only if it
is reported as a port with two changes, not as an unmodified run. As such it is evidence that a
real C daemon needs about one file and one patch, and that `fork` is the cost.

## Open questions for calef

1. **Ship monit patched?** Ruled 2026-10-12 (UTC): yes, as recommended. See *The modification
   question*.
2. **How does the daemon outlive the prompt?** swish has no background jobs (milestone 48 (job
   control: `jobs`, `wait`, `kill`, `fg`, `bg`, and a stopped state) is NOT-STARTED), and monit's
   `daemonize` forks. Recommended: a package may declare a service the progenitor starts at boot
   under a supervision endpoint, unwritten today, because a supervisor tied to a session is not one.
   The cheaper path is `monit -I &` once milestone 48 lands.
3. **What is a pid in the C library?** Today every pid is 0. Recommended: a tid within a
   supervision domain the process holds (§223), as `pidwait` already takes tids (§226 (`pidwait`
   takes tids and composes with `pgrep`, because a program does one thing)). This belongs in
   milestone 838's mapping and is asked here because monit is its first consumer.
4. **The section 13 offer.** Recommended: the same patch adds a source link to the web UI's
   footer, so every nife user is compliant by default, and monit ships `--without-ssl` until
   OpenSSL is packaged (its exception permits it later).

Reuse: monit 5.35.2 itself, with upstream's porting seam; relibc's `posix_spawn` file actions
through milestone 838; `crates/ps`'s survey walk for the sysdep; milestone 810's and 897's basalt
packaging. Writing a nife-native supervisor instead was considered: `sub_server_supervisor` is the
mechanism, and what calef asked for is monit's policy, configuration and checks, which nothing in
the tree has.

## Index row

The first foreign service supervisor on nife, installed by `jig` from basalt. A stranger's C daemon
with threads, sockets and process creation, ported through its own `sysdep` seam and one patch off
`fork`. It restarts a nife service it spawned and reports it at the prompt.
