//! The capability shell, at EL0: designation is authorization (notes/grant-expression.md). A Unix
//! child inherits your whole authority; here the command line is a **grant expression**, so naming
//! a resource grants it, a program that names nothing gets only the report channel every spawn
//! carries, and a refusal reads "you hold no such capability". `grant_plan` parses and checks the
//! line on the host; this file turns a checked grant into delegated capabilities.
//!
//! It grants only from what it holds: `prog --mem N` splits N pages from its own budget (slot 3),
//! and a bare name in a file position resolves only if the progenitor granted it a directory
//! ([`DIR_TERMINAL`]); see [`holdings`] and notes/pipes.md. `caps` prints its whole endowment and
//! `caps <command>` previews exactly what a command would get, including the one authority no
//! token designates, the clock a manifest like `date`'s declares. This shell holds its own clock
//! `READ` without `GRANT`, so `time <command>` can measure and cannot hand a clock on
//! (notes/time-command.md). No `run` verb and no `file:` prefix: a bare name spawns, and the
//! manifest declares a file token's direction.
//!
//! - slot 0: the terminal rendezvous (CALL: `OPERATION_WRITE` / `OPERATION_READLINE`).
//! - slot 1: a spawn rendezvous (direct the progenitor to start a program; `grant_plan::spawnproto`).
//! - slot 2: a result rendezvous (receive a spawned program's answer).
//! - slot 3: an untyped budget, the memory it grants with `--mem`.
//!
//! Pages shared with the terminal: `OUT_VA` (text and prompts out) and `LINE_VA` (lines in). The
//! directory ([`DIR_TERMINAL`], slot 4) and the clock page ([`CLOCK_SLOT`], from `x2`) depend on
//! the boot's wiring, so their numbers are not constants.
//!
//! # BUGS
//!
//! - A child holding the result endpoint can send `SPAWN_FAILED`, `SPAWN_NO_DISPLAY` or a wrong
//!   exit status about itself, though never about another job. Every `RECEIVE` returns the sender's
//!   badge in `x3`, so a badged copy per sender would close it at no new authority.
//! - A job that faults reaches the prompt as [`swish::FAULTED_SENTENCE`] with `$?` 1, through
//!   `job_undertaker`'s `JOB_FAULTED`; a job that hangs without faulting still hangs the prompt,
//!   and only an interruptible spawn can be ended with `^C`.
//! - A fault reported while watching a screen-narrowed tail arrives one command late
//!   (`job_undertaker`'s `BUGS`).
//!
//! Name: ratified 2026-08-01 (calef), replacing `shell`. Refused `shell` (a category, where `bash`
//! and `fish` are identities), `capsh` (Linux's libcap ships one) and `sheesh` (dated, and an
//! interjection). A swish goes through the net touching nothing: least authority in one word.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]
// `Vec::push_within_capacity`: the only way this shell adds to a `Vec`, because it cannot allocate
// behind the caller's back (see [`heap`]).
#![feature(vec_push_within_capacity)]

use filesystem_protocol::{dirent, fs};
use grant_plan::expand::{Expander, NameSet, Resume};
use grant_plan::line::{self, Line, Source};
use grant_plan::nav::{self, Cwd, Refused, Step};
use grant_plan::{
    Action, Command, DirGrant as GrantDir, Endowment, Escalation, Refusal, RunSpec, Streams,
    TouchArgs, job_page_frame, spawnproto,
};
use line_editor::proto;
use swish::{Route, Say, Status, Untimed, sequence};
use user_mode_runtime::mapped_window::MappedWindow;
use user_mode_runtime::{
    call, cap_delete, cntfrq, destroy_memory_region, exit, monotonic_nanos, now, reap, receive,
    receive_fault, retype_into_kernel_object, retype_sleeper, send, sleep_until,
    split_memory_region, yield_now,
};

// **The shell is a directory of modules** (milestone 844 (the shell program is split along its
// seams), §273 (the shell program becomes a directory of modules)). Each is a child of this crate
// root, so each sees everything declared here, and this file is the shell's shared vocabulary:
// what four or more of the modules use lives here rather than in one of them. The globs below are
// how this file and its siblings name what a module lends them with `pub(super)`.
mod capabilities;
mod jobs;
mod navigation;
mod pipeline;
mod prompt;
mod running;
mod witness;

use capabilities::*;
use jobs::*;
use navigation::*;
use pipeline::*;
use prompt::*;
use running::*;
use witness::*;

// Pages shared with the terminal (must match the wiring in the progenitor).
//
// `OUT_VA` used to be 0x0060_0000, which is also [`FS_VA`]. That was sound while a shell wired to a
// terminal could not also hold a filesystem, and milestone 50's `>` is exactly the wiring where it
// holds both, so the terminal page moved rather than the FS one: `FS_VA` is `fs_service`'s
// `FILE_VA_CLIENT`, which six other programs map, and this is the one address only the shell and its
// the progenitor know.
const OUT_VA: u64 = address_space_map::pair_page(0x0000_0000_00c0_0000); // we write; the terminal reads
const LINE_VA: u64 = address_space_map::pair_page(0x0000_0000_00b0_0000); // the terminal writes; we read

// SAFETY: the wiring (`crates/system_initializer`'s `SH_OUT_VA` grant) maps one page read/write at
// OUT_VA before this shell runs, in every wiring that has a terminal at all (milestone 139 round
// 3; see `user_mode_runtime::mapped_window`, which is what collapsed the hand-rolled
// read_volatile/write_volatile loop in `stage` below, the same way [`FS_WINDOW`] collapsed the FS
// cluster's in round 2). Constructing a window touches no memory.
const OUT_WINDOW: MappedWindow =
    unsafe { MappedWindow::new(OUT_VA, user_mode_runtime::mapped_window::PAGE) };
// SAFETY: as `OUT_WINDOW`'s, but the wiring's `LINE_VA` grant, mapped read-only, and this is what
// collapsed the hand-rolled loop in `read_line` below.
const LINE_WINDOW: MappedWindow =
    unsafe { MappedWindow::new(LINE_VA, user_mode_runtime::mapped_window::PAGE) };

// Capability slots.
const TERM: u64 = 0; // CALL requests on the terminal
const SPAWN: u64 = 1; // SEND a spawn request to the progenitor
const RESULT: u64 = 2; // RECEIVE a spawned program's answer
const BUDGET: u64 = 3; // our own untyped; SPLIT a grant off it for `--mem`

