# Milestone 139 appendix: round 7, the `invoke` cluster

An appendix to [milestone 139 (drive the unsafe count down, and cinch the ratchet behind it)](../0139-drive-down-unsafe.md),
moved out of that page on 2026-10-13 (UTC) under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md).
The main page is complete without it. This file holds round 7, which read the raw `invoke` calls
method by method and wrapped them. Long sentences were split as they moved; no figure, date, name or
quotation was dropped.

## Round 7 (2026-08-26): the `invoke` cluster, read method by method and mostly resolved

[Round 6](round-6.md) read and counted the `invoke(cap, method, ...)` cluster: 123 of `user/`'s 284
blocks, 43%, its largest single share. It deliberately did not migrate it, and it named the open
question precisely: *"deciding which of the 18-plus obligations are real... versus vestigial... is a
design fork for calef."* This round did that reading, method by method, and it resolves almost the
whole cluster.

The re-read began by replicating round 6's own count. `user/` has 123 `invoke(...)` call sites,
confirmed against round 6's number exactly; the 124th grep hit is a doc comment on
`os_primitives_benchmarker.rs`, not code. The calls follow each file's own `use abi::... as ...`
aliases (`ut` for `memory_region`, `fr` for `page_frame`). Resolved through those, they name 22
distinct methods. That is four more than round 6's own floor of "at least 18", because round 6's
count did not fully resolve aliases. Grouped by method rather than by call site, the shape round 6
predicted is exactly what is there. Some methods have one or two call sites, and several have a
dozen or more.

The reading, method by method, and the answer to round 6's own question. Every one of the 22
methods carries the *identical* safety argument `invoke`'s own doc already states: *"the kernel
validates the capability and the method before acting; the caller is trusting the kernel, not the
other way around."* That argument does not vary by method. It is not about what the method *does*.
It is about what the syscall boundary *is*: a trap the kernel validates before touching anything,
on every one of the 22.

[Milestone 134 (the register of measures)](../0134-the-measurements-that-decide.md)'s census read
`abi::address_space::MAP_INTO` as carrying a "real" obligation because it "can perturb the caller's
own address space," which is true of the *result* of a successful call. But it is exactly as true of
`abi::page_frame::MAP`, which round 1 already wrapped as `map_page_frame`. That is a fully safe
function, with no `unsafe` at any of its seventeen call sites, and nobody treated it as unsound. The
two methods carry the same shape. The kernel checks rights and alignment, and either performs the
mapping or returns an error. Nothing about the *call itself* can violate a Rust invariant the wrapper
could have checked and did not.

What can go wrong after a successful call is a different matter: aliasing a page a Rust reference
already assumes is private, or racing a mapping change. That is a caller-side correctness question
every syscall in this cluster already has, `map_page_frame`'s included. It is the argument the raw
call site's own SAFETY comment already discharges onto "the caller is trusting the kernel." So round
6's "real vs. vestigial" question resolves this way. For Rust-safety purposes, all 22 are the
`send`/`receive`/`reap` shape, not the exception `MAP_INTO` was flagged as being.

The genuinely separate question, "should a supervisor be able to remap a child's memory out from
under it without the child's cooperation," is real. But it is a capability-policy question, and the
kernel's rights model already answers it (`WRITE` on the address-space capability). It is not a
Rust-safety gap a wrapper's absence was leaving open.

What that reading bought, mechanically. Three of the 22 methods already had a safe wrapper sitting
unused. Nine `abi::reply::REPLY` call sites were migrated onto [`reply`]. Three of them were local
one-line `fn reply(slot, r0)` re-wrappers, in `fs_file_caretaker.rs`, `fs_nameset_caretaker.rs` and
`fs_subtree_caretaker.rs`, each duplicating [`reply`]'s own body. One `abi::rendezvous::SEND` call
site was migrated onto [`send`]. Both wrappers already existed and needed no change. Seventeen
`abi::page_frame::MAP` call sites moved onto [`map_page_frame`], likewise pre-existing. That is 27
sites, zero new code.

