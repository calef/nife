//! **`terminal_supervisor`: holds the line editor, and replaces it live** (milestone 23 (a
//! capability-routed component OS with live replacement); calef's ruling of 2026-09-27, option A of
//! `design/roadmap/0661-a-terminal-supervisor-holds-the-line-editor.md`).
//!
//! `system_initializer` builds this rather than `line_editor`, and hands it the objects a
//! `line_editor` is made of plus a budget for two instances. It builds `line_editor` from the
//! declaration in `line_editor::component`, keeps its endowment, control endpoint, handoff page and
//! region, and runs the swap of DECISIONS §209 (state handoff is an opaque blob over a granted
//! frame, and it is optional) when asked:
//!
//! ```text
//!   1 BUILT      lay the replacement out, wired from the same declaration, not started.
//!   2 QUIESCED   OPERATION_QUIESCE, answered within QUIESCE_BOUND_MS. The endpoint's FIFO drains
//!                everything ahead; a parked reader is answered FLAG_RETRY; the blob is written.
//!   3 ABSORBING  start the replacement with START_ABSORB. It reads the blob before it serves.
//!   4 COMMIT     NOTE_ABSORBED: tell the incumbent CTL_QUIT and collect it.
//!     or UNDO    NOTE_NOT_ABSORBED: collect the refuser and tell the incumbent CTL_RESUME.
//!   2' HUNG      no answer: start the replacement empty, destroy the incumbent's region (its
//!                callers are answered Gone), and reply REPLACED_HUNG.
//! ```
//!
//! A hung `line_editor` looks like an idle one from here, `BLOCKED` and refused by `REAP`; only an
//! unanswered quiesce tells them apart (notes/hung-component.md). A `CALL` takes no deadline, so a
//! stand-in built from this program's own image makes it and signals the answer on a notification
//! the supervisor's timer also signals. It is `swapper`'s shape and its bounded quiesce, on a
//! component a person uses. The two control endpoints alternate, so a word meant for the incumbent
//! never reaches its replacement.
//!
//! # BUGS
//!
//! Nothing on a real boot asks it to swap: that is milestone 694 (the installer asks the terminal
//! to swap), which also brings a new image with `SWAP`. A hung `line_editor`'s state is lost, and
//! `swish` and `rmle` read the `Gone` its callers get as a failed read. A slow one is convicted as
//! hung. One that faults is not restarted. notes/interactive-stack-swap.md has each in full.
//!
//! Name: ratified 2026-10-06 (calef, "Ratify" it, in conversation). Minted
//! provisionally 2026-09-27 by the lane for milestone 23.

#![no_std]
#![allow(missing_docs)]
#![no_main]

use component_plan::Provisions;
use line_editor::component::{self, supervisor as s};
use line_editor::proto;
use supervision_protocol::{ChildEndowment, Retention};
use user_mode_runtime::{
    cap_delete, cntfrq, notification_poll, notification_signal, notification_wait, now, receive,
    receive_fault, receive_request, reply, send, timer_arm,
};

const MODE_DISPLAY: u64 = 1;
/// The first start argument that makes this image a stand-in rather than a supervisor. Clear of
/// `line_editor`'s own sink modes, `0` and `1`, which share the argument.
const MODE_STAND_IN: u64 = 2;

/// A stand-in's slots: the terminal endpoint it quiesces, and the notification it answers on.
const STAND_IN_TERMINAL: u64 = 0;
const STAND_IN_ANSWER: u64 = 1;
/// A stand-in makes three syscalls from `_start`'s own frame.
const STAND_IN_STACK_PAGES: u64 = 4;
/// The bits of the quiesce's notification: the timer's deadline, and the stand-in's two answers.
const DEADLINE: u64 = 1 << 0;
const ANSWERED_QUIESCED: u64 = 1 << 1;
const ANSWERED_OTHERWISE: u64 = 1 << 2;

#[unsafe(no_mangle)]
pub extern "C" fn _start(mode: u64, lengths: u64, swap: u64) -> ! {
    if mode == MODE_STAND_IN {
        stand_in()
    }
    supervise(mode, lengths, swap)
}