/// **This shell's heap** (milestone 47 (navigation and naming), calef's ruling of 2026-09-26).
/// Capped at [`swish::HEAP_MAX_BYTES`] and mapped from [`BUDGET`] before any other carve, so a
/// leak exhausts the heap and never the budget children are built from.
#[global_allocator]
static HEAP: user_mode_runtime::heap::MemoryRegionHeap =
    user_mode_runtime::heap::MemoryRegionHeap::new();

/// Wire the allocator to [`BUDGET`] and map the whole capped heap at once. First thing at `_start`,
/// for the roles whose slot 3 is a budget: the heap's pages and their page tables are then the
/// budget's first retypes, under every later carve, which is what those carves' last-in first-out
/// return needs (`MemoryRegionHeap::commit_all`). A budget too small to cover it leaves what did
/// map usable, and an allocation past it refuses its line with [`Say::HeapFull`].
fn heap_init() {
    HEAP.init(
        BUDGET,
        user_mode_runtime::heap::DEFAULT_BASE,
        swish::HEAP_MAX_BYTES,
    );
    HEAP.commit_all();
}

/// **Every allocation this shell makes, and every one of them can fail.** calef's first condition:
/// the boot shell is the owner's root console, so running out of heap is a refusal with a sentence
/// ([`Say::HeapFull`]) and never a panic. Nothing outside this module names `alloc`, and nothing in
/// it calls an allocating method that panics on failure: capacity is reserved with
/// `try_reserve_exact`, and elements go in with `push_within_capacity`, which cannot allocate.
/// The compiler holds the first half (below), and `script/lint` holds that the declaration stays
/// here.
mod heap {
    // Declared here and nowhere else, so no other module can even name `alloc`: a `no_std` crate
    // has no `alloc` in its extern prelude unless the crate root declares it.
    extern crate alloc;

    /// **Room for a fixed number of values, on the heap.** A slice once filled (indexing works), and
    /// there is no way to grow it: `Vec` is private to this module, so nothing outside can call a
    /// method that would allocate and panic on failure.
    pub struct Room<T>(alloc::vec::Vec<T>);

    impl<T> core::ops::Deref for Room<T> {
        type Target = [T];
        fn deref(&self) -> &[T] {
            &self.0
        }
    }

    impl<T> core::ops::DerefMut for Room<T> {
        fn deref_mut(&mut self) -> &mut [T] {
            &mut self.0
        }
    }

    /// `n` copies of `x`, or `HeapFull`. Capacity is reserved with `try_reserve_exact`, and the
    /// copies go in with `push_within_capacity`, which cannot allocate.
    pub fn filled<T: Copy>(n: usize, x: T) -> Result<Room<T>, swish::Say> {
        let mut v = alloc::vec::Vec::new();
        v.try_reserve_exact(n).map_err(|_| swish::Say::HeapFull)?;
        for _ in 0..n {
            v.push_within_capacity(x)
                .map_err(|_| swish::Say::HeapFull)?;
        }
        Ok(Room(v))
    }
}

/// The budget the progenitor granted us at boot, [`spawnproto::SHELL_BUDGET_PAGES`], which it reads
/// too. We cannot query how much remains (there is no such syscall), so `caps` prints the initial
/// grant.
const SH_BUDGET_PAGES: u64 = spawnproto::SHELL_BUDGET_PAGES;

// ---- the shell's own clock (milestone 86, notes/time-command.md) ----

/// **The slot holding the clock page**, or [`NO_CLOCK`] when this wiring granted none. Told to us in
/// `x2` at [`_start`] rather than fixed, because the clock is granted after the filesystem pair and a
/// boot with no disk attached has one fewer capability under it.
///
/// Being *told* is the same shape as `arg1` carrying the directory's rights, and for the same reason
/// recorded there: nothing in this system reports what a process holds, so a shell that guessed would
/// be guessing about its own capability table. A wrong guess here is worse than the directory's, because
/// probing the wrong slot would find some *other* object and map a page that is not a clock.
static CLOCK_SLOT: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(NO_CLOCK);

/// **Whether this session holds the run-unvouched capability** (DECISIONS §219 gate D2), at
/// [`spawnproto::RUN_UNVOUCHED_SLOT`], `WRITE` only.
///
/// Probed once, at [`_start`], and that timing is the soundness argument rather than a detail: at
/// `_start` this shell has allocated nothing, so the slot holds exactly what its builder placed
/// there. Probed any later, a frame or a region this shell retyped could have landed in it. Probed
/// rather than told because `x0` to `x2` are all spoken for (role, directory rights, clock slot),
/// and a named slot is how this tree already places an authority a program looks for
/// (`grant_plan::NETWORK_SLOT`).
///
/// Holding it changes one thing: an image request claims it and sends one message on it
/// ([`run_image`]), so a digest miss runs with only what the line granted instead of being
/// refused. It is not something this shell can hand on: it carries no `GRANT`.
static HOLDS_RUN_UNVOUCHED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

// `grant_plan` states the slot without depending on `abi`; the relation is held by each reader.
const _: () = assert!(spawnproto::RUN_UNVOUCHED_SLOT == abi::fault::FAULT_EP_SLOT - 1);

/// **Whether this shell holds a read-only view of the inert-configuration page**
/// ([`grant_plan::SHELL_CONFIG_SLOT`], mapped at [`grant_plan::SHELL_CONFIG_VA`]; milestone 47
/// (navigation and naming), DECISIONS §111 (inert configuration is a validated page)). Only the
/// progenitor places it, and it maps the page in the same build, so a capability in the slot at
/// `_start` means the page is there to read. Probed once at `_start` for [`HOLDS_RUN_UNVOUCHED`]'s
/// reason. `caps` reads it to print the values a child declaring `config` will read; nothing else
/// in this shell looks at it.
static HOLDS_CONFIG: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

// The named slot sits under the run-unvouched one and so under the kernel's fault slot.
const _: () = assert!(grant_plan::SHELL_CONFIG_SLOT < spawnproto::RUN_UNVOUCHED_SLOT);
/// **Whether this session holds the machine statistics page** (milestone 126 (the `procps`
/// package), DECISIONS §225 (`free` sees the machine and your share)), at
/// [`spawnproto::MACHINE_PAGE_SLOT`], `READ | GRANT`. Probed once at [`_start`], for
/// [`HOLDS_RUN_UNVOUCHED`]'s reason.
///
/// Holding it changes one thing: a program whose manifest declares `machine` is sent the page with
/// its spawn request ([`machine_wiring`]), so it can see how the machine is doing. A session the
/// owner withheld it from cannot pass on what it does not hold, and `free` then says so.
static HOLDS_MACHINE_PAGE: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

