//! **`user_mode_runtime`**: the tiny user-mode runtime shared by nife userspace programs (milestone
//! 19f.6). It ships on all three architectures, so the level it runs at is named the way the tree
//! names it everywhere a single ISA is not in view: `EL0` on aarch64, `U-mode` on riscv64, `ring 3`
//! on `x86_64`, and "user mode" when the sentence covers all three.
//!
//! One syscall wrapper (`invoke`) and the three things every program builds on it: `send`, `receive`,
//! and `exit`. That is the whole crate. It exists because milestones 19f.2-5 split the userspace
//! into distinct binaries (`least_authority_demo`, `console`, `input`, `shell`, plus `hello`), each
//! of which had
//! copied these functions verbatim. The extraction waited on purpose until the split was done: only
//! then was the shared surface known rather than guessed, which is the DECISIONS rule about not
//! building an abstraction before its requirements exist.
//!
//! Milestone 27 added one more thing every program *may* build on: [`heap`], the untyped-backed
//! `GlobalAlloc` that turns the budget a program was granted into `Vec` and `String`. It is a
//! module, not a default: a program that never allocates links no allocator.
//!
//! Milestone 139 added a third: [`mapped_window`], the raw volatile-access-into-a-mapped-page
//! seven drivers had each hand-rolled (a DMA page or a shared IPC frame, read and written by
//! `unsafe { core::ptr::read_volatile/write_volatile }` at every call site). Same shape as the
//! panic handler above: the invariant was one thing asserted N times by hand, and only the
//! declaration needed to be per-caller.
//!
//! And a fourth, in the same round: [`initial_ramdisk`], the `INITIAL_RAMDISK_VA` slice seven
//! `init`-shaped programs each rebuilt by hand with `core::slice::from_raw_parts`. A different
//! pattern from `mapped_window` (a whole `'static` slice, not a bounds-checked accessor), but the
//! same §94 shape underneath: one invariant, copied verbatim into seven declarations.
//!
//! Round 7 read every remaining raw `invoke(...)` call site in `user/` (123 of them, the
//! milestone's own largest unmigrated cluster) and found the same shape at nearly all of them: a
//! method whose own `# Safety` obligation is [`invoke`]'s own ("the kernel validates the capability
//! and the method before acting"), asserted by hand at every call site with nothing
//! call-site-specific to check. Fourteen thin wrappers, now in `objects.rs` and `ipc.rs`
//! (`retype_into_page_frame` through `send_capability`), plus [`is_granted`] (§94 (what may live in
//! a userspace library) again: five programs' identical probe) and the opt-in [`virtio`] module
//! (device-specific, so scoped like `mapped_window` rather than added here), cover all but one of
//! them; see that one call site's own comment (`window.rs`'s refusal probe) for why it stays raw.
//!
//! The `#[panic_handler]` is **still not an item here, and now the trap underneath it is**
//! (milestone 130). A panic handler is per-final-binary: exactly one may exist in a linked program,
//! so an item in this library would force it on every program that links the crate and collide with
//! any program that wants its own (as `hello` does). That has been recorded since 19f.6 and it is
//! still true. What went stale beside it was the sentence "each binary keeps its own one-line
//! handler; it is trivial": the handler grew to fifteen lines with two `unsafe` blocks and two
//! `// SAFETY:` comments, and by the time anyone counted, the trap instruction was inlined at
//! **forty-eight sites** in seven variants, one of which called [`exit`] instead of trapping and so
//! reported a clean death for a panicking program. The constraint was right and the inference from
//! it was not: a *handler* cannot live in a library, but the *trap* always could.
//!
//! So [`trap`] is here, and [`panic_handler!`] is a macro that expands to the handler in the
//! binary. The linking property survives, and the claim below about this being the one place in
//! userspace that names the two ABIs becomes true rather than aspirational. Device helpers (a UART
//! `putc`, echo logic) still stay in the drivers that own them: those are not runtime, they are the
//! program.
//!
//! # Examples
//!
//! **This crate is the one place in the tree where an example genuinely cannot run**, and the reason
//! is worth stating rather than hiding behind a fence marker. Every function here traps to the kernel
//! from EL0: `svc`/`ecall` on a machine with no nife kernel under it is a fault, not a syscall, and
//! `script/test`'s host pass excludes this crate and everything that depends on it (the exclusion set
//! is derived and checked by `script/lint`). So the examples below are `no_run`: they are type-checked
//! against the real signatures on an aarch64 host and are **not executed anywhere**. The things that
//! *can* be checked are the wire contracts layered over them, which is where those crates put their
//! examples (`byte_sink_protocol`, `filesystem_protocol`, `entropy_protocol`).
//!
//! A program's whole life, in the four calls that make up this crate. Note what is absent: there is
//! no `open`, no path, and no way to name anything that was not handed over.
//!
//! ```no_run
//! use user_mode_runtime::{exit, receive, send};
//!
//! /// A pipeline stage: read three words off the endpoint in slot 0, pass them to slot 1.
//! fn relay() -> ! {
//!     const IN: u64 = 0;
//!     const OUT: u64 = 1;
//!     loop {
//!         // `receive` blocks until a sender rendezvouses. The rendezvous IS the flow control: there
//!         // is no buffer to fill and no back-pressure to invent.
//!         let (w0, w1, w2) = receive(IN);
//!         if send(OUT, w0, w1, w2) < 0 {
//!             // A negative return is an `abi::Error`. `Gone` here means the reader exited, which
//!             // is this system's SIGPIPE, arriving as a return code rather than as a signal.
//!             exit();
//!         }
//!     }
//! }
//! ```
//!
//! A client of a service uses [`call`], which blocks until the reply lands. The reply arrives through
//! a one-shot capability the kernel mints, so the client never names the server and the server never
//! names the client:
//!
//! ```no_run
//! use user_mode_runtime::call;
//!
//! # fn ask() {
//! const SERVICE: u64 = 2;
//! let (r0, r1) = call(SERVICE, 0x0100_0000_0000_0008, 0);
//!
//! // Negative-as-u64 is enormous, which is how a wire contract tells "no capability in that slot"
//! // from an answer without a probe request. See `entropy_protocol::delivered`.
//! assert!((r0 as i64) >= 0 || abi::Error::from_ret(r0 as i64).is_some());
//! # let _ = r1;
//! # }
//! ```
//!
//! # Three ABIs, one surface
//!
//! The syscall instruction and the register file differ by architecture, and this is the one place
//! in userspace that names them. aarch64 uses `svc #0` with the syscall number in `x8` and arguments
//! in `x0..x5`; RISC-V uses `ecall` with the number in `a7` and arguments in `a0..a5`; `x86_64` uses
//! `syscall` with the number in `rax` and arguments in `rdi`, `rsi`, `rdx`, `r10`, `r8`, `r9`
//! ([DECISIONS §124](../../../design/decisions/0124-x86-64-syscall-abi.md), ratified 2026-08-24).
//! All three return in the first argument register (`x0` / `a0` / `rdi`). The kernel reconciles them
//! in `TrapFrame` (DECISIONS §17); here we simply select the right asm at compile time. Every
//! function's signature, semantics, and the `abi` constants are identical across all three.
//!
//! **`x86_64` is the arm that costs more than a transliteration**, and there are exactly three places
//! it does. `syscall` clobbers `rcx` and `r11` unconditionally, so every site declares them; `rdtsc`
//! answers in two halves rather than one register (see [`now`]); and there is **no architected
//! counter frequency at all**, which is why [`cntfrq`] carries a `BUGS` section rather than a number
//! with a comment. Everything else is the same three instructions in a different spelling.
//!
//! Name: ratified 2026-09-13 (calef, milestone 285), replacing `user_rt`. Two halves, argued
//! separately. **`rt`** was an **abbreviation that needs a decoder**, the first of the three
//! failure modes AGENTS.md names, and this crate's own first line had always spelled it out; the
//! precedent is `cred_proto` to `credential_proto`, ratified 2026-08-23 for "spell out the
//! contraction fully" (milestone 265 has since taken that crate to `credential_protocol`). Nothing outside the tree owns the spelling (no specification, no wire
//! format, no command-line flag), so the acronym test applies at full force here in a way it did
//! not to `initrd`. **`user_`** expanded to **`user_mode_`** because in this tree `user` means *a
//! person* several hundred times over: milestone 49 is users and attribution, `identity_provisioner`
//! creates them, `login` authenticates them, and `schedule_store` calls itself the per-user
//! schedule store. `user_runtime` would have kept reading as "the runtime belonging to a user".
//! "User mode" is this tree's architecture-neutral phrase for the unprivileged level, where `EL0`
//! is aarch64's word for it and `U-mode` and `ring 3` are the other two architectures'. Refused
//! `user_runtime` (leaves the prefix ambiguous between a person and a privilege level) and
//! `el0_runtime` (a crate name correct on one architecture and wrong on the two others, which is
//! what AGENTS.md rule 5 exists to catch).
//!
//! Introduced 2026-07-25 as `user_rt`, when the shared runtime was lifted out of five binaries that
//! had copied it verbatim, and provisional from then until milestone 285. It was half-argued in a
//! way that kept it unargued: milestone 63 ratified `user_heap` over `uheap` in part on the ground
//! that "`user_rt` already establishes `user_` as the prefix", so the prefix was on the record
//! because this crate had established it, which is a circle rather than a reason. Milestone 285
//! closed the circle by ruling on the prefix itself rather than by citing either crate.

#![no_std]

pub mod child_stub;
pub mod entropy;
pub mod heap;
pub mod initial_ramdisk;
pub mod mapped_window;
pub mod virtio;

mod clock;
mod devices;
mod ipc;
mod objects;
mod process;
mod syscall;

pub use abi::cycle_counter::CycleMeaning;
pub use clock::*;
pub use devices::*;
pub use ipc::*;
pub use objects::*;
pub use process::*;
pub use syscall::*;
