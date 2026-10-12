# §103 appendix: options B and C, a `Frame` stays one page

Part of the survey behind [§103 (what a `Frame` names)](../0103-what-a-frame-names.md), the premise checks and the reading that bear on the two options that keep a `Frame` one page: growing the cspace, and mapping at spawn. Moved out of that page on 2026-10-12 (UTC) under [§212 (a prose budget)](../0212-a-prose-budget-for-every-document.md), word for word: only headings and line breaks changed. The main page is complete without this one, which exists to verify or challenge it. Its section numbers are the original page's.

## 4. Is the premise true?

Two premises were checked before anything was priced, and both hold.

The arithmetic. `800 x 600 x 4 = 1,920,000` is 468.75 pages, so `gfx_proto`'s
`SURFACE_BYTES.is_multiple_of(4096)` assertion fails on it; PR #364 found this and it is right.
800x608 is the nearest shape with an 800 width (the height must be a multiple of 32), giving
`1,945,600` bytes, 475 pages exactly, and a 100x43 grid at gohufont-14's 8x14. A 2 MiB page is
`2,097,152` bytes, so the whole surface fits inside one with 151,552 bytes (37 pages) unused.
`SURFACE_VA` is `0x60_0000`, which is 2 MiB-aligned, so no address moves.

The ceiling. `cap::CSPACE_SLOTS = 16`, `abi::fault::FAULT_EP_SLOT = 15`, and
`DRIVER_SLOT_DMA = 5`, so the driver's region may hold at most ten slots and `DMA_FRAMES` is
`1 + SURFACE_FRAMES`. Nine surface frames is the ceiling, 36,864 bytes, exactly as reported.

**And the obvious workaround does not work, which is worth writing down before somebody proposes
it.** The driver is not what is scarce. It maps the surface only to read pixels back
(`user/src/display.rs`, `surface_pixel`); the device is handed a *physical* address it already has
in `a1`, so a driver that gave up its readback could hold one DMA frame instead of ten. That
moves the ceiling to the painting client, whose cspace starts its surface at slot 3
(`CLIENT_SLOT_SURFACE`) and so holds at most twelve frames: 49,152 bytes, a 128x96 surface, and
a 16x6 text grid. Rearranging slots buys two rows. The binding limit is sixteen slots against
a run of hundreds, and no allocation of them among the parties changes the order of magnitude.

---

## The prior art

### L4 flexpages: the geometry is right and "names" is wrong

The claim was that an fpage is *"a power-of-two-sized, power-of-two-aligned region named as one unit
in map and grant operations, the canonical prior art for exactly this problem."*

The geometry is exactly right. L4 X.2 (the *L4 eXperimental Kernel Reference Manual*, Rev 7,
`l4ka.org/l4ka/l4-x2-r7.pdf`, its section 4.1):

> Fpages (Flexpages) are regions of the virtual address space. [...] An fpage of size 2^s has a
> 2^s-aligned base address b, i.e., b ≡ 0 (mod 2^s), where s≥10 for all architectures.

and, which is the part that reads most like what we want, until its second half:

> Mapped fpages are considered inseparable objects. That is, if an fpage is mapped, the mapper can
> not later partially unmap the mapped page; the whole fpage must be unmapped in a single operation.
> The mappee can, however, separate the fpage and map fpages (objects) of smaller size. Partially
> unmapping an fpage might or might not work on some systems. The kernel will give no indication as
> to whether such an operation succeeded or not.

That last sentence is L4 conceding exactly the invariant section 5 is worried about, and conceding
it as undefined behavior rather than as a rule. A receiver may split a run into smaller runs, and
what a partial unmap then does is unspecified and unreported. Whatever this tree decides, it should
not decide that.

**But an fpage is not a capability, and classic L4 has none.** An fpage is a two-word descriptor
placed in message registers and interpreted by the map/grant operation; the authority behind it is
the mapper's *own existing mapping*, not a slot in a table. From the same manual, under MapItem:

> An fpage (see page 40) or IO fpage that should be mapped is sent to the mappee as part of a
> message. [...] The fpage is specified by a two-word descriptor:

> The effective access rights for the newly mapped page are calculated by bitwise AND-ing the access
> rights specified in the snd fpage and the access rights that the mapper itself has on that fpage.

The lane's check on this: **the string "capabilit" does not occur anywhere in the L4 X.2 reference
manual.** X.2 has no capability space at all; the address space *is* the protection domain. The V2
manual (Au and Heiser, UNSW-CSE-TR-9801, `cgi.cse.unsw.edu.au/~reports/papers/9801.pdf`) says the
same thing in its own words: *"Fpages are specified by the mapper and received by the mappee as part
of an IPC message."* The fpage survives into L4Ka::Pistachio and OKL4 2.1.1 in that same role
(OKL4's `fpage.h`: *"Flexpages are size-aligned memory objects and can cover multiple hardware
pages"*).

**So the fpage is prior art for a map operation over a run, not for a capability that names one.**
That distinction is the entire fork: option C is an operation over a run and options A and D are
capabilities that name one, and citing fpages as support for A or D would have been citing the wrong
half.

The L4 family's actual answer arrives with its capabilities, and it is an indirection object.
Fiasco.OC and L4Re did add a capability table (`l4re.org/doc/l4re_concepts_naming.html`:
*"Capabilities are stored in per-task capability tables (the object space)"*), and in that system
memory is named by a dataspace capability, with the fpage demoted to the transfer descriptor:

> A dataspace is an abstraction for any thing that is available via usual memory access
> instructions. (`l4re.org/doc/classL4Re_1_1Dataspace.html`)

One dataspace capability, arbitrary size, attached to a task's region map; `map()` then *"will
attempt to map the largest possible flexpage that covers the given local address."* That is option
A's shape, arrived at by a system that started from option C's and found it insufficient once it had
capabilities to name things with. It is the single most relevant piece of prior art on this fork
and it was not the one offered.

One aside worth recording because it is a lever nobody listed. L4Re's fpage grew a type field
(`L4_FPAGE_MEMORY`, `L4_FPAGE_IO`, `L4_FPAGE_OBJ`), and an `L4_FPAGE_OBJ` fpage describes a
power-of-two-aligned range of capability slots. If the sixteen-slot cspace ever becomes the
binding limit for a reason that is not this one, that is the prior art for addressing slots in runs.

---

## The options

### B. Grow `CSPACE_SLOTS`

Dominated by arithmetic (section 5). Include it only as the small, orthogonal change it actually is.

### C. `aspace::MAP_INTO` at spawn

What changes. Nothing in the model. The kernel maps 475 pages into the client's space through the
recorded engine.

Cost. The client holds no object for its pixels; 475 records; a different spawn path for this
service.

What it forecloses. It re-opens on the display path the gap milestone 108 closed everywhere else,
and it makes the display the one service whose memory is not a capability. If the answer to "why does
this path look different?" is "because a Frame is one page", the model has been worked around rather
than decided.