/// Whether a spawn of `e` sends the machine statistics page: the program declared it and this
/// session holds it.
fn machine_wiring(e: &Endowment) -> bool {
    e.prog.manifest().machine && HOLDS_MACHINE_PAGE.load(core::sync::atomic::Ordering::Relaxed)
}

/// **Send the machine statistics page, the last delegated capability of a request that set
/// `machine`** (`spawnproto::MACHINE_BIT`). We keep our own copy.
fn delegate_machine_page(wired: bool) {
    if wired {
        delegate(
            spawnproto::MACHINE_PAGE_SLOT,
            abi::rights::READ | abi::rights::GRANT,
        );
    }
}

/// The `x2` value meaning "this shell was granted no clock". Zero rather than a sentinel, because
/// slot 0 is the terminal in every wiring, so no clock can ever legitimately be there.
const NO_CLOCK: u64 = 0;

/// **What this shell holds, which is what decides whether a named file can be backed.**
///
/// A per-file grant is a directory capability narrowed to one name (milestone 31 phase 2,
/// `components/src/fs_file_caretaker.rs`), and a redirection is a file opened through one. Both need a
/// directory to narrow, so both answer to this: a shell whose the progenitor granted it a directory
/// capability (slot [`DIR_TERMINAL`]) can back `>` and `<`, and a shell whose the progenitor granted it none
/// gets the milestone's headline refusal, which is **true** rather than a placeholder.
///
/// The same binary is in both positions, which is the point: `date > report.txt` is refused in the
/// wiring with no filesystem and runs in the wiring with one, and nothing in this file branches on
/// which. See notes/pipes.md.
fn holdings(nav: &Nav) -> grant_plan::Holdings {
    nav.holds
}

// ---- the navigation builtins (milestone 47) ----

/// Where the page this shell shares with the FS server is mapped, **in the wiring that has one**.
///
/// It is `fs_service`'s `FILE_VA_CLIENT`, the address every FS client in this system maps its half
/// of the contract at, which is why [`OUT_VA`] moved out of the way rather than this.
const FS_VA: u64 = address_space_map::pair_page(0x0000_0000_0060_0000);

// SAFETY: the navigating wiring maps one page read/write at FS_VA before this shell runs, when it
// has an FS_VA at all (milestone 139 round 2; see `user_mode_runtime::mapped_window`, which is what
// collapsed the hand-rolled read_volatile/write_volatile loops below). Constructing the window
// touches no memory; every caller of `put_page`/`get_page` already runs behind a `dir.is_some()`
// check, which is only true in that wiring, matching the comment this replaces.
const FS_WINDOW: MappedWindow =
    unsafe { MappedWindow::new(FS_VA, filesystem_protocol::PAGE as u64) };

/// The directory capability's slot in the navigating wiring (`fs_service::start_granted_dir` grants
/// it at 0). A shell that was granted none has no such slot at all, which is why [`Nav::dir`] is an
/// `Option` rather than a constant: "this shell holds no directory" is a fact about a capability table.
const DIR: u64 = 0;

/// The directory capability's slot in the **terminal** wiring, where slots 0..3 are already the
/// terminal, the spawn channel, the result channel and the budget. A shell wired this way holds a
/// filesystem *and* a way to spawn, which is the whole of what `>` and `<` needed (notes/pipes.md).
const DIR_TERMINAL: u64 = 4;

/// **What the command now running has done so far** (milestone 67), as a [`Status`] code.
///
/// A global for the reason [`DIAG_EP`] is one: the refusal and outcome printers are free functions
/// reached from every command path, and threading a `&mut Nav` through all of them to carry one
/// small fact would be a wide diff. Single-threaded EL0, so `Relaxed` is the whole ordering story.
///
/// This was an `AtomicBool` called `TROUBLE` until milestone 67, set by whichever printer had bad
/// news, and read by `xargs` to stop a sweep. Widening it from a bit to a status is the whole of
/// what `$?` needed: the shell already knew that something had gone wrong, and what it did not
/// record was **which kind**.
static CURRENT: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

/// **What the command before this one did**, which is what `$?` reports and what `&&` reads.
///
/// Two cells rather than one, because a segment has to be able to read the previous segment's
/// answer *while* accumulating its own: `false || echo $?` prints the status of the thing that
/// failed, which is exactly the case one cell could not serve.
static LAST: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

/// Record what just happened, **first one wins**: the first thing that went wrong is what stopped
/// the line, and a later printer describing a consequence should not overwrite the cause.
fn record(s: Status) {
    if CURRENT.load(core::sync::atomic::Ordering::Relaxed) == Status::Ran.code() {
        CURRENT.store(s.code(), core::sync::atomic::Ordering::Relaxed);
    }
}

/// **This shell declined to run it**, decided from what this shell holds with nothing spawned.
fn refused() {
    record(Status::Refused);
}

/// **Something was attempted and did not work**: a filesystem errno, a spawn the progenitor could not back,
/// a job torn down. See [`Status`] for why this is a different number from [`refused`].
fn failed() {
    record(Status::Failed);
}

/// What the command now running has done so far.
fn current() -> Status {
    Status::from_code(CURRENT.load(core::sync::atomic::Ordering::Relaxed))
}

/// What the command before this one did.
fn last() -> Status {
    Status::from_code(LAST.load(core::sync::atomic::Ordering::Relaxed))
}

// `is_pattern`, `expansion`, `write_set` and `echo` moved to the `swish` crate in milestone 70:
// each is a decision or a rendering with no capability in it, and each is now host-tested there.
// What is left below is the shell's IO: reading a directory, and putting bytes on a terminal.
//
// The two wrappers here are the seam. The crate's versions take the directory read as a callback,
// because matching a pattern is pure and only *reading* the directory needs a capability; these
// bind that callback to this shell's [`Nav`] so every call site reads exactly as it did before.

/// [`swish::expansion`], against the directory this shell holds.
fn expansion(nav: &mut Nav, spec: &RunSpec) -> Result<grant_plan::expand::Expansion, Say> {
    swish::expansion(spec, &mut |token| nav.expand(token))
}

/// [`swish::echo`], against the directory this shell holds.
/// `$?` reads [`LAST`], which is the segment *before* this one: during `false || echo $?` the
/// shell is running the second segment and the first one's answer is what the word is about.
fn echo(nav: &mut Nav, text: &[u8], out: &mut dyn FnMut(&[u8])) -> Say {
    swish::echo(text, last(), &mut |token| nav.expand(token), out)
}