/// **The stand-in: one `CALL` the supervisor cannot make itself**, because it has to be able to
/// stop waiting for the answer. Its whole authority is `WRITE` on the terminal endpoint and on the
/// supervisor's notification. A hung incumbent leaves it blocked here until the supervisor
/// destroys its region, which finishes it in place and unlinks its request from the endpoint's
/// queue (milestone 133 (ending a permanently blocked thread, and deciding who may)).
fn stand_in() -> ! {
    let (q, _) = user_mode_runtime::call(
        STAND_IN_TERMINAL,
        proto::req(proto::OPERATION_QUIESCE, 0),
        0,
    );
    let answer = if q == proto::QUIESCED {
        ANSWERED_QUIESCED
    } else {
        ANSWERED_OTHERWISE
    };
    let _ = notification_signal(STAND_IN_ANSWER, answer);
    user_mode_runtime::exit()
}

#[inline(never)]
fn supervise(mode: u64, lengths: u64, swap: u64) -> ! {
    let (editor_len, own_len) = s::unpack_image_lengths(lengths);
    // SAFETY: the builder copied the `line_editor` image to ELF_VA, at the length it packed, before
    // starting us, and nothing writes it afterwards.
    let bytes = unsafe { core::slice::from_raw_parts(s::ELF_VA as *const u8, editor_len as usize) };
    let Ok(image) = elf::Elf::parse(bytes) else {
        fail()
    };
    // Only a supervisor that can be asked to swap ever quiesces, so only its spawner copies our own
    // image (`supervisor::OWN_IMAGE_ADDRESS`). One that passed a swap endpoint without it is
    // refused here, at start, rather than at the first swap.
    let own = if own_len == 0 {
        None
    } else {
        // SAFETY: as above, for our own image at OWN_IMAGE_ADDRESS.
        let own = unsafe {
            core::slice::from_raw_parts(s::OWN_IMAGE_ADDRESS as *const u8, own_len as usize)
        };
        elf::Elf::parse(own).ok()
    };
    let deadline = match (swap, own) {
        (0, _) => None,
        (_, Some(own)) => Some(Deadline {
            own,
            wake: obj(abi::objtype::NOTIFICATION),
            timer: obj(abi::objtype::TIMER),
        }),
        (_, None) => fail(),
    };

    let faultep = obj(abi::objtype::RENDEZVOUS);
    let controls = [obj(abi::objtype::RENDEZVOUS), obj(abi::objtype::RENDEZVOUS)];
    let notify = obj(abi::objtype::RENDEZVOUS);
    let handoff =
        match user_mode_runtime::retype_page_frame_run(s::BUDGET, component::HANDOFF_PAGES) {
            f if f >= 0 => f as u64,
            _ => fail(),
        };

    let decl = if mode == MODE_DISPLAY {
        &component::DISPLAY
    } else {
        &component::CONSOLE
    };
    let control_in_child = component_plan::slot_of(decl, "control");

    // **Two bays, one per instance, split once and reused.** A child's region comes home to the
    // region it was split from, and only un-bumps that region's watermark when it is on top. Built
    // straight from the budget, the incumbent's region is always under its replacement's when it
    // dies, so every swap would leave a hole the size of an instance: the second swap failed for
    // want of budget, which is how this was found. Each instance is split from its own bay, where
    // it is the only child and so always on top.
    let bays = [bay(decl), bay(decl)];

    let mut sup = Supervisor {
        image,
        deadline,
        decl,
        mode,
        control_in_child,
        faultep,
        controls,
        notify,
        handoff,
        bays,
        instances: [0, 0],
        current: 0,
        started: 0,
    };
    let plan = sup.plan_for(sup.controls[sup.current]);
    sup.instances[sup.current] = launch(
        &sup.image,
        &plan,
        sup.bays[sup.current],
        faultep,
        [mode, control_in_child, proto::START_HANDOFF],
    );
    sup.started += 1;

    if swap == 0 {
        // No trigger on this boot (see BUGS). Hold everything and wait, so the objects stay ours.
        loop {
            let _ = receive(faultep);
        }
    }

    loop {
        let req = receive_request(swap);
        let verb = req.w0;
        // Only a CALL is answered; a delegation is deleted (milestone 706 (a `CALL` server can tell
        // a Reply from a delegation)).
        let Some(caller) = req.delivered.into_reply() else {
            continue;
        };
        if verb == s::STOP {
            sup.stop(caller)
        }
        if verb != s::SWAP {
            reply(caller, proto::BAD_REQUEST, 0);
            continue;
        }
        sup.swap(caller);
    }
}

