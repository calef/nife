/// `invoke(cap, SEND, w0, w1, w2)` -> 0. **Blocks until a receiver takes the message.**
///
/// The three words travel in registers and never touch memory. That is the whole of the
/// fastpath, and it is DECISIONS §10's rule made real: *IPC carries control.* Bulk data will
/// move later by handing over a frame capability, not by copying bytes into a message.
///
/// Two refusals a sender must tell apart, and the ABI does (milestone 50): [`crate::Error::NoSuchSlot`]
/// means the slot is empty, and [`crate::Error::Gone`] means the endpoint this capability names has
/// been destroyed, including while this thread was blocked inside the send. See
/// `crates/byte_sink_protocol`.
pub const SEND: u64 = 0;

/// `invoke(cap, RECEIVE, _, _, _)` -> w0, with w1 in x1 and w2 in x2. **Blocks until a message
/// arrives.** `x3` and `x4` are written only by the kernel: for an ordinary message `x3` is the
/// badge on the endpoint capability the sender invoked ([`BADGE`], `0` when unbadged) and `x4`
/// is `0`; the fault address and a reserved `0` for a §26 death message; and
/// [`notification::BOUND`](crate::notification::BOUND) in `x4` when a bound notification ended
/// the receive (milestone 151 (notification objects)).
///
/// The badge in `x3` is milestone 613 (a system log service: the in-memory half)'s amendment
/// to §230 (badged endpoint capabilities), which delivered it on [`RECEIVE_CAP`] only: the log
/// stamps a byte-sink writer from its badge, and a byte-sink writer `SEND`s. Before it, `x3` was
/// always `0` for an ordinary message. A death message is told apart by its first word, as
/// before; nothing reads `x3` before checking that.
///
/// **A plain `RECEIVE` never takes a capability**, whichever side reached the rendezvous first
/// (§246 (a plain `RECEIVE` never takes a capability), PROVISIONAL number; calef's ruling A,
/// 2026-10-04 UTC). A [`SEND_CAP`] delivers its data word in `x0`, `0` in `x1` and `x2`, and the
/// capability is dropped (the sender keeps its own copy). A [`CALL`] delivers its two words in
/// `x0` and `x1`, and the caller is answered [`Gone`](crate::Error::Gone), because the one-shot
/// Reply has nowhere to go. Take capabilities with [`RECEIVE_CAP`], and serve a `CALL` with it.
/// Until the ruling a receiver parked first had the capability installed in its table and its
/// slot in `x1`, so any holder of `WRITE` here and `GRANT` on anything could fill the table of a
/// receiver that never asked (found by milestone 752 (a seeded syscall driver with a shadow
/// model)).
///
/// # BUGS
///
/// **`x0` is the sender's own `w0`, so a sender can make a receiver read an error.** A negative
/// `w0` equal to an [`Error`](crate::Error) code is decoded as that error by any wrapper that
/// checks the sign, and the receiver cannot tell it from a real refusal. Found by milestone 151
/// while checking the premise of §101 (notification objects) that `w0` could carry a tag; the same fact is why the
/// notification tag lives in `x4`. Not fixed: the fix is a register convention for `RECEIVE`'s
/// status, which every receiver in the tree is written against.
pub const RECEIVE: u64 = 1;

/// `invoke(cap, SEND_CAP, cap_slot, rights, w0)` -> 0. **Delegate a capability.** Passes the
/// capability in the sender's `cap_slot`, narrowed to `rights` (see [`crate::rights`]), plus one data
/// word, over this endpoint; blocks until a receiver takes it. The endpoint capability needs
/// `WRITE` (you may send here), and the *delegated* capability needs `GRANT` (you were trusted
/// to pass it on). `rights` may only narrow what the sender holds, never widen it. This is the
/// operation that makes nife a capability system a process can actually compose in:
/// authority moves between processes at runtime instead of being wired by the kernel at spawn.
/// Only a [`RECEIVE_CAP`] takes the capability: a plain [`RECEIVE`] gets the data word and the
/// copy is dropped, on either arrival order (§246, PROVISIONAL number).
pub const SEND_CAP: u64 = 2;

