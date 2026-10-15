//! **The scripts swish-check types**: [`Line`], its two constructors, and the three tables of
//! lines with the substrings each answer must hold. The main table is cut into topic files
//! beside this one, and adding a check is adding a row in the topic file the line belongs to;
//! [`SWISH_CHECK_SCRIPT`] is them concatenated in table order.
//!
//! Cut out of `swish_check.rs` unchanged by milestone 847 (the swish-check harness is split
//! along its seams); cut into topic files by milestone 900 (the swish-check script splits by
//! topic before it crosses the ceiling).
//!
//! Name: provisional (milestone 847's lane, 2026-10-11 UTC; §280 (the swish-check harness
//! splits into submodules, with its script in one)).

use std::sync::LazyLock;

use super::*;

/// **One line this gate types, what it must answer, and how many jobs it runs.**
///
/// The job count sits beside the line because it is a fact about the line, and until 2026-09-26 it
/// was a hand-kept total in the success summary instead: every lane that added a line had to edit
/// that total and the script's declared length, so any two such lanes conflicted (five rebases in
/// one day: #1318, #1320, #1322, #1329 and #1330). Now the script is a slice, the total is summed
/// from the lines each leg actually typed, and adding a line touches the line.
///
/// `jobs` is how many children the progenitor builds from its bounded job pool for this line: one
/// per program stage that runs. **Zero** for a builtin (`echo`, `ls`, `apropos`, `package`), a
/// `caps` preview, a line refused at the prompt with nothing spawned, an image the activation set
/// refuses, and a supervised job (`interrupt_heeder` and `interrupt_ignorer`, built from the
/// shell's own untyped rather than the pool). A pipeline counts each program stage, so
/// `wc gate.txt | wc` is two; a `builtin | program` pipeline is one. `rm` with its caretaker is one
/// job whose region holds two processes. It is a required field with no default, so a line cannot
/// be added without saying.
///
/// **The count is checked, not only printed.** A line with `jobs > 0` whose answer carries the
/// shell's "could not spawn" or "faulted" sentence fails the gate even when its wanted phrases are
/// empty (`uuid > id.txt`), which is what makes "ran N jobs" in the summary a statement this gate
/// verified. What it cannot catch is a tag that is too low, since nothing at the prompt reports a
/// job that nobody counted; the host test `a_job_count_names_a_program` bounds a tag from above.
#[derive(Clone)]
pub(crate) struct Line {
    pub(crate) typed: &'static str,
    /// What the prompt shows once `typed` has been edited: the same text for a plain line, and
    /// the finished word for one with a Tab in it, or `^C` for one abandoned (see [`keys`]).
    pub(crate) echoed: &'static str,
    pub(crate) jobs: u8,
    pub(crate) answer: &'static [&'static str],
}

/// **What the shell prints when a job it asked for never answered**, either half:
/// `components/src/swish.rs` prints the first when the progenitor could not build the job, and the
/// second is `swish::FAULTED_SENTENCE`, for a job that trapped first. Spelled here rather than
/// imported because xtask does not depend on `swish`, and a phrase is enough for a transcript.
pub(super) const JOB_DID_NOT_RUN: [&str; 2] = [
    "could not spawn",
    "that command faulted and was killed before it answered",
];

/// **What the keyboard boot types before it launches** (milestone 715 (provisional)): one `caps`,
/// for its census, because this is the only boot with a keyboard and so the only one that can show
/// the keyboard's three slots (26 to 28) are not the shell's either. Before 715 the progenitor
/// placed them there beside the gpu's four. See the same line in [`SWISH_CHECK_AFTER_REBOOT`].
pub(super) const SWISH_CHECK_KEYBOARD_BOOT: &[Line] =
    &[line(0, "caps", &["slots held: 0 1 2", " 21 62\n"])];

/// A [`Line`], positionally, so the script reads as the prompt does. The name is provisional.
const fn line(jobs: u8, typed: &'static str, answer: &'static [&'static str]) -> Line {
    Line {
        typed,
        echoed: typed,
        jobs,
        answer,
    }
}

/// **A line typed with editing keys in it** (milestone 47 (navigation and naming), DECISIONS §227
/// (how Tab reaches the shell)): `typed` goes to the UART byte for byte, and `echoed` is what the
/// prompt then shows. A line ending in `^C` (`\x03`) is sent with no Enter, and must answer
/// nothing, because the shell discarded it. The name is provisional.
const fn keys(
    jobs: u8,
    typed: &'static str,
    echoed: &'static str,
    answer: &'static [&'static str],
) -> Line {
    Line {
        typed,
        echoed,
        jobs,
        answer,
    }
}

