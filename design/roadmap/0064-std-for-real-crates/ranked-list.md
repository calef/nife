# The ranked list, pass by pass, 2026-08-04 to 2026-08-22

An appendix to [milestone 64 (enough `std` to run somebody else's
crate)](../0064-std-for-real-crates.md), moved out of that page on 2026-10-12 (UTC) under [§212 (a
prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md). The main page is complete
without it. This is the block's account of how the ranked list was worked, from the first pass to
the fourth, as the block carried it before the last pass bound file times.

## Why the third pass's headline was wrong

No gate held it back, and the fourth pass had to establish that rather than assume it, because the
third pass's report said the opposite: *"the ranked list is now genuinely exhausted for a lane under
these constraints: every remaining row is a decision."* That sentence was wrong twice, and this
block carried a version of it too:

- `std::process::exit` was a trap instruction, found by building the gate the third pass said
  was missing. Not a decision, and not on the list, because the list cannot see a function that ends
  the process. Fixed here.
- `TcpListener` is a binding, not a decision. Rank 21 was a contract gap when it was written and
  stopped being one at milestone 107; `sys/net/connection/nife.rs`'s own doc comment has said so
  since, in the words *"small and mechanical"*. It was the largest piece of ordinary work left in
  this milestone and needed no decision from calef. It is bound now ([the fourth
  pass](tcp-listener-and-file-times.md), PR #341, 2026-08-18).

## The measurement became a script

The measurement is a script now, `script/crate-probes` (name provisional), and that is the third
pass's first deliverable. It had been a prose recipe re-derived by hand three times, and two of those
three produced a wrong headline. A probe is a `[[bin]]` whose `main` calls the crate. Both halves
were learned by recording `diesel` as a pass twice. A probe that fails is rebuilt for the host and
reports `BODY` rather than `FAIL` if the host fails too. That is the check fifty hand-written call
sites could not give anyone.

That split was recorded as 35/15, then 39/11, and is 43/7. The first correction (2026-08-17) was
arithmetic: the headline summed the four failure-class headings, and four crates are in two classes
at once (`zip` and `ring` in A and C, `gix-config` and `gix` in A and B). The second (2026-08-18) is
that 39/11 describes the tree *before* `entropy_backend`, which had landed the same day the number
was re-derived. `rand`, `uuid`, `gix-object` and `gix-actor` build now. The second run confirming
they still fail without it is one flag rather than an argument (`script/crate-probes --no-backend`).

## What the first and second passes closed

Five ranks turned out to be bindings rather than verbs (`create_dir`, `read_dir`, `remove_file`,
`remove_dir`, `rename` were all dispatched by the FS server since milestones 47 and 48; only the
client refused). The second pass added four more, working the list from the top:

- Rank 1, the `getrandom` backend, which was 8 of the 11 build failures and is the one gap on the
  list that is not a `std` gap at all. `entropy_backend` is the fix, on `getrandom`'s own
  documented custom-backend hook over `std::random::SystemRng`. `rand`, `uuid`, `gix-object` and
  `gix-actor` build for nife now, and the other four no longer fail on `getrandom` at all.
- Rank 4, `env`, and it is the sting in a second place: `env::var` was recorded as "no PAL at
  all", which sounded harmless because `getenv` answered `None` honestly. The same fallback's `env()`
  is a `panic!`, so `std::env::vars()` aborted the process, and it compiled perfectly.
- Rank 8, `File::set_len` and rank 26, `fs::copy`. Rank 9 (`symlink_metadata`) needed nothing:
  the row was stale, std routes it to `lstat`, which this PAL binds.

## The third pass: what the neighboring functions do

The third pass asked a different question, and it is the one worth carrying. Not "what is next on
the ranked list" but "what do the neighboring functions in that module do", which is where the
second pass's best finding had come from by accident. Run across every module the PAL falls through
rather than binds, it found three more std calls that abort a nife process, all of which compiled
perfectly: `std::env::temp_dir()`, `std::env::split_paths()` and `std::process::id()`. Two are now
answered and one is a constant with its reasoning recorded (`sys/paths/nife.rs`, `sys/process/nife.rs`);
`std_exerciser` asserts both the answers and the four neighboring refusals on both ISAs.

None of the three could have appeared on the ranked list, and that is the finding rather than the
functions. The list is built from PAL functions that answer `Unsupported`, and a function that aborts
never answers. `env::temp_dir` did have a row, at rank 16, reading "no PAL at all ... needs a
namespace answer, not a PAL one", which is how a fatal defect got filed as a design question; its
demand was undercounted by four probes too. It also corrects this note's most-quoted sentence:
`tempfile` did not "build, link, and return an error", it died inside `std::env::temp_dir` before
reaching its own refusal.

## The fourth pass: the method became a check

The fourth pass turned the third pass's method into a check, and the check found a fifth.
`cargo xtask std-aborts` (name provisional) asks cargo's dep-info which `library/std/src/sys/**`
sources rustc compiled for the nife targets. It greps exactly those for bodies that end a process,
against a list carrying a reason per entry. It runs inside `script/test`. That is rung two of
AGENTS.md's ladder where this block's own BUGS section had rung four. It was worth building rather
than reading a fourth time, because `std::process::exit()` was `crate::intrinsics::abort()` and no
amount of reading would have found it.

`sys/exit.rs` is not a `sys/<module>/mod.rs` backend; its `cfg_select!` lives inside a function, so
"read every module the PAL falls through" has nothing to read. It hid behind a second thing too.
`_start` calls the PAL's `rt::exit` on `main`'s return directly, and `std::process::exit` is std's
only caller of `sys::exit::exit`, so every std test in this tree ended by the path that worked. A
clean exit was arriving at its supervisor as `EVENT_FAULT` with a pc and a faulting address.

What that leaves open is a kernel change rather than a PAL arm. `sched::exit()` is
`depart(EVENT_EXIT, 0, 0)`, so a supervisor can tell exit from crash and cannot tell `exit(0)` from
`exit(1)`. §301 (exit status and output) rules the wire (the status rides the `EVENT_EXIT` message's zero word) and
milestone 172 (a capability-native subprocess primitive) builds it; notes/std.md records the gap.

## What remained after the fourth pass

What remains, and why each one stops here rather than being unfinished. Rank 2, the `std::os::unix`
fallthrough, wants a uid and a file mtime set that this system does not have in the form the crates
ask for. Answering would be a Unix fiction over a capability refusal. Rank 3, `thread::spawn`, is
this block's own scheduling question, now decided (§105 (`std::thread::spawn` stays declined)):
declined for now, for want of a customer (see the page's BUGS), and has no build failures behind it.

*(The next three sentences are the 2026-08-18 reading and are wrong now: the `touch` of milestone 47
(navigation and naming) added the verbs on 2026-08-24, and both ranks were bound by path on
2026-09-19; see [the last pass](tcp-listener-and-file-times.md).)* Rank 19, `Metadata::modified`, is
one field in `FSTAT`'s reply away and that makes it a wire-format change; it wants a `DECISIONS`
section and calef, same as rank 2. Rank 28, `File::set_times`, is the same shape as rank 19 and was
never triaged by name in an earlier pass. Setting an mtime needs a verb the FS server does not have
any more than reading one does, so it is a second wire-format row behind the same decision rather
than a second decision.

Rank 21, `TcpListener`, is closed ([the fourth pass](tcp-listener-and-file-times.md)). It was a
contract gap when the list was written and stopped being one at milestone 107 (the socket contract
learns to accept). What was left was a PAL binding (`bind` -> `OP_LISTEN` on a socket id this PAL
allocates, `accept` -> `OP_ACCEPT` into a second id with a frame attached). It also needed a listen
grant on the stack the program is spawned with, which this milestone's fourth pass bound.
`sys/net/connection/nife.rs` called it *"small and mechanical"* in its own doc comment and
notes/net.md's "The inbound half" said what it took. It was also on the customer path: milestone 55
(Time Machine) wants Samba-shaped code, and nothing that serves a share accepts no connections.

Everything else that resolves a path waits on the `File::open` fork, now tier two of milestone 154
(a process that holds two directory capabilities). The third pass drew the line at exactly that
edge: fix the ones that abort, leave the ones that refuse. `current_dir`, `current_exe`, `chdir` and
`home_dir` can each say no in their own signature and still do. `temp_dir` and `split_paths` could
not, so they were answered from an existing decision, §27 (the filesystem service): `.` is the
granted directory, in `one_name`'s own words. They were not answered from a new one.

The sting the measurement found is the one worth carrying forward: a green build is not evidence.
`tempfile` compiles, links, and returns "operation not supported" at run time. Raised 2026-08-01,
from a question with a number behind it: does milestone 27 mean ordinary Rust programs run here?
