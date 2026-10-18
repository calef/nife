---
status: NOT-STARTED
raised: 2026-08-25
milestone_dependencies: 809, 801
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 170. `nano`: a real, full-featured screen editor on the primitive milestone 169 builds, compiled unmodified against the C library

GNU nano is the editor a newcomer expects to find. This milestone brings it to nife the way SQLite's
`speedtest1`, ioping and hackbench arrived: compiled unmodified against `c_library`. That is the C
library §265 (a C library started from relibc) started, and milestones 835 (a C library, stage 1:
files, clock and memory) and 836 (a C library, stage 2: threads) built. It does not go through §31 (the
foreign-language seam) or milestone 181 (the foreign-language seam, extended to a process the
shell calls more than once).

calef, 2026-10-18 (UTC): *"Yes: rewrite milestone 170 for the C-library route, with a kilo.c port
as the first step that proves termios; the port itself still waits on 801 and 809."*

Reuse: nano and ncurses, both taken whole as upstream source and built by basalt. Nothing here is
written in place of an existing program. §135 (running GPL software is aggregation) covers nano's
GPL: it arrives as a package.

## Stage 1: antirez's `kilo.c`, unmodified, to prove the terminal layer

[`kilo`](https://github.com/antirez/kilo) is one file of about 1,000 lines. Read 2026-10-18 (UTC),
it includes `<termios.h>` and `<sys/ioctl.h>` and needs nothing nano would not also need. No
ncurses, no regex, no `fork` or `exec`. It enters raw mode with `tcgetattr` and `tcsetattr`, draws
with hand-written ANSI sequences, and installs a `SIGWINCH` handler.

Its job is to prove the terminal layer below before nano's much larger surface sits on it. A
failure in a 1,000-line program points at the C library. The same failure under nano and ncurses
could be in any of three code bases.

The stage is done when `jig install kilo` from basalt's index puts `kilo.c` on a nife system, and it
edits and saves a file on all three architectures. It rides the same package path as nano, so it
waits on the same milestones. It is a stage and not a milestone of its own because nothing outside
this block depends on it, and as a milestone its dependency fields would match this one's exactly.
If another program comes to need the terminal layer before nano does, split it then
(`notes/roadmap.md`: a dependency on part of a milestone means split it).

`rmle` (`components/src/rmle.rs`), from milestone 169 (`kilo`: the smallest real text editor),
stays. Its raw-keystroke primitive, `OPERATION_RAWMODE` and `OPERATION_READRAW` in
`crates/line_editor`, is what termios sits on. Whether the `rmle` program itself is retired once
nano runs is calef's call then, not this block's.

## What is missing, checked against the tree 2026-10-18 (UTC)

### A C program has no terminal at all

This is the largest gap, and it sits below termios. `c_library` reads standard input through
`std::io::stdin()` (`c_library/src/platform/nife.rs`). nife's `std` answers every read of it with
end of file (`patches/std-nife/overlay/std/src/sys/stdio/nife.rs`). Standard output goes over the
sink contract, not the terminal contract. `isatty` is always false (`c_library/README.md`, BUGS),
and `kilo` refuses to start when `isatty(STDIN_FILENO)` is false.

So a C program needs a terminal endpoint granted to it. `std`'s slot convention
(`pal/nife/rt.rs` in the same overlay) has no slot for one; `rmle` holds the terminal in its own
slot 0, outside `std`. A slot for the terminal is a convention the spawner and every `std` program
share, so it is calef's to rule on (the first open question below).

With a terminal granted, `c_library` gains a terminal description: `read` and `write` on it speak
the `line_editor` contract, and `isatty` answers true.

### `termios.h`

No `termios` module in `vendor/relibc/header`. The work is a header and its functions in
`c_library`: `tcgetattr`, `tcsetattr`, `cfmakeraw` and the flag constants. Clearing `ICANON` and
`ECHO` maps onto `OPERATION_RAWMODE`, and a read in raw mode onto `OPERATION_READRAW`. Flags the
terminal cannot honor are accepted and listed in BUGS rather than faked.

### The terminal's size

relibc's `ioctl` (`vendor/relibc/header/sys_ioctl/mod.rs`) answers every request with `ENOTTY`.
The terminal contract carries no size (`notes/terminal-contract.md`), which is why `rmle` is fixed
at 80x24. `kilo` falls back to moving the cursor to the far corner and asking for its position
with `ESC [6n`. A terminal emulator on the far end of the serial line answers that. nife's own
screen console (`crates/screen_console`) does not, and `kilo` would then block waiting for the
reply. ncurses reads `LINES` and `COLUMNS`, then the terminal type's defaults.

A size has to enter the contract between the terminal and its program, and that is a format two
programs share. The second open question below names the candidates. Until it is answered, a
program sees 80x24.

### ncurses, wide-character build

nano will not build without it, and the tree has none. basalt cross-builds `ncursesw` against
`c_library` with its terminal descriptions compiled in (`--with-fallbacks`, `--disable-database`),
so no terminfo database has to exist on nife. The first set: `vt100`, `xterm-256color` and `linux`.

### `regex.h`

Absent from `vendor/relibc/header`. nano needs `regcomp` and `regexec` for search and for syntax
coloring. nano's own release tarball carries gnulib's `regex` module (read from nano's
`autogen.sh` 2026-10-18 UTC), which gnulib builds into nano when the system has none. Whether that
suffices, or `c_library` gains a POSIX regex of its own, is a dependency, so it is calef's under
§46 (thin primitives or whole subsystems). The third open question below names the candidates.

### Wide characters and locale

Present in `c_library`: `setlocale`, `nl_langinfo`, `mbrtowc`, `wcrtomb`, `mbstowcs`, `mbtowc`,
`btowc`, `mbsinit`, `wcwidth`, `wcswidth`, `iswprint`, `iswalnum`, `towlower` and `wctype`.
`nl_langinfo(CODESET)` answers `UTF-8`, which is what nano checks before it handles UTF-8.

Two limits. `wcwidth` is 1 for every printable character, wrong for East Asian wide characters and
combining marks (`c_library/README.md`, BUGS). nano's tarball also carries gnulib's `wcwidth`,
which may cover nano's own column math; ncurses still calls the C library's. And `setlocale` with
any name other than `C`, `POSIX` or the empty string opens `/usr/share/i18n/locales/<name>`, which
nife does not have, so it fails. nano calls `setlocale(LC_ALL, "")`, which succeeds.

### Signals

`c_library` records a handler and delivers only a signal the program raises itself (BUGS). So
`SIGWINCH` never arrives, and a resized terminal goes unnoticed. nano and `kilo` are both built
without suspend (`^Z`), since there is no job control to suspend into. Both are BUGS entries in the
package, not workarounds.

### The speller and `^T` (execute a command)

Both start another program. §264 (`fork` is declined for good) rules out `fork`, and
`posix_spawn` is milestone 838 (a C library: `posix_spawn`, and no fork), not started. nano is configured with the speller
off and with execute unreachable, and the package's BUGS section says so. They can return once
milestone 838 is built.

### `pwd.h`, and `~/.nanorc`

No `pwd` module in `vendor/relibc/header`, and nife attaches no user to a process. nano is built
without reading an rc file or keeping histories, so it never looks up a home directory.

## Stage 2: nano

With the terminal layer proven by `kilo`, basalt builds `ncursesw` and then nano against
`c_library`. Configuration is a recipe in basalt, not a patch to nano. nano's file work (swap files,
backups, lock files) is ordinary file I/O against the directory it is granted.