/// Whether a scripted line is abandoned with `^C` rather than run.
pub(super) fn interrupted_at_prompt(typed: &str) -> bool {
    typed.ends_with('\x03')
}

/// **What the second boot types** (milestone 198 (a package manager) rung 3a): the installed
/// program still runs after a reboot, a removal takes its vouch away without deleting it, and a
/// rollback vouches for it again. The disk is the only thing the first boot hands this one.
///
/// A removed program is refused only at a prompt that does not hold the run-unvouched capability
/// (DECISIONS §219 gate D2). The boot prompt holds it, provisionally, so here the removal shows in
/// `caps` and the bytes still run; before D2 this script typed the refusal.
///
/// `greeting` rides along (milestone 198 rung 3a's fetch): it was installed as generation 2, it
/// runs after the reboot, and removing `noteless` leaves it running, because a generation drops
/// one program and not its neighbors. Its 0.2.0 rides beside it (milestone 614): generation 3,
/// which lists both versions of `greeting` and `noteless`. `noteless` took `uptime`'s place here
/// when DECISIONS §229 (calef, 2026-09-27) refused installing a package named after an image
/// program.
///
/// **The numbers skip one** because the first boot vouched for a build as generation 4 and rolled
/// it back (DECISIONS §221). A generation is never rewritten, so the removal is generation 5, and
/// a rollback is by number (`notes/packages.md`'s BUGS), so it lands on 4: the vouch's generation,
/// which lists `noteless` too. The extra generation ahead of it is milestone 614 (two installed
/// versions of one program, each runnable, and a caller granted the one it needs)'s second
/// version, installed before the vouch.
pub(super) const SWISH_CHECK_AFTER_REBOOT: &[Line] = &[
    // **The shell holds no display device** (milestone 715 (provisional), the 2026-10-03 security
    // audit's follow-up; row 32 of notes/confinement-claims.md). On aarch64 and riscv64 this boot
    // has a gpu, and until 715 the progenitor placed its four capabilities in the shell at
    // `spawnproto::SHELL_GPU_SLOT` (22) onward for the life of the boot: measured on aarch64,
    // `slots held: 0 1 2 3 4 5 20 21 22 23 24 25 30` (slot 30 then, 62 since milestone 754 (the capability table grows to 64 slots)). Now the spawn service keeps them, so the
    // census runs straight from the configuration page (21) to the run-unvouched slot (62; 30 before milestone 754 (the capability table grows to 64 slots)), and
    // " 21 62" is the assertion that nothing sits between them. On x86_64 this boot has no gpu and
    // the line holds trivially; that leg's gap is milestone 632's (no virtio-gpu in its runner).
    line(0, "caps", &["slots held: 0 1 2", " 21 62\n"]),
    line(
        1,
        "packages/noteless/0.1.0/noteless",
        &["noteless: installed, and carrying no manifest note"],
    ),
    line(
        1,
        "packages/greeting/0.1.0/greeting",
        &[
            "hello from a package this image never carried",
            "clock: held at slot 1, as its manifest note asked",
        ],
    ),
    line(1, "jig remove noteless", &["removed; generation 5 is live"]),
    // **Removed means unvouched, not unrunnable, for a session holding D2** (DECISIONS §219 gate
    // D2). Until D2 this line was a refusal. The boot prompt now holds the run-unvouched
    // capability (provisionally), so the bytes still run, with the ruling's endowment rather than
    // the installed manifest; `caps` is what shows the vouch is gone.
    line(
        0,
        "caps packages/noteless/0.1.0/noteless",
        &[
            "provenance: unvouched (digest sha256:",
            "runs on this session's capability to run unvouched bytes",
        ],
    ),
    line(
        1,
        "packages/noteless/0.1.0/noteless",
        &["noteless: installed, and carrying no manifest note"],
    ),
    line(
        1,
        "packages/greeting/0.1.0/greeting",
        &[
            "hello from a package this image never carried",
            "clock: held at slot 1, as its manifest note asked",
        ],
    ),
    line(1, "jig rollback", &["rolled back; generation 4 is live"]),
    line(
        1,
        "packages/noteless/0.1.0/noteless",
        &["noteless: installed, and carrying no manifest note"],
    ),
    // **The distribution's index, over HTTPS** (milestone 801 (packages over the internet)).
    // `jig update` reads the image's index sources (an archive entry the kernel measured), fetches
    // the stand-in index from `basalt.test` at the address the sources give, pinned to the test
    // authority, and writes this architecture's lines to `activation/indexes/image`. The peer's
    // index is `disk::stage_index`'s: `greeting` 0.1.0, `uptime`, a `rebound` twin
    // (`helpers/tls-peer` adds it) and `greeting` 0.3.0, which the image's catalog does not name.
    line(
        1,
        "jig update",
        &[
            // Four from the fixtures, and one more per package from basalt's repository when one
            // was fetched (milestone 810), so the count is not asserted.
            "index updated: ",
            "from basalt.test over TLS",
        ],
    ),
    // `list` reads the copy once there is one, and says so.
    line(
        1,
        "jig list",
        &[
            "packages the index offers (the copy jig update wrote):",
            "  greeting 0.3.0",
        ],
    ),
    // **A lying repository, refused by the index's digest** before the installer is asked: the
    // peer serves `uptime` with a byte flipped. `jig` tries the entry's listed location first,
    // which this prompt cannot resolve, and then the repository's own copy.
    line(
        1,
        "jig install uptime",
        &[
            "refused: the index does not vouch for the bytes that arrived; nothing was sent to the \
             installer",
        ],
    ),
    // **Installed on the index copy's word alone** (ruling I2): the image's catalog names no
    // `greeting` 0.3.0, so the progenitor's installer accepts it only because the copy in the
    // store the owner's console granted lists its digest.
    line(
        1,
        "jig install greeting@0.3.0",
        &[
            "fetched greeting-0.3.0-",
            "from basalt.test over TLS",
            "fetched and installed; generation 6 is live",
        ],
    ),
    line(
        1,
        "packages/greeting/0.3.0/greeting",
        &["hello from a package this image never carried"],
    ),
    // **A package carries several programs** (milestone 919 (a package carries several
    // programs)). `duet` carries `bin/alto` and `bin/tenor`, neither named after it: one install
    // writes a row and a pointer for each, in one generation, and each runs by its own bare name. Their notes differ, so they are
    // endowed differently: `alto` holds the clock and `tenor` does not.
    // `jig` is granted the directory holding the file it names, so the line runs from there.
    line(0, "cd downloads", &[]),
    line(
        1,
        "jig install ./duet.nifepkg",
        &["installed; generation 7 is live"],
    ),
    line(
        1,
        "alto",
        &[
            "alto: one of two programs in the package duet",
            "clock: held at slot 1, as its manifest note asked",
        ],
    ),
    line(
        1,
        "tenor",
        &[
            "tenor: one of two programs in the package duet",
            "clock: not held; slot 1 is empty",
        ],
    ),
    line(
        0,
        "caps alto",
        &["its manifest note asks for: output bytes, the clock\n"],
    ),
    line(
        0,
        "caps tenor",
        &["its manifest note asks for: output bytes\n"],
    ),
    // **A name one package holds refuses another package whole** (calef, 2026-10-14 (UTC): "If
    // two packages offer the same program name, refuse the second package. Need to remove the
    // first one before installing the second one."). `solo` carries `bin/alto` too. The refusal
    // names the program and its holder, and writes no generation.
    line(
        1,
        "jig install ./solo.nifepkg",
        &[
            "refused: the package duet already provides the program alto; jig remove duet first; \
             generation 7 is live",
        ],
    ),
    // **`jig remove` names a package** (calef, 2026-10-14 (UTC): "jig remove should work on
    // packages, not programs."): a program's name removes nothing, and the package's name takes
    // both programs, so neither bare word reaches anything after it.
    line(
        1,
        "jig remove alto",
        &["refused: no package of that name is in the live generation; generation 7 is live"],
    ),
    line(1, "jig remove duet", &["removed; generation 8 is live"]),
    line(0, "alto", &["no such program"]),
    line(0, "tenor", &["no such program"]),
    // And with the holder gone, the second package installs, and the name is its.
    line(
        1,
        "jig install ./solo.nifepkg",
        &["installed; generation 9 is live"],
    ),
    line(0, "cd /", &[]),
    line(
        1,
        "alto",
        &["alto: one of two programs in the package duet"],
    ),
    line(1, "jig remove solo", &["removed; generation 10 is live"]),
    // **`ripgrep`, built and published by basalt** (milestone 810 (`ripgrep` is packaged in
    // basalt and installed with `jig`)): the package basalt published, fetched by its pinned
    // digest (`cargo xtask basalt-fetch`), or the repository basalt's own gate just built
    // (`disk::BASALT_REPOSITORY`). The peer serves its index lines beside the fixtures'. The
    // package is `ripgrep`; its program is the member `bin/rg`
    // (`package_archive::PROGRAM_PREFIX`), so the bare word is `rg`. Skipped without a repository.
    line(
        1,
        "jig install ripgrep",
        &[
            "fetched ripgrep-",
            "from basalt.test over TLS",
            "fetched and installed; generation 11 is live",
        ],
    ),
    line(1, "rg --version", &["ripgrep "]),
    // **`ioping`, basalt's first C package** (milestone 906 (ioping is packaged in basalt, and its
    // test installs it with `jig`)), on `ripgrep`'s terms: compiled by basalt against the C library
    // at the nife commit basalt pins. Installed, it times five 4 KiB reads of a file its word
    // names, and the file is its own installed program: unvouched, it is granted that file
    // read-only and the clock page. The line stands in `packages` and names the file relative to
    // it: only a word whose first component names an entry here designates anything (an absolute
    // one reached ioping with nothing granted, and its `stat` answered `ENOSYS`), and a word in
    // the shell's root is refused, because a caretaker is built by descending into a directory.
    // Pointed at a directory, ioping would need to write a working file there. The kernel suite's
    // `c_program_tests` runs the same assertions on the same bytes against a granted directory.
    line(
        1,
        "jig install ioping",
        &[
            "fetched ioping-",
            "from basalt.test over TLS",
            "fetched and installed; generation 12 is live",
        ],
    ),
    line(1, "ioping -v", &["ioping 1.3"]),
    line(0, "cd packages", &[]),
    line(
        1,
        IOPING_LINE,
        &[
            "4 KiB <<< ioping/1.3/ioping ",
            "request=1 ",
            "request=5 ",
            "(warmup)",
            "ioping statistics ---",
            "4 requests completed in ",
            ", 16 KiB read, ",
            "generated 5 requests in ",
            "min/avg/max/mdev = ",
        ],
    ),
    line(0, "cd /", &[]),
    // **`hackbench`, basalt's second C package** (milestone 907 (hackbench is packaged in basalt,
    // and its test installs it with `jig`)), installed the same way. Its thread mode is the run
    // `c_thread_tests` makes, smaller: one group of two senders and two receivers, four threads
    // and the main one, whose 64 KiB stacks fit an ordinary `std` program's heap. Unvouched, it
    // needs no grant beyond the clock page it times itself with.
    line(
        1,
        "jig install hackbench",
        &[
            "fetched hackbench-",
            "from basalt.test over TLS",
            "fetched and installed; generation 13 is live",
        ],
    ),
    line(
        1,
        HACKBENCH_LINE,
        &[
            "Running in threaded mode with 1 groups using 4 file descriptors each (== 4 tasks)",
            "Each sender will pass 10 messages of 100 bytes",
            "Time: ",
        ],
    ),
    // **SQLite, basalt's third C package** (milestone 897 (SQLite is packaged in basalt)): the
    // package `sqlite` carries the program `speedtest1`. The prompt runs the kernel suite's
    // in-memory line, which opens no file and needs only the clock page, and holds it to the same
    // hash of every result SQLite read back.
    line(
        1,
        "jig install sqlite",
        &[
            "fetched sqlite-",
            "from basalt.test over TLS",
            "fetched and installed; generation 14 is live",
        ],
    ),
    line(
        1,
        c_program_fixture::SPEEDTEST1_MEMDB,
        &[
            "-- Speedtest1 for SQLite 3.50.4",
            "TOTAL",
            c_program_fixture::SPEEDTEST1_HASH,
        ],
    ),
    // **`rmle`, the first first-party package basalt builds** (milestone 921 (rmle is packaged in
    // basalt)): the editor's `[[bin]]`, built by basalt at the nife commit it pins, installed from
    // basalt's index. It is not typed afterwards: no prompt line can start `rmle` yet, because only
    // the kernel harness wires its raw terminal (`kernel/src/user/rmle_service.rs`).
    line(
        1,
        "jig install rmle",
        &[
            "fetched rmle-",
            "from basalt.test over TLS",
            "fetched and installed; generation 15 is live",
        ],
    ),
    // **The demos, the second first-party package** (milestone 922 (the demos are packaged in
    // basalt)): `demos` carries `least_authority_demo`. The package is fetched from basalt and
    // accepted by its digest, and then the installer refuses it, because this image carries that
    // program and a base program is updated by a new base, not installed over. The refusal is the
    // answer until milestone 909 (basalt composes the bootable image from its packages) leaves
    // optional programs out of the image.
    line(
        1,
        "jig install demos",
        &[
            "fetched demos-",
            "from basalt.test over TLS",
            "refused: the image carries the program least_authority_demo; a new base updates it, not \
             install; generation 15 is live",
        ],
    ),
];

