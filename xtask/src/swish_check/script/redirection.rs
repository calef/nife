//! **Redirection on both streams** (DECISIONS §67 (a program's second stream is a declaration, not a number)), with `uuid` and `time` as the programs
//! that exercise it: `>`, `<` and `2>`, the empty second streams, and the previews that
//! name each destination.
//!
//! Cut from the table in `script.rs` verbatim by milestone 900 (the swish-check script
//! splits by topic before it crosses the ceiling).
//!
//! Name: provisional (milestone 900's lane, 2026-10-15 UTC).

use super::*;

/// The redirection rows of [`SWISH_CHECK_SCRIPT`], in table order.
pub(super) const LINES: &[Line] = &[
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
];
