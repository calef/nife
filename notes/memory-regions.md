# Memory regions: the kernel stops allocating

*Renamed 2026-08-24 (DECISIONS §113, milestone 158): the kernel object this file describes was
called `Untyped` (seL4's own word for it) until this rename. It is now `MemoryRegion`, chosen
because it names what the capability actually is (a run of physical memory) rather than seL4's
own word for what has not yet been done to it. This file is rewritten throughout to use the
current name; the historical number in "The one number that proves it" is untouched because it is
a measurement, not a name.*

Milestone 11, and DECISIONS §10's deliberately-deferred third axis. It is the strangest idea in
the whole project, and the one that makes seL4 verifiable: **a kernel that does not own a pool of
memory to hand out.**

## The idea

Normally a kernel has an allocator. Ask it for a page and it finds a free one and gives it to you.
Which means a process, by asking enough, can make the kernel allocate until it runs out, and a
kernel out of memory is a dead machine. Kernel-memory exhaustion is a whole class of attack:
fork bombs, fd floods, socket buffers, every one of them is "make the kernel allocate on my
behalf until it can't."

Memory regions remove the allocator from the hot path. A process holds a capability to a chunk of
raw physical memory (a **memory region**), and to get a page it retypes part of that memory
into the page. The kernel is a bookkeeper: it advances a watermark and hands back a physical
address. It does not choose a page from a pool it owns, because it owns no pool. Every page a
process spends comes from the region it was handed.

## The one number that proves it

```
  milestone 11: a process mapped 23 pages out of a memory region it was handed,
  and the kernel's used-frame count went 991 -> 991 (it did not move).
```

That flat frame count is the whole thing. The process allocated twenty-three pages of memory, and
the kernel's free memory did not change, because the pages came out of the process's own
region, carved once at the start. A process cannot make the kernel allocate, so it cannot exhaust
kernel memory. It runs out of *its own* budget, the retype returns `OutOfMemory`, and the kernel is
untouched. There is a test that asserts exactly this equality, and it fails loudly if the memory
comes from anywhere but the region (with the kernel allocator as the source, the same process maps
thirty thousand pages and the kernel loses thirty thousand frames).

## How it works here

- `Object::MemoryRegion(region)` is a capability to a region: a run of physical pages and a bump
  watermark. `kernel/src/memory_region.rs` is the whole allocator: `retype_page` advances the
  watermark and returns the next page, zeroed, or `None` when the region is spent.
- `invoke(region, MAP, va)` retypes one page and maps it, writable, at `va` in the caller's own
  address space. Both the page and any page tables it needs come from the region (the mapper's
  only source of memory is a closure that bumps the watermark), so the kernel allocates nothing.
- The region's backing is carved from the frame allocator once, when the region is created.
  That single allocation is the seL4 boundary, where all free RAM becomes a region handed to the
  first process. Everything after spends it.

## What a retype mints

Since milestone 824 (a retype mints no right its budget lacks), on calef's ruling (a) of 2026-10-07
(UTC) on PR #1798: a frame or kernel object retyped from a region carries the region capability's
rights, no more. `RETYPE` and every `RETYPE_OBJ` arm mint through `Cap::mint_child`, the inheriting
mint `SPLIT` already used and `split_never_widens_rights` proves. Before 824 each minted
`Rights::ALL`, so a holder of a budget without `GRANT` could carve a page and lend it. `GRANT` is
now delegated down the budget tree and never minted at a leaf.

seL4 gives a retyped object full rights over itself (recalled, not read). nife departs from that
because its leaf budgets withhold `GRANT` on purpose, and a full-rights mint made the withholding
cosmetic. Revoking afterwards was refused with the ruling: the window opens at the mint.

What a budget carries is now what everything made from it carries, so three shapes matter:

| Budget | Rights | Who holds it |
|---|---|---|
| The root (`cap::memory_region_root_cap`) | every right | the progenitor, which hands `BIND` and `GRANT` on |
| A lending budget (`abi::rights::ALL`, `cap::memory_region_lending_capability`) | every right | a builder, a shell, a program that lends a page it retyped: a networked program's socket pages, `jig`'s staged package |
| Spend-only (`abi::rights::SPEND_ONLY`, `cap::memory_region_cap`) | every right but `GRANT` | a leaf |

The spend-only budget was `WRITE` alone until 824. It gained `READ`, `ENUMERATE` and `BIND` so a
leaf can still wait on its own notification and join its own process. On a region those rights
grant nothing new: no method consults `READ` or `BIND`, and `ENUMERATE` answers `USAGE`, the
holder's own spending.

`BUGS`: a builder that lends what it carves now holds `GRANT` on its budget, which is also the right
to lend the budget. The sharpest case is a networked `std` program (`std_layout.rs`'s slot 3): it
needs `GRANT` to lend socket pages, so it may also hand its whole job region to a server it holds,
which could then spend or destroy the program's heap. It reaches nothing outside its own grant. A
separate socket-page region split off the job region would narrow this to the pages lent, at the
cost of a region per networked job and a reclaim that must destroy the child first.

## What this is, and what it is not

This converts a process's memory (its pages and their page tables) to region-backed, and
demonstrates the property with a hard number. It is honest to be equally clear about the
boundary: the kernel's *own* objects still come from the kernel heap. `Thread` structs, the
scheduler's `BTreeMap` and `VecDeque`, rendezvous objects, capability tables, thread stacks: all
still allocated the old way.

Converting each of those is the same retype mechanism applied to a kernel object rather than a
page. In seL4 you retype untyped into a TCB, into a CNode, into an endpoint, into a page table, and
the kernel genuinely has no heap at all. That is the long tail, the part seL4 spent years and a
proof on, and it is what "the allocators leave" in the milestone table ultimately means. What
milestone 11 establishes is the mechanism and the property, for the memory a process spends, which
is the load-bearing idea. The rest is the same move, object type by object type.

## What is deliberately not here

**Revocation.** In seL4, destroying an untyped invalidates every object ever retyped out of it,
tracked by a capability derivation tree. We free a region as a whole and do not chase down the
capabilities derived from it, because nothing in the current system outlives its region in a way
that would matter. Revocation and the derivation tree are the natural next depth, and they are
where the model gets genuinely hard.
