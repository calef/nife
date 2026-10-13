# Milestone 139 appendix: round 6

An appendix to [milestone 139 (drive the unsafe count down, and cinch the ratchet behind it)](../0139-drive-down-unsafe.md),
moved out of that page on 2026-10-13 (UTC) under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md).
The main page is complete without it. This file holds round 6: the survey of `user/` the block's
BUGS section asked for, two migrations, and the one round whose count went up. Long sentences were
split as they moved; no figure, date, name or quotation was dropped.

## Round 6 (2026-08-25): the `user/` survey this milestone's BUGS section asked the first lane for, finished, plus two more migrations and one honest count-regression

This milestone's own BUGS section, verbatim: *"`user/`'s 285 is unexplained... nobody has read
enough of it to say whether that number is raw shared-page handling, a missing safe wrapper, or
something else."* Five rounds had already done most of that reading as a side effect of chasing
specific clusters. This round did the rest. It read every remaining `unsafe` block in `user/`. It
confirmed the census against `script/lint`'s own regex rather than a hand grep. And it closed the
one named gap, `login_test_client.rs`'s `PAGE_VA` (named in this block's "What is still open" at
the time), plus five more files that reading found in the same shape.

The re-measured count. `user/` carries 284 `unsafe {` blocks today, replicating `script/lint`'s
exact stripping-and-counting regex, scoped to `user/*.rs`. The milestone's own opening baseline was
285 (2026-08-18), and a mid-milestone reading of this same section recorded 287. Roughly flat, and
that flatness is itself the finding. Five rounds removed real duplication: net -21, -16, -6, -3, -2
across rounds 1-5, most of it in `user/`. Unrelated tree growth in `user/` over the same five weeks
put almost exactly that much back. That growth was milestone 176 (the x86_64 discovery seam's wide
half: COM1's IRQ and a CMOS RTC)'s COM1 IRQ wiring, milestone 158 (build the eleven kernel object
and identifier renames)'s kernel-object renames, and ordinary feature work. The tree-wide density
ceiling is the number that shows the reduction rather than hiding it: 93.4 to 87. Meanwhile the raw
count outside `arch/` went from 799 to 792 (as of this round's start), net-lower despite five weeks
of concurrent development. `user/`'s own raw count is not the number to read; the density is.

The breakdown, by what the first non-comment token inside each block is:

| shape | blocks | what it is |
|---|---|---|
| `invoke(cap, method, ...)` | 123 | a raw capability invocation: the userspace syscall itself |
| `read_volatile`/`write_volatile` | 36 | device registers and shared-frame fields with no further collapse available (see below) |
| `core::arch::asm!` | 16 | entry stubs, the trap, and a handful of driver-specific instructions (`wfi`, `fence`) |
| `core::slice::from_raw_parts[_mut]` | 12 | whole-page slice construction; down from ~30 before this round's own migration |
| everything else | 97 | `MappedWindow`/`RegisterBlock`-family constructors (new, mostly this round: see below), the C ABI shim (`c_shim.rs`, `malloc`/`free`, already documented per milestone 82 (`unsafe_op_in_unsafe_fn`: the obligation moves inside the fn)'s survey), deliberate-fault test programs (`flaky.rs`, `outlaw.rs`, `hello.rs`'s `.bss`/`.data` probes), and single one-off writes (`memory_grant_depleter.rs`, `swapper.rs`) |

Two clusters migrated this round, on `MappedWindow`, the same primitive round 1 built.

`user_rt::initrd::initrd_bytes` (new, provisional name). Seven programs (`builder`, `c_confiner`,
`hello`, `login`, `root_supervisor`, `swapper`, `timetable`) each declared their own `const
INITRD_VA: u64 = 0x2000_0000`. Each also had its own `unsafe { core::slice::from_raw_parts(INITRD_VA,
initrd_len) }`. Each carried one hand-written `// SAFETY:` comment per file asserting the identical
invariant: "the kernel maps `initrd_len` bytes of the initrd, read-only, at this VA, before
`_start` runs". `timetable.rs`'s own comment had already named the duplication out loud ("the same
contract `components/src/builder.rs` is started under") without anyone lifting it out. That is the
same shape `ntp.rs`'s comment named for round 1's cluster. One `unsafe fn` in
`crates/user_rt/src/initrd.rs` now holds that assertion once. Measured from the diff: 7 `unsafe {`
blocks removed at the seven call sites, 7 added at the same sites (calling the shared function),
plus 1 added inside it, net +1. It is flat at the call sites, and still a real reduction by this
milestone's own criterion 1. Seven independently worded assertions of one invariant collapsed into
one declaration plus seven one-line "forwarded from `initrd_bytes`'s own contract" comments. That
is the same "flat count, real reduction" shape earlier rounds had in `smb_server.rs`, `swish.rs`'s
terminal pair, `display_terminal.rs`'s `paint` and `console.rs`'s PL011 half. Verified compiling
clean on all three target architectures (aarch64, riscv64, x86_64), since the seven call sites
split across both boot paths.

`MappedWindow::as_slice`/`as_mut_slice` (new methods on round 1's own type, provisional).
`login_test_client.rs`'s `PAGE_VA` was this milestone's one explicitly named gap ("what is still
open", in the block as it then stood). It was a `core::slice::from_raw_parts_mut` site. The
`read_volatile`/`write_volatile` grep that found the clusters of rounds 2-5 could not see it, being
a different pattern. Reading it in context surfaced five more files in the identical shape, none
previously investigated:

- `credentialer.rs` (5 sites: `PROV_VA` and `VERIFY_VA`, one shared page apiece, read in some
  functions and written in others depending on protocol phase);
- `credentialer_test_client.rs` (5, the same two pages from the client side);
- `identity_provisioner.rs` (4: `REQ_VA`, `PROV_VA`, `FS_VA`);
- `session_reviver.rs` (2: `FS_VA`; it already had a sibling `MappedWindow` for a different page
  from an earlier round);
- `smb_server.rs` (1: `CRED_VA`).

Eighteen call sites total. Each independently asserted "this page is mapped read/write here, shared
with exactly this peer," often in near-identical wording. `MappedWindow` gained two methods
returning the whole window as an ordinary (non-volatile) `&'static [u8]`/`&'static mut [u8]`. They
serve the "hand the page to a parser" call shape all eighteen sites have, distinct from round 1's
"read/write one typed field" shape. Six new window constants now hold the eighteen invariants as
six declarations: `PROV_WINDOW`, `VERIFY_WINDOW`, `PAGE_WINDOW` x2 in two files, `REQ_WINDOW`,
`FS_WINDOW`, `CRED_WINDOW`. `credentialer.rs`'s `wipe`/`publish` and
`credentialer_test_client.rs`'s `request`/`page_is_clean` changed from taking a raw `va: u64` to
taking a `MappedWindow`. So the type carries the "which page" fact through the call rather than a
bare integer.

The honest count, which this round is not going to round off. Unlike round 1's `r8`/`w8` cluster,
this migration does not reduce the raw block count, and it should not be reported as if it does.
Round 1's `read`/`write` are ordinary safe functions. The bounds check they perform
(`offset + size <= len`) is a real runtime assertion that lets the call site drop `unsafe`
entirely. `as_slice`/`as_mut_slice` have no equivalent check to add. The risk they carry is not "an
offset out of range". It is "something else touches this memory while the slice is alive," which
no bounds check catches and no cheap runtime check can verify. So both methods stayed `unsafe fn`.
Every one of the eighteen call sites still carries its own `unsafe {}` block (unchanged in count).
Centralizing the "which page" half of the invariant cost one new block per shared window (ten
constructors), plus two new blocks inside `MappedWindow` itself for the methods' own bodies.
Measured from the diff against this round's own base commit: 18 `unsafe {` blocks removed, 30
added, net +12. With `initrd_bytes`'s +1, this round is net +13 against a tree-wide
`outside_arch` count of 792 at its own start. It lands at 805: 89 per 10,000 lines, truncated,
against the still-unmoved ceiling of 94. That is 5 points of headroom, one point less than the
7-point cushion every prior round preserved. It was spent by unrelated growth and this round's own
choice, not by a ceiling that fired.

What this round believes it bought for that cost, and why it might be the wrong trade. Eighteen
independently worded assertions became six canonical declarations plus eighteen one-line
forwarding comments. A reader auditing "is `PROV_VA` really exclusive to the provisioner while a
request is in flight" now checks one comment instead of five worded slightly differently across two
files. That is a real reduction in *the number of distinct claims a reader has to independently
verify*. It is the milestone's own stated test ("the test is not the token count, it is the number
of distinct invariants asserted by hand"). But it is not a reduction in the number the ratchet
watches. A careful reader comparing this round to rounds 1-5 should notice that difference rather
than take "round 6" as more of the same shape. This is easy to revert. It touches exactly six
`user/` files plus two new methods on one type. None of it is wire format, syscall surface, or
anything else two programs must agree on. calef might rather the raw count stay the primary signal,
even at the cost of the six duplicate comments. If so, reverting this specific piece costs nothing
but the six files' worth of diff. That excludes `initrd_bytes`, which is unambiguously a reduction.

The one cluster this round did not touch, and the reason is a design fork rather than a reduction
this lane could invent. `invoke(cap, method, a0, a1, a2)` calls are 123 of `user/`'s 284 blocks,
by far the largest single share (43%). Every one carries the identical comment `invoke` itself
carries: *"the kernel validates the capability and the method before acting; the caller is
trusting the kernel, not the other way around."* Milestone 134 (the register of measures: every
number this kernel owes itself)'s own census already named this precisely: *"a per-method
obligation carried by a single all-methods signature... some methods (`aspace::MAP_INTO` among
them) can perturb the caller's own address space, so some obligation is real,"* and its own
conclusion was "neither is this milestone's work; the handoff in its lane report proposes it."

This round counted rather than proposed. At least 18 distinct methods are invoked directly this way
across `user/`, counted by literal `abi::module::CONST` at the call site. Calls that compute the
method dynamically are not counted, so 18 is a floor, not a ceiling. They are led by `RETYPE` (10
sites), `REPLY` (9), `page_frame::MAP` (8), `page_frame::REVOKE` (3), `memory_region::DESTROY` (3),
`irq::WAIT` (3), `address_space::MAP_INTO` (3), and nine more at one or two sites each.
`send`/`receive`/`reap`/`call` and the rest of `user_rt`'s existing safe surface already cover the
handful of methods common to nearly every program. What remains uncovered is long-tail and
program-specific. It is page-frame and address-space construction verbs, mostly used by the
handful of programs that build child processes (`hello`, `builder`, `login`, the caretakers). It is
also IRQ and virtio methods, used only by the drivers that own those devices.

Why this is an architect's call and not a migration to invent. Building a safe wrapper per method,
the way `send`/`reap` already exist, would need a decision this lane has no standing to make. First:
whether the per-method obligation is real (as `MAP_INTO`'s is, per milestone 134's own reading) or
vestigial (as most of `send`/`receive`/`reap`'s turned out to be), for each of at least 18 methods.
Second: whether the wrappers belong in `user_rt` or in a smaller per-purpose crate. `user_rt` is
available to every program, and its surface would grow by a wrapper per verb. A per-purpose crate
would be a construction-verbs module used only by the handful of programs that build children.
Getting this wrong in either direction costs more than the code. Too permissive, and a genuinely
dangerous method (one that perturbs the caller's own address space) reads as safe. Too
conservative, and the exercise reduces to renaming 123 identical comments without moving the count.
That is the exact "relocates unsafe... hides it behind a [wrapper]" anti-pattern this block's own
text refuses. Left as a named follow-on, not attempted here. The next lane's job is not "wrap
`invoke`". It is "decide, method by method, which of the 18-plus obligations are real, the same
reading milestone 112 (SAFETY comments) already did for the four SAFETY comments that
discharged onto nobody". Only then does a mechanical wrapping pass become safe to
write.

A realistic floor for `user/`, which this milestone's own BUGS section asked the first lane to
report rather than pick a target here. The `invoke` cluster is the whole question: it is 123 of 284
blocks. The achievable reduction ranges from near zero to on the order of 100. Near zero holds if
most of the 18-plus methods turn out to carry the real, per-call obligation `MAP_INTO` does. On the
order of 100 holds if most turn out to be the same non-obligation `send`/`receive`/`reap` already
were. Setting the `invoke` cluster aside, the rest of `user/` (roughly 161 blocks) is close to its
practical floor already. Six rounds read it. What is left is `asm!` entry stubs, deliberately
unmigrated device registers ([round 5](rounds-4-and-5.md) names them), deliberate-fault test
programs and one-off writes. So: no single number, but a bounded one, roughly 160 to 260, narrowed
only by the method-by-method reading above. [Round 7](round-7.md) did that reading; the bound
closed at 162.

The ceiling's own open question, answered as far as it can be from five weeks of data. It has not
fired. The density moved from 93.4 through 90.8, 89, 88, 87, 87, and back to 89 after this round's
own count-regression. It was always 5 to 7 points under the ceiling; the full series is in
`notes/unsafe-obligations.md`'s table. This round's own +13 is the first commit in the milestone's
history to spend headroom rather than widen it, which is worth calef seeing plainly.
