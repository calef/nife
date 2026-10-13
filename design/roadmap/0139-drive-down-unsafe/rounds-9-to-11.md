# Milestone 139 appendix: rounds 9 to 11

An appendix to [milestone 139 (drive the unsafe count down, and cinch the ratchet behind it)](../0139-drive-down-unsafe.md),
moved out of that page on 2026-10-13 (UTC) under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md).
The main page is complete without it. This file holds rounds 9 to 11 and the 2026-10-12 ceiling
cinch; the rulings they answer are on the main page under "What is still open". Long sentences were
split as they moved; no figure, date, name or quotation was dropped.

## Round 9 (2026-10-07): the identified work, finished or handed on

calef ruled on 2026-10-07 (UTC): address the remaining identified work, then call the block BUILT.
This round did the code items, settled the ratchet's scope, and wrote the three questions only calef
can answer as rulings to make (see "What is still open" on the [main page](../0139-drive-down-unsafe.md)).

Measured from the diff against base commit `3dfbf1fd4`: 25 `unsafe {` blocks removed, 2 added,
net -23. Outside `kernel/src/arch/` the census went 1,147 to 1,124 blocks, density 66.6 to 65.3.
One `unsafe fn` went too (`revoke::log_page`). Proven by `script/test` on aarch64, riscv64 and
x86_64.

### What collapsed

- The revocation log's page walk, the proposed milestone [round 8](round-8.md) named. Nine call sites restated
  `log_page`'s contract: a page this module linked, with `SPACES` held. Round 8 counted six; the
  tree had grown three more. `revoke::chain` (new; names provisional) now holds both halves without
  a comment. A walk borrows `&mut LogChain`, which lives only inside the locked registry, so the
  lock half is a borrow. `head`, a page's `next` and the walk's cursor are private to that small
  module, and `LogChain::grow` is their only writer, so the membership half is privacy. One block
  remains, in the iterator. Ten blocks and the `unsafe fn` became one block. `list_mapping`'s
  cursor check is now the same walk as the listing. The defect found by milestone 779 (fuzz the
  surface a confined process can reach), a caller's word followed as a log page, can no longer be
  written outside that module. The two
  falsification records over this code were re-cut to the new shape.
- `AddressSpace::while_installed` (new; name provisional). Five system tests and the RISC-V boot
  tour each installed a space by hand under their own wording of `mmu::activate_user`'s contract.
  It is a fact about `AddressSpace`, so it is asserted there once. It takes a closure rather than
  returning a guard, because `mem::forget` is safe and a forgotten guard could leave a dropped
  space installed. Six blocks became one.
- `code_page` in `system_tests::user` (new; name provisional). Nine tests in five files wrote a
  child's code stub into a fresh page with the same raw-pointer loop. None checked that the stub
  fit. One block now, and the copy is a bounds-checked slice copy.
- `system_tests::user::tests`'s shared-memory test kept its capture in a `static mut` buffer. It
  is now an array of atomics, with `CAPTURED`'s store as the handoff: unsafe that was never needed.

The old `kernel/src/user/tests.rs` is `system_tests/src/user/tests.rs` since milestone 609 (the
system tests leave the kernel crate). It carried 15 blocks; it carries 7. The seven left are one-offs
with their own arguments. Two read through a live user translation, which is the point of their
tests. One installs a space on another core from a `'static` closure, which a borrow cannot
express. One builds a `Mapper` over a live root, and one writes a fresh frame through the direct
map.

### The ratchet

Cinched in the same commit, 88 to 72 (`notes/unsafe-obligations.md`, `notes/counted-claims.md`).
The ceiling had stood 22 points over the tree; 72 keeps round 1's seven.

The opening census counted three things and the gate watched one and a half: blocks, and the
`Send`/`Sync` half of `unsafe impl`. Measured at five dates from git blobs before deciding:

| Date | `unsafe fn` | per 10,000 lines | `unsafe impl` | of which `Send`/`Sync` |
|---|---|---|---|---|
| 2026-08-18 | 48 | 6.0 | 24 | 17 |
| 2026-09-01 | 91 | 10.1 | 32 | 23 |
| 2026-09-15 | 95 | 10.1 | 32 | 23 |
| 2026-09-25 | 129 | 11.2 | 32 | 23 |
| 2026-10-07 | 152 | 8.8 | 34 | 23 |

The other `unsafe impl`s (11 today) are now gated at the tree's exact value, as
`unsafe-trait-claims` (provisional), the shape the `Send`/`Sync` claims already had. They moved
twice in seven weeks, and each is a hand-written claim that a trait's contract holds. `unsafe fn`
gets no ceiling. Every call of one is already a block the density counts, because
`unsafe_op_in_unsafe_fn` is on, and its own density rose and fell by half in seven weeks. A ceiling
would have fired on honest work in September. The `# Safety` contracts check already binds each one.

### Also done

`abi::thread_control_block::START`'s doc comment now says what the kernel does with its three
arguments: they are the child's first `x0` to `x2` (`a0` to `a2`, `rdi`/`rsi`/`rdx`). Round 7 found
the stale text.

## Round 10 (2026-10-08): the scheduler queue ownership token, ruling C

Ruling C's item 1 under "What remains", built in the ruling's order: the crates first on the
host, then `sched.rs` under the full suite and the icount gates.

