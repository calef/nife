# §103 appendix: option D, a large `Frame`

Part of the survey behind [§103 (what a `Frame` names)](../0103-what-a-frame-names.md), the reading and the measured costs that bear on option D, one 2 MiB page. Moved out of that page on 2026-10-12 (UTC) under [§212 (a prose budget)](../0212-a-prose-budget-for-every-document.md), word for word: only headings and line breaks changed. The main page is complete without this one, which exists to verify or challenge it. Its section numbers are the original page's.

## The prior art

### seL4 frame sizes: correct, with one caveat about what the manual actually says

The claim was that seL4 keeps one capability per frame, but frames come in sizes, so a 2 MiB
framebuffer is one frame capability. **Correct**, verified against the *seL4 Reference Manual*
version 16.0.0, 22 July 2026 (`sel4.systems/Info/Docs/seL4-manual-latest.pdf`).

AArch64, the manual's section 7.1.2.2: `seL4_PageBits` 4 KiB at mapping level 3, `seL4_LargePageBits` 2 MiB at level 2,
`seL4_HugePageBits` 1 GiB at level 1. The manual's own gloss on that column is the block-descriptor
confirmation: the mapping level *"refers to the level of the paging structure at which this page must
be mapped."* The object types are `seL4_ARM_SmallPageObject`, `seL4_ARM_LargePageObject`
(`libsel4/arch_include/arm/sel4/arch/objecttype.h`) and `seL4_ARM_HugePageObject`
(`libsel4/sel4_arch_include/aarch64/sel4/sel4_arch/objecttype.h`).

**And the size is chosen by the object type, not by a size argument**, which is a shape decision
worth copying or refusing deliberately. The manual's section 2.4.2:

> For all other object types, the size is fixed, and the size_bits argument to
> seL4_Untyped_Retype() is ignored.

Only CNode, SchedContext and Untyped are variable-sized. So seL4 spells a large frame as an
**order**, and there is nothing between 4 KiB and 2 MiB.

The riscv64 parity is exact, which matters for rule 5: the manual's section 7.1.2.6 gives 4 KiB at level 2, 2 MiB at
level 1, 1 GiB at level 0, as `seL4_RISCV_4K_Page`, `seL4_RISCV_Mega_Page`, `seL4_RISCV_Giga_Page`.
(The naming is inconsistent between the two architectures, `*PageObject` against `*_Page`. If we take
the shape, we should not take that.)

Note also what seL4 does *not* do, because section 5's overlap argument depends on it: it has no
`Frame::SPLIT`. A large frame is retyped from untyped, the untyped is spent, and the same memory
cannot also be a small frame. That is what keeps its frame capabilities equal-or-disjoint, and it is
the invariant we would be putting at risk.

### KeyKOS: the same sixteen slots, answered with a tree

The coincidence is worth the citation on its own. From the KeyKOS architecture notes
(`cap-lore.com/Agorics/Library/KeyKos/Architecture/Pages.Nodes.html`):

> A page holds 4096 bytes of data and a node has sixteen slots.

and from the *KeyKOS Nanokernel Architecture* paper (§4):

> A segment is a collection of pages or other segments. [...] Nodes are the glue that holds segments
> together. KeyKOS implements segments as a tree of nodes with pages as the leaves of the tree.