/// Copy a name into the page shared with the FS server.
fn put_page(bytes: &[u8]) {
    for (i, &b) in bytes.iter().take(filesystem_protocol::PAGE).enumerate() {
        FS_WINDOW.w8(i as u64, b);
    }
}

/// Copy `n` bytes out of that page (a listing landed there).
fn get_page(n: usize, out: &mut [u8]) {
    for (i, b) in out.iter_mut().take(n).enumerate() {
        *b = FS_WINDOW.r8(i as u64);
    }
}

/// Print through the terminal: write the text into the shared page, CALL `OPERATION_WRITE`. The reply
/// means the bytes are on the wire and the page is ours again.
fn print(s: &[u8]) {
    let n = s.len().min(4096);
    stage(s, n);
    call(TERM, proto::req(proto::OPERATION_WRITE, n as u64), 0);
}

/// Copy `n` bytes into the outgoing shared page.
fn stage(s: &[u8], n: usize) {
    for (i, &b) in s[..n].iter().enumerate() {
        OUT_WINDOW.w8(i as u64, b);
    }
}

/// Print a small unsigned number in base 10.
fn print_num(v: u64) {
    swish::write_num(v, &mut print);
}

/// The interactive shell: a terminal at slot 0, a spawn channel, a result channel, a budget. It is
/// role **0** because `crates/system_initializer` starts this program with
/// `(0, fs_rights, clock_slot)`, and it is also what any unrecognized role falls through to: this
/// program's failure mode should be a prompt.
/// **The navigating witness** (milestone 47): no terminal, a directory capability at slot 0, and a
/// report rendezvous at slot 1. It runs the same builtins this file's prompt runs and reports a
/// bitmap; see [`navigate`].
const ROLE_NAVIGATE: u64 = 1;
/// **The globbing witness** (milestone 47's globbing lane): the same wiring as [`ROLE_NAVIGATE`],
/// pointed at the fixture's `globset`, running `echo` and the grant planner over one pattern and
/// reporting whether they agree. See [`globbing`].
const ROLE_GLOB: u64 = 2;
/// **The pipeline witness** (milestone 50): the interactive wiring minus the keyboard. It holds a
/// terminal it writes to, a spawn channel, a result channel and a budget, and it types a fixed
/// script of command lines through the same [`dispatch`] the prompt uses. See [`piping`].
const ROLE_PIPELINE: u64 = 3;
/// **The redirection witness** (milestone 50): [`ROLE_PIPELINE`]'s wiring plus a directory
/// capability at [`DIR_TERMINAL`], which is the endowment `>` and `<` need and the one the
/// interactive boot grants. It types a fixed script through the same [`dispatch`] the prompt uses.
/// See [`redirecting`].
const ROLE_REDIRECT: u64 = 4;
/// **The timing witness** (milestone 86): [`ROLE_PIPELINE`]'s wiring plus, in the wiring that has
/// one, a clock page at the slot `x2` names. It types a fixed script through the same [`dispatch`]
/// the prompt uses, and the same script is run three times against three clock states, which is what
/// makes the refusals assertions rather than unreachable branches. See [`timing`].
const ROLE_TIMING: u64 = 5;
/// **The two-tree witness** (milestone 154): no terminal, grant `a` at slot 0, grant `b` at slot 1
/// and a report rendezvous at slot 2, which is `fs_service::start_granted_two_dirs`' wiring. It
/// runs the prompt's own builtins across both trees and reports a bitmap; see [`two_trees`].
/// Provisional name.
const ROLE_TWO_TREES: u64 = 6;

/// **What `arg1` carries into every role that holds a directory**: the `filesystem_protocol::dir` rights that
/// capability was granted, with 0 meaning "you were granted no directory at all".
///
/// The shell is **told** rather than asking, and that is a gap in `filesystem_protocol` rather than a shortcut:
/// nothing on that wire reports what a handle carries, and `OPENDIR` refuses a request wider than
/// the parent instead of narrowing it, so a shell that guessed `dir::ALL` from a narrower capability
/// could not `cd` at all. See notes/shell-navigation.md.
///
/// **`x2` carries the clock slot** (milestone 86), or [`NO_CLOCK`] for a wiring that granted none.
/// Told for the same reason and a sharper one: the slot moves with what else the boot granted, and a
/// shell that probed the wrong number would find some other object and map a page that is not a
/// clock. See [`CLOCK_SLOT`].
#[unsafe(no_mangle)]
pub extern "C" fn _start(role: u64, arg: u64, clock: u64) -> ! {
    CLOCK_SLOT.store(clock, core::sync::atomic::Ordering::Relaxed);
    // Probed here and nowhere later: see [`HOLDS_RUN_UNVOUCHED`].
    HOLDS_RUN_UNVOUCHED.store(
        user_mode_runtime::is_granted(spawnproto::RUN_UNVOUCHED_SLOT),
        core::sync::atomic::Ordering::Relaxed,
    );
    HOLDS_CONFIG.store(
        user_mode_runtime::is_granted(grant_plan::SHELL_CONFIG_SLOT),
        core::sync::atomic::Ordering::Relaxed,
    );
    // After the probes, which must see the table as its builder left it. The two witness roles
    // hold no budget at slot 3.
    if !matches!(role, ROLE_NAVIGATE | ROLE_GLOB) {
        heap_init();
    }
    // Probed here too: see [`HOLDS_MACHINE_PAGE`].
    HOLDS_MACHINE_PAGE.store(
        user_mode_runtime::is_granted(spawnproto::MACHINE_PAGE_SLOT),
        core::sync::atomic::Ordering::Relaxed,
    );
    match role {
        ROLE_NAVIGATE => navigate(arg),
        ROLE_GLOB => globbing(arg),
        ROLE_PIPELINE => piping(),
        ROLE_REDIRECT => redirecting(arg),
        ROLE_TIMING => timing(),
        ROLE_TWO_TREES => two_trees(arg),
        _ => interactive(arg),
    }
}

