//! **A process's own lifetime**: yielding, exiting, trapping on a failed check, and the panic
//! handler every program declares with one macro.
//!
//! Cut out of `lib.rs` unchanged on 2026-10-12 (UTC) by lane maintainer/files-over-1500, for §282
//! (when a file is split, the parent becomes an index); the crate root re-exports every public
//! item, so each keeps its path.
//!
//! Name: provisional, lane maintainer/files-over-1500 (#2017), 2026-10-12 (UTC), for the module and
//! the file. Named for the calls it holds, one module per seam of the old crate root; the pull
//! request records no other name considered.

use crate::syscall::trap6;

/// Give up the CPU (`SYS_YIELD`). Returns when the scheduler runs this thread again; if another
/// thread is ready, control goes there and back, which is one context-switch round trip.
///
/// Through [`trap6`] like every other syscall, for the reason its doc gives: the kernel writes its
/// result into `x0`/`a0`/`rdi` on the way out of a yield too. And not `nomem`: while this thread
/// is away, others run and write memory it shares with them, so a yield is a point where memory
/// changes, which is what a caller spinning on a shared word with `yield_now` relies on.
pub fn yield_now() {
    // SAFETY: SYS_YIELD gives up the CPU and comes back; it reads no argument and touches no
    // memory of this thread's own.
    let _ = unsafe { trap6(abi::SYS_YIELD, [0; 6]) };
}

/// Terminate this process. The kernel reaps the thread and frees its whole address space. Never
/// returns; the trailing spin is only there to satisfy the `-> !` type if the trap ever came back.
pub fn exit() -> ! {
    // SAFETY: the syscall never returns; the trailing spin only satisfies the `-> !` type.
    let _ = unsafe { trap6(abi::SYS_EXIT, [0; 6]) };
    loop {
        core::hint::spin_loop();
    }
}

/// **Die where the mistake was.** Raise a breakpoint the kernel turns into a fault, so the process
/// is killed rather than allowed to limp on.
///
/// This is the other way a program can end, and the difference from [`exit`] is not a spelling.
/// `exit` reports `EVENT_EXIT` to a supervisor and this reports `EVENT_FAULT`
/// (`kernel/src/sched.rs`, DECISIONS §26), so a supervised child that traps is legible as having
/// failed and one that exits is not. A panic must take this path or it lies about what happened.
///
/// The instruction differs per architecture and `x86_64`'s is not the obvious one: `brk #0` on
/// aarch64, `ebreak` on riscv64, **`ud2`** on `x86_64`. See that arm for why its breakpoint
/// instruction cannot be used from ring 3.
///
/// The trailing spin never runs. It is here for the same reason [`exit`]'s is, to satisfy `-> !`
/// if the trap ever came back, and it spins rather than calling `exit` on purpose: `exit` would
/// turn an impossible situation into a clean-looking death, which is precisely the confusion the
/// paragraph above exists to prevent.
///
/// A verb, which is right for a function here: `send`, `receive`, `reap` and `exit` are all verbs, and
/// the naming tenet's noun rule is about crates, programs and modules rather than about the things
/// they do.
///
/// # BUGS
///
/// **The fault line names this function and nothing else, so every trap in a program looks
/// identical.** The kernel prints the faulting `pc`, and the `pc` of a trap is always the
/// breakpoint instruction below: one address per program, shared by every caller. A program with
/// forty `must(...)` sites that dies at one of them reports the same line whichever it was. Found
/// the hard way by milestone 230, whose whole first day was spent working out *which* call in
/// `crates/system_initializer` had failed, from a fault line that could not say. The return address
/// is right there in `x30` (aarch64), `ra` (riscv64), or on the stack (`x86_64`) at the moment the
/// kernel takes the fault, and printing it beside `pc` would have answered the question in one
/// boot. That is a change to the three `arch/*/exceptions.rs` fault printers rather than to this
/// crate, so it is a milestone of its own; until it exists, the workaround that worked is to make
/// the failure a *data* abort at an address derived from the return address, since `far` is
/// printed and carries whatever you fault on.
pub fn trap() -> ! {
    #[cfg(target_arch = "aarch64")]
    // SAFETY: `brk` traps; the kernel turns a trap from userspace into a kill. The options promise
    // it touches neither memory nor the stack.
    unsafe {
        core::arch::asm!("brk #0", options(nostack, nomem));
    };
    #[cfg(target_arch = "riscv64")]
    // SAFETY: `ebreak` traps; the kernel turns a trap from userspace into a kill. The options
    // promise it touches neither memory nor the stack.
    unsafe {
        core::arch::asm!("ebreak", options(nostack, nomem));
    };
    #[cfg(target_arch = "x86_64")]
    // SAFETY: `ud2` faults; the kernel turns a fault from ring 3 into a kill. The options promise
    // it touches neither memory nor the stack.
    //
    // **`ud2` rather than `int3`, and this was measured rather than chosen** (milestone 161).
    // `int3` is the obvious transliteration of `brk`/`ebreak` and it does not work from ring 3
    // here: a *software* interrupt is refused unless the IDT gate's DPL admits the caller's
    // privilege level, and this kernel's gates are all DPL 0, so `int3` from a process raises
    // **#GP with error code 0x1a** (`(3 << 3) | 2`, the vector it was refused, tagged as an IDT
    // selector) instead of #BP. The process does die, so the first version of this looked like it
    // worked; what it reported was a general protection fault at address zero, which names neither
    // the instruction nor the reason and would have sent the next reader hunting a segmentation
    // bug.
    //
    // Opening vector 3 to ring 3 is the other fix and Linux takes it, because Linux has ptrace and
    // wants a debugger to be able to plant breakpoints. There is no debugger here, so that would
    // widen what a process may do to buy nothing. `ud2` is a **fault the CPU raises** on an opcode
    // permanently reserved to be invalid, so no gate DPL is involved, and "this must never execute"
    // is exactly what the instruction means.
    unsafe {
        core::arch::asm!("ud2", options(nostack, nomem));
    };
    loop {
        core::hint::spin_loop();
    }
}