One more method is the sharpest instance of the §94 (what may live in a userspace library, and what
must be per-binary) shape this milestone keeps finding. `date.rs` had a `granted(slot) -> bool`
probe. It invokes a method number no object type defines, so the call can only be refused, and it
reads *which* refusal came back to tell an empty slot from a real object. It was documented as
lifted from `clock_page`'s own probe. Four more programs (`pgrep`, `pmap`, `ps`, `watch`) had each
copied the identical function, byte-for-byte body and comment. One of them said so out loud
("`ps`'s and `date`'s probe, verbatim") without anyone lifting it out. [`granted`] is now one
declaration in `user_rt`; the five call sites are gone.

The remaining eighteen methods had no wrapper at all. Each got the thin, one-`unsafe`-block wrapper
whose shape round 1's `map_page_frame` already established:

- [`retype_page_frame`] (`memory_region::RETYPE`, 15 sites)
- [`retype_object`] (`RETYPE_OBJ`, 7)
- [`split_region`] (`SPLIT`, 3)
- [`destroy_region`] (`DESTROY`, 4)
- [`map_region_page`] (`MAP`, 2)
- [`revoke_frame`] (`page_frame::REVOKE`, 3)
- [`map_into`] (`address_space::MAP_INTO`, 9)
- [`tcb_cap_insert`], [`tcb_configure`], [`tcb_start`] (`thread_control_block::CAP_INSERT`/`CONFIGURE`/`START`, 2 each)
- [`irq_wait`], [`irq_ack`] (`irq::WAIT`/`ACK`, 7 and 6)
- [`send_cap`] (`rendezvous::SEND_CAP`, 10)

Four more (`virtio::READ_REG`/`WRITE_REG`/`SETUP_QUEUE`/`NOTIFY`, 18 sites across `display.rs`,
`entropy.rs`, `kbd.rs`, `net_transport.rs`) went into a new opt-in `user_rt::virtio` module rather
than the crate root. That is the same "scoped to the programs that actually touch this capability"
shape `mapped_window`/`initrd` already established, since only four of `user/`'s programs hold a
`Virtio` capability. Provisional names, all of them, per this milestone's own naming discipline:
none is ratified, and `calef`'s call on all fourteen (plus `granted`) is open.

One `tcb_start` correction was found while wrapping it. `abi::thread_control_block::START`'s own doc
comment reads `invoke(cap, START, _, _, _)`, implying the three arguments are ignored. But
`kernel/src/syscall.rs`'s `START` arm forwards all three to `sched::start_thread_control_block`
unconditionally. `builder.rs`'s own call relies on exactly that: `START`'s second argument is the
worker's input. The wrapper takes `(tcb_slot, x0, x1, x2)` to match what the kernel actually does.
The stale `abi` doc comment is a separate, smaller finding, left for whoever next touches that file.

Every one of the 122 migrated call sites lost its `unsafe {}` entirely, rather than moving it. The
wrapper functions are ordinary safe `fn`s. Each holds exactly one `unsafe { invoke(...) }` inside,
forwarding `invoke`'s own contract. So a caller with a valid capability slot number, of any value,
cannot violate a Rust invariant by calling one. That is what makes this a §94 collapse rather than a
relocation. Eighteen new declarations (plus `granted`, plus four `virtio` functions) replace 122
independently-worded `// SAFETY:` comments that all said the same thing.

The one call site left raw, and why it is not this shape. `window.rs`'s `ROLE_PROBE_INPUT` path calls
`invoke(INPUT, abi::rendezvous::RECEIVE, 0, 0, 0)` directly. It reads the raw negative `abi::Error`
that a `RECEIVE` against an empty capability slot returns. `user_rt::receive`'s own contract assumes
success and returns the three data words, discarding the syscall's own return code. So it cannot
serve this call site's actual purpose, proving the kernel refuses cleanly, without becoming a
second, differently-shaped `receive` written for one caller. That is not a reduction by this
milestone's own test ("the number of distinct invariants asserted by hand"). So it stays, with an
expanded `# Safety` comment recording why it is the one exception rather than an oversight.