/// The interactive prompt. `rights` is the [`_start`] convention: the `filesystem_protocol::dir` rights of the
/// directory capability at [`DIR_TERMINAL`], or 0 for a boot that wired no filesystem.
fn interactive(rights: u64) -> ! {
    // **The line that says userspace is up**, and it is the top rung of milestone 268's boot
    // ladder: a default boot ends at this prompt rather than at a halt, so this is the sentence a
    // watcher reads to learn that the boot finished. Its head comes from `crates/boot_ladder`
    // rather than from a literal here, because `crates/board_console` matches it and AGENTS.md
    // rule 7 says two binaries agreeing on a string means a crate.
    print(boot_ladder::PROMPT.as_bytes());
    print(b". naming a resource in a command IS granting it.\n");
    print(b"commands: help, echo <text>, caps [command], time <command>, xargs <command>,\n");
    print(b"          cd, pwd, ls, mkdir, touch, apropos <word>, rm, wc, mdr, <prog> [--mem N] [arg]\n");
    print(b"          and the operators  >  >>  <  |\n");
    print(b"          'quote a whole word'   and   ;  &&  ||   with  echo $?  for the status\n");

    // Whether the navigation builtins and the redirection operators have anything to name is
    // decided here, by one capability. A boot that wired no FS service starts this program with
    // `rights = 0` and every one of them answers "this shell holds no directory capability".
    let mut nav = if rights == 0 {
        Nav::empty()
    } else {
        Nav::rooted_at(DIR_TERMINAL, rights)
    };
    let mut line = [0u8; 128];
    loop {
        let (n, flags) = edit_line(&mut nav, b"$ ", &mut line);
        if flags & proto::FLAG_INTERRUPTED != 0 {
            // ^C at the prompt: the terminal discarded the line. Account for this interrupt so it
            // does not leak into the next job's watch, then come back for the next line.
            CONSUMED.store(intr_count(), core::sync::atomic::Ordering::Relaxed);
            continue;
        }
        if flags & proto::FLAG_EOF != 0 {
            // ^D on an empty line. This shell is the session; there is nowhere to exit to.
            print(b"  (end of input; this shell has nowhere to exit to)\n");
            continue;
        }
        dispatch_line(&mut nav, &line[..n]);
    }
}

/// **Run one line if it is a navigation builtin**, calling `each` for every entry a listing
/// produces. `None` means the line was not one of them.
///
/// Split out from [`dispatch`] because the navigating witness ([`navigate`]) runs the *same*
/// builtins from the same command lines with no terminal to print to. One implementation, two
/// callers, and the guest test therefore exercises what the prompt exercises rather than a
/// reimplementation of it.
fn builtin(nav: &mut Nav, cmd: &[u8], each: &mut dyn FnMut(&[u8], bool)) -> Option<Say> {
    match grant_plan::parse(cmd) {
        // They spawn nothing and grant nothing: the shell is rebinding and reading a capability it
        // already holds, which is why these are builtins, and why the worry about an over-granted
        // listing *program* (which would hold the power to read everything it lists) does not arise.
        Command::Cd(path) => Some(nav.cd(path)),
        Command::Ls(path) => Some(nav.ls(path, each)),
        Command::Mkdir(path) => Some(nav.mkdir(path)),
        Command::Touch(args) => Some(nav.touch(args)),
        Command::Bind(target, name) => Some(nav.bind(target, name)),
        _ => None,
    }
}

/// **Read one line and act on it**, which since milestone 50 means asking first whether it has any
/// operators on it.
///
/// A line with no `>`, `<` or `|` takes exactly the path it took before those existed, which is
/// deliberate: the operators must not make an ordinary command more expensive or more complicated,
/// because an ordinary command is nearly every command.
/// The routing itself is [`swish::route`], host-tested there, including the one ordering with a bug
/// behind it: `caps` is answered **before** the line is split, or `caps date | wc` would lose the
/// pipe. What is left here is which capability each answer reaches for.
/// **Read one typed line and run the commands on it** (milestone 67, notes/swish-language.md).
///
/// This is the outermost thing the prompt calls. It cuts the line on `;`, `&&` and `||` and hands
/// each segment to [`dispatch`], which is the function every path in this file already went
/// through, so **a segment is a whole command line and nothing under here learned a new grammar**.
///
/// # A connector carries one bit and no capability
///
/// Each segment is planned from scratch against what this shell holds, exactly as if it had been
/// typed alone. That is not a restriction adopted here, it is what the shell already was, and
/// sequencing is the first construct that could have broken it. The concrete hazard is the pipeline
/// region: a line splits one off this shell's budget, mints its endpoints in it, and `DESTROY`s it
/// when the line is over, and that destroy is what turns a stalled writer's next `SEND` into
/// `abi::Error::Gone` (notes/pipes.md). Reusing one region across a chain would keep every earlier
/// segment's endpoints alive for the whole chain. It does not, because the split is **here**, above
/// [`pipeline`](fn@pipeline), rather than inside it.
///
/// # A skipped segment leaves `$?` alone
///
/// [`LAST`] is written only by a segment that ran, which is bash's rule: `false && echo hi` leaves
/// `$?` at the status of `false`. There is no "skipped" status, because nothing happened.
fn dispatch_line(nav: &mut Nav, cmd: &[u8]) {
    let seq = match sequence::split(cmd) {
        Ok(s) => s,
        Err(r) => {
            // A line that cannot be cut runs nothing, so it is one refusal and one status.
            CURRENT.store(Status::Ran.code(), core::sync::atomic::Ordering::Relaxed);
            say(Say::Cannot(r));
            LAST.store(
                CURRENT.load(core::sync::atomic::Ordering::Relaxed),
                core::sync::atomic::Ordering::Relaxed,
            );
            return;
        }
    };
    for &(joint, segment) in seq.segments() {
        if !joint.runs_after(last()) {
            continue;
        }
        CURRENT.store(Status::Ran.code(), core::sync::atomic::Ordering::Relaxed);
        dispatch(nav, segment);
        LAST.store(
            CURRENT.load(core::sync::atomic::Ordering::Relaxed),
            core::sync::atomic::Ordering::Relaxed,
        );
    }
}

fn dispatch(nav: &mut Nav, cmd: &[u8]) {
    match swish::route(cmd) {
        Route::Caps(tail) => caps(nav, tail),
        Route::Time(tail) => time_command(nav, tail),
        Route::Xargs(tail) => xargs(nav, tail),
        Route::One(stage) => dispatch_one(nav, stage),
        Route::Pipeline(l) => pipeline(nav, l),
        Route::Cannot(r) => say(Say::Cannot(r)),
    }
}

