#![no_std]
// `Result<_, ()>` throughout the build path, for `supervision_protocol`'s reason: every failure here is
// a syscall that already returned its own error through the ABI, and a second, richer error would be
// inventing detail the kernel did not provide. See that crate's head comment.
#![allow(clippy::result_unit_err)]
//! **The interactive system, built once** (milestone 96 (one init: the spawn service written twice)).
//!
//! There is **one** first process, `components/src/progenitor.rs`, on all three architectures
//! (milestone 266), and **one** system it builds, which is this crate. What that program still
//! holds is the table of slot numbers its own kernel granted, which is a fact about a boot path and
//! nothing else; everything from parsing the archive to serving the shell's last `run` is here.
//!
//! There used to be two programs and twice-written construction code, and a fix that landed in
//! one copy and not the other was a boot that reached userspace and printed nothing at all: no
//! fault, no message. That failure, which cost three lanes an evening each, is why milestone 96
//! exists; its block holds the story. `script/swish-check` boots both ISAs and types at the
//! prompt, the only thing that runs the real progenitor, which is what proves this.
//! # Examples
//!
//! **This one cannot run either, and the reason is structural.** [`boot`] returns `!` and every step
//! it takes is a syscall on a capability the kernel granted at spawn, so there is nothing to assert
//! and nowhere to assert it: `script/test`'s host pass excludes this crate (it depends on `user_mode_runtime`'s
//! EL0 `asm!`, an exclusion `script/lint` derives and checks), and the thing that actually proves this
//! code is `script/swish-check`, which boots both ISAs and types at the prompt. So the example below
//! is `no_run`: type-checked against the real signatures, and executed by that gate.
//!
//! A progenitor's whole source, near enough. Everything a board contributes is the **table of slot numbers
//! its own kernel granted**, which is a fact about that boot path and nothing else:
//!
//! ```no_run
//! use system_initializer::{BootEndowment, boot};
//!
//! /// riscv64's the progenitor. The slot numbers come from `kernel::user::riscv_shell_boot` and are the only
//! /// thing this file knows that the other board's the progenitor does not.
//! fn init(initial_ramdisk_len: u64, fs_rights: u64) -> ! {
//!     let endowment = BootEndowment {
//!         untyped: 2,
//!         uart_dev: 3,
//!         uart_irq: 4,
//!         clock_page: 5,
//!         config_page: 6,
//!         fs_ep: 7,
//!         fs_page: 8,
//!         // Always these three slots, whether or not this boot has a disk (`grant_at`, not
//!         // first-free numbering, on the kernel side): a boot with no virtio-rng device leaves
//!         // them empty, and `boot`'s own probe is what tells it apart from a granted one. Fixed
//!         // past the filesystem pair's own max reach (slot 8), not slot 7, because the
//!         // inert-configuration page (slot 6) shifted that pair down by one.
//!         virtio_rng: 9,
//!         virtio_rng_irq: 10,
//!         virtio_rng_dma: 11,
//!         // A terminal on the firmware's screen (milestone 198's rung 1b): empty on every boot
//!         // whose console has no screen, the same "absence, not failure" shape as the rng trio.
//!         disp_term_ep: 12,
//!         disp_term_page: 13,
//!         // The network card (milestone 590 (the booted system starts its network stack)): empty
//!         // on a boot with no virtio-net device, which is every real board today. Numbered here
//!         // past this example's own screen pair; the real table is
//!         // `components/src/progenitor.rs`'s.
//!         virtio_net: 15,
//!         virtio_net_irq: 16,
//!         virtio_net_dma: 17,
//!         // An entropy service the kernel built on the CPU's seed instruction (milestone 595
//!         // (provisional)): empty whenever the virtio-rng trio is granted, and on a CPU with no
//!         // such instruction, which is every riscv64 part.
//!         entropy_ep: 18,
//!         // The graphical terminal stack's raw materials (milestone 600 (provisional)): a
//!         // virtio-gpu's four and a virtio keyboard's three, which the boot hands to the shell
//!         // rather than building anything from (milestone 632 (provisional): graphics is
//!         // launched from the prompt). Empty with no GPU.
//!         gpu_surface: 14,
//!         gpu: 19,
//!         gpu_irq: 20,
//!         gpu_dma: 21,
//!         keyboard: 22,
//!         keyboard_irq: 23,
//!         keyboard_dma: 1, // this example's table starts at slot 2, so 0 and 1 are free
//!         // The machine statistics page (milestone 126 (the `procps` package)): granted on
//!         // every boot, so always this slot. The last free one in this example's table.
//!         machine_page: 0,
//!         // The kernel's ring, its cursor page and its notification (milestone 342 (the kernel
//!         // and the `console` server drive one UART from two address spaces)): past the end of
//!         // this example's table, as on the real one.
//!         kernel_ring: 24,
//!         kernel_ring_cursor: 25,
//!         kernel_ring_notification: 26,
//!         // A USB keyboard driver's attach endpoint (milestone 242 (USB host and HID)): empty on
//!         // a machine with no xHCI controller.
//!         usb_keyboard_attach: 27,
//!         // This process's own address space (§249): granted on every boot, `WRITE` alone.
//!         own_space: 28,
//!         // A network stack the kernel built on an `e1000e` (milestone 198 (a package
//!         // manager)): empty on a boot with a virtio-net NIC, or with no NIC it may drive.
//!         net_stack_ep: 29,
//!         net_stack_report: 30,
//!         // The reboot object (milestone 805, DECISIONS §251): granted on every boot.
//!         reboot: 31,
//!         // Empty here. On aarch64 this holds the kernel's report endpoint and a test SGI, because
//!         // that boot path is shared with milestone 19d's test roles; the progenitor deletes them with the
//!         // device authority once the drivers exist, rather than keeping delegable authority for
//!         // nothing.
//!         for_test_roles: &[],
//!     };
//!
//!     // `fs_rights` of 0 means this boot attached no RedoxFS disk, in which case `fs_ep` and
//!     // `fs_page` hold nothing at all and the system comes up without a filesystem.
//!     //
//!     // The fourth argument is milestone 154's second directory grant: `None` here, as at every
//!     // real entry point today, because what the second subtree should *be* is a boot-time
//!     // policy decision, not something this call site decides for itself.
//!     boot(&endowment, initial_ramdisk_len, fs_rights, None)
//! }
//! ```
//!
//! Reading that struct literal is meant to tell you the complete authority of the system about to
//! exist, which is why the fields are named for what they *are* rather than numbered. Note what is
//! not in it: no second budget. (It said "no network" too until milestone 590 (provisional) granted
//! the network card; see [`BootEndowment::virtio_net`].)
//!
//! # What the kernel hands over, and what this builds from it
//!
//! [`BootEndowment`] names the capabilities the kernel granted: a construction budget, the UART's registers
//! as a device capability, the UART receive interrupt, a read-only view of the wall clock, when
//! this boot attached a RedoxFS disk the file service and the page its clients share with it and,
//! when a GPU is attached, the gpu's transport, interrupt, DMA run and surface run, plus a virtio
//! keyboard's three when there is one (milestone 600 (provisional) moved these out of the kernel).
//! The gpu's and the keyboard's go no further than the shell (milestone 632 (provisional)), which
//! holds them until a person at the prompt delegates them into a `graphical_terminal` session; from those,
//! and nothing else, [`boot`] builds the whole interactive system out of its own budget:
//!
//! 1. **output**: the **console** server, reading text from a shared page and writing it to the
//!    UART. The one output path on every boot: the graphical terminal stack is *not* built here,
//!    on any boot (milestone 632 (provisional), calef's 2026-09-30 ruling that graphics is
//!    launched from the prompt rather than present at boot), so a boot with a gpu attached is the
//!    same minimal UART system as one without. A terminal on the firmware's screen (a PC's
//!    framebuffer, milestone 198's rung 1b) is the one addition this crate wires: the console
//!    then writes every byte to that terminal too, so the UART and the screen say the same thing.
//! 2. **keystrokes**: the **input** driver, waiting on the UART receive interrupt and forwarding
//!    bytes. The virtio keyboard's grants, when a boot has them, ride this endowment out to the
//!    shell ([`BootEndowment::keyboard`]) and reach a driver only inside a `graphical_terminal` session the
//!    user launches, never the boot itself.
//! 3. the **line discipline** (`line_editor`, milestone 28): editing, echo, history, between
//!    whichever pair of the above this boot has. The same server either way; only its output-side
//!    wire shape changes (`mode`, `components/src/line_editor.rs`'s own `MODE_CONSOLE`/`MODE_DISPLAY`).
//! 4. the **shell**: prints and reads lines through the terminal endpoint, runs commands, and since
//!    milestone 86 holds a `READ` view of the wall clock so `time <command>` can measure one;
//! 5. the **terminal's sink adapter** (`terminal_sink_caretaker`, milestone 50 (pipes and redirection: one sink protocol)), when the archive
//!    carries one: it holds the terminal and serves the sink contract, so a declared second stream
//!    can be pointed at the screen without handing anyone the endpoint that also reads the keyboard;
//! 6. the **undertaker** (`job_undertaker`), which collects a finished job's corpse so its region
//!    comes back to the pool.
//!
//! They are wired together with endpoints and shared pages this code creates. The kernel wires none
//! of it. Then the progenitor stays alive as the spawn service (milestone 31): the shell resolves a `run` into
//! a grant expression and directs the progenitor to load the named program and endow it with exactly what the
//! command named. Nothing here names an architecture: the console and input drivers hold the one
//! device-specific fact (the UART register layout), and the kernel grants the right device.
//! # What it gives away once the system is up (milestone 22, the interactive increment)
//!
//! It carves two bounded budgets off the kernel's construction root and **deletes the root**, so
//! no process here is one bug away from being built wrong:
//!
//! - [`INIT_OWN_PAGES`] for its own scratch page tables, which is all it spends on itself; and
//! - [`JOBS_BUDGET_PAGES`] for the jobs the prompt asks for, one reclaimable region per job.
//!
//! It also gives up the UART device capability and the UART interrupt as soon as the drivers that
//! need them are built, and everything in [`BootEndowment::for_test_roles`] with them. **It keeps
//! the file service** (milestone 31 (a capability shell) phase 3, 2026-08-17): DECISIONS §219 (how
//! the shell names an installed program to the spawner) rules that the spawner can read an
//! installed program, so the old claim that it could not is retired; see "The filesystem stays" in
//! `boot`. The proof is a negative control taken from inside the process and printed at the
//! prompt, exactly the shape `root_supervisor` uses: after the delete, `RETYPE` and `RETYPE_OBJ`
//! on that slot must answer `NoSuchSlot` (there is nothing there) rather than `NotPermitted`
//! (there is, and you may not).
//!
//! The job budget is **renewable**, which is what makes bounding it cheap. Every job is built in its
//! own region split off [`JOBS_BUDGET_PAGES`] and is born supervised: `job_undertaker`, a process
//! holding one endpoint capability and nothing else, collects each corpse through `Rendezvous::REAP`
//! (DECISIONS §32 (a supervisor may collect a corpse without being able to build one)) and the region's pages come back here (§13: a reclaimed region returns to its
//! owner, which is whoever split it). Before that, a spawned job's memory was spent for the life of
//! the boot.
//!
//! # What it refuses to load (milestone 104)
//!
//! The kernel measures the one program *it* loads, which is this one. Everything else in the
//! archive is loaded here, and those bytes used to be unchecked, so the chain of trust stopped at
//! the progenitor's entry. It does not now. The build packs a table of digests into the archive
//! ([`measured_boot::PROGRAM_MEASUREMENTS`]), the kernel's trust root vouches for that table exactly
//! as it vouches for this program's own bytes, and [`boot`] looks every program up in it before
//! loading it.
//!
//! **One rule: the progenitor grants nothing of its own to what it cannot vouch for.** Restated
//! 2026-09-26 by DECISIONS §219 (how the shell names an installed program to the spawner), from
//! "runs nothing it cannot vouch for": unvouched bytes a caller sends may run with the caller's
//! grants and the clock and configuration pages, but only for a session holding §219's D2
//! capability (`Channels::run_unvouched`; the boot prompt holds it by DECISIONS §221). That is
//! `spawn_service`'s image path. Everything this section describes below is the archive's, where
//! the rule still means refusal: a digest that does not match is a refusal, and so is a
//! name the table does not mention, for the reason the kernel's empty trust root is refused: a
//! check that passes when there is nothing to check against is not a check.
//!
//! What a refusal *costs* is not a second policy. It falls out of what the program was for, which is
//! a question this crate already had to answer for an archive entry that is simply missing, so a
//! refused program is treated exactly as a missing one:
//!
//! - **console, input, `line_editor`, swish, `job_undertaker`.** The system is made of these, so not
//!   running one and not having a system are the same outcome. The progenitor prints which one it refused and
//!   traps, which is `kernel::trust::require`'s decision one link down.
//! - **`terminal_sink_caretaker`.** Already optional: a boot without an adapter comes up and a
//!   declared second stream finds an empty slot. A refused adapter costs that same feature.
//! - **The spawnable programs.** Left out of the table `spawn_service` indexes, so the prompt
//!   answers "could not spawn" for them and everything else still works. Halting a running machine
//!   because `wc` changed would turn a build defect into an unbootable system, and it buys nothing:
//!   the guarantee is that nothing unvouched-for runs, and not spawning it is that guarantee.
//!
//! Recording a mismatch and loading anyway was considered and refused. There is no audit log to put
//! a record in, and a measurement that changes nothing about what runs is theater; a chain whose
//! second link is advisory is not a chain.
//!
//! # BUGS
//!
//! **Nothing a host test could run proves any of this.** This crate reaches `user_mode_runtime`
//! through the dependency graph, so it does not build for the host at all; `script/lint`,
//! `script/coverage` and `.cargo/mutants.toml` all exclude it, and a gate in `script/lint`
//! derives that set from cargo metadata so the lists cannot drift apart again.
//! `script/swish-check` is what proves this code: it boots both ISAs and types at the prompt.
//! Milestone 244 (the largest crate in the tree is proved by nothing a mutation can reach)'s
//! block holds the mutation measurement (196 mutants, a sixth of them pure, nothing lifted), and
//! milestone 246 (measured boot's refusal path is tested by nothing, and one mutant turns it off)
//! lifted the one that mattered: `measured_boot::verdict`, host-tested and
//! falsified.
//!
//! **A refused `console` or `line_editor` stops in silence.** Those two carry the progenitor's
//! output, so a refusal of either has no route to a person: the trap is indistinguishable from
//! any other early failure. Everything refused after them is named on the console. There is no
//! debug-print syscall (the kernel-served `Console` object went away at milestone 8 (the console driver leaves the kernel)), and driving
//! the UART from here would be a second copy of the console driver, per ISA, inside the process
//! the drivers exist to keep small.
//!
//! **An absent required component still traps with no message.** That is a build that did not
//! pack it rather than bytes somebody swapped, and it has never had one.
//!
//! **The measurement is of the archive, not of memory over time.** It is checked once, when the
//! program is loaded. Nothing re-measures a running process, and nothing measures the pages the
//! progenitor wrote into a child after `build_child` copied them.
//!
//! The return of pages is **LIFO** (§16 (object revocation), `crates/memory_regions`): a job
//! region that is not at the top of the budget's watermark when it is reclaimed returns nothing
//! at once, and its run is a hole until no job above it is live. Until 2026-10-03 (UTC) the hole
//! lasted until this process died, which it never does; milestone 676 (the NTP and login tests
//! give their regions back) closed that on CI's riscv64 runner. A hole under a job that stays
//! live is still not reusable while it does, because a carve still only bumps.
//!
//! **A `graphical_terminal` session's size is measured, after the count was wrong** (milestone
//! 632). [`GRAPHICAL_TERMINAL_SESSION_PAGES`] was first bounded from the constants at 464, and
//! every launch failed with `could not spawn`; its doc holds the bisection. While a session runs,
//! its 528-page region is one of the pool's 672, so a `std` job and a second session are refused
//! until it ends, which the shell also reports as `could not spawn`.
//!
//! **A session whose build fails after its first driver started may keep those pages for the
//! boot.** The failure path reclaims the region, and reclaim's sweep wakes the half-built
//! drivers, but whether they exit on a swept endpoint is a property of *their* serve loops that
//! no leg has forced yet. The same applies to a driver that dies on its own inside a live
//! session: its pages wait for the session program's reap.
//!
//! **A session is not §24-interruptible.** It ends on its own rather than through a job frame
//! the shell watches, so a second `^C` cannot escalate and a hung session holds its region until
//! something reboots the machine. The supervised-session shape is follow-on work in 632's block.
//!
//! **The progenitor gives up every page it lays down for a child as soon as the page is in the
//! child** (milestone 95 (an unmap primitive, and the mappings init never lets go), §249 (a
//! running address space stays nameable)). The kernel grants it its own address space at slot 28
//! (`BootEndowment::own_space`), and the loader `UNMAP`s each scratch page through it
//! (`supervision_protocol::give_up_own_page`), since 2026-10-05. The other three windows it
//! opened onto memory it hands on go the same way: the shell's output page once the last boot
//! line is printed, and the virtio-rng and NIC DMA pages once their physical base is read.
//! `system_tests`' `running_space_tests` has the negative control, a builder faulting on the
//! page. A kernel that grants no slot 28 leaves every window open, as before.
//!
//! **A boot with a NIC waits for DHCP before it has a prompt** (milestone 590). `net_stack`
//! reports its lease with a blocking send and serves nobody until that send is taken. On QEMU's
//! user-mode network the answer is immediate; on a network with a virtio NIC and no DHCP server
//! the boot would sit there with no console to say why, a case unreached rather than closed
//! because nothing grants a NIC on real hardware today. Milestone 590's block records the fix
//! shape and its slot cost.
//!
//! **The progenitor's capability table is finite, and running out of it prints nothing at all.**
//! Every capability held across a `build_child` is one the child's address space, frames and TCB
//! cannot have, and `build_child` answering `Err(())` is a silent halt. Three of the four
//! evenings this file has cost were exactly that, and the fourth was a slot count silently
//! lowered below what the login stack needs: milestone 230 (`script/shell-check` is red on
//! `main`) is that incident's record. The table is thirty-two now (`kernel::cap::
//! CAPABILITY_TABLE_SLOTS`, raised to twenty-four by milestone 230 and to thirty-two by calef on
//! 2026-09-27 for the machine statistics page), the measured peak is twenty-three of them on a
//! boot with a NIC, and `kernel::cap::CAPABILITY_TABLE_PEAK_MEASURED` holds the history of both
//! numbers. Nine slots over the peak is deliberate: every raise that took the table to exactly
//! the day's need was hit in silence by the next addition. If a change here needs another
//! permanent capability, the honest buy-backs are the readiness endpoint and the file page
//! (`notes/shared-page-audit.md`).
//!
//! Name: ratified 2026-08-04 (calef, milestone 96; the ratification that raised milestone 115 (the names that were ratified, and the ones that were refused)).
//! The refusals (`system_builder`, `system_bootloader`), the lane that re-proposed one anyway,
//! the 2026-09-13 re-ask and its answer (*"init is the issue not initializer."*), and the
//! verb-versus-agent-noun reasoning are in git history. `BootEndowment` was
//! ratified the same day, replacing `Grants`.

