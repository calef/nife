# Milestone 139 appendix: round 8, the kernel

An appendix to [milestone 139 (drive the unsafe count down, and cinch the ratchet behind it)](../0139-drive-down-unsafe.md),
moved out of that page on 2026-10-13 (UTC) under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md).
The main page is complete without it. This file holds round 8, the first round to work the kernel
outside `arch/`. Long sentences were split as they moved; no figure, date, name or quotation was
dropped.

## Round 8 (2026-09-01): the kernel, which seven rounds had never touched, categorized and two clusters collapsed

Rounds 1 through 7 worked `user/` and `crates/`. None of them went near `kernel/src`. By 2026-09-01
that had become the largest unworked pool in the tree: 242 blocks outside `kernel/src/arch/`. Compare
`user/`'s 162 after round 7, `crates/user_rt`'s 64, and `crates/inter_process_communication`'s 44
(settled in round 2). This block's own opening measurement recorded the kernel at 203 on 2026-08-18.
It had grown to 242 while six rounds went through userspace. It is also the part DECISIONS §14 (the
project's direction: a verified-Rust capability microkernel) calls verified, which is what this
milestone's own "Why it matters" says the number is measuring.

Round 8 was possible in a way earlier rounds were not. [Milestone 193 (put `kernel/src` within reach
of the prover)](../0193-kernel-kani-reachable.md) landed the day before, so `cargo kani`
can compile the kernel now. This milestone's criterion 2 ("a typed abstraction whose invariant the
compiler or Kani holds") is therefore available inside the kernel for the first time. In the event
neither collapse below needed the prover, because both turned out to be criterion 1. But the
categorization below says where the prover is the tool for what remains.

### The per-category table for the kernel's 242, so nobody re-derives it

Categorized by what the first token inside each block is. It uses the same stripping-and-counting
regex `script/lint`'s census uses, so these add up to its number rather than a grep's.

| shape | blocks | verdict |
|---|---|---|
| `core::ptr::write_bytes` | 41 | the §94 shape; 37 collapsed this round (see below). The 4 left have different provenance: `kmem.rs`'s pool recycle, `memory_region.rs`'s two retype paths, and one partial-page write in `user.rs` |
| `core::ptr::read_volatile` | 34 | mixed. MMIO (`ns16550.rs`, `plic.rs`, `pci.rs`, `non_volatile_memory_express.rs`, `virtio.rs`) is not a target; the rest read a shared frame the kernel just handed a process, in test fixtures that each read a different field |
| `core::ptr::write_volatile` | 23 | same split as the row above, same verdict |
| `&`-first (a reference built from a raw pointer) | 25 | mostly `sched.rs` (10) and the graphical test fixtures; each names a different object at a different lifetime |
| `(`-first (a call through a raw pointer or a cast) | 16 | 13 of them `sched.rs`'s thread-control-block pointer arithmetic; see the design fork below |
| `let`-first | 13 | multi-statement blocks, each doing something particular |
| `core::slice::from_raw_parts[_mut]` | 11 | whole-page and whole-region slices, one per distinct region |
| `log_page(...)` | 6 | a real §94 shape, not taken this round; see the proposed follow-on |
| `q.push_back` / `inbox.push_back` | 8 | `sched.rs`'s run-queue handoff; a design fork, see below |
| `mmu::activate_user` | 6 | test fixtures in `user/tests.rs`, which another lane holds this session |
| `dtb::Dtb::from_ptr` | 5 | the §94 shape; all 5 collapsed this round (see below) |
| `crate::stack::paint` / `high_water` | 9 | three stacks, three different facts each; the `crates/inter_process_communication` shape, no collapse available |
| `drivers::plic::init` | 3 | MMIO init, one per boot path |
| everything else | 42 | one-offs: `force_unlock`, `ManuallyDrop::drop`, `from_utf8_unchecked`, `assume_init`, the fastpath pad, `Thread::spawn_into` |

### What collapsed

Page zeroing: thirty-seven sites, one declaration. Every service, driver and test fixture that
allocated a frame went on to zero it by hand. Each did so with its own `// SAFETY:` comment over
`core::ptr::write_bytes`, asserting the same two facts. A frame just handed back by `memory::alloc`
or `memory::alloc_contiguous` is exclusively the caller's, and the direct map reaches it. That is not
thirty-seven facts. It is one fact about what the allocator returns, and the allocator is the only
thing in the tree that can actually check it.

Two of the copies had already said out loud that they were copies, without anyone lifting it out.
`user/rmle_service.rs` said *"`fs_service`'s own `page_frame` does the same thing... this is a second
copy of three lines"*, and `user/session_reviver_service.rs` said *"matches `fs_service::frame`'s own
shape"*. That is the same tell `ntp.rs` carried for [round 1's cluster](rounds-1-and-2.md) and
`timetable.rs` for [round 6's](round-6.md).

`crate::memory::alloc_zeroed` and `crate::memory::alloc_contiguous_zeroed` (both new; ratified by
calef 2026-09-01) hold it once. Every migrated call site is ordinary safe code afterwards. A caller
cannot violate a Rust invariant by asking the allocator for a zeroed frame, whatever it then does
with it. That is what makes this a collapse rather than the relocation this block refuses.

The four sites left behind are named in the table above. They are genuinely different provenance
rather than an oversight. `kmem.rs` zeroes a page recycled through its own pool, so the ownership
half of the assertion is the pool's, not the allocator's. `memory_region.rs` zeroes a page carved out
of a memory region (likewise). And `user.rs`'s timebase arm writes `PAGE_BYTES` into a frame it
already holds rather than a whole fresh one.

The device tree: five sites, one declaration. `memory.rs`, `console.rs`, `pci.rs` and `smp.rs`
(twice) each took the boot pointer and handed it to `dtb::Dtb::from_ptr`. Each did so under a
hand-written comment rewording the same two facts. It is the pointer firmware put in `x0`/`a1`,
which `kernel_main` stashes in `crate::DTB` as its first statement before any of these can run. And
it is physical, so the direct map names it.

`crate::device_tree` (new; ratified by calef 2026-09-01) holds that once, in the module the static
lives in. It returns `Result` rather than panicking, because two callers legitimately continue
without a tree. The early RISC-V console keeps its defaults. And `x86_64` stores a PVH
`hvm_start_info` pointer in `crate::DTB` rather than an FDT, so `from_ptr`'s magic check is what
tells those apart from a real failure (`smp.rs`'s own fixture already documents that hazard). Three
functions (`memory::init`, `smp::read_cpu_list`, `console::configure_from_dtb`) lost a `dtb_ptr`
parameter. It was always `crate::DTB`'s value at every one of their call sites, so this is the same
one-source-of-truth gain one level out.

Measured from the diff against this round's own base commit (`8fc30efb`): 42 `unsafe {` blocks
removed, 2 added, net -40. `kernel/src` outside `arch/` goes 242 to 202; tree-wide outside `arch/`,
738 to 698. Density 83 to 78 (78.8 exactly). The tree-wide census confirms the diff exactly, nothing
else having landed on this branch in between. That is the same discipline every round since round 2
has used.

The ratchet, cinched a sixth time: `<!--count-at-most:unsafe-density-outside-arch-->` lowered from
94 to 88 (`notes/unsafe-obligations.md`, `notes/counted-claims.md`), ratified by calef 2026-09-01.
That leaves ten points of headroom over the 78 this round reached.

An earlier draft called that a departure from a "seven-point convention". There is no such
convention: the headroom on record runs from six points (round 1) to seventeen (round 7). What every
round holds to is this block's own rule. The ceiling falls when a real reduction lands, and the
headroom is argued beside the marker. So this is that argument.

It rests on a measurement none of the earlier rounds had. Round 7 reached 77 on 2026-08-26, and this
round found 83 on 2026-09-01: six points of unrelated growth in six days, the steepest stretch on
record. A ceiling seven points over the current density is therefore about one week of ordinary lane
traffic before it fires on honest work. That is the "only ever rejects legitimate work" signature
that BUGS item 1, on the parent page, exists to keep this milestone from producing. Ten points is
roughly ten days at the observed rate.

And the gain is more than kept, stated exactly because this is a measurement. The density fell five
points (83 to 78, truncated) and the ceiling fell six (94 to 88), one point further cinched than was
gained.

### What this round deliberately did not take, and why, so nobody re-derives it

- `kernel/src/arch/` (248 blocks, up from 141 on 2026-08-23). Not a target, per this block's own
  text. The growth is [milestone 161 (the x86_64 kernel port)](../0161-x86-64-kernel-port.md)'s
  `x86_64` port, which is a third architecture's worth of assembly and system registers. That is
  exactly the population the measurement excludes on purpose.
- MMIO and device-register access (`ns16550.rs`, `plic.rs`, `gic.rs`, `pl011.rs`, `pci.rs`,
  `non_volatile_memory_express.rs`, `virtio.rs`). This is the same finding [round 3](round-3.md)
  and [round 5](rounds-4-and-5.md) reached for the userspace drivers, and for the same reasons.
  Each block names a different device at a different offset table. The one file that could carry a
  compile-time layout already does: `pl011.rs` uses `tock_registers::register_structs!`, and
  `ns16550.rs`'s own module doc explains why it cannot (the runtime-variable register stride). No
  §94 shape.
- The three stack helpers (`crate::stack::paint`, `high_water`, 9 blocks across
  `interrupt_stack.rs`, `smp.rs`, `thread.rs`, `stack.rs`). The comments rhyme and the facts do not.
  An interrupt-stack slot, a not-yet-handed-out `KernelStack`, and a secondary core's boot stack are
  three different ownership arguments. This is the `crates/inter_process_communication` shape from
  round 2.
- `kernel/src/user/tests.rs` (14 blocks) and the `mmu::activate_user` fixtures. Left alone for a
  scheduling reason rather than a technical one. AGENTS.md names that file the tree's merge hotspot,
  and another lane held it this session. A later round should read it; nothing here says it is
  irreducible.
- The prover. Milestone 193's harnesses reach `kernel/src/syscall.rs`; `arch/`, `asm!` and MMIO are
  still out of reach, and a stub is a hole in a proof. So criterion 2's Kani half was not available
  for either of the clusters this round found. It is the right tool for the `sched.rs` fork below,
  which is the reason that fork is worth raising rather than guessing at.

### Two things identified and not done, in the two shapes this project accepts

A proposed milestone (provisional number, the integrator mints it at merge): give the revocation
log's page chain a safe iterator. `kernel/src/revoke.rs` walks its per-space chain of log pages in
six places. Every one of them is `let mut page_phys = space.head; while page_phys != 0 { let page =
unsafe { log_page(page_phys) }; ... }`. Every one asserts the identical fact that `log_page`'s own
`# Safety` already states: the page is one this module linked into a chain, and `SPACES` is held.
Six hand-written copies of one invariant, the §94 (what may live in a userspace library) shape
exactly.

The collapse is not the trivial one, which is why it wants a lane rather than a paragraph here.
Making the call sites safe means the *chain itself* has to carry the "`SPACES` is held" half, as an
iterator borrowing the lock guard and yielding `&mut LogPage`. And the six loops are not uniform: two
mutate entries in place, one carries an index cursor across calls, and two break early. Wrapping
`log_page` in another function would move the assertion and not collapse it, which this block
refuses. It is worth roughly 6 blocks. More to the point, it is worth removing the one place in the
kernel where a future edit could walk a chain without the lock and nothing but a comment would
notice. ([Round 9](rounds-9-to-11.md) built it.)

A recorded limitation, in this block's own BUGS section on the parent page rather than invented as
scope: `sched.rs` is 47 of the kernel's 202 and the largest single share, and it is a design fork
rather than a migration. Its blocks are the run-queue handoff (`cpu::current().with_runq(|q| unsafe
{ q.push_back(ptr) })`, 6 sites, plus 2 inbox pushes). Underneath it is the raw
thread-control-block pointer arithmetic (13 `(`-first sites). Every one of the eight queue pushes
asserts the same sentence: *this thread is live, Ready, and on no other queue.*

That reads like the §94 shape and it is not. Unlike `alloc_zeroed`'s invariant, this one is not a
fact the callee can establish. It is a fact the caller established two lines earlier by
transitioning the thread. A safe `enqueue_ready(ptr)` would carry the identical argument in a
different place, which is this block's named anti-pattern. The reduction that would be real is rung
one of AGENTS.md's ladder: make the wrong state unrepresentable. A thread pointer could then reach
`push_back` only by way of a type that only a Ready-transition can mint.

That is a scheduler-core typestate change. It touches the one subsystem where a mistake is an
intermittent hang rather than a compile error. And it is exactly the kind of thing milestone 193's
prover should be pointed at first. An architect's call, not a lane's to invent. (calef ruled it on
2026-10-08 UTC as option C, and [round 10](rounds-9-to-11.md) built it.)