/// The run `jig install hackbench` is followed by (milestone 907): `c_thread_tests`' pipe run,
/// with one group of two and ten messages a sender.
const HACKBENCH_LINE: &str = "hackbench -T -p -g 1 -f 2 -l 10";

/// The timed reads `jig install ioping` is followed by (milestone 906): `c_program_fixture::IOPING`'s
/// flags, pointed at the installed program's own file rather than a directory.
const IOPING_LINE: &str = "ioping -c 5 -i 0 -s 4k ioping/1.3/ioping";

/// The lines that need a repository from basalt (milestones 810, 906, 907, 897, 921 and 922),
/// skipped without one outside CI.
pub(super) const FROM_BASALT: &[&str] = &[
    "jig install ripgrep",
    "rg --version",
    "jig install ioping",
    "ioping -v",
    IOPING_LINE,
    "jig install hackbench",
    HACKBENCH_LINE,
    "jig install sqlite",
    c_program_fixture::SPEEDTEST1_MEMDB,
    "jig install rmle",
    "jig install demos",
];

/// The text this gate types and what each line must answer. `None` is a line whose answer is
/// checked by a later one rather than by itself, which is every line that writes a file.
///
/// `hello world` plus the newline `echo` adds is twelve bytes; the append arm is exactly twice
/// that. The numbers are spelled out here rather than derived because this is a **boot** gate: if
/// the arithmetic and the boot were both wrong, deriving one from the other would hide it.
///
/// The rows live in the topic files under this one, and this concatenates them in table order,
/// so the leg still types one slice and the transcript is the one the single table typed.
/// `Line` is `Clone` for `concat`.
pub(super) static SWISH_CHECK_SCRIPT: LazyLock<Vec<Line>> = LazyLock::new(|| {
    [
        pipes_files::LINES,
        clock_config::LINES,
        navigation::LINES,
        process_tools::LINES,
        installer::LINES,
        std_programs::LINES,
        packages::LINES,
        owner::LINES,
        redirection::LINES,
        directories::LINES,
        words::LINES,
        network::LINES,
        interrupts::LINES,
        std_exerciser::LINES,
    ]
    .concat()
});

/// **The lines after which this gate presses `^C`**, once the shell has said the job is running.
/// Each runs until interrupted, so without the keystroke the prompt never comes back. A program
/// here is also one `every_spawnable_program_has_a_swish_check_line` would otherwise have to excuse.
pub(super) const SWISH_CHECK_INTERRUPTED: [&str; 2] = ["interrupt_heeder", "interrupt_ignorer"];

/// What the shell prints once a supervised job is running and watched. Pressing `^C` before it
/// would still be counted (the shell's watermark catches an early one), but after it is the case a
/// person types, so it is the one this gate proves.
pub(super) const SWISH_CHECK_RUNNING: &str = "^C interrupts it.";

mod clock_config;
mod directories;
mod installer;
mod interrupts;
mod navigation;
mod network;
mod owner;
mod packages;
mod pipes_files;
mod process_tools;
mod redirection;
mod std_exerciser;
mod std_programs;
mod words;
#[cfg(test)]
mod tests;