`intrusive_fifo::Unqueued<T>` (name provisional, in calef's queue) is the token:
`#[repr(transparent)]` over `NonNull<T>`, so `Option<Unqueued<T>>` is still one word (a host
test pins that). It is not `Clone` or `Copy`. `Fifo::push_back` consumes it; `pop_front` is the
one place a queue hands it back. A node cannot reach a second queue without minting a second
token. Minting is the one `unsafe` step left in queueing, and it carries rule 2 (the node
outlives its time on a queue), which no type here can express; stated once at
`Unqueued::new`.

`inter_process_communication`'s `send`, `receive` and `Notification::wait` are safe now.
`remove_sender`, `remove_receiver` and `remove_waiter` return the victim's token instead of a
bool. The verdict enums carry the caller's token back on every verdict that did not queue it,
which created one obligation the old API did not have: the token handed back must be the
caller's own. The new harness `a_token_comes_back_exactly_when_its_node_did_not_queue` proves
that over every seeded state for both `send` and `receive`; its record's defect is a collect's
two tokens handed back transposed. The crate's production code carries no `unsafe` at
all: the six `unsafe fn`s round 9 counted went with the signatures that carried them.

`sched.rs` went 90 to 54 `unsafe {` blocks. The twenty-one sites round 9 counted (6 run-queue
pushes, 2 inbox pushes, 13 rendezvous operations) pass a token where they passed a pointer,
bracketed by `take_token` and `hold_token`, which absorbed the old `(*waiter.as_ptr()).id`
reads. `Threads::mint_token`, at table insertion, is the kernel's only minter. `ctx_switch`
measured about 5% fewer instructions (icount).
`script/fastpath-footprint`'s closures stayed inside their band once the seven inline `expect`s
became one cold `missing_token`. Proven by `script/test` on aarch64, riscv64 and x86_64.

The ratchet, cinched at merge: `<!--count-at-most:unsafe-density-outside-arch-->` lowered from
72 to 66 in the merge's integrator commit (`notes/unsafe-obligations.md`,
`notes/counted-claims.md`, `notes/register-of-measures/unsafe-series.md`), the count taken from
the merged tree: 1,029 blocks over 172,506 lines, density 59.7, 7 points of headroom.

### Where the running thread's token lives

The ruling's letter says "its core's current slot". It lives on `Thread::own_token`: a thread
holds its own token while it runs, the requeue takes it from there, and the choosing pop hands
it home before the switch. Two measured costs forced the deviation. The slot is written by the
core that pops and read by whichever core wakes the thread next, so the token crosses cores
inside it. That wants either a new `unsafe impl Sync` for the slot, a hand-written claim in the
one subsystem this milestone exists to make such claims rare in, or about 5% of the fastpath's
band. On the TCB the token follows the thread it names, `IPC_TABLES` already serializes every
access, and `hold_token`'s write lands in the TCB the switch touches anyway. Pending calef
(#1852): confirm the TCB over the slot, and ratify `Unqueued`.

### The falsification records

Eight `inter_process_communication` records and one `system_tests` record were re-cut to the
token API; §134 (a harness carries a machine-replayable falsification record) says a moved
record is redone, not trusted. The two `receive_cap_attack_tests` records that rot against
`sched.rs` were re-cut and replayed red by `script/falsifications --sweep system_tests`
(2026-10-08 UTC), each at its predicted assertion. The sweep's one finding was not
the token's. The syscall fuzzer's revocation-in-flight corpus seed (342) rotted on main before
this lane's cut, verified by hand at the claim commit with the defect applied. A 5,000-seed
sweep under the record's patch re-found the defect at seed 559, green on the clean tree.
Milestone 752 (a seeded syscall driver with a shadow model)'s replay table still names 342; that
block is the integrator's to edit, and the fuzzer module's BUGS names it.

## 2026-10-12 (UTC): the ceiling at the tree's own value

Not a reduction. Four days of lane traffic took the density from 59.7 to 61.09 (1,103 blocks over
180,545 lines, base `0e72ca92a`) under a ceiling of 66, so the marker went 66 to 61, the truncated
value. That gives up round 1's headroom: `notes/unsafe-obligations/ceilings.md` says what it costs.
calef ruled on it the same day (UTC): "Keep 61." The zero-headroom trade is decided.

## Round 11 (2026-10-12): question 2's renames

Every rename question 2's table rules is applied in `user_mode_runtime`, `abi` and their callers,
each with a `Name: ratified` block at the definition: the thirteen crate-root wrappers,
`virtio::read_register` and its three siblings, and `initrd::initrd_bytes`, now
`initial_ramdisk::bytes(len)`. `design/naming.md` says the foreign spelling stops at the boundary.

Left alone as outside the ruling: same-named private helpers (swish's `retype_page_frame()` and
one-argument `send_cap`, `hello.rs`'s `retype_page_frame`, `system_initializer`'s alias of
`supervision_protocol::retype_page_frame_from`, `system_tests`' two `map_into`s), the kernel's
`syscall::irq_wait` and `irq_ack`, `retype_page_frame_run` and `virtio_ring_barrier`. The drivers
call the virtio wrappers by full path, because a bare `virtio::` would shadow the `virtio` crate
they also link.

How far the sweep reached past that module was asked on #1998, and calef ruled B on 2026-10-12
(UTC): "B for #1998." Everything the tree authors now says `initial_ramdisk`: the kernel, `xtask`,
`uefi_loader`, `cfg(initial_ramdisk)`, the derived constants and parameters, and the environment
variable. Only `linux,initrd-start`/`-end` and QEMU's `-initrd` keep the foreign spelling.

`abi::irq` became `abi::interrupt` and `abi::virtio::READ_REG`/`WRITE_REG` became
`READ_REGISTER`/`WRITE_REGISTER` once the `abi` split (#1990) landed.