/// **The panic handler every nife program wants**, as a macro so it stays per-final-binary.
///
/// Write `user_mode_runtime::panic_handler!();` once at the top level of a binary and it expands to a
/// `#[panic_handler]` that calls [`trap`].
///
/// # Why a macro and not a plain item in this crate
///
/// A `#[panic_handler]` is per-final-binary: exactly one may exist in a linked program, so a
/// library that defines one forces it on every binary that links the library and collides with any
/// binary wanting its own. That constraint is real and this crate's header has recorded it since
/// milestone 19f.6. A macro keeps it: nothing is defined until a binary asks, and a program with
/// its own handler simply does not invoke this.
///
/// What the header got wrong, and what milestone 130 is fixing, is the clause after it: "each
/// binary keeps its own one-line handler; it is trivial." It stopped being one line. By the time
/// anyone counted it was fifteen, with two `unsafe` blocks and two `// SAFETY:` comments, at
/// forty-eight sites across `user/`, `crates/` and `redoxfs_server/`, in **seven** variants. One of
/// them (`terminal_sink_caretaker`) called `exit` instead of trapping, which reports a clean death
/// for a panicking program; it was latent only because that program happens to be spawned
/// unsupervised.
///
/// So the decision not to put a *handler* in a library was right, and the inference that each
/// binary must therefore hand-roll the *trap* did not follow. The trap belongs here, in the crate
/// whose header calls itself the one place in userspace that names the two ABIs, and the macro is
/// what lets that be true without breaking the linking property.
///
/// # Examples
///
/// ```ignore
/// #![no_std]
/// #![no_main]
///
/// user_mode_runtime::panic_handler!();
///
/// #[unsafe(no_mangle)]
/// pub extern "C" fn _start() -> ! {
///     user_mode_runtime::exit()
/// }
/// ```
///
/// # BUGS
///
/// It takes no arguments and ignores the `PanicInfo`, because a program with no console cannot
/// print one. A program that *can* print (it holds a terminal endpoint) and wants the message on
/// the way down still writes its own handler; this macro is the default, not a mandate.
///
/// Named for the item it expands to, which is the one thing a reader needs it to say.
#[macro_export]
macro_rules! panic_handler {
    () => {
        #[panic_handler]
        fn panic(_: &::core::panic::PanicInfo) -> ! {
            $crate::trap()
        }
    };
}