/// `invoke(cap, RECEIVE_CAP, _, _, _)` -> w0, with the received capability's new slot in x1, a
/// second data word in x2, and the **sender's badge in x3** (milestone 599, provisional), or
/// [`NO_CAP`] in x1 if the message carried no capability. **Blocks until a message arrives.**
/// The received capability lands in a free slot of the receiver's own capability table, chosen
/// by the kernel; x1 is where. This is also how a server receives a [`CALL`]: the slot in x1
/// holds a one-shot [`crate::reply`] capability naming the caller. **x3 is the badge on the
/// endpoint capability the sender invoked** ([`BADGE`]), or 0 when it was unbadged, which is how
/// a server serving many clients on one endpoint tells them apart. **x4 is
/// [`REPLY_DELIVERED`] when x1 holds a `CALL`'s Reply** (milestone 706), `notification::BOUND`
/// when a bound notification ended the receive, and `0` otherwise; x4 is written only by the
/// kernel. Needs `READ`.
///
/// # BUGS
///
/// **A plain `SEND` received here used to fill `x1` differently depending on who arrived
/// first**, and on the receiver-first order `x1` was the sender's second word, a number the
/// sender chose where a `CALL` server reads a Reply slot. Found by milestone 613 (a system log
/// service: the in-memory half)'s audit of `RECEIVE` consumers, 2026-10-03 UTC; **fixed the same
/// day by milestone 634 (a plain SEND received by `RECEIVE_CAP` never hands the receiver a
/// sender-chosen slot)**: the receive side returns [`NO_CAP`] in `x1` unless a capability was
/// installed for this delivery, on both orders. (This entry said "not fixed" until the
/// 2026-10-03 security audit found the fix had landed without the record moving.)
///
/// **A §26 death message read here loses the faulting `pc`.** The kernel's five words are
/// `(event, tid, pc, addr, 0)`, and this method, having no capability to deliver, returns
/// `(event, NO_CAP, tid, addr, 0)`: `x1` is `NO_CAP`, `x2` is the dead thread's id, and `pc` is
/// not returned. The same on both arrival orders since milestone 634; before it, the
/// receiver-first order returned `tid` in `x1`. No supervisor in the tree receives deaths this
/// way; every one uses `RECEIVE` through `user_mode_runtime::receive_fault`. Recorded by the
/// 2026-10-03 security audit so the next supervisor written against `RECEIVE_CAP` learns it here.
///
/// **A real capability passes the `NO_CAP` guard, and a `CALL` server used to have no way to
/// tell it from a Reply.** Any client with `WRITE` on the endpoint and `GRANT` on a capability
/// of its own can [`SEND_CAP`] it here; `x1` is then a real slot, and a server that invokes
/// [`crate::reply::REPLY`] on it runs method `0` of whatever object it is: `SEND` on a
/// rendezvous the client never receives on parks the server for the life of the machine, and
/// every delivery that is not a Reply leaves its slot behind, since only a Reply is one-shot.
/// Found by the 2026-10-03 security audit's follow-up; **fixed by milestone 706, DECISIONS §245
/// (a `CALL` server tells a Reply from a delegation)**: the kernel writes [`REPLY_DELIVERED`]
/// in `x4` when, and only when, `x1` is a `CALL`'s Reply, and every `CALL` server in the tree
/// receives through `user_mode_runtime::receive_request`, whose typed Reply is the only thing
/// `user_mode_runtime::reply` accepts, and which deletes a delegation the server did not ask
/// for. **What is still open:** the check lives in the server, so a server that reads `x1` raw
/// (its own `invoke`, outside the runtime) is as exposed as before. The rung-one answer, an
/// endpoint that refuses `SEND_CAP` outright, is the milestone's recorded follow-on (option 2
/// in its block).
pub const RECEIVE_CAP: u64 = 3;

/// `invoke(cap, CALL, w0, w1, _)` -> r0, with r1 in x1 and in x2 the slot of a capability the
/// server's [`crate::reply::REPLY_CAPABILITY`] carried, or [`NO_CAP`]. **Send two words and block until
/// replied.** The atomic send-and-wait a server can answer safely: at the rendezvous the kernel
/// mints a one-shot [`crate::reply`] capability naming *this* caller and hands it to the server (through
/// [`RECEIVE_CAP`]), so the server can answer a caller it was never wired to, exactly once, and only
/// that caller. Needs `WRITE`. Milestone 12 (call/reply IPC); see notes/ipc-naming.md. A `CALL`
/// that reaches a plain [`RECEIVE`] returns [`Gone`](crate::Error::Gone): the receiver gets the
/// two words, and there is nowhere to put the Reply (§246, PROVISIONAL number).
pub const CALL: u64 = 4;