/// **Run a command and say how long it took** (milestone 86, notes/time-command.md).
///
/// The tail goes back through [`dispatch`], which is the function the prompt calls, so `time` adds no
/// second way to run anything: a pipeline is timed as a pipeline, a builtin as a builtin, and a
/// refusal is refused exactly as it would have been. That is what makes "what you time is what you
/// run" a property of this code rather than a claim about it.
///
/// **The clock is this shell's, and the command is told nothing.** `time wc report.txt` times a
/// program whose whole endowment is one rendezvous and no clock at all, which is the Unix behavior and
/// the capability-model answer at the same time: a duration is observable to anyone who can watch a
/// thing start and stop, so measuring it needs the observer's authority and not the subject's.
/// Delegating a clock to the child instead would change what the child can do, and that is a
/// different tool.
///
/// # What the number is, and what it is not
///
/// Wall clock, between this shell deciding to run the line and the line being over. Not CPU time:
/// nothing in this kernel is asked what a thread spent, so there is no `user` or `sys` row to print
/// and inventing one would be worse than the gap. If that arrives it is another row here.
///
/// The two readings are taken **around** the dispatch and nothing else is between them, so what is
/// measured includes this shell's own planning, spawning and draining. For a spawned program that is
/// the honest number: the shell is the thing that started it and the thing that noticed it finish,
/// and there is no other vantage point from which the question has an answer.
fn time_command(nav: &mut Nav, tail: &[u8]) {
    // Collapse a nested prefix rather than recursing through [`dispatch`]. `time time date` is a line
    // a person can type, and each level of recursion here costs a `dispatch` frame on a stack that
    // has run out four times already (notes/pipes/the-boot.md). Timing something twice measures
    // nothing twice.
    let mut tail = grant_plan::trim(tail);
    while let Command::Time(inner) = grant_plan::parse(tail) {
        tail = grant_plan::trim(inner);
    }
    if tail.is_empty() {
        refused();
        return swish::write_untimed(Untimed::NothingToTime, &mut print);
    }
    // **A duration needs no clock** (§72). Wall time is an offset plus this counter; across a
    // command the offset cancels, so the subtraction below is the whole measurement, and
    // `user_mode_runtime::monotonic_nanos` is ambient by the kernel's own choice (it opened the counter to
    // EL0). `time` therefore cannot be refused and has no clock wiring: what a capability gates
    // here is wall-clock *identity*, which is `date`'s business, not elapsed time.
    //
    // This is also strictly more correct than the clock version it replaces. A clock stepped
    // mid-command used to make the answer a difference of two readings on different clocks,
    // which the shell had to detect and disclaim; a monotonic counter cannot be stepped, so
    // there is nothing left to disclaim.
    let start = monotonic_nanos();

    dispatch(nav, tail);

    // Saturating out of habit rather than need: the counter is monotonic, so `end` cannot precede
    // `start`, and a wrap would take centuries at either board's frequency.
    swish::write_timing(monotonic_nanos().saturating_sub(start), &mut print);
}

/// The most batches one sweep will run. A ceiling so a resume rule that failed to advance costs a
/// bounded number of spawns rather than a prompt that never comes back, in exactly the spirit of
/// [`ROUNDS`]. Eight names a batch, so this is 2048 names, and a directory that large is a machine's
/// problem before it is this shell's.
const MAX_BATCHES: u64 = 256;

/// **Run a command once per batch of what its pattern matched** (milestone 109).
///
/// # Why this is a prefix word and not a program
///
/// A batching *program* would have to hold, at once, at least the union of every batch it hands
/// out. There is no carrier for that union: that is the premise of the whole milestone, since a set
/// larger than [`grant_plan::expand::MAX_NAMES`] is exactly what cannot be handed over. So such a
/// program could only be given the **directory** plus a pattern, which is milestone 47's rejected
/// "the directory plus a name list" answer wearing a different hat: it over-grants catastrophically,
/// and it hands the batcher authority over names the pattern never matched.
///
/// Unix's `xargs` can be a program precisely because it holds *nothing*. Names in a pipe are text,
/// and `rm`'s authority there comes from its uid. Here a name in a pipe is still text, so a
/// `find | xargs rm` analogue would spawn an `rm` holding no authority at all; the pipe cannot carry
/// the thing that has to be batched. What can mint a per-batch caretaker is whatever holds the
/// directory capability with the right to delegate it, and in this system that is the shell asking
/// the progenitor. So the batching goes where the authority already is.
///
/// This is **not** milestone 47's rejected "make `rm` a builtin", either. `rm` stays a program with
/// an attenuated grant; what became a builtin is the *iteration*, which is a property of how the
/// shell delegates rather than of anything any command does. `caps` and `time` are the precedent.
///
/// # It stops at the first batch that does not run
///
/// Unix carries on and reports 123 at the end, and that is its mechanism talking: its `xargs` cannot
/// know what a child did to the names it was handed. Here the shell printed each batch's set before
/// that batch ran, so it can name the boundary, and a sweep that stopped is a **prefix** of the
/// match rather than an arbitrary subset. Batch four succeeding after batch three failed is the
/// outcome worth designing against, because the user would have no way to tell which names it left.
fn xargs(nav: &mut Nav, tail: &[u8]) {
    // Collapse a nested prefix rather than recursing, [`time_command`]'s reason: `xargs xargs rm
    // *.txt` is a line a person can type, batching a batch is still one sweep, and every level of
    // recursion costs a frame on a stack that has run out four times already.
    let mut tail = grant_plan::trim(tail);
    while let Command::Xargs(inner) = grant_plan::parse(tail) {
        tail = grant_plan::trim(inner);
    }
    if tail.is_empty() {
        refused();
        print(b"  xargs: name a command to batch\n");
        return;
    }
    let mut sweep = swish::Sweep::default();
    let mut resume = Resume::Start;
    for _ in 0..MAX_BATCHES {
        nav.batching = Batching {
            resume: Some(resume),
            index: sweep.batches + 1,
            next: None,
            names: 0,
        };
        // Each batch is its own command as far as the status is concerned, and the sweep stops at
        // the first one that did not run. That is the same reading `&&` takes (see [`Status`]): a
        // refusal and a failure are both a no, and which it was stays in the status the sweep
        // leaves behind for `$?`.
        CURRENT.store(Status::Ran.code(), core::sync::atomic::Ordering::Relaxed);
        dispatch(nav, tail);
        let batching = nav.batching;
        nav.batching = Batching::default();
        if !current().is_ok() {
            sweep.stop();
            break;
        }
        sweep.ran(batching.names);
        match batching.next {
            Some(r) => resume = r,
            None => break,
        }
    }
    swish::write_sweep(&sweep, &mut print);
}