mod activation;
mod announcement;
mod boot_sequence;
mod boot_steps;
mod children;
mod client_windows;
mod endowment;
mod fault;
mod image_request;
mod layout;
mod measurement;
mod spawn_loop;
mod std_layout;

use core::sync::atomic::AtomicU16;
use core::sync::atomic::Ordering::Relaxed;

use activation::{Activating, activate, copy_text, package_manager_parts, serve_installer};
use announcement::*;
pub use boot_sequence::boot;
use children::*;
use client_windows::*;
pub use endowment::{BootEndowment, SecondDirGrant};
use fault::*;
use filesystem_protocol::{dir, fs as fs_operation};
use grant_plan::job_windows::{ReapsDue, Windows};
use grant_plan::{Prog, spawnproto};
use image_request::*;
use layout::*;
pub use layout::{
    CHILD_STACK_PAGES, GRANT_MACHINE_PAGE, IMAGE_POOL_PAGES, INIT_OWN_PAGES, JOBS_BUDGET_PAGES,
};
use line_editor::proto;
use measurement::*;
use spawn_loop::*;
use std_layout::{StdGrants, StdLayout};
// The loader, and the tree's only one since milestone 96. Named here rather than qualified at every
// call, because the point of the crate is that there is one of these.
use supervision_protocol::{
    ChildEndowment, Retention, build_child, retype_obj_from as retype_obj,
    retype_page_frame_from as retype_page_frame, start_child,
};
use user_mode_runtime::{
    call, cap_delete, invoke, is_granted, receive, receive_badged, receive_cap_badged, send,
    send_capability,
};