/// `invoke(cap, REAP, tid, _, _)` -> 0. **Collect a corpse this endpoint supervises**
/// (DECISIONS §32). `tid` is the thread id the kernel stamped on the death message
/// ([`fault`](super::fault)); the kernel reclaims that thread's TCB, its address space, and the
/// region behind them, exactly what [`memory_region::DESTROY`](super::memory_region::DESTROY) would have
/// reclaimed. Needs `READ` on this endpoint: the authority to collect is the authority to
/// *receive* deaths here, which is what a supervisor holds.
///
/// **Authorization is the supervision relationship, not a rights bit and not a registry.** The
/// kernel checks that the named thread's recorded fault endpoint *is this endpoint*, so a tid
/// means something only relative to the capability it arrived through. A supervisor therefore
/// needs no capability to the child's memory, and gains none: the reclaimed region returns to
/// its owner under §13 region ownership, which is the **builder**. A supervisor can free a
/// child's memory; it cannot spend it.
///
/// Three distinct refusals, because a restart policy wants to tell them apart:
///
/// - [`crate::Error::StillAlive`] the thread is not dead. Collecting a corpse is not killing; killing a
///   live child is the stronger act and stays with `MemoryRegion::DESTROY` (§24's forcible `^C`).
/// - [`crate::Error::NotSupervised`] no thread by that tid supervised by *this* endpoint: another
///   supervisor's child, an already-collected one, or a stale tid whose generational name no
///   longer resolves. The two are deliberately one error, so a supervisor cannot probe the tid
///   space of children it does not supervise.
/// - [`crate::Error::NotPermitted`] the corpse's region cannot be reclaimed yet (the child `SPLIT` its
///   own budget and those children are still live), the same refusal `DESTROY` gives.
pub const REAP: u64 = 5;

/// `invoke(cap, SURVEY, cursor, record, 0)` -> `(next_cursor, tid, word)`. **Read one entry of
/// the domain this endpoint supervises** (milestone 126 (who else is running, and who is allowed to ask)).
/// `next_cursor` returns in x0, `tid` in
/// x1, and the selected record's own word in x2.
///
/// **`record` is a selector, and that is the shape rather than an implementation detail.**
/// calef ruled on 2026-09-21 that observing another thread is a selector: a new per-thread fact
/// becomes a new [`survey::record`](super::survey::record) value, never a new return register.
/// The reason was a forecast rather than an argument. A register row holds a fourth word with
/// no new mechanism and appending one is the fewest moving parts *if the row stops*; he expects
/// a third and a fourth fact, and a mechanism that must be redesigned at the sixth field is the
/// wrong mechanism at the fourth. Nobody grows a register row: Linux's
/// `/proc/[pid]/task/[tid]/stat` is 52 fields behind a pseudo-file, and Zircon's
/// `zx_object_get_info` is a selector over 41 topics.
///
/// **x0 and x1 are the frame; only x2 belongs to the record.** The cursor walk and the tid are
/// identical whichever record is asked for, so a caller that wants two facts about one domain
/// walks it twice and joins on the tid, and a caller that wants one is unaffected by the
/// existence of the others. That is what makes a new record cost an existing reader nothing.
///
/// **[`survey::record::STATE`](super::survey::record::STATE) is 0, which is not a coincidence
/// and is the backward-compatibility claim stated out loud.** Every caller that shipped before
/// the selector existed passed 0 in x1 because the argument was unused, and 0 selects the
/// record those callers were already reading. The claim is that no existing caller changes, and
/// it is a claim about a wire rather than a hope: it holds precisely because the numbering was
/// chosen to make it hold.
///
/// **An unknown record is [`crate::Error::BadMethod`], refused before the walk begins.** The
/// selector is part of the method's name, so an unrecognised one gets the same refusal an
/// unrecognised method word gets. It is refused *before* the domain is examined so that a bad
/// selector against an empty domain is a refusal rather than a
/// [`survey::DONE`](super::survey::DONE) that reads as "nothing here": a plausible wrong answer
/// is worse than an error, which is the ruling this tree already made about a counter it could
/// not trust. `BadMethod` from a `SURVEY` names the selector without ambiguity, because `SURVEY`
/// itself is a known method and the selector is the only other word this arm dispatches on.
///
/// **The scope is the supervision domain, because the kernel already maintains it.** A thread
/// is in this survey exactly when its recorded fault endpoint *is* this endpoint, which is the
/// same relationship [`REAP`] is authorized by and needs no second bookkeeping. So a `ps`
/// launched from a shell sees the shell's children and nothing else, and an operator's `ps`
/// sees the whole machine only because somebody handed it the endpoint that supervises the
/// whole machine. Authority is a domain, not a global, and the difference between the two is
/// a capability a reader can point at.
///
/// **Needs [`rights::ENUMERATE`](super::rights::ENUMERATE), and pointedly not `READ`.** This
/// is the decision that makes the method safe rather than merely scoped. `READ` on a
/// supervision endpoint is what [`RECEIVE`] and [`REAP`] take, so a viewer holding `READ` could
/// reap a child, and **a domain names its members, it does not act on them** (calef,
/// 2026-08-17). With a right of its own, a `ps` cannot express a reap rather than being
/// refused one, and the difference is the ladder's top rung against its middle.
///
/// A holder without it (a send-only peer that reports to this supervisor, or a supervisor's own
/// `READ` handle that was never widened) is refused with [`crate::Error::NotPermitted`],
/// **loudly**. It is not shown an empty domain, because a monitor that reports nothing when it
/// could not look is the worst failure this tool has available; `filesystem_protocol` chose `EPERM` over an
/// empty listing for the same reason.
///
/// **The cursor is a resume point, not an index into a result.** Start at 0. Each entry returns
/// the `next_cursor` to pass to get the one after it, and `next_cursor` of
/// [`survey::DONE`](super::survey::DONE) means the survey is finished (`tid` and the record
/// word are then 0). An empty domain finishes on the first call, which is a different answer
/// from the refusal above and deliberately so.
///
/// **It is a snapshot per call, not per survey.** The domain may change between calls: a child
/// that dies is simply absent from a later one, and one born into an already-passed slot is
/// missed until the next survey. That is `readdir`'s bargain, taken knowingly, because holding
/// `IPC_TABLES` across a whole survey would put a userspace program in charge of how long the
/// scheduler is locked.
pub const SURVEY: u64 = 6;