/// Parse one line with `grant_plan` and act on it. All parsing and the manifest check are the host-tested
/// crate; this function is only IO and capability moves.
fn dispatch_one(nav: &mut Nav, cmd: &[u8]) {
    if let Some(said) = builtin(nav, cmd, &mut |name, is_dir| {
        print(b"  ");
        print(name);
        if is_dir {
            print(b"/");
        }
        print(b"\n");
    }) {
        say(said);
        return;
    }
    match grant_plan::parse(cmd) {
        Command::Empty => {}
        Command::Help => help(),
        // `echo` expands, and that is half of the globbing demonstration: what it prints is
        // literally the authority the same words would transfer to a program. The newline goes out
        // only when the line itself did, so a refused expansion is one line and not two.
        Command::Echo(text) => match echo(nav, text, &mut print) {
            Say::Nothing => print(b"\n"),
            said => say(said),
        },
        // Unreachable through [`dispatch`], which answers the two prefix words before it splits the
        // line so they can see the operators. Kept so this function is complete on its own: it is
        // the whole of "what one command means", and a reader should not have to know about the
        // interception to find every arm.
        Command::Caps(tail) => caps(nav, tail),
        Command::Time(tail) => time_command(nav, tail),
        Command::Xargs(tail) => xargs(nav, tail),
        Command::Pwd => print_pwd(nav),
        // **Search, which grants nothing** (milestone 40 phase 2). It is here rather than in
        // [`builtin`] because its answer is columns of text rather than a listing, so the witness's
        // `each(name, is_dir)` callback is the wrong shape for it; what it shares with the
        // navigation builtins is the argument for being a builtin at all.
        Command::Apropos(term) => say(apropos(nav, term)),
        Command::Vouch(tail) => vouch(nav, grant_plan::vouch_verb(tail)),
        Command::User(tail) => user(nav, grant_plan::user_verb(tail)),
        Command::Run(spec) => run(nav, cmd, spec),
        // Handled above, by the one implementation the witness also runs.
        Command::Cd(_)
        | Command::Ls(_)
        | Command::Mkdir(_)
        | Command::Touch(_)
        | Command::Bind(_, _) => {}
    }
}

/// **`user suspend <name>` and `user resume <name>`** (milestone 152 (durable delegation), calef's
/// §108 (disabling credentials kills the durable session) ruling of 2026-09-26, names ratified): add the name to, or take it off,
/// `login_protocol::SUSPENDED_LIST` at the root of this shell's directory, in the one format
/// `login` and the boot-time re-deriver read it in. `login` refuses a listed identity from its next
/// login attempt, ends its durable session then, and the re-deriver skips it at boot.
///
/// **Owner-only by authority, not by a check.** The file `login` reads is at the file service's
/// root, and only the owner's console holds that root. A confined shell running this edits a file
/// of that name in its own subtree, which nothing reads.
///
/// # BUGS
///
/// **It does not send `login_protocol::SUSPEND`**, the word that ends a durable session at once:
/// this shell holds no capability to `login`'s front door. On the boot that runs this prompt that
/// costs nothing yet, because `login` is started there with no schedule archive and so holds no
/// durable session to end; the day it is, this shell needs the front door, `WRITE`, beside its
/// root. Until then the cascade runs at the suspended identity's next login attempt.
fn user(nav: &mut Nav, verb: grant_plan::UserVerb<'_>) {
    use grant_plan::UserVerb;
    let (name, suspend) = match verb {
        UserVerb::Suspend(n) => (n, true),
        UserVerb::Resume(n) => (n, false),
        UserVerb::Usage => {
            refused();
            print(swish::USER_USAGE);
            return;
        }
    };
    if nav.dir.is_none() {
        return say(Say::NoDirectory);
    }
    // The first tree, as `caps` reads the activation table from: the owner's lists are the
    // system's, whichever tree this shell stands in.
    let t = nav.first();
    let file = login_protocol::SUSPENDED_LIST.as_bytes();
    let mut list = [0u8; USER_LIST_MAX];
    let had = read_named(nav, t, fs::ROOT, file, &mut list).unwrap_or(0);
    let mut edited = [0u8; USER_LIST_MAX];
    let n = if suspend {
        login_protocol::with_listed(&list[..had], name, &mut edited)
    } else {
        login_protocol::without_listed(&list[..had], name, &mut edited)
    };
    let Some(n) = n else {
        refused();
        print(b"  user: the suspended list is full\n");
        return;
    };
    let mut h = nav.name_call_in(t, fs::CREATE, fs::ROOT, file, 0);
    if h < 0 {
        h = nav.name_call_in(t, fs::OPEN, fs::ROOT, file, 0);
    }
    if h < 0 {
        return say(Say::Failed(-h as i32));
    }
    let h = h as u64;
    let truncated = call(t.slot, fs::req(fs::TRUNCATE, h, 0), 0).0 as i64;
    put_page(&edited[..n]);
    let wrote = call(t.slot, fs::req(fs::WRITE, h, n as u64), 0).0 as i64;
    nav.close_in(t, h);
    if truncated < 0 || wrote != n as i64 {
        return say(Say::Failed(
            if truncated < 0 { -truncated } else { 5 } as i32
        ));
    }
    print(if suspend {
        b"  suspended "
    } else {
        b"  resumed "
    });
    print(name);
    print(b"\n");
}

/// The longest suspended list `user` edits: two kilobytes, more than a hundred names.
const USER_LIST_MAX: usize = 2048;

/// Print where we are, relative to our own root.
fn print_pwd(nav: &Nav) {
    if nav.dir.is_none() {
        say(Say::NoDirectory);
        return;
    }
    swish::write_pwd(&holdings(nav), &mut print);
}

/// Print what a builtin had to say. Every line is a statement about a name or a capability, never
/// about a policy, and the wording is [`swish::write_say`]'s so a host test can hold it: this shell
/// does not get to invent a friendlier word for a refusal.
fn say(s: Say) {
    // **Which kind of no it was** (milestone 67). The filesystem answering with an errno is
    // something that was attempted and did not work; everything else here is this shell declining
    // from what it holds, with nothing sent and nothing spawned. Under `xargs` either ends the
    // sweep, which is what the old single bit recorded.
    match s {
        Say::Nothing => {}
        Say::Failed(_) => failed(),
        _ => refused(),
    }
    swish::write_say(s, &mut print);
}

fn help() {
    swish::write_help(&mut print);
}

