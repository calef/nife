# Milestone 139 appendix: rounds 1 and 2

An appendix to [milestone 139 (drive the unsafe count down, and cinch the ratchet behind
it)](../0139-drive-down-unsafe.md), moved out of that page on 2026-10-13 (UTC) under [§212 (a prose
budget)](../../decisions/0212-a-prose-budget-for-every-document.md). The main page is complete
without it. This file holds the first two rounds: the `MappedWindow` cluster, the `user_rt` trap
collapse and the FS page-copy loops. Long sentences were split as they moved; no figure, date, name
or quotation was dropped.

## Round 1 (2026-08-23): the `MappedWindow` cluster

The `MappedWindow` cluster, the milestone's first real reduction. Seven userspace programs
(`entropy`, `kbd`, `net_transport`, `multicast_dns_responder`, `socket_test_client`, `smb_server`,
`ntp`) each hand-rolled the same `r8`/`w8`/`r16`/`w16`/`r32` volatile-access functions. Each ran
over a DMA page or a shared IPC frame. Each function carried one hand-written `// SAFETY:` comment
asserting the same invariant ("this offset is inside the page the kernel mapped here") by hand at
every call site. That is the exact §94 (what may live in a userspace library, and what must be
per-binary) shape this milestone's own text names as the best available reduction.
`user_rt::mapped_window::MappedWindow` (new) holds that invariant once, at construction. It turns
every access into a bounds-checked call. A wrong offset used to be a silent out-of-bounds volatile
access and is now a panic naming the access. That is a real soundness improvement the hand-written
copies never had, not just a relocation.

Measured precisely from the diff. A before/after tree census is contaminated by unrelated
concurrent growth: five days between this milestone's baseline and this reduction added 30
unrelated `unsafe` blocks elsewhere in the tree, at roughly the tree's own rate. 32 `unsafe {`
blocks were removed across the seven programs and 11 added. The 11 are 9 window constructions plus
the 2 generic `read`/`write` methods inside `MappedWindow` itself. There is one construction per
program except `smb_server`, which needs two: one for its boot-wired FS channel sized to
`fs::TRANSFER_MAX`, and one for its runtime-mapped socket frame. Net -21. `smb_server.rs` alone is
flat (11 blocks before and after). It is still a real reduction by this milestone's own criterion 2
(raw pointer arithmetic replaced by a typed, bounds-checked abstraction), even though it does not
move that file's own count.

Density: 93.4 (799 blocks over 85,476 lines) immediately before this reduction, 90.8 (778 over
85,526) after. That is essentially the density the milestone's own baseline recorded on 2026-08-18
(93.0), despite five days of unrelated tree growth in between. This is what makes density rather
than raw count the number worth trusting. Full measurement history is in
`notes/unsafe-obligations.md`'s table.