The honest count. `user/` carries 162 `unsafe {` blocks (was 284). That uses the exact regex from
round 6, re-verified against `script/lint`'s own printed number before and after. Tree-wide, outside
`kernel/src/arch/`: 701 (was 805), density 77 per 10,000 (was 89). The ceiling is still unmoved at
94, so that is 17 points of headroom, the widest margin on record; round 6 had spent it down to 5.

`crates/user_rt` itself grew from 31 to 45 blocks (`lib.rs`) plus a new 4-block `virtio.rs`, which is
the 18 wrapper bodies. That growth is inside the same directory this round's own reduction counts
against. So the 104-block net (122 removed from `user/`, 18 added to `user_rt`) is the real
tree-wide number, not the 122 `user/`-local one. Verified compiling clean and `cargo clippy` clean on
all three target architectures (aarch64, riscv64, x86_64). The full `script/test` suite was green on
all three (295/298/175 passed respectively). The x86_64 skips are the recorded hardware-scope gaps
already in the suite, none of them touching this round's own migration.

What this round believes it resolved in round 6's own design fork. The "which of the 18-plus methods
carry a real per-call obligation" question is answered. For Rust-safety purposes, none of them do,
in the way `MAP_INTO` was flagged as possibly doing. The obligation `invoke`'s own contract already
discharges is the whole of it. It was already the same for the five methods round 1-6 had already
wrapped. The "where do the wrappers belong" question is answered by precedent already in the tree:
the crate root for anything more than one or two programs touch, and a scoped opt-in module
(`virtio`) for a capability only a handful of drivers hold. That is exactly `mapped_window`'s and
`initrd`'s own shape.

What is not resolved, and is worth naming rather than leaving implicit. This round did not build a
generic, checked `invoke` entry point that all 22 (now: 1 unwrapped) methods share. Each wrapper
above is its own thin, per-method function, not a shared primitive underneath them. That is
deliberate. A cross-cutting `invoke` abstraction is new shared infrastructure every future capability
call would depend on. Building one without a proposal first is exactly what this milestone's own
brief asked not to do.

Nothing observed while doing this reading suggests such a primitive would buy anything the fourteen
per-method wrappers do not already buy. There is no shared runtime check across the 22 methods
beyond `invoke`'s own trap. Compare `map_page_frame`'s bounds check, which IS method-specific and
already lives in that one wrapper. So this round is not raising it as an open question for calef. It
is recording that the question was considered, and the answer, on the evidence gathered, is "the
per-method shape is the right one and nothing here argues for more machinery."

Provisional, easy to revert. All fifteen new names (fourteen wrapper functions plus `granted`) and
the new `virtio` module are provisional, per this milestone's own naming discipline; `calef`'s call.
One naming tension is worth surfacing rather than deciding here. `user_rt::virtio` shares its last
path segment with the pre-existing `crates/virtio` crate (the block driver, unrelated). The two
never collide in code: nothing in `user/` imports both unqualified in one file. But a reader
grepping "virtio" meets two different things under that name.

`user_rt::virtio` reads correctly as "the wrapper module for `abi::virtio`'s methods," which is the
reason it was named that on first pass. A different name (`user_rt::virtio_cap`, or similar) would
remove the ambiguity entirely if calef would rather. Reverting any single wrapper (moving its call
sites back to raw `invoke`) costs nothing but that one function and its call sites. None of this
touches the syscall surface, a wire format, or anything else two programs must agree on.

The names above are the round's own. calef's rulings on them, and the renames that followed, are in
the parent page's "What is still open" and in [rounds 9 to 11](rounds-9-to-11.md).