/// Everything the swap loop works with: the image and declaration every instance is built from,
/// the objects this supervisor made, and which bay holds the live instance.
struct Supervisor<'a> {
    image: elf::Elf<'a>,
    /// What a bounded quiesce is made with. `Some` whenever there is a swap endpoint, which is the
    /// only way a quiesce is reached.
    deadline: Option<Deadline<'a>>,
    decl: &'static component_plan::Requirements,
    mode: u64,
    control_in_child: u64,
    faultep: u64,
    controls: [u64; 2],
    notify: u64,
    handoff: u64,
    bays: [u64; 2],
    /// The region of the instance in each bay, `0` for none. Kept, unlike any other child's,
    /// because destroying it is the one way to end an instance that will never cooperate.
    instances: [u64; 2],
    /// Which bay, and which control endpoint, belong to the live instance.
    current: usize,
    /// How many instances have been started, reported back on `SWAPPED` and `STOPPED`.
    started: u64,
}

impl Supervisor<'_> {
    /// The plan for an instance whose control endpoint is `control`.
    fn plan_for(&self, control: u64) -> component_plan::Plan {
        // The sink's reply endpoint is routed whether or not the declaration names it: a provision
        // nothing asks for is not an error, and one table serves both modes.
        let held: [(&'static str, u64); 9] = [
            ("terminal", s::TERMINAL),
            ("sink", s::SINK),
            ("sink_reply", s::SINK_REPLY),
            ("control", control),
            ("supervisor", self.notify),
            ("sink_page", s::SINK_PAGE),
            ("client_out", s::CLIENT_OUT),
            ("client_in", s::CLIENT_IN),
            (component_plan::STATE_HANDOFF_ROLE, self.handoff),
        ];
        let Ok(plan) = component_plan::plan(self.decl, &Provisions { held: &held }) else {
            fail()
        };
        plan
    }

    /// `STOP`: quiesce and retire the live instance, give both bays back, and exit.
    fn stop(&self, caller: user_mode_runtime::Reply) -> ! {
        let live = self.instances[self.current];
        match self.quiesce() {
            Quiesce::Answered => {
                send(self.controls[self.current], proto::CTL_QUIT, 0, 0);
                collect(self.faultep, live);
            }
            Quiesce::Unanswered { .. } => end_region(live),
        }
        // Both bays are empty now, and a budget with children cannot be reclaimed by whoever
        // owns it, so give them back before going.
        for b in self.bays {
            if !supervision_protocol::memory_region_destroy(b) {
                fail()
            }
        }
        reply(caller, s::STOPPED, self.started);
        user_mode_runtime::exit()
    }

    /// `SWAP`: build a replacement in the other bay, quiesce the incumbent, start the replacement
    /// absorbing its state, and commit or roll back on what the replacement says.
    fn swap(&mut self, caller: user_mode_runtime::Reply) {
        let next = 1 - self.current;
        let plan = self.plan_for(self.controls[next]);
        let Ok(region) = supervision_protocol::memory_region_split(self.bays[next], plan.pages())
        else {
            fail()
        };
        let Ok((child, aspace)) = supervision_protocol::build_child_space(
            s::BUDGET,
            region,
            &self.image,
            &endowment(&plan, self.faultep),
        ) else {
            fail()
        };

        let waited_ms = match self.quiesce() {
            Quiesce::Answered => None,
            Quiesce::Unanswered { waited_ms } => Some(waited_ms),
        };
        // A hung incumbent wrote no blob, so the replacement starts empty rather than absorbing.
        let start = match waited_ms {
            None => proto::START_HANDOFF | proto::START_ABSORB,
            Some(_) => proto::START_HANDOFF,
        };
        if supervision_protocol::configure_child(child.tcb, aspace, self.image.entry()).is_err()
            || !supervision_protocol::start_child(child, self.mode, self.control_in_child, start)
        {
            fail()
        }
        self.instances[next] = region;
        self.started += 1;

        if let Some(waited_ms) = waited_ms {
            // After the start, so a caller freed below finds a server when it asks again rather
            // than parking in the down window. `DESTROY` finishes the blocked incumbent where it
            // sits (milestone 133) and answers every caller it held a reply capability for with
            // `Gone` (milestone 254 (a caller stranded by a server that died)). A finished thread
            // sends no death message, so there is nothing to collect.
            end_region(self.instances[self.current]);
            self.instances[self.current] = 0;
            self.current = next;
            reply(caller, s::REPLACED_HUNG, waited_ms);
            return;
        }

        let (note, why, _) = receive(self.notify);
        if note == proto::NOTE_ABSORBED {
            send(self.controls[self.current], proto::CTL_QUIT, 0, 0);
            collect(self.faultep, self.instances[self.current]);
            self.instances[self.current] = 0;
            self.current = next;
            reply(caller, s::SWAPPED, self.started);
        } else {
            collect(self.faultep, region);
            self.instances[next] = 0;
            send(self.controls[self.current], proto::CTL_RESUME, 0, 0);
            reply(caller, s::ROLLED_BACK, why);
        }
    }

    /// **`OPERATION_QUIESCE`, with a deadline a `CALL` cannot have.** Builds a stand-in, starts it to
    /// make the call, arms the timer, and waits on the notification both of them signal. The
    /// stand-in is built before the clock starts, so the bound measures the incumbent and not a
    /// build.
    ///
    /// **The bound, `QUIESCE_BOUND_MS` (250 ms), is about thirty times the slowest healthy answer
    /// measured.** Timed from the stand-in's start to its answer, on 2026-10-19 (UTC), eleven swaps
    /// per run under `script/test`, on a 16 GB Apple Silicon host with a load average of 5 to 6
    /// from other lanes' builds: aarch64 2.0 to 2.8 ms over three runs, riscv64 3.6 to 7.8 ms over
    /// three, `x86_64` 1.9 to 2.6 ms over its two legs of one run. It covers a parked reader
    /// answered `FLAG_RETRY` and the blob written, which `swapper`'s fixture does not have, so it
    /// is this component's bound and not a copy of `swapper`'s. It is wall time, since
    /// `script/test` runs TCG without `-icount`, and the margin is for a loaded host. The deadline
    /// fires at the first 10 ms tick at or after it; the hung test prints the real wait.
    fn quiesce(&self) -> Quiesce {
        let Some(d) = &self.deadline else { fail() };
        let Ok(region) = supervision_protocol::memory_region_split(s::BUDGET, s::STAND_IN_PAGES)
        else {
            fail()
        };
        let granted = [
            (s::TERMINAL, abi::rights::WRITE),
            (d.wake, abi::rights::WRITE),
        ];
        let Ok(child) = supervision_protocol::build_child(
            s::BUDGET,
            region,
            &d.own,
            &ChildEndowment {
                caps: &granted,
                fault: Some(self.faultep),
                stack_pages: STAND_IN_STACK_PAGES,
                ..ChildEndowment::new(Retention::Nothing)
            },
        ) else {
            fail()
        };

        let bound = cntfrq() * s::QUIESCE_BOUND_MS / 1000;
        let started = now();
        if !supervision_protocol::start_child(child, MODE_STAND_IN, 0, 0)
            || timer_arm(d.timer, started + bound, d.wake, DEADLINE) < 0
        {
            fail()
        }
        let word = notification_wait(d.wake);
        if word <= 0 {
            fail()
        }
        let word = word as u64;
        if word & (ANSWERED_QUIESCED | ANSWERED_OTHERWISE) != 0 {
            // `0` means the deadline had already passed and signaled, after the word just taken.
            // Clear it now, or the next quiesce's wait would end on it at once and convict a
            // healthy incumbent.
            if user_mode_runtime::timer_cancel(d.timer) == 0 {
                let _ = notification_poll(d.wake);
            }
            collect(self.faultep, region); // the stand-in, which exits after its answer
            if word & ANSWERED_QUIESCED == 0 {
                fail()
            }
            return Quiesce::Answered;
        }
        let waited = now() - started;
        // First, because the stand-in's request is still queued on the terminal endpoint, and the
        // replacement about to serve it would otherwise be quiesced by it. Then clear an answer the
        // stand-in managed to signal before it was finished, which the next quiesce would read.
        end_region(region);
        let _ = notification_poll(d.wake);
        Quiesce::Unanswered {
            waited_ms: waited * 1000 / cntfrq(),
        }
    }
}

