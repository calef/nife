//! **The scripts swish-check types**: [`Line`], its two constructors, and the three tables of
//! lines with the substrings each answer must hold. Adding a check is adding a row here.
//!
//! Cut out of `swish_check.rs` unchanged by milestone 847 (the swish-check harness is split
//! along its seams).
//!
//! Name: provisional (milestone 847's lane, 2026-10-11 UTC; §280 (the swish-check harness
//! splits into submodules, with its script in one)).

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
            "fetched and installed; generation 7 is live",
        ],
    ),
    line(1, "rg --version", &["ripgrep "]),
    // **`ioping`, basalt's first C package** (milestone 906 (ioping is packaged in basalt, and its
    // test installs it with `jig`)), on `ripgrep`'s terms: compiled by basalt against the C library
    // at the nife commit basalt pins. Installed, it times five 4 KiB reads of a file its word
    // names, and the file is its own installed program: unvouched, it is granted that file
    // read-only and the clock page. The word is relative because only a word whose first
    // component names an entry here designates anything; `/packages/...` reached ioping with
    // nothing granted, and its `stat` answered `ENOSYS`. Pointed at a directory, it would need to
    // write a working file there. The kernel suite's `c_program_tests` runs the same assertions on
    // the same bytes against a granted directory.
    line(
        1,
        "jig install ioping",
        &[
            "fetched ioping-",
            "from basalt.test over TLS",
            "fetched and installed; generation 8 is live",
        ],
    ),
    line(1, "ioping -v", &["ioping 1.3"]),
    line(
        1,
        IOPING_LINE,
        &[
            "4 KiB <<< packages/ioping/1.3/ioping ",
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
];

/// The timed reads `jig install ioping` is followed by (milestone 906): `c_program_fixture::IOPING`'s
/// flags, pointed at the installed program's own file rather than a directory.
const IOPING_LINE: &str = "ioping -c 5 -i 0 -s 4k packages/ioping/1.3/ioping";

/// The lines that need a repository from basalt (milestones 810 and 906), skipped without one
/// outside CI.
pub(super) const FROM_BASALT: &[&str] = &[
    "jig install ripgrep",
    "rg --version",
    "jig install ioping",
    "ioping -v",
    IOPING_LINE,
];

/// The text this gate types and what each line must answer. `None` is a line whose answer is
/// checked by a later one rather than by itself, which is every line that writes a file.
///
/// `hello world` plus the newline `echo` adds is twelve bytes; the append arm is exactly twice
/// that. The numbers are spelled out here rather than derived because this is a **boot** gate: if
/// the arithmetic and the boot were both wrong, deriving one from the other would hide it.
pub(super) const SWISH_CHECK_SCRIPT: &[Line] = &[
    line(1, "echo hello world | wc", &["1 2 12"]),
    line(0, "echo hello world > gate.txt", &[]),
    line(1, "wc < gate.txt", &["1 2 12"]),
    line(0, "echo hello world >> gate.txt", &[]),
    line(1, "wc < gate.txt", &["2 4 24"]),
    // **Milestone 31's headline, at the one interface a human touches**: naming a resource in a
    // command IS granting it. The answer has to be the same as the `<` above it, because it is the
    // same designation with the operator left out, and the pair is what makes that a claim rather
    // than an assertion: one line reaches the file through an operator and one through a name, so
    // if they disagree, one of them opened something else.
    line(1, "wc gate.txt", &["2 4 24"]),
    // **And the same name at the head of a pipeline**, which is the line that answered nothing at
    // all until milestone 50's draining lane. An input operand is resolved by the planner, and the
    // shell used to wire a pipeline's head off the `Line` (which has no `<` on it), so the planned
    // source was dropped and the stage counted an empty stream (a `receive` on an empty slot answers
    // `NoSuchSlot`, which reads as end of document). Two spawned processes, and this shell feeds the
    // first.
    //
    // `2 4 24` plus a newline is seven bytes and three words on one line, so the answer is the
    // answer above it counted. Spelled out rather than derived for this file's reason: it is a boot
    // gate, and deriving one number from another would hide the case where both are wrong.
    line(2, "wc gate.txt | wc", &["1 3 7"]),
    // The negative control the pair would be weaker without. `wc` alone is refused **at the
    // prompt**, before anything is spawned, because its manifest declares that it reads a stream;
    // on Unix the same command is a shell that appears to hang. So the line above granted
    // something, rather than falling back on a default.
    line(0, "wc", &["name a file"]),
    // Milestone 632 (graphics on demand)'s refusal, on every ISA: these legs attach no gpu, so the
    // shell holds no display and `graphical_terminal` is refused at the prompt with a sentence rather than
    // spawned into nothing. It is the half of the milestone x86_64 can prove (no virtio-gpu is
    // wired there); the launch itself is `launch_graphical_terminal`'s, on a later boot of the same leg.
    line(0, "graphical_terminal", &["no display on this boot"]),
    // And `caps` says which file and how, which is the honest half: the shell reads it and streams
    // it in, so what the child holds is an endpoint and not a capability naming the disk.
    line(0, "caps wc gate.txt", &["input    gate.txt"]),
    // **Milestone 40 at the same interface.** `doc` is in the image, is spawnable, and declares that
    // it reads a stream, so bare `doc` is refused at the prompt before anything is spawned, exactly
    // as `wc` is and for the same reason: a viewer that could open the page it renders could open
    // any page.
    line(0, "mdr", &["name a file"]),
    // **The named file reaches the viewer and comes back rendered**, which two of this gate's own
    // comments said it did not until 2026-08-18. Both halves of that were fixed elsewhere and the
    // record was never corrected: the input operand now comes off the plan rather than off the
    // `Line` (`components/src/swish.rs`, the same fix `wc gate.txt | wc` above pins), and
    // `MAX_OUTPUT_CHUNKS` is 4096 rather than the 32 that would have truncated a page to 512 bytes.
    //
    // The numbers are the assertion and not decoration. `gate.txt` is 2 lines, 4 words, 24 bytes
    // (`wc gate.txt`, above). What comes back is **1 line, 4 words, 26 bytes**: the two source
    // lines are one paragraph re-flowed to one output line, and the two bytes are the body indent.
    // A viewer handed an empty stream would answer `0 0 0`, which is what this line answered when
    // the operand was being dropped, so the count is what separates rendering from silence.
    line(2, "mdr gate.txt | wc", &["1 4 26"]),
    // **And the line a person actually wants now renders**, which is milestone 40's whole
    // remaining phase (DECISIONS §106, 2026-08-22). `mdr gate.txt` alone used to make this shell
    // both the writer and the reader of one line, refused rather than hung, because it has one
    // wait point; see `grant_plan::check_chain` and notes/documentation.md for the refusal this replaced.
    // Now the render defaults to `terminal_sink_caretaker` instead of this shell's own result
    // endpoint, so there is no second reader for the shell to wait behind and the page appears at
    // the prompt with no `| wc` in front of it. The text is the same paragraph `mdr gate.txt | wc`
    // counted three lines up, reflowed and indented by the renderer: `gate.txt`'s two source lines
    // become the one line, four words, twenty-six bytes that count asserted, and this line checks
    // the words themselves arrived rather than merely being countable.
    line(1, "mdr gate.txt", &["hello world hello world"]),
    // **The negative control on the viewer itself**, and it is the whole milestone in one screen: a
    // documentation viewer is exactly the program a reader expects to go and fetch things, and this
    // one is handed a stream. `caps` prints what would be granted before anything is spawned, and
    // there is no file capability, no directory and no filesystem endpoint in it. The manifest is
    // byte-identical to `wc`'s, which is why the assertion is the same string.
    line(0, "caps mdr gate.txt", &["input    gate.txt"]),
    // **Milestone 40 phase 2, at the same interface**: the documentation store is installed, and a
    // search of it answers with pages a person can then open.
    //
    // `doc/bundles` is the manifest the search reads, and it is checked as a *file* first, with an
    // ordinary designation, because that is the claim underneath everything below it: the store is
    // real, it is where the reader thinks it is, and nothing special is needed to read it. The
    // numbers are the four bundle names and their newlines, so a bundle added to `DOC_BUNDLES`
    // fails here, which is right: the manifest is what the guest enumerates by.
    line(1, "wc doc/bundles", &["4 4 25"]),
    // **The query a person would type, against shards built from this repository's own markdown.**
    // Two bundles, so the answer is the merge across shards rather than one shard's list, and both
    // named pages are ones a reader wanting to know what a capability is here would want. The
    // counts are deliberately not asserted: they move whenever the notes are edited, and the claim
    // is which pages were found, not how often the word appears in them.
    line(
        0,
        "apropos capability",
        &["doc/swish/pipes.md", "doc/kernel/ipc-naming.md"],
    ),
    // The negative control, and the word is chosen to appear in **no bundled page**. See
    // notes/documentation.md's BUGS for why this one cannot be written into the note that documents it:
    // that note is itself in the store, so a word written there is a word the store then says.
    line(
        0,
        "apropos photosynthesis",
        &["no page in the store says photosynthesis"],
    ),
    // And a search with nothing to search for is refused, in the same sentence every other verb
    // that needs an operand uses.
    line(0, "apropos", &["name what you mean"]),
    // **The payoff, and the reason a search may be a builtin at all.** The name the search printed
    // is an ordinary designation: this line grants `wc` exactly that one page out of the store and
    // nothing else, resolved by the shell against the directory it holds. So search produced a
    // *name*, and the authority moved on the line where a person typed it. A search that had
    // handed a program the store's directory would have moved it three lines earlier and silently.
    line(
        0,
        "caps wc doc/kernel/ipc-naming.md",
        &["input    ipc-naming.md"],
    ),
    // **The clock, from the prompt** (milestone 51's wiring). The answer cannot be a constant, so
    // the check is the one word that separates a real time from both ways of not having one:
    // `Format::Human` ends in the offset's name and the two unknown-clock sentences ("the machine
    // has no clock it believes" / "this process holds no clock capability") contain no `UTC` at
    // all. So this fails if the clock service did not run, if the kernel granted the progenitor no page, if
    // the progenitor did not endow `date`, or if `date` was handed a page nobody published to.
    line(1, "date", &["UTC"]),
    // And the visibility surface agrees with the wiring. `caps` is the only thing in this system
    // that claims to print a process's whole authority, so a clock endowed and not printed would
    // make that claim false. Its wording is host-tested; this proves the wording is about a
    // capability the boot really moves.
    line(0, "caps date", &["cap 1  frame     clock"]),
    // **The inert-configuration page, from the prompt** (milestone 47's environment-variable fork,
    // DECISIONS §111). `date`'s own proof, one manifest field over: this fails if the kernel
    // granted the progenitor no config page, if the progenitor did not endow `printenv`, or if the page's validated
    // domains rejected the boot's own defaults, none of which a host test can see, because
    // `crates/system_initializer`'s spawn wiring is provable only against a real progenitor
    // (this file's module doc names `script/swish-check` as exactly that gate).
    line(1, "printenv", &["TZ=UTC", "LANG=C", "TERM=dumb"]),
    // And the visibility surface agrees with the wiring, `date`'s own check repeated for `config`:
    // `caps` claims to print a process's whole authority, so a config page endowed and not printed
    // would make that claim false.
    //
    // **And the values, before anything runs** (DECISIONS §111 (inert configuration is a validated
    // page)'s preview, milestone 47 (navigation and naming)). The shell prints them from its own
    // read-only view of the frame `printenv` was just handed, so this fails if the progenitor did
    // not place the view at `grant_plan::SHELL_CONFIG_SLOT`, did not map it at `SHELL_CONFIG_VA`,
    // or the shell's probe missed it (it then says it "cannot show their values" and none of the
    // three appears).
    line(
        0,
        "caps printenv",
        &[
            "cap 1  frame     config",
            "the page this shell reads too",
            "TZ=UTC",
            "LANG=C",
            "TERM=dumb",
        ],
    ),
    // **The shell edits its own line** (milestone 47 (navigation and naming), DECISIONS §227 (how
    // Tab reaches the shell) option D). Tab in command position finishes a program name from the
    // names the image carries, and the finished line runs; Tab after it finishes a file name from
    // the directory the word leads into, read with the shell's own `ENUMERATE`. Each fails if the
    // shell is not in raw mode (the terminal would drop the Tab and the echo would not match), if
    // the completion inserted the wrong text, or if the completed line did not run.
    keys(1, "printe\t", "printenv ", &["TZ=UTC"]),
    keys(
        0,
        "caps wc doc/kernel/ipc-nam\t",
        "caps wc doc/kernel/ipc-naming.md ",
        &["input    ipc-naming.md"],
    ),
    // **`^C` at the prompt is a byte now**, and the shell's editor discards the line: this one must
    // answer nothing (the check is `interrupted_at_prompt`'s), and the prompt must come back for the
    // next. The supervised jobs below still take `^C` through the terminal's count, which this
    // gate presses under `interrupt_heeder` and `interrupt_ignorer`.
    keys(0, "echo abandoned\x03", "echo abandoned^C", &[]),
    line(0, "echo kept", &["kept"]),
    // **`ps`, at the real prompt** (milestone 126 (the `procps` package)). The listing itself: a header, and at least the
    // row for `ps` itself, which is a member of the domain the progenitor spawned it into. Asserting the
    // header rather than a tid is deliberate: a tid is a generational name that moves with how many
    // jobs ran before it, and a gate that pinned one would be pinning the boot's history.
    line(1, "ps", &["TID  STATE"]),
    // **`ps` cannot see the machine, and this is the shape of the evidence at the prompt.** The
    // listing above is short: at this line the shell's domain holds `ps` itself and whatever else
    // the shell has running, which is nothing. A `/proc`-shaped `ps` would be listing the progenitor, the
    // shell, the terminal, the FS server, the compositor, the net stack and every driver.
    //
    // **The count is deliberately not asserted here.** `ps | wc` answered three lines on one run
    // and two on the next, because a pipeline spawns both stages into the same domain and whether
    // `ps` walks before or after `wc` exists is a race. That is truthful (a survey is a snapshot,
    // notes/process-view.md) and it makes a count a bad gate. The confinement claim is asserted
    // deterministically and on both ISAs in `kernel::user::survey_tests`, which builds the domain
    // it measures instead of inheriting one.
    // And `caps ps` prints the scope **before** anything is spawned, which is the half Linux has no
    // way to express: there, "which processes can this see" has one answer for every program on the
    // machine and no command line chose it.
    line(0, "caps ps", &["cap 7  endpoint  domain"]),
    // **`pgrep`, at the real prompt** (milestone 126), and what is asserted is deliberately not the
    // tids. `pgrep` prints nothing but names, one per line, and a tid is a generational name that
    // moves with how many jobs ran before it; a gate that pinned one would be pinning the boot's
    // history. So the claim is made through the **second stream**, which is the same trick the four
    // `date 2>` lines above use: `pgrep`'s diagnostics carry a sentence in exactly three cases (the
    // walk was refused, the selector can never match, or nothing matched), so an *empty* second
    // stream is the assertion that none of the three happened. This one line fails if the progenitor endowed
    // no domain, if it endowed one the kernel refuses, or if the filter came back empty.
    line(1, "pgrep 2> pgrep.txt", &[]),
    line(1, "wc < pgrep.txt", &["0 0 0"]),
    // And the asymmetry, printed before anything is spawned, which is the whole of what milestone
    // 126 has instead of the `pgrep`-beside-`pkill` comparison it originally promised. Two phrases:
    // the right (`ENUMERATE`, not `READ`, so the finder cannot receive a death or collect a corpse)
    // and the sentence that says so in English. There is no `caps pkill` line to put beneath this
    // one, because a tid is a name and no method turns one into authority.
    line(
        0,
        "caps pgrep",
        &["cap 7  endpoint  domain   ENUMERATE", "do nothing to them"],
    ),
    // **`top`, at the real prompt** (milestone 282 (a thread's CPU time, and the `top` it makes possible)), and what is asserted is the two things a
    // boot cannot make untrue: the summary line's opening, which proves the ambient uptime counter
    // and the domain count both reached the output stream, and the `TIME(ms)` header, which proves
    // the **second** walk happened. A `top` whose CPU-time walk was refused would print the table
    // without that column and this line would fail, which is the one failure a
    // `pgrep`-style empty-diagnostics check could not catch.
    //
    // The tids and the figures are deliberately not pinned. A tid is a generational name that moves
    // with the boot's history, and a CPU figure is a measurement of a real machine; a gate that
    // pinned either would be pinning this boot rather than the program.
    line(
        1,
        "top",
        &[
            "up ",
            "threads: ",
            "machine: ",
            "% busy since boot",
            "TID  STATE     TIME(ms)",
        ],
    ),
    // And the second stream is empty, the same trick the `pgrep 2>` line above uses: `top`
    // complains in exactly the cases `ps` does, so an empty second stream says none of them
    // happened and the table above it is the domain.
    line(1, "top 2> top.txt", &[]),
    line(1, "wc < top.txt", &["0 0 0"]),
    // The scope, printed before anything is spawned. `top` holds `ps`'s three capabilities and not
    // one more: the ranking costs no authority, because the CPU figures are a second walk of the
    // same endpoint under the same right.
    line(
        0,
        "caps top",
        &[
            "cap 7  endpoint  domain   ENUMERATE",
            "cap 11 frame     machine  read-only",
        ],
    ),
    // **`free`, `vmstat` and `slabtop`, at the real prompt** (milestone 126, DECISIONS §225 (`free`
    // sees the machine and your share)). What each line proves is that a read reached the output:
    // `Mem:` is the kernel's machine statistics page mapped into the child and recognized, `Yours:`
    // is `MemoryRegion::USAGE` answering on the job budget's `ENUMERATE` view. No figure is pinned,
    // since the free count is a measurement of this boot. The empty second streams say neither
    // read was refused.
    line(
        1,
        "free",
        &["total        used        free", "Mem:  ", "Yours:"],
    ),
    line(1, "free 2> free.txt", &[]),
    line(1, "wc < free.txt", &["0 0 0"]),
    line(
        0,
        "caps free",
        &[
            "cap 11 frame     machine  read-only",
            "cap 12 region    share    ENUMERATE",
        ],
    ),
    line(
        1,
        "vmstat",
        &["    r       free      total     in     cs busy  id"],
    ),
    line(1, "vmstat 2> vmstat.txt", &[]),
    line(1, "wc < vmstat.txt", &["0 0 0"]),
    // `slabtop`'s `threads` row is the proof that the kernel summed the budget's subtree: the budget
    // itself holds no thread, every job region split from it holds at least one.
    line(1, "slabtop", &["job budget: ", "SPENT ON", "threads"]),
    line(1, "slabtop 2> slabtop.txt", &[]),
    line(1, "wc < slabtop.txt", &["0 0 0"]),
    // **`uptime`, at the real prompt** (milestone 126). No domain, no clock: the manifest is
    // `least_authority_demo`'s, because `monotonic_nanos` is granted to every process unconditionally
    // (kernel/src/arch/*/timer.rs's exception to DECISIONS §10). A green line here proves the
    // program was loaded, measured, granted its report endpoint and actually ran at EL0; the exact
    // elapsed time is not asserted because a real boot's timing is not this check's business.
    line(1, "uptime", &["up "]),
    // **The installer** (milestone 198 (a package manager) rung 3a, DECISIONS §208 (installing a
    // package is granting it, and the activation set is versioned)), asked by `jig` since milestone
    // 809 (the package client becomes a program): a program holding an installer endpoint, where
    // this was the shell's `package` builtin. A package with one byte of its program flipped, and
    // its table of contents rewritten to agree, is refused by the image's catalog before anything
    // is written: the catalog is the only thing that can tell (`disk::stage_installed`). Nothing
    // is installed afterwards: the line after says generation 1.
    //
    // **What `jig` holds, before it runs** (DECISIONS §270 (a package manager holds an installer
    // endpoint, not the spawn endpoint)): the installer endpoint, which cannot spawn or vouch.
    line(
        0,
        "caps jig install greeting@0.1.0",
        &[
            "cap 2  endpoint  network  WRITE",
            // Milestone 892 (a std program at the prompt holds the network and a resolver): and the
            // resolver, for any name (calef's Q5 on #1884).
            "cap 9  endpoint  resolver  WRITE. it may look up the addresses of any name",
            "cap 15 endpoint  installer  WRITE",
            "cannot vouch for bytes, and cannot hand this on",
            "cap 17 frame     catalog  read-only",
        ],
    ),
    // **Grants by verb** (milestone 809, option V2): `jig list` holds the catalog and nothing
    // else, which the next line shows by what it can do. `caps` of it shows no installer row, and
    // a line can only assert what is printed, so the absence is proved by `jig list` running and
    // the preview tests in `crates/swish/src/preview/tests.rs`.
    line(
        1,
        "jig list",
        &[
            "packages this image vouches for (its catalog; no index yet):",
            "  greeting 0.1.0",
        ],
    ),
    // **A program without the installer grant is refused** (DECISIONS §270 (a package manager
    // holds an installer endpoint, not the spawn endpoint)): the same bytes, run by path, are
    // installed bytes nobody vouched for, endowed from their note, which asks for no installer and
    // could not ask for one. So the slot is empty, and nothing reaches the progenitor.
    line(
        1,
        "installed/jig rollback",
        &["this jig holds no installer endpoint; only the owner's console grants one"],
    ),
    // **From inside `downloads`, because a word is designated at the current directory and the
    // root cannot be narrowed** (`swish::designation`; `rm` meets the same refusal there). `./`
    // makes each word a file rather than a package name, and designates that one file read-only.
    line(0, "cd downloads", &[]),
    line(
        1,
        "jig install ./tampered.nifepkg",
        &[
            "refused: neither this image's catalog nor its index copy vouches for those bytes; nothing is installed",
        ],
    ),
    // **A package cannot take a name the image carries** (DECISIONS §229, calef's ruling of
    // 2026-09-27). `uptime`'s package is genuine and cataloged, and the image carries `uptime`, so
    // it is refused and nothing is written. Under §235 (the OS is built and updated from packages)
    // a base program is updated through the boot slot, never by install, so this blocks no update.
    line(
        1,
        "jig install ./uptime.nifepkg",
        &[
            "refused: the image carries a program of that name; a new base updates it, not \
             install; nothing is installed",
        ],
    ),
    // A genuine package the image lacks, whose digest the image's catalog carries: the
    // progenitor writes the program under `packages/<stem>/`, writes generation 1, and renames
    // `current` onto it.
    line(
        1,
        "jig install ./noteless.nifepkg",
        &["installed; generation 1 is live"],
    ),
    line(0, "cd /", &[]),
    // **And what it installed runs, by its bytes** (DECISIONS §219 (how the shell names an
    // installed program to the spawner) option D). A path, so the shell reads the file into frames
    // and the progenitor hashes its own copy and finds the digest in the generation just written.
    // Its line is the proof it ran: a refusal prints no such thing.
    line(
        1,
        "packages/noteless/0.1.0/noteless",
        &["noteless: installed, and carrying no manifest note"],
    ),
    // **`caps` names who vouched** (§219: "or the source that vouched"): the digest the shell
    // hashed is in the generation the install just wrote.
    line(
        0,
        "caps packages/noteless/0.1.0/noteless",
        &[
            "provenance: vouched by activation generation 1 (digest sha256:",
            // **No note, the default** (milestone 597 (a program carries its manifest in an ELF
            // note), provisional): `noteless` carries no manifest note, so it is bound and endowed
            // as `grant_plan::NO_NOTE_MANIFEST`, its output alone.
            "it carries no manifest note, so it asks for its output and nothing else",
        ],
    ),
    // **And bytes nobody installed, previewed** (§219 gate D2): no generation lists them, and the
    // boot prompt holds the run-unvouched capability (its provisional grant,
    // `crates/system_initializer`), so the preview is the ruling's endowment and says why it runs.
    line(
        0,
        "caps installed/unvouched",
        &[
            "cap 1  page      clock",
            "cap 2  page      config",
            "provenance: unvouched (digest sha256:",
            "runs on this session's capability to run unvouched bytes (slot 62)",
            // **What the note asks, beside what is granted** (milestone 597, provisional). The
            // witness's note asks for the three authorities it probes, and §219 says an unvouched
            // note grants nothing: the rows above are the ruling's three and no more.
            "its manifest note asks for: output bytes, entropy, the network, the process domain",
            "and its note grants nothing",
        ],
    ),
    // **And run: milestone 202 (every confinement test is a ritual until somebody breaks the confinement)'s claim that an unvouched child holds no capability the caller did
    // not delegate** (DECISIONS §219 gate D2). The bytes are `unreachable_network_witness`
    // stripped, so no table vouches for them and they run on the session's capability alone. The
    // line delegated the output and nothing else; the ruling adds the clock and configuration
    // pages. So each of the three authorities the progenitor holds must answer "refused", and the
    // census must read exactly slots 0, 1 and 2. Falsified once per authority before it was
    // committed: granting the domain, entropy or the network to an unvouched child turned this
    // line red (design/roadmap/202's block records the three runs).
    line(
        1,
        crate::disk::INSTALLED_UNVOUCHED,
        &[
            "network: refused (no capability at slot 10)",
            "entropy: refused (no capability at slot 9)",
            "domain: refused (no capability at slot 7)",
            "reboot: refused (no capability at slot 13)",
            "slots held: 0 1 2\n",
        ],
    ),
    // **An installed `std` program run by path hears its words** (milestone 205 (how a foreign
    // program is told what to do)). `std_echo`'s note declares `ArgSpec::Words` in the `std`
    // runtime, so the shell sent the line as its argv, the progenitor sized a `std` region from the
    // argv bit and built the `std` layout for an image, and `std::env::args_os()` read the page.
    // Unvouched, so its grants are §219's two pages at the `std` slots, and the preview says so.
    line(
        0,
        "caps installed/std-echo one",
        &[
            "cap 5  frame     clock",
            "cap 7  frame     config",
            "cap 8  frame     args",
            "its manifest note asks for: output bytes, its words, the clock, the configuration page",
        ],
    ),
    line(
        1,
        "installed/std-echo one 'two words'",
        &["words [\"installed/std-echo\", \"one\", \"two words\"]"],
    ),
    // **An image the size of `rg` runs from the prompt** (milestone 595 (the shell runs a `std`
    // program)). `crate::disk::INSTALLED_STD_ECHO_LARGE` is `std_echo` padded to 768 pages, twelve
    // times the cap this path had, so the shell staged it across two 2 MiB page-table spans, the
    // progenitor copied and hashed all of it in its image pool, and built it in a region sized from
    // its length. Before, the shell refused it as "larger than an image may be".
    line(
        1,
        "installed/std-echo-large big",
        &["words [\"installed/std-echo-large\", \"big\"]"],
    ),
    // **A foreign program's words designate what it may read, and nothing else** (milestone 205's
    // designation half, §170 (how a foreign program is told what to do) clauses 2 to 5). In a
    // directory of its own, `std_grep` (unvouched, so read-only) is granted `docs` because a word
    // named it, and finds the needle there. With no word naming anything it is granted nothing
    // (calef's N1 ruling, 2026-09-27T06:27Z), and its search of `.` fails loudly. Run by absolute
    // path, because the shell stands in `hay`; `/` is this shell's own root.
    line(0, "mkdir hay", &[]),
    line(0, "cd hay", &[]),
    line(0, "mkdir docs", &[]),
    line(0, "cd docs", &[]),
    line(0, "echo find the needle here > n.txt", &[]),
    line(0, "cd ..", &[]),
    line(
        0,
        "caps /installed/std-grep needle docs",
        &["cap 4  endpoint  dir", "docs", "read-only"],
    ),
    line(
        1,
        "/installed/std-grep needle docs",
        &["docs/n.txt:find the needle here"],
    ),
    line(
        1,
        "/installed/std-grep needle",
        &["std_grep: .: no directory was granted to search"],
    ),
    // **Unmodified `ripgrep` searches what its words name, from the prompt** (milestone 595 (the
    // shell runs a `std` program), fatal risk 1). The same lines `std_grep` answered, typed to
    // somebody else's program: crates.io's `rg` 14.1.1 as basalt packaged it, unvouched, carrying only the note `cargo
    // xtask foreign-note` linked in. `docs` is granted read-only because a word named it, and the
    // match is found there. `../hay/secret.txt` names the same directory's other file by a path
    // whose first component is not an entry here, so nothing is granted for it and `rg` cannot
    // open it, though the needle is in it: the caretaker refuses the `..` by name, in the parallel
    // walker's words when milestone 812 (`std::thread::spawn` runs real threads in one address
    // space) gives `rg` more than one core. With no word naming anything `rg` holds no directory and asks for its working
    // directory, which the PAL refuses by design (its `sys/paths/nife.rs`),
    // so `rg` says so with a wrong guess ("did your CWD get deleted?"). Typed only when
    // `cargo xtask basalt-fetch` has run, as CI's rows do.
    line(0, "echo the needle outside the grant > secret.txt", &[]),
    line(
        0,
        "caps /installed/rg needle docs",
        &[
            "cap 4  endpoint  dir",
            "(only docs in it, named on the line)",
            "...read-only",
            "provenance: unvouched",
        ],
    ),
    line(
        1,
        "/installed/rg needle docs ../hay/secret.txt",
        &[
            "docs/n.txt:find the needle here",
            "rg: ../hay/secret.txt: ", // then walkdir's "IO error for operation on", one core only
            "`..` would leave the granted directory, which no capability designates",
        ],
    ),
    line(
        1,
        "/installed/rg needle",
        &["rg: failed to get current working directory: operation not supported on this platform"],
    ),
    line(0, RG_WALK_CD, &[]),
    // **Unmodified `ripgrep` walks a real tree from the prompt** (milestone 121 (`ripgrep`:
    // enumeration as a capability), calef's 2026-10-07 ruling on fatal risk 1). `/search/crates` is
    // a copy of this repository's `crates/` (`crate::disk::RIPGREP_CORPUS`), hundreds of files in nested
    // directories, granted read-only because the word named it. No flag narrows the walk: `rg`
    // lists every directory it meets, opens every file, skips the binary ones itself, and prints
    // every matching line. The answer is wanted whole, so this line lists nothing here and
    // [`rg_walk_verdict`] compares it line for line with what the host's `rg` printed over the same
    // copy. Typed only when `cargo xtask basalt-fetch` has run, like the lines above.
    line(1, RG_WALK_LINE, &[]),
    line(0, "cd /", &[]),
    // **A note that asks more than its vouch allows is refused** (milestone 597, provisional).
    // `least_authority_demo`'s note declares an argument, which only a command line designates,
    // and unvouched bytes may hold only what the D2 ruling names. The shell binds the line
    // against the note (it cannot know the verdict), so the refusal is the progenitor's, on its
    // own copy, and nothing is built. `caps` says so first.
    line(
        0,
        "caps installed/asks-an-arg 5",
        &["more than unvouched bytes may hold"],
    ),
    line(
        0,
        "installed/asks-an-arg 5",
        &["the progenitor read those bytes' own manifest, and it does not allow this line"],
    ),
    // **A note that cannot be read is refused, not ignored** (milestone 597): the witness again,
    // with its note's version word set to 9 (`disk::with_note_version`). Running it as if it
    // carried no note would endow a program with a manifest it did not declare.
    line(
        0,
        crate::disk::INSTALLED_MALFORMED_NOTE,
        &["carries a manifest note that cannot be read"],
    ),
    // **Fetching, refused before the network is touched**: the catalog names no such package,
    // so nothing is asked of the package source. `jig` reads its copy of the catalog and sends
    // nothing, so it names no generation: it holds no view of the activation set.
    line(
        1,
        "jig install nosuch",
        &["refused: this image's catalog names no such package, so nothing was fetched"],
    ),
    // **And twice more, so this boot starts nine `jig install`s** (milestone 892 (a std program
    // at the prompt holds the network and a resolver)). Each holds the network and so a resolver
    // badge, and the resolver has eight windows: without the forget at each job's reap, the ninth
    // `jig` of the boot is refused a badge and the shell says it could not spawn.
    line(
        1,
        "jig install nosuch",
        &["refused: this image's catalog names no such package, so nothing was fetched"],
    ),
    line(
        1,
        "jig install nosuch",
        &["refused: this image's catalog names no such package, so nothing was fetched"],
    ),
    // **A lying package source.** The gate serves this leg a copy of `uptime` whose program has one
    // byte flipped and whose table of contents agrees (`disk::stage_installed`), under the genuine
    // name: a whole, well-formed HTTP exchange of a well-formed package. Only the image's
    // catalog can refuse it. `jig` checks the bytes against its copy before it sends them
    // (milestone 809's item 8), so the installer is not asked.
    line(
        1,
        "jig install uptime",
        &[
            "refused: this image's catalog does not vouch for those bytes; nothing was sent to \
             the installer",
        ],
    ),
    // **Fetched over the booted system's network and installed** (rung 3a's first gap): the
    // progenitor finds `greeting`'s stem in the catalog, fetches it from the gate's package
    // source through the stack it built at boot, and installs what arrived as it installs a file.
    // **Two versions cataloged, so a bare name is refused** (milestone 614 (two installed
    // versions of one program, each runnable, and a caller granted the one it needs)). The image
    // vouches for `greeting` at 0.1.0 and at 0.2.0 (every archive build builds every recipe), and
    // nothing orders versions, so a bare fetch names no one package. Before this refusal the first
    // catalog line won, which recipe filenames ordered as 0.2.0; the source serves only 0.1.0,
    // and every leg that fetched answered "the package source did not send a whole package". The
    // catalog refuses before the network is asked.
    line(
        1,
        "jig install greeting",
        &[
            "refused: this image's catalog vouches for several versions of that package; \
             name one with <package>@<version>",
        ],
    ),
    // **Fetched by `jig`, not by the progenitor** (milestone 809): `jig` holds the network and
    // reads the package source itself, and the progenitor, which no longer links an HTTP reader,
    // installs what `jig` sends exactly as it installs a file.
    line(
        1,
        "jig install greeting@0.1.0",
        &["fetched and installed; generation 2 is live"],
    ),
    // **And it runs** (rung 3a's second gap): bytes no boot image carries, vouched only by the
    // generation just written. The line it prints is its own; no program in the image prints it.
    // That the image lacks `greeting` is checked on the host before the boot, by reading the
    // archive (`disk::stage_installed`), because the prompt cannot tell: `greeting` is no
    // `grant_plan::Prog`, so its bare name is refused whether or not the archive packs it.
    line(
        1,
        "packages/greeting/0.1.0/greeting",
        &[
            "hello from a package this image never carried",
            "clock: held at slot 1, as its manifest note asked",
        ],
    ),
    // **And the manifest it runs with is the one it carries** (milestone 597, provisional): a
    // vouched note is honored, which the `clock: held` line above proves at the child, and `caps`
    // shows here at the prompt.
    line(
        0,
        "caps packages/greeting/0.1.0/greeting",
        &[
            "clock    read-only. it can read the time",
            "its manifest note asks for: output bytes, the clock",
        ],
    ),
    // **And by its bare name** (DECISIONS §229 (how a bare name at the prompt reaches an installed
    // program), B2): the live generation's entry of that name, run down the same road as its path,
    // so `caps` names the same generation.
    line(
        1,
        "greeting",
        &["hello from a package this image never carried"],
    ),
    line(
        0,
        "caps greeting",
        &["provenance: vouched by activation generation 2 (digest sha256:"],
    ),
    // **Milestone 614: the second version installs beside the first** (rulings 2 and 3). Rows key
    // on the digest, so installing over a live version appends and moves the default pointer
    // instead of replacing. The package is `greeting` at 0.2.0, whose member is `greeting_v2`'s
    // bytes under the name `greeting` (`as` in the recipe), so its digest differs from 0.1.0's and
    // the table holds both. The gate installs from the disk on all three legs; the image's
    // catalog carries the stem because every archive build builds every recipe for its
    // architecture.
    line(0, "cd downloads", &[]),
    line(
        1,
        "jig install 0.2.0/greeting.nifepkg",
        &["installed; generation 3 is live"],
    ),
    line(0, "cd /", &[]),
    // **And the new version runs by its path**, printing its own line, which is how the transcript
    // tells the two copies apart.
    line(
        1,
        "packages/greeting/0.2.0/greeting",
        &["hello from the second copy of the package"],
    ),
    // **And the old version still runs by its path, at two versions live.** This is the line the
    // one-entry table made impossible: its digest left that table at the install above, and a
    // digest not in the live generation is `SPAWN_UNVOUCHED`.
    line(
        1,
        "packages/greeting/0.1.0/greeting",
        &["hello from a package this image never carried"],
    ),
    // **The version set, the ruled selection** (ruling 4). The nearest `.jig-versions` file at or
    // above the working directory (here the root's) says `greeting 0.1.0`, and the bare word runs
    // that version, although the default pointer names 0.2.0. The set only selects among live
    // versions; a cloned repository can ask, and cannot run uninstalled bytes.
    line(0, "echo greeting 0.1.0 > .jig-versions", &[]),
    line(
        1,
        "greeting",
        &["hello from a package this image never carried"],
    ),
    // **The explicit ask** (ruling 4's other override): `program@version` answers its own row, and
    // is not reached by the image's claim on the bare name because it is not the bare name.
    line(
        1,
        "greeting@0.1.0",
        &["hello from a package this image never carried"],
    ),
    // **And the divergence notice**, the guard on the whole mechanism: the set now names a version
    // nobody installed, so the default runs and the spawn line says both, naming the file to fix
    // (wording ratified 2026-10-07, §258 (names for two installed versions of one program)).
    line(0, "echo greeting 0.9.9 > .jig-versions", &[]),
    line(
        1,
        "greeting",
        &[
            "greeting 0.2.0 (.jig-versions asks for 0.9.9)",
            "hello from the second copy of the package",
        ],
    ),
    // **A name the image and a package both have** is refused at the prompt, naming both (§229
    // B2), but no line here can make one: install now refuses an image program's name (§229,
    // 2026-09-27), and only a later base adding a name a package holds can produce the pair.
    // `swish::bare`'s host tests prove the refusal; its BUGS say what is not gated.
    // The bare name of what generation 1 installed, which is not the image's either.
    line(
        1,
        "noteless",
        &["noteless: installed, and carrying no manifest note"],
    ),
    // **The owner suspends and resumes an identity** (milestone 152 (durable delegation), calef's
    // §108 (disabling credentials kills the durable session) ruling of 2026-09-26). `user` edits `login_protocol::SUSPENDED_LIST` at this prompt's
    // root, the file service's, in the format `login` reads at a login and at start-up; the kernel
    // suite's `login_tests` prove what it does with it. Here: the
    // two verbs answer, the file is where `login` looks, and a malformed line changes nothing.
    line(0, "user suspend alice", &["suspended alice"]),
    line(0, "ls", &["suspended"]),
    line(0, "user resume alice", &["resumed alice"]),
    line(
        0,
        "user delete alice",
        &["user suspend <name> | user resume <name>"],
    ),
    // **The owner vouches for a local build** (DECISIONS §221 (the boot prompt is the owner's
    // console), ruling 1). `installed/unvouched` is the fresh build the D2 lines above ran on the
    // ruling's endowment (slots 0, 1 and 2). Vouching writes a generation that lists its digest,
    // so the same bytes now run vouched, endowed from the manifest note they carry (milestone 597
    // (a program carries its manifest in an ELF note)). The witness's note asks for the process
    // domain, entropy and the network and no clock, so the census moves from D2's `0 1 2` to
    // `0 7 9 10` (10 is the network stack's endpoint): the grant changed with the
    // vouch, and nothing else on the line did. The probe lines then read as reached, which their
    // wording ("declared no ...") was not written for. A rollback takes the vouch away and the
    // census is D2's again.
    line(
        0,
        "vouch installed/unvouched",
        &["vouched; generation 4 is live"],
    ),
    line(
        0,
        "caps installed/unvouched",
        &["provenance: vouched by the owner in activation generation 4 (digest sha256:"],
    ),
    // **A vouch claims no name** (§229 B2): the entry is found by the bytes' digest and never by
    // the name it was recorded under, so the bare word reaches nothing.
    line(0, "unvouched", &["no such program"]),
    line(1, crate::disk::INSTALLED_UNVOUCHED, &["slots held: 0 7 9"]),
    line(1, "jig rollback", &["rolled back; generation 3 is live"]),
    line(
        0,
        "caps installed/unvouched",
        &["provenance: unvouched (digest sha256:"],
    ),
    line(
        1,
        crate::disk::INSTALLED_UNVOUCHED,
        &["slots held: 0 1 2\n"],
    ),
    // **`uuid`, at the real prompt** (milestone 111), and this is the only gate that can run it.
    // The endowment is `crates/system_initializer`'s to make: the progenitor holds the entropy service's
    // request endpoint and places a `WRITE` view of it at `grant_plan::ENTROPY_SLOT` for a child
    // whose manifest declares it, exactly as it places the clock. Every other test that runs the
    // shell has the KERNEL play the progenitor, and `Spawn` fills a capability table from slot 0 upward, so
    // nothing else in this tree can put a capability at a slot a manifest names.
    //
    // **The value is deliberately not asserted, and the shape is.** A version-4 identifier that a
    // gate could predict would be a version-4 identifier drawn from nothing, so pinning one would
    // assert the opposite of what this milestone is about. What is pinned is the framing: `uuid >
    // id.txt` puts 36 characters and a newline in a file, and `wc` counts one line, one word, 37
    // bytes. A `uuid` the progenitor endowed nothing would leave that file **empty** (its refusal goes to the
    // second stream, `Manifest::output`'s whole reason here), so `1 1 37` fails on exactly the
    // condition this milestone exists to create.
    line(1, "uuid > id.txt", &[]),
    line(1, "wc < id.txt", &["1 1 37"]),
    // And the second stream is empty, which is the same trick the `pgrep 2>` line above uses: this
    // program complains in exactly one case (it holds no entropy capability), so an empty second
    // stream is the assertion that the case did not happen. The two lines together are "it drew
    // real bytes and had nothing to complain about", said without asserting any byte of them.
    line(1, "uuid 2> ent.txt", &[]),
    line(1, "wc < ent.txt", &["0 0 0"]),
    // **A `std` program's words on a redirected line** (milestone 205 (how a foreign program is
    // told what to do)): `>` sends the line down the pipeline path, `spawn_stage`, rather than
    // `spawn`'s, so the argv takes the other of the shell's two routes. The count is
    // `std_tests::EXPECTED`'s transcript with its `args []` line read as `args ["std_exerciser",
    // "redirected"]`: 15 lines, 42 words, 272 bytes.
    //
    // **Not `std_exerciser | wc`**, which was this line until CI said otherwise on 2026-09-27. The
    // job pool returns a region only when it is the most recent carve, and a pipeline's head is
    // carved first and reaped first, so a `std` head strands its whole region and no `std` job
    // runs again until reboot. See 205's BUGS.
    line(1, "std_exerciser redirected > args.txt", &[]),
    line(1, "wc < args.txt", &["15 42 272"]),
    // And the visibility surface, which is what a person meets before anything is spawned. On
    // Linux there is nothing here to say: no tool reports whether a program will read
    // `/dev/urandom`, and nothing about running one reveals it either. Here it is a row, and the
    // right on it (`WRITE`, so it may ask the service and may not receive another client's
    // request) is the claim rather than decoration.
    line(
        0,
        "caps uuid",
        &[
            "cap 9  endpoint  entropy  WRITE",
            "draws no randomness at all",
        ],
    ),
    // **`2>`, at the one interface a human touches** (DECISIONS §67). The four lines below are the
    // whole of the decision, and only this gate runs them through the real progenitor: the guest tests
    // wire the shell from the kernel, whose `Spawn` fills a capability table from zero and cannot place a
    // capability at the slot a manifest names, so `date` there never receives a second stream.
    //
    // `date` is the declarer, and at *this* prompt it has a clock and nothing to complain about. So
    // the assertion is that its second stream exists, is separate, and is **empty**: `2> err.txt`
    // creates the file, `date` closes the stream with nothing on it, and `wc` counts zero of
    // everything. A shell that had merged the two streams would put a timestamp in there.
    line(1, "date 2> err.txt", &["UTC"]),
    line(1, "wc < err.txt", &["0 0 0"]),
    // And the visibility surface names the second destination, which is what stops `caps date >
    // when.txt` being a half-truth: two destinations on one line, and a reader can see that the
    // complaint is not going into the file.
    line(0, "caps date 2> err.txt", &["diags    err.txt"]),
    // The refusal, which is the other half of "a declaration, not a number". `wc` writes one stream
    // and its diagnostics ride it, so `2>` names nothing and the line does not run.
    line(0, "wc gate.txt 2> err.txt", &["declares no second output"]),
    // **`time`, at the one interface a human touches** (milestone 86). Only this gate runs the real
    // inits, and the clock the shell times with is granted by them: the guest tests wire it from the
    // kernel, so a boot where the progenitor never handed the shell a clock would pass every one of those and
    // print "this shell holds no clock capability" here.
    //
    // The answer is the same three numbers `wc gate.txt` gave four lines up, which is the claim the
    // milestone rests on: the tail runs exactly as typed and timing it changes nothing about it. A
    // `time` that re-tokenized its tail, or spawned a differently endowed child, would answer
    // something else here and the duration would still look fine.
    line(1, "time wc gate.txt", &["2 4 24"]),
    // And the duration itself, checked for its shape rather than its value: the number is a real
    // measurement and cannot be a constant, but `real` and a unit are what a stopwatch prints.
    line(1, "time date", &["time: real"]),
    // The visibility surface agrees with the wiring, the same pairing `caps date` makes for the
    // child's clock. This one is about the shell's own: `caps` is the only thing in this system that
    // claims to print a process's whole authority, and a clock the boot really grants would make
    // that claim false if it went unprinted. The rights half is the load-bearing word: READ without
    // GRANT is why nothing typed here can hand a clock to a child.
    //
    // **The second wanted phrase is milestone 31 phase 3's**, and it is the machine-checked form of "flip
    // `holdings()`": the shell's `holdings().dir` is true exactly when the progenitor granted it a directory,
    // and this row is the only place a person can read that at the real prompt. Every other test
    // that runs the shell has the kernel play the progenitor, so a boot that stopped granting it would fail
    // nothing; `wc gate.txt` above would keep working, because the shell opens that file itself.
    line(
        0,
        "caps",
        &[
            "frame     clock      READ only, NOT delegable",
            "endpoint  directory",
        ],
    ),
    // **Milestone 31 phase 3, at the one interface a human touches** (2026-08-17). Naming a
    // directory in a command IS granting a capability to it, and until this landed the prompt could
    // say so and not do it: the progenitor deleted the file service during the boot, so a directory grant had
    // nothing to build a caretaker out of and `rm` was a refusal.
    //
    // Four lines, and they are one argument in order. **The preview first**, because the whole claim
    // of this milestone is that the authority is legible before it moves: the row names the
    // directory the grant is over and the sentence says what `-r` would have added, so a reader can
    // see the narrower of the two capabilities being chosen.
    line(
        0,
        "caps rm rmtree/rm-solo",
        &[
            "dir      /rmtree  (the directory holding rm-solo)",
            "and nothing under it: no -r, so it cannot even look",
        ],
    ),
    // **The removal, through a caretaker the progenitor built for this one command.** `-v` because `rm`'s
    // default is silence and a gate needs something to read; the name it prints is the name the
    // command line designated, which is the whole of the endowment.
    line(1, "rm -v rmtree/rm-solo", &["rm-solo"]),
    // **And exactly that name went.** Two entries left in a directory that had three, so the grant
    // took what was designated and not what it could reach: `rm-keep` and the whole `rm-doomed`
    // subtree were inside the same capability and are still there, because nothing named them.
    // Eleven bytes of `rm-doomed/` and eight of `rm-keep`, newlines included.
    line(1, "ls rmtree | wc", &["2 2 19"]),
    // **The one shape this still cannot deliver, and the refusal now says what is true.** It used to
    // read "needs the progenitor to build the caretaker", which stopped being true on the line above. A
    // caretaker's whole attenuation is one `OPENDIR` *into* the granted directory, and a name typed
    // at the top prompt designates the root of this shell's namespace, which has no name to descend
    // into; the contract has no verb for "the directory I already hold, with fewer rights". So this
    // is a refusal at the prompt with **nothing spawned**, which is the one outcome this model must
    // never trade away, and it is a design fork rather than a missing line of code. See
    // design/roadmap/0031-capability-shell.md and notes/dir-capability.md's BUGS.
    line(0, "rm gate.txt", &["there is no name here to descend into"]),
    // **`xargs`, at the one interface a human touches** (milestone 109). `globmany` holds eleven
    // names one pattern matches, which is more than the eight a single grant can carry.
    //
    // The negative control first, and it is the state of the world this milestone answers: unbatched,
    // a match over the bound is a **refusal at the prompt with nothing spawned**, which is milestone
    // 47's answer at the bound and the reason `xargs` was raised. It still is the answer, because
    // batching is opt-in: a line that silently ran N times would make `caps rm *.txt`'s single
    // printed grant a lie.
    line(
        0,
        "echo globmany/m-*.txt",
        &["matched more names than one grant can carry"],
    ),
    // And batched, the same pattern is swept. **Asserting the second batch is what pins the resume
    // rule**: `m-08.txt` first means batch one ended at `m-07.txt` and the watermark carried, so
    // this one line rules out an off-by-one at the boundary, a batch that restarted from the top,
    // and a batch that took the first eight the directory happened to yield.
    line(
        0,
        "xargs echo globmany/m-*.txt",
        &["batch 2: m-08.txt m-09.txt m-10.txt"],
    ),
    // **And the authority per batch is exactly that batch**, which is the claim the milestone rests
    // on and the one only `caps` can make before the delegation chain exists. The preview prints
    // what `rm` would be handed, and what it would be handed in the second invocation is the three
    // remaining names: not the eleven the pattern matched, and not the directory they live in.
    line(
        0,
        "xargs caps rm globmany/m-*.txt",
        &["the directory holding m-08.txt m-09.txt m-10.txt"],
    ),
    // **Quoting, at the one interface a human touches** (milestone 67). The gap it closes is an
    // authority one: a file called `my notes.txt` could not be *named* before this, so it could not
    // be granted, in a shell whose whole thesis is that naming a resource is granting it.
    //
    // Three lines, and the third is the one that makes the pair a claim. The `>` writes twelve bytes
    // plus a newline into a name only quoting can express; the `<` reads them back; and `wc "my
    // notes.txt"` is the same designation with the operator left out, so if the two disagree, one of
    // them opened something else. That is `gate.txt`'s trio above, asked of a name with a space in
    // it.
    line(0, "echo hello world > \"my notes.txt\"", &[]),
    line(1, "wc < \"my notes.txt\"", &["1 2 12"]),
    line(1, "wc \"my notes.txt\"", &["1 2 12"]),
    // **And the one thing quoting does to authority**: it suppresses expansion, so the same four
    // characters are one name quoted and a set unquoted. `echo` prints what a grant would move, so
    // this line is the narrowing made visible before anything moves. Unquoted, the same pattern is
    // the refusal five lines up.
    line(0, "echo \"*.txt\"", &["*.txt"]),
    // And the preview names it, which is the pairing `caps` exists for: what the line designates is
    // on the screen before anything moves, and a name with a space in it is now something that
    // sentence can be about.
    line(0, "caps wc \"my notes.txt\"", &["input    my notes.txt"]),
    // **Sequencing and the status** (milestone 67). `least_authority_demo 3` runs and `least_authority_demo` alone is refused at
    // the prompt for the integer its manifest requires, so these three lines cover both arms of the
    // condition table with real commands rather than with a branch written for a gate.
    line(1, "least_authority_demo 3 && echo yes", &["yes"]),
    line(0, "least_authority_demo || echo no", &["no"]),
    // **The decision this milestone settled, read at a prompt.** `least_authority_demo` alone is refused, and a
    // refusal is not an error: nothing was spawned, nothing was opened, and the status says so with
    // its own number. Unix cannot draw this line, because there `127` and a program's own `exit(1)`
    // are the same kind of integer.
    //
    // The bare `least_authority_demo` is here because the *first* draft of this gate put `echo $?` straight after
    // `least_authority_demo || echo no` and got `0`, which was the shell being right: the last thing that ran was
    // the `echo`. `$?` is the previous **command**, not the previous line, and that is bash's rule
    // and this shell's.
    line(0, "least_authority_demo", &["needs an integer argument"]),
    line(0, "echo $?", &["2"]),
    // **The progenitor's job budget is bounded and comes back** (milestone 22, the interactive increment).
    // The progenitor now holds a pool with room for six live jobs instead of the kernel's whole construction
    // budget, and every job runs in a region of its own that `job_undertaker` returns when the job ends.
    // **Every job above plus these six go through a six-job pool, several times over**, so a boot
    // where nothing collected would answer "could not spawn (the progenitor is out of memory)"
    // somewhere in here rather than the arithmetic. How many that is was a hand-kept figure in this
    // comment until 2026-09-26 and had drifted; it is now each line's `jobs` tag, summed in the
    // success summary. The `rm` above is the one worth noticing, because it is the first job whose
    // region holds **two** processes, the program and the `fs_subtree_caretaker` carrying its
    // grant, and it is therefore the first thing in this script that would fail if
    // `job_undertaker`'s retry did not collect both. Six distinct arguments rather than one
    // repeated, because the transcript is walked with a moving cursor and six identical answers
    // would let a missed line pass as its neighbor.
    line(1, "least_authority_demo 3", &["3*3 = 9"]),
    line(1, "least_authority_demo 4", &["4*4 = 16"]),
    line(1, "least_authority_demo 5", &["5*5 = 25"]),
    line(1, "least_authority_demo 6", &["6*6 = 36"]),
    line(1, "least_authority_demo 7", &["7*7 = 49"]),
    line(1, "least_authority_demo 8", &["8*8 = 64"]),
    // **The one spawnable program that answers in a register and had no line** until milestone
    // 150's coverage test (`every_spawnable_program_has_a_swish_check_line`) asked. After the six
    // above, so their count is unchanged; the page count it reports depends on page-table overhead,
    // so the assertion is the sentence that says the grant was spent rather than the number.
    line(
        1,
        "memory_grant_depleter --mem 4",
        &["-page budget you granted"],
    ),
    // **The network, from the booted system** (milestone 590 (the booted system starts its network
    // stack)). Every network test
    // before these four started `net_stack` from the kernel's harness; here the progenitor built it
    // at boot from the NIC the kernel granted it, and a program a person typed reaches the runners'
    // echo peer through it. The preview first, because the row it prints is the whole of who may
    // reach the network from this prompt.
    line(
        0,
        "caps network_echo_client --mem 4",
        &[
            "cap 10 endpoint  network  WRITE",
            "a program without this row reaches no network at all",
        ],
    ),
    line(
        1,
        "network_echo_client --mem 4",
        &["echo peer 10.0.2.9:7777 answered: nife-net!"],
    ),
    // **Twice, and the second is the one that says the stack outlives its clients.** The first job
    // handed `net_stack` a page and exited; its region was reclaimed, which revokes that page out
    // of the stack's address space too. A stack that kept a stale window, or a socket number that
    // could not be opened again, answers the first run and fails this one.
    line(
        1,
        "network_echo_client --mem 4",
        &["echo peer 10.0.2.9:7777 answered: nife-net!"],
    ),
    // **The negative control.** A program that declares no network `CALL`s the network's slot
    // anyway; the kernel must refuse it for want of a capability. A spawn service that endowed the
    // stack to every child prints `REACHED` here instead.
    line(
        1,
        "unreachable_network_witness",
        &[
            "network: refused (no capability at slot 10)",
            "entropy: refused (no capability at slot 9)",
            "domain: refused (no capability at slot 7)",
            "reboot: refused (no capability at slot 13)",
            "slots held: 0\n",
        ],
    ),
    // **`caps reboot` names the authority** (milestone 805 (`reboot` at the prompt), exit criterion
    // 3), at the real prompt and without running it: typing `reboot` mid-script would end this
    // boot. [`reboot_phase`] types it, after the last scripted line.
    line(
        0,
        "caps reboot",
        &[
            "cap 13 reboot    WRITE. restart the machine",
            "cap 14 endpoint  sync   WRITE. the file server answers SYNC on it and refuses",
        ],
    ),
    // **A supervised job, interrupted**, under DECISIONS §24 (interrupting the foreground
    // process), and these two are the only lines this gate presses `^C` for (see
    // [`SWISH_CHECK_INTERRUPTED`]). Neither was typed here
    // until 2026-09-25, and in that time the first supervised job of every boot failed with `could
    // not map the job frame`, because the shell mapped its job frame over its own terminal page.
    // The heeder is the cooperative tier: one `^C`, and the job reports it stopped. It goes first
    // because it is the first supervised job of the boot, which is the one that failed.
    line(
        0,
        "interrupt_heeder",
        &[
            "^C interrupts it.",
            "the job caught the interrupt and stopped cleanly after",
        ],
    ),
    // The forcible tier: the ignorer never looks at the flag, so the grace timeout escalates one
    // `^C` to a `DESTROY` of the region the shell built it from, which force-kills its live thread
    // (DECISIONS §16 (object revocation), as amended). A shell whose teardown was refused says so
    // instead.
    line(
        0,
        "interrupt_ignorer",
        &["^C again: tearing the job down.", "its memory reclaimed"],
    ),
    // **A `std` program, spawned by the progenitor rather than by the kernel's test harness**
    // (milestone 595 (provisional)). Until this, every `std` program that ran on nife was built by
    // `system_tests/src/user/std_service.rs`, and the progenitor had never produced a child in the layout
    // nife's `std` reads (`crates/std_runtime_protocol`): eight fixed slots, three shared pages,
    // thirty-two stack pages. The preview first, because it is where a person learns the slots
    // moved, and slot 0 is not even the same kind of object as a native child's.
    line(
        0,
        "caps std_exerciser",
        &[
            "cap 0  untyped   heap.",
            "cap 1  endpoint  result   stdout and stderr",
            "cap 5  frame     clock",
            "cap 6  endpoint  entropy  WRITE",
            "cap 7  frame     config",
            "cap 8  frame     args",
        ],
    ),
    // **Every phrase is a slot or a page landing where `std` looks for it**, which is why the
    // transcript is the test: the program asserts rather than prints wherever the answer is not
    // deterministic, and panics (a fault this gate fails on) when one is missing. `vec sum` is the
    // heap at slot 0; that anything prints at all is stdout at slot 1; the two `honestly
    // unsupported` lines are slots 4 and 2 left empty, so there is no ambient filesystem or network
    // to fall back on; `wall clock ok` is slot 5 and the page at `CLOCK_PAGE`; `entropy ok` is slot
    // 6; `config seeded` is slot 7 and the page at `CONFIG_PAGE`, read before `main`; and the last
    // line is `process::exit` reaching the supervisor as an exit rather than a fault. The stack is
    // the one thing with no phrase of its own: too little of it is a fault partway through.
    //
    // **`args [...]` is milestone 205 (how a foreign program is told what to do)'s** (DECISIONS §170 (how a foreign program is told what to do)): the shell wrote the line's words onto a
    // page, the progenitor copied it into the child's region at slot 8, and `std::env::args_os()`
    // read it back, `argv[0]` first and the quoted phrase as one word with its quotes off.
    line(
        1,
        "std_exerciser one 'two words'",
        &[
            "hello from std on nife",
            "args [\"std_exerciser\", \"one\", \"two words\"]",
            "vec sum 149985000",
            "fs honestly unsupported",
            "net honestly unsupported",
            "wall clock ok",
            "entropy ok",
            "config seeded",
            "exiting through process::exit",
        ],
    ),
    // **An unquoted pattern is refused at the prompt, with nothing spawned** (§170): the words
    // carry no authority, so expanding one would hand over names and nothing they name.
    line(0, "std_exerciser *.rs", &["its words are not expanded"]),
    line(0, "echo shell-boot-gate-done", &["shell-boot-gate-done"]),
];

/// **The lines after which this gate presses `^C`**, once the shell has said the job is running.
/// Each runs until interrupted, so without the keystroke the prompt never comes back. A program
/// here is also one `every_spawnable_program_has_a_swish_check_line` would otherwise have to excuse.
pub(super) const SWISH_CHECK_INTERRUPTED: [&str; 2] = ["interrupt_heeder", "interrupt_ignorer"];

/// What the shell prints once a supervised job is running and watched. Pressing `^C` before it
/// would still be counted (the shell's watermark catches an early one), but after it is the case a
/// person types, so it is the one this gate proves.
pub(super) const SWISH_CHECK_RUNNING: &str = "^C interrupts it.";

#[cfg(test)]
mod tests;