## Done when

calef, 2026-10-12 (UTC), under §297 (software reaches a nife system as a package `jig` installs
from basalt's index, the kernel included): "They should all be installed via jig through basalt."

`kilo` and then `nano` run on nife installed by `jig install` from basalt's package index, on all
three architectures. basalt builds each package against the nife commit it pins. The install waits
on milestone 809 (the package client becomes a program, `jig`, with the verbs an index needs). It
also waits on milestone 801 (packages over the internet: a host name, HTTPS and the distribution's
index). No work toward that starts until those milestones are built (calef, 2026-10-12 UTC: "For
our other programs let's block start on packaging.").

## Open questions for calef

- How a terminal reaches a `std` program, and so a C program: a slot in `std`'s convention, or
  something else.
- How the terminal's size reaches its program. Candidates: a new `line_editor` operation answering
  rows and columns; `LINES` and `COLUMNS` in the environment the shell builds; or the screen console
  answering `ESC [6n` the way a terminal emulator does. Resize notification is a further step for
  any of them.
- Which regex: gnulib's, carried in nano's tarball and built into nano alone; or one vendored into
  `c_library` for every C program, from `libtre`, musl, NetBSD's or FreeBSD's libc, or gnulib.

## Index row

nano, compiled unmodified against the C library (§265, milestones 835 and 836) and installed from
basalt with `jig`. Stage 1 ports antirez's `kilo.c` to prove a terminal layer the C library lacks:
a terminal grant, `termios.h` over `line_editor`'s raw mode, and a size. Stage 2 adds `ncursesw`
and nano, with the speller and `^T` off until `posix_spawn` exists.