/// What a bounded quiesce is made with: this program's own image, which every stand-in is built
/// from, and the notification and timer the wait ends on. Retyped from the budget, so a spawner
/// grants nothing for them.
struct Deadline<'a> {
    own: elf::Elf<'a>,
    wake: u64,
    timer: u64,
}

/// What a bounded quiesce found.
enum Quiesce {
    /// The incumbent answered within the bound, and is drained and waiting for its control word.
    Answered,
    /// The bound passed with no answer.
    Unanswered { waited_ms: u64 },
}

/// **Destroy a child's region, whatever its resident is doing** (`swapper`'s helper of the same
/// name). A `Blocked` resident is finished in place on the first try. A `Ready` or `Running` one is
/// marked killed and the `DESTROY` refused; it dies at its next preemption, and a retry then finds
/// a corpse and reclaims.
fn end_region(region: u64) {
    for _ in 0..1000 {
        if supervision_protocol::memory_region_destroy(region) {
            return;
        }
        user_mode_runtime::yield_now();
    }
    fail()
}

fn endowment<'a>(plan: &'a component_plan::Plan, faultep: u64) -> ChildEndowment<'a> {
    ChildEndowment {
        caps: plan.caps(),
        maps: plan.maps(),
        blobs: &[],
        fault: Some(faultep),
        stack_pages: component::STACK_PAGES,
        ..ChildEndowment::new(Retention::Nothing)
    }
}