A system with this tree's exact slot count met this exact problem and answered it with a tree
rather than by widening the slot. A segment's size is a power of sixteen, and sub-segments are
page-granular, so attenuation is finer than the size quantum. EROS narrowed the same idea by putting
the height of the tree in the capability itself (*"Node capabilities encode the height of the
tree that they name"*, SOSP'99 §3.1), which is the first design on this list to put extent metadata
in the capability word.

This is a fifth option, and this lane is not proposing it: a tree of capabilities is a much larger
change than any of the four, and this tree has no node object. It is here because it is the answer a
sixteen-slot system actually gave.

### Coyotos: the documented failure mode of the power-of-two family

Coyotos replaced EROS's nodes with GPTs (also sixteen slots) and made the size explicit as `l2v`:
*"Each slot of the GPT names a subspace of size 2^l2v bytes."* And it names, precisely, what a
power-of-two answer costs at delegation time. From the Coyotos Microkernel Specification v0.6+,
§4.3:

> the invoker may wish to transmit a 2^11 page (2^23 byte) subspace, but the subspace may currently
> be dominated by a GPT having l2v=21. That is: there is no single slot in the GPT that directly
> holds a capability of the desired span. [...] When such a send is attempted, the invoker will
> receive a `SplitFault` exception.

with the whole scheme conditioned on the region being *"naturally aligned"*. **Read against this
fork: 475 pages can never be one Coyotos capability, and it could never be one L4 fpage either.** The
power-of-two family (fpages, KeyKOS segments, EROS heights, Coyotos `l2v`) buys a compact encoding
and pays in expressiveness, and option D is a member of that family. That is not an argument against
D, because D rounds *up* into a 2 MiB page and wastes 37 pages rather than failing; it is the reason
D's waste is structural rather than incidental.

*Provenance caveat: `coyotos.org` returns HTTP 522, so this was read from a third-party cache
(`hydra-www.ietfng.org/capbib/cache/shapiro:coyotosspec.html`) and not diffed against the canonical
copy.*

---

## What it costs

### aarch64 and riscv64 are not symmetric, and the asymmetry is in the encoder

| | aarch64 (`Aarch64`, 4 levels) | riscv64 (`Sv39`, 3 levels) |
|---|---|---|
| 2 MiB is | a block at level 2 (`index` shift 21) | a leaf at level 1 (shift 21) |
| 1 GiB is | a block at level 1 (shift 30) | a leaf at level 0 (shift 30) |
| Encoding a block | **new code.** `leaf_entry` writes bits[1:0] = `0b11`, which at L0-L2 means *table*. A block is `0b01`, which at L3 is a *fault*. So the format needs a `block_entry`, and it is level-sensitive. | **none.** `leaf_entry` already writes a valid leaf at any level; a superpage is the same PTE with the low PPN bits zero. |
| Telling table from leaf | `entry & (1 << 1)`, and the answer depends on the level | `entry & (R\|W\|X) == 0`, level-independent |
| Alignment the hardware requires | VA and PA both 2 MiB-aligned | PPN[0] must be zero, i.e. PA 2 MiB-aligned, or the walk page-faults |

So riscv64 needs no new descriptor construction at all and aarch64 does; but both need the same new
trait method and the same three walk changes, because the discrimination predicate differs from
`is_present` on both. Rule 5's parity gate is satisfiable in one change; it is the aarch64 encoder
that carries the extra risk, because `0b01` at the wrong level is a translation fault rather than a
compile error.

### The Kani cost, which is smaller than the fork's framing assumed

`crates/paging` carries 18 harnesses: six in `aarch64.rs`, six in `sv39.rs`, six in `domain.rs`
(133 in the tree as a whole, counted from the merged tree at `22da81ae`). The premise offered when
this fork was raised was that "a second page size is a second case in every one of them, and that may
be the real price rather than the mapping code." **Measured, that is not so.**

- **Twelve are size-independent and change not at all.** `index_is_always_in_bounds`,
  `the_indices_and_offset_tile_the_address`, `distinct_pages_take_distinct_paths` and
  `the_two_halves_are_disjoint` are quantified over `level` or over the whole address already, on
  both architectures. All six `domain.rs` harnesses are about `DmaRegion` arithmetic and never see a
  descriptor.
- Two widen (one per architecture): `the_user_va_gate_admits_only_the_aligned_low_half` pins
  `is_user_page_va` to `va & 0xfff == 0`, and a block VA gate is a different predicate.
- Two need a twin (one per architecture): `the_leaf_keeps_address_and_permissions_apart` must be
  proved for `block_entry` as well as `leaf_entry`.
- And one genuinely new property is owed, per architecture, and it is the important one: that
  table and block are totally and exclusively distinguishable at every level, so no descriptor is
  ever read as the other kind. Nothing in the tree proves that today because nothing needed it, and
  it is the property that stands between a block descriptor and a mapper writing PTEs into pixels.

So: roughly +6 harnesses and 2 rewritten, against 18. The real price is the walk rewrite, not the
proofs, and the proofs are what make the walk rewrite affordable.

### The allocator and the region table

- `crates/frames` has no aligned allocation. `alloc_contiguous(count)` scans for a run of `count`
  free pages with no alignment argument, and `Frame::new` panics on anything not 4 KiB aligned. A
  2 MiB frame needs a 512-page run at a 512-page-aligned physical address, so this is a new method
  plus a harness (the crate has five). Cheap, and the fragmentation headroom exists: the ledger in
  `notes/frames.md` measures the longest free run at 14,080 pages after a full test boot.
- `crates/regions` is where option D gets expensive, and only if userspace can mint a large
  frame. `untyped::retype_page` bumps a watermark one page at a time, and `regions` is proved for
  no double free on the strength of `split_new_watermark` and `destroy_outcome`'s LIFO un-bump.
  Aligning a watermark up to a 2 MiB boundary leaves a hole, and a hole is exactly what the LIFO
  return does not model. That is the scariest place in the tree to add a case.

  It is avoidable, and this is the finding that most changes option D's price. The display path's
  memory does not come from a user retype at all: `display_service.rs` calls
  `crate::memory::alloc_contiguous(DMA_FRAMES)` directly and mints the capabilities itself. So option
  D splits in two:

  - D-narrow. The kernel may mint a large `Frame`; `Untyped::RETYPE` still mints 4 KiB only. No
    `regions` change, no watermark alignment, no touching the no-double-free proof.
  - D-full. Userspace can retype a large frame from its own untyped. Adds the `regions`
    alignment problem and its proof obligation.

  D-narrow unblocks milestone 29 completely. D-full is a later decision that D-narrow does not
  foreclose.

---

## The option

### D. A large `Frame`: one 2 MiB page

What changes. `PageFormat` gains block-descriptor support (a discrimination method and, on
aarch64, an encoder); the three walks gain a block branch; `Object::Frame` gains an order or a size;
`crates/frames` gains an aligned allocation. In D-narrow, `Untyped::RETYPE` is untouched and
`crates/regions` is untouched.

Cost. The walk rewrite, which is shared with the IOMMU domain builder, plus ~6 new Kani harnesses
and 2 rewritten. Internal fragmentation: 37 pages (151,552 bytes) wasted on an 800x608 surface, and
up to 511 pages wasted on a surface that just clears a boundary.

What it buys beyond this milestone. A 2 MiB mapping is one TLB entry instead of 512, which is
a measurable win for a framebuffer that is written by a compositor every frame and read by a device.
Nothing else in the tree gets that today, and `script/bench` could measure it.

What it forecloses. It commits the tree to power-of-two sizes as the way memory scales, which is
seL4's answer, L4's, KeyKOS's, EROS's and Coyotos's. That family's cost is documented rather than
speculative: Coyotos raises a `SplitFault` when a region is not naturally aligned and power-of-two
sized, because such a region **is not expressible as one capability at all**. D is the gentler member
of the family, because it rounds up and wastes rather than failing, but the 37 wasted pages are
structural and not an artifact of this surface's dimensions.

It does not foreclose A: a run of large frames is still a run.