/// Resolve an invocation, then either refuse it at the prompt (a mismatch the manifest caught) or
/// spawn it, granting exactly what the command named and nothing else.
fn run(nav: &mut Nav, cmd: &[u8], spec: RunSpec) {
    // **A path runs bytes, not a name** (DECISIONS §219 (how the shell names an installed program to the spawner) option D, milestone 198 (a package manager) rung 3a). A program
    // the image names is a word with no `/` in it; anything with one is a file this shell reads and
    // hands the progenitor as frames. `installed/uptime` and `./a.out` both land here.
    if spec.prog.contains(&b'/') {
        return run_image(nav, spec);
    }
    // **A bare word is the image's, an installed program's, or refused** (DECISIONS §229 (how a
    // bare name at the prompt reaches an installed program), B2). An installed program runs down
    // the same road its path does, so what it is granted and who vouched for it do not depend on
    // how it was named.
    match bare(nav, spec.prog) {
        swish::bare::Bare::Installed { path, notice } => {
            // **The spawn line says both when a version set asked for a version that is not live**
            // (milestone 614 (two installed versions of one program, each runnable, and a caller
            // granted the one it needs), ruling 4): what runs, and what the set specified. A
            // preview ([`caps`]) says nothing ran, so it prints no notice.
            if let Some(notice) = notice {
                swish::bare::write_divergence(spec.prog, &notice, &mut print);
            }
            let mut by_path = spec;
            by_path.prog = path.as_bytes();
            return run_image(nav, by_path);
        }
        swish::bare::Bare::Both(path) => {
            refused();
            return swish::bare::write_both(spec.prog, &path, &mut print);
        }
        swish::bare::Bare::Image | swish::bare::Bare::Unknown => {}
    }
    // **Expand first.** A pattern designates the names it matched, so the planner has to see the
    // set; and a pattern that matched nothing, or too much, is refused here with nothing spawned.
    let expanded = match expansion(nav, &spec) {
        Ok(e) => e,
        // A grant refusal is attributed to the program when there is one, because "rm: no name here
        // matches that pattern" reads better than the bare sentence and is just as true.
        Err(Say::Cannot(r)) => return refuse(spec, r),
        Err(said) => return say(said),
    };
    match grant_plan::plan(&spec, holdings(nav), expanded) {
        Err(refusal) => refuse(spec, refusal),
        // A supervised job runs under the two-tier ^C path (DECISIONS §24); a fast job is simply
        // spawned and waited on.
        Ok(endow) if endow.interruptible => spawn_interruptible(endow),
        // **`wc report.txt`**, which is `wc < report.txt` with the operator left out: the planner
        // put the designated name in the *source* (`grant_plan`'s input operand), so this line is a
        // one-stage pipeline the shell feeds, and it runs down exactly the path a `<` runs down.
        // There is no second mechanism here, which is the reason to do it this way: the bytes reach
        // the child the same way, and the child still cannot tell a file from a pipe.
        Ok(endow) => match endow.source {
            Source::File(g) => {
                // The line reparses into the one stage it is. It cannot fail (this command came out
                // of it), and a `Line` is what `run_pipeline` reads its stage count off.
                let Ok(l) = line::split(cmd) else { return };
                // **One element, not `MAX_STAGES`**, and that is a stack decision rather than a
                // tidiness one. An `Endowment` carries a whole `NameSet` by value, so a full-width
                // array here overflowed the shell's stack at the *first* line this arm ran, which
                // presented as a data abort one word below the lowest mapped stack page. This line
                // is a single stage by construction, and `run_pipeline` takes a slice.
                run_pipeline(nav, l, &[Some(endow)], false, None, Some(g), None);
            }
            // **A program that hears words gets the line as its argv** (milestone 205,
            // DECISIONS §170 (how a foreign program is told what to do)). Assembled here, where a refusal can still name the program, and
            // before anything is sent, so a line that does not fit spawns nothing.
            _ if endow.prog.manifest().arg.hears_words() => match assemble_argv(spec.line(), 0) {
                Ok(a) => spawn(endow, Some(a)),
                Err(Some(r)) => refuse(spec, r),
                Err(None) => out_of_budget(),
            },
            _ => spawn(endow, None),
        },
    }
}

/// Carve `pages` off our own untyped budget (slot 3) into a delegatable child untyped. `None` when
/// the budget is exhausted.
fn memory_region_split(pages: u64) -> Option<u64> {
    let r = split_memory_region(BUDGET, pages);
    if r < 0 { None } else { Some(r as u64) }
}

const PAGE: u64 = 4096;
/// Where the wiring maps this shell's clock page read-only: `crates/system_initializer`'s
/// `SH_CLOCK_VA`, which must match. Nothing here reads through it by address; it is named so that
/// [`JOBFRAME_WINDOWS`] can be proven clear of it.
const SH_CLOCK_VA: u64 = address_space_map::pair_page(0x0000_0000_00d0_0000);

// Every fixed window this shell maps, and the job frames' range, are disjoint. A window added to
// the shell belongs in this list.
const _: () = {
    let fixed: [(u64, u64); 6] = [
        (OUT_VA, PAGE),
        (LINE_VA, PAGE),
        (FS_VA, filesystem_protocol::PAGE as u64),
        (SH_CLOCK_VA, PAGE),
        (grant_plan::SHELL_CONFIG_VA, PAGE),
        // The argv and name-set pages, and the image window above them (DECISIONS §219 option D).
        (IMAGE_WINDOW, IMAGE_WINDOW_END - IMAGE_WINDOW),
    ];
    let mut i = 0;
    while i < fixed.len() {
        let (va, len) = fixed[i];
        assert!(
            va + len <= JOBFRAME_WINDOWS.start || va >= JOBFRAME_WINDOWS.end,
            "a supervised job's frame window overlaps one of the shell's fixed windows"
        );
        i += 1;
    }
    assert!(address_space_map::PAIR_PAGES.holds(JOBFRAME_WINDOWS.start, JOBFRAME_WINDOWS.end));
    assert!(address_space_map::PAIR_PAGES.holds(IMAGE_WINDOW, IMAGE_WINDOW_END));
};

/// Delegate the capability in `slot` to the progenitor over the spawn rendezvous, narrowed to WRITE|GRANT (the progenitor
/// builds from an untyped or maps a frame, and narrows further from there). We keep our own copy.
fn send_cap(slot: u64) {
    delegate(slot, abi::rights::WRITE | abi::rights::GRANT);
}

/// **Delegate one capability to the progenitor, with exactly the rights it should travel with.**
///
/// `GRANT` has to be in `rights` for the send itself to be legal, so what varies is the other half,
/// and that half is load-bearing for milestone 50: a pipe's write end goes over as `WRITE|GRANT` and
/// its read end as `READ|GRANT`, so the child at the far end of a `|` **cannot write back up its own
/// input**. A pipeline that granted both directions would be a two-way channel nobody asked for, and
/// the narrowing is what stops it rather than a convention the programs are trusted to keep.
fn delegate(slot: u64, rights: u64) {
    user_mode_runtime::send_capability(SPAWN, slot, rights, spawnproto::CAP_TAG);
}

user_mode_runtime::panic_handler!();