/// Build and start an instance in `bay`, and return its region, which the caller keeps.
fn launch(
    image: &elf::Elf,
    plan: &component_plan::Plan,
    bay: u64,
    faultep: u64,
    args: [u64; 3],
) -> u64 {
    let Ok(region) = supervision_protocol::memory_region_split(bay, plan.pages()) else {
        fail()
    };
    let Ok(child) =
        supervision_protocol::build_child(s::BUDGET, region, image, &endowment(plan, faultep))
    else {
        fail()
    };
    if !supervision_protocol::start_child(child, args[0], args[1], args[2]) {
        fail()
    }
    region
}

/// Wait for one death and collect it, so the child's region comes home to the budget, then drop
/// the region capability that was kept for it.
fn collect(faultep: u64, region: u64) {
    let (_event, tid, _pc, _addr, _, _) = receive_fault(faultep);
    if user_mode_runtime::reap(faultep, tid) != 0 {
        fail()
    }
    cap_delete(region);
}

fn bay(decl: &component_plan::Requirements) -> u64 {
    match supervision_protocol::memory_region_split(s::BUDGET, decl.pages) {
        Ok(slot) => slot,
        Err(_) => fail(),
    }
}

fn obj(objtype: u64) -> u64 {
    match supervision_protocol::retype_obj_from(s::BUDGET, objtype) {
        Ok(slot) => slot,
        Err(()) => fail(),
    }
}

fn fail() -> ! {
    user_mode_runtime::trap()
}

user_mode_runtime::panic_handler!();