The ratchet, cinched: `<!--count-at-most:unsafe-density-outside-arch-->` lowered from 100 to 97 in
the same commit (`notes/unsafe-obligations.md`, `notes/counted-claims.md`,
`notes/register-of-measures.md`). That leaves 7 points of headroom above the 90.8 this reduction
reached. It is the same absolute headroom the original 100-vs-93 ceiling carried. A zero-headroom
ceiling would fail the next legitimate unsafe block anywhere outside `arch/`. The reasoning for why
this measurement gets headroom (unlike `unsafe-thread-safety-claims`' zero-headroom ceiling) is
recorded in `notes/unsafe-obligations.md` beside the marker.

## Round 2 (2026-08-24): `user_rt`, `ipc`, and the broader `read_volatile`/`write_volatile` sweep

The three concrete next steps round 1 named, taken in the order it suggested.

`crates/user_rt`'s `SYS_INVOKE` round trip collapsed, a second real §94-shaped reduction. Six
methods (`receive`, `receive_cap`, `receive_fault`, `call`, `survey`, `list`) were each duplicated
once per architecture. Each hand-rolled its own `asm!` block asserting the identical invariant
("`svc`/`ecall` traps to the kernel, which validates before acting"). The register layouts differed
only in which of the five return words the caller happened to read back. That made twelve
hand-written copies of one assertion. `invoke5` (new, private to the crate, one per architecture)
holds the trap once. Every caller above it, including `invoke` itself, is now a safe wrapper with
no `asm!` of its own. This is the exact "collapse N hand-written assertions of one invariant into
one" shape this block's own text names as the best available reduction.

One honest behavioral note is recorded in `notes/unsafe-obligations.md` and in the code. Three of
the collapsed functions (`receive`, `receive_cap`, `receive_fault`) used to leave one input register
unset. The kernel read it as whatever value happened to be there, which was harmless, since those
methods read no input words. Routing them through the shared primitive means they now pass an
explicit `0`, a strict tightening rather than a behavior change. Measured from the diff: 14
`unsafe {` blocks removed, 9 added, net -5, entirely inside `crates/user_rt/src/lib.rs`.

`crates/inter_process_communication` was read in full: no reduction found. That is the milestone's
own predicted outcome for at least one target. Production code carried exactly three `unsafe`
blocks (`send`, `receive`, `remove_sender`). Each asserts a genuinely different fact (which of two
queues, which node, under what caller contract), so there is no §94 shape. The other 41 sites of the
baseline "44 blocks" were doc examples, harnesses and unit tests, each a distinct scenario. The
crate's doc comments already stated the shared proof obligations once at module level. Nothing to
migrate; reported honestly rather than forcing a relocation to move a number.

The broader `read_volatile`/`write_volatile` sweep round 1's own BUGS section asked for, run for
real this time. Round 1 searched by the `r8`/`w8`/`r16` naming convention. Grepping directly for
`read_volatile`/`write_volatile` across `user/src/` surfaced roughly thirty files. Most turned out
not to be the pattern: device register blocks with their own poll loops, framebuffer/graphics code,
or programs whose entire point is deliberately invalid or one-off memory access. "What is still
open" on [the main page](../0139-drive-down-unsafe.md) has the accounting.

One cluster was a clean, large match. Eight programs (`rm`, `fs_file_caretaker`, `sink`,
`fs_subtree_caretaker`, `fs_nameset_caretaker`, `login_test_client`, `fs_test_client`, `swish`) each
hand-rolled a `put_page`/`get_page`-shaped byte-copy loop over the page shared with the FS server.
Every one asserted "this VA is a mapped page of this size" by hand in near-identical wording.
`fs_nameset_caretaker` carries a second, read-only window for its name set. `fs_test_client`
carries five such helpers over one window sized to `fs::TRANSFER_MAX` rather than one page. All were
migrated onto the existing `user_rt::mapped_window::MappedWindow`, round 1's type, reused rather than
duplicated per this block's own instruction. 21 `unsafe {` blocks removed, 10 added, net -11 across
the nine files. `fs_subtree_caretaker.rs` alone is flat (1 block before and after: one hand-rolled
function traded for one window construction). That is the same "still real by criterion 2" case
`smb_server.rs` was in round 1: raw pointer arithmetic replaced by a typed, bounds-checked
abstraction, even though the file's own count does not move.

Combined round 2: 35 `unsafe {` blocks removed, 19 added, net -16. It was measured from the diff
against this round's own base commit (`a269403e`), the same discipline round 1 used and for the
same reason: a tree-wide census is contaminated by unrelated concurrent growth. This round's paired
measurement happens to be uncontaminated regardless. Nothing else landed on this branch between the
base commit and this reduction, so the tree-wide census confirms the diff exactly: 792 blocks
outside `arch/` at the base commit, 776 after, precisely -16. Density (the ceiling's actual unit)
moved only 90 to 89 truncated. This reduction also removed lines (duplicated `asm!` blocks and
SAFETY comments along with the blocks themselves). So the denominator moved with the numerator, for
the first time this ceiling has had to account for.

The ratchet, cinched again: `<!--count-at-most:unsafe-density-outside-arch-->` lowered from 97 to
96 in the same commit (`notes/unsafe-obligations.md`, `notes/counted-claims.md`,
`notes/register-of-measures.md`), 7 points above the 89 this round reached.