/// `invoke(cap, BADGE, badge, _, _)` -> the slot the badged copy landed in, or a negative
/// [`crate::Error`]. The slot comes back in x0, the way `memory_region::RETYPE_OBJ` answers.
///
/// **Mint a badged copy of this endpoint capability** (milestone 599 (a frame per filesystem
/// client channel), provisional name and number, kept free by DECISIONS §148 (a supervisor restarts by asking)). The new
/// capability names the same endpoint with the same rights, plus `badge` stamped on it; the
/// kernel delivers that `badge` to a server's [`RECEIVE_CAP`] whenever this copy's holder `CALL`s
/// or `SEND_CAP`s here, so a server serving many clients on one endpoint can tell them apart.
///
/// Needs `GRANT` (minting a delegatable view is a delegation-class power) and refuses a `badge`
/// of 0 (that is the unbadged value) or a source that is already badged (seL4's rule: a badge is
/// set once and never changed, which is what lets a holder trust the badge it was given). The
/// caller then hands the badged copy to a client with `SEND_CAP`, narrowing rights as usual; the
/// badge rides through that delegation because `SEND_CAP` copies the object.
pub const BADGE: u64 = 7;

/// The x1 value from [`RECEIVE_CAP`] when the message carried no capability.
pub const NO_CAP: u64 = u64::MAX;

/// **The tag on a receive that delivered a `CALL`'s Reply** (milestone 706 (a `CALL` server
/// can tell a Reply from a delegation), DECISIONS §245; name provisional).
///
/// [`RECEIVE_CAP`] returns this in `x4` exactly when the kernel installed the one-shot
/// [`crate::reply`] capability it minted for a [`CALL`], and `x1` is the slot it landed in.
/// `x4` is written only by the kernel, so no sender can forge it: a [`SEND_CAP`] delegation, a
/// plain [`SEND`], an interrupt signal and a §26 death message all leave it `0`, and a bound
/// notification leaves [`notification::BOUND`](crate::notification::BOUND). A `CALL` whose
/// Reply could not be installed (the receiver's table was full) arrives with `x1 ==`
/// [`NO_CAP`] and `x4 == 0`, because there is no Reply to name.
///
/// **Test this, never `x1 != NO_CAP`, before answering.** A delegation passes the `NO_CAP`
/// guard; only this says the slot holds a Reply. `user_mode_runtime::receive_request` reads it and
/// returns a typed Reply, which is the only thing `user_mode_runtime::reply` accepts.
///
/// Ruled by calef on 2026-10-03 (UTC): "Option 1 with the typed runtime helper."
///
/// Name: ratified 2026-10-03 (calef, reviewing milestone 706 (a `CALL` server can tell a Reply from a
/// delegation): "Ratify `REPLY_DELIVERED`.").
pub const REPLY_DELIVERED: u64 = 1;

// `x4` carries both tags, so they must differ from each other and from the untagged `0`.
const _: () = assert!(
    REPLY_DELIVERED != 0
        && REPLY_DELIVERED != super::notification::BOUND
        && super::notification::BOUND != 0
);
