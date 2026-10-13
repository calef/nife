# Milestone 139 appendix: round 3

An appendix to [milestone 139 (drive the unsafe count down, and cinch the ratchet behind
it)](../0139-drive-down-unsafe.md), moved out of that page on 2026-10-13 (UTC) under [§212 (a prose
budget)](../../decisions/0212-a-prose-budget-for-every-document.md). The main page is complete
without it. This file holds round 3: three migrations, two investigations, and the correction and
ruling that followed them. Long sentences were split as they moved; no figure, date, name or
quotation was dropped.

## Round 3 (2026-08-24): `swish`'s remaining windows, `disk_surveyor`'s flag, `net_stack`'s cluster, and two investigations that ended in a recommendation rather than a migration

Round 2's own handoff list named five items precisely enough that this round did not have to
re-derive a net. It took the three it named as clear migrations and investigated the two it named
as open questions.

`swish.rs`'s remaining two windows. The first is `OUT_VA`/`LINE_VA` (`stage`/`read_line`, the
shell's terminal pages). The second is the job frame `spawn_interruptible`/`watch` signal through
(`jf_load`/`jf_store`, parametrized by a runtime `va`). In round 2's own words that frame is
"actually a *better* `MappedWindow` fit than the FS cluster, since `new` already takes a runtime
base". The terminal pair is flat by block count (2 removed, 2 added at the two `const` window
declarations). It is still a real reduction by criterion 2, the "typed abstraction replaces raw
pointer arithmetic" case `smb_server.rs` and `fs_subtree_caretaker.rs` were in rounds 1 and 2 (see
[rounds 1 and 2](rounds-1-and-2.md)). The job frame collapses for real. `jf_load`/`jf_store` were two
functions with their own `// SAFETY:` comments, called eight times combined across
`spawn_interruptible` and `watch`. One `MappedWindow`, constructed once right after the frame is
mapped, replaced both. 4 `unsafe {` blocks removed, 3 added, net -1, in `components/src/swish.rs`
alone.

`disk_surveyor.rs`'s `ROSTER_VA`. A single shared `u64` flag at a fixed VA the program maps itself
at runtime (`Frame::MAP`, not a boot-time wiring). It is read once in `ROLE_HOLDER`. It is read
again after the kernel deliberately revokes the mapping, which is the module's own negative control:
the second read must fault. It is written once in `ROLE_PROBE`, which the kernel refuses because the
mapping is read-only. The two deliberate-fault sites are the one honest exception, recorded where a
reader meets it. `MappedWindow`'s bounds check cannot catch either fault, because offset 0 is inside
the declared window both times. So the real hardware fault happens inside `read`/`write` at exactly
the access the hand-written version made, and the test's behavior is unchanged by the migration. 3
`unsafe {` blocks removed, 2 added, net -1, in `components/src/disk_surveyor.rs` alone.

`net_stack.rs`'s `a_r8`/`a_r16`/`a_w16`/`a_w8` cluster. This is the exact naming variant
`user_rt::mapped_window`'s own doc comment already named as a shape round 1's search should have
caught and did not. It is genuinely harder than the FS cluster, as round 2 flagged. The VA is not a
fixed constant but `socket_va(sid) = 0x00A0_0000 + sid * 0x1000`, a different page per open socket.
Every caller computed an absolute VA (`sk.va + OFF_X`) rather than holding a `(window, offset)` pair.

Migrating cleanly meant restructuring the socket-lifecycle state itself. Swapping the four
functions' bodies for wrapper calls that still took a raw absolute VA would not have moved the
invariant at all, per this milestone's own instruction. So `Sock.va: u64` (0 meaning "no frame")
became `Sock.window: Option<MappedWindow>` (`None` meaning the same thing). The parallel `frame_va:
[u64; MAX_SOCKETS]` array became `frame_window: [Option<MappedWindow>; MAX_SOCKETS]`. It is
constructed once in `OPERATION_ATTACH_PAGE_FRAME`, right after the kernel maps the frame. That is
the one place in the whole socket lifecycle that needs to assert the invariant, instead of every one
of the four functions' bodies. Every downstream call site (`read_dst`, `udp_sendto`, `sock_receive`,
`tcp_connect`, `tcp_accept`, `udp_bind`, `tcp_send`) now takes or holds a `MappedWindow` rather
than a raw VA, so the restructuring reaches the caller side.

One further site collapsed for the same reason though it was never named `a_w8`. `sock_receive`'s
payload-write loop had its own hand-rolled `write_volatile`, identical in shape, folded into the
same window. 5 `unsafe {` blocks removed (the four functions' bodies plus the hand-rolled loop), 1
added (the window construction in `OPERATION_ATTACH_PAGE_FRAME`), net -4, in
`components/src/net_stack.rs` alone. `script/test`'s aarch64 and riscv64 net suites (DHCP, UDP,
TCP connect/accept/listen, the mDNS responder) passed clean. That is the load-bearing evidence here.
The restructuring touches per-socket lifecycle state, exactly the kind of change where a mistake
shows up as a flaky network test rather than a compile error.

Combined round 3: 12 `unsafe {` blocks removed, 6 added, net -6. It was measured from the diff
against this round's own base commit (`f731894d`), uncontaminated. Nothing else landed on this
branch between the base commit and this reduction, so the tree-wide census confirms it exactly.
It found 776 blocks outside `arch/` at the base commit (89 per 10,000, matching round 2's own final
reading), and 770 after, exactly -6. Density moved 89 to 88 (truncated). The line count moved by only 15, mostly the
comments explaining the new windows, so the denominator barely moved this round, unlike round 2's
`asm!`-collapse.

The ratchet, cinched a third time: `<!--count-at-most:unsafe-density-outside-arch-->` lowered from
96 to 95 in the same commit (`notes/unsafe-obligations.md`, `notes/counted-claims.md`,
`notes/register-of-measures.md`), 7 points above the 88 this round reached.

Investigation: device-register blocks (`console.rs`, `input.rs`, `driver.rs`, `clock.rs`,
`jh7110_trng.rs`). They are genuinely per-driver distinct, and `MappedWindow` is the wrong fit. A
stronger idiom already lives in this tree and is unused by these five files. Read in full.

- `driver.rs` is already a different idiom (a raw `*const u8` with `.add(offset)`, not a named
  accessor pair) and needs nothing.
- `clock.rs`'s two RTC drivers (`pl031_unix_nanos`, `goldfish_unix_nanos`) are each a single
  one-shot function with its own `// SAFETY:` comment, called once. There is no duplication inside
  the file to collapse.
- `console.rs`, `input.rs` and `jh7110_trng.rs` each already collapse their own register access to
  one or two functions (`uart_put`; `rd`/`wr`; `r32`/`w32`). Each has one `// SAFETY:` comment
  apiece, reused at every call site. The §94 (what may live in a userspace library) shape is already applied *within* each
  file.

What is not collapsed is *across* files. The wording of each file's comment rhymes ("this VA is our
device mapping, handed to us at spawn, for the whole lifetime of this process"). That looks like the
same invariant copied, but reading closely it is not. Each asserts a genuinely different fact (a
different VA, a different device, a different offset table). It is the same shape
`crates/inter_process_communication`'s three call sites turned out to have in round 2 ("which of two
queues, which node, under what caller contract... there is no §94 shape to collapse here"). A shared
`RegisterBlock` type wrapping these would relocate the assertion from "a local `rd`/`wr` function"
to "a shared type's constructor," not collapse it. That is this milestone's own named anti-pattern.

But the investigation did not end at "nothing to do." The tree already has the right idiom for this
shape, in the kernel, for the very same two devices. `kernel/src/drivers/pl011.rs` and
`kernel/src/drivers/ns16550.rs` drive the identical PL011/NS16550 hardware these five userspace
programs also drive. They use `tock-registers`' `register_structs!`/`register_bitfields!` macros:
one `unsafe` block for the whole driver (the base-pointer construction). Every register offset is
checked at compile time rather than asserted by hand. That is a stronger property than
`MappedWindow`'s runtime bounds check gives (`kernel/src/drivers/pl011.rs`'s own comment: "an
off-by-four here is a build error rather than a mystery at runtime"). This is exactly the "what does
this tree already do in the analogous case" answer AGENTS.md's fork-readiness section asks for. It
settles the "MappedWindow or a new RegisterBlock type" question this round's brief posed: neither.

`tock-registers` is already a tree dependency (§46 (thin primitives or whole subsystems)'s "thin
architectural primitive" category). Migrating `console.rs`/`input.rs`/`jh7110_trng.rs` onto it
would be a real reduction with a stronger property than either alternative. But it is new scope
this round did not take, for two reasons worth naming rather than a guess at effort. Adding
`tock-registers` to the `user` crate's dependency graph is a dependency decision (rule 6, DECISIONS
§46) this round did not have standing to make unilaterally. And the three files it would touch gate
boot output and keyboard input, exactly the kind of blast radius this round's brief said to weigh
before attempting a risky restructuring. Left as a named follow-on: migrate `console.rs`, `input.rs`
and `jh7110_trng.rs` onto `tock_registers::register_structs!`, matching
`kernel/src/drivers/pl011.rs`'s and `kernel/src/drivers/ns16550.rs`'s own shape. `clock.rs` and
`driver.rs` need nothing, per above.

Correction, 2026-08-24 (round 5's lane, found by reading the file rather than trusting the
paragraph above): `kernel/src/drivers/ns16550.rs` does not use `tock_registers`. The two paragraphs
above claim it does ("drive the identical PL011/NS16550 hardware... using `tock-registers`'
`register_structs!`/`register_bitfields!` macros"), twice, and both are wrong. `ns16550.rs`'s own
module doc says plainly: "This one uses plain volatile access rather than `tock_registers` register
blocks: the register indices are what the 16550 defines, and the stride between them is a runtime
value no static layout macro can express." The JH7110's real UART spaces its registers four bytes
apart (`reg-shift = <2>`). QEMU's emulated NS16550 spaces them one byte apart. The stride is carried
as runtime data (`Shape`, since the VisionFive 2 prep) rather than a compile-time layout, which is
exactly what `register_structs!` cannot express. Only `pl011.rs` uses `tock_registers` today. This
does not overturn round 3's conclusion: the PL011 follow-on is still real and still worth taking. It
changes which half of `console.rs` and `input.rs` that follow-on actually applies to. Round 5 worked
that out per-file rather than assuming "the identical hardware" meant identical treatment (see
[rounds 4 and 5](rounds-4-and-5.md)).

Decided, 2026-08-24: calef, in conversation: *"Take the dependency for user, launch the lane."*
`tock-registers` is now a dependency of the `user` crate (`user/Cargo.toml`), pinned to `"0.10"`.
That matches `kernel/Cargo.toml`'s own pin, so the two crates using this library never skew. See
[round 5](rounds-4-and-5.md) for what actually migrated, including the runtime-stride finding above
that narrowed it.

Investigation: framebuffer/graphics code (`display.rs`, `painter.rs`, `window.rs`, `compositor.rs`,
`display_terminal.rs`). One decisive structural finding, and the performance question narrowed but
not settled. Read in full. The part of this pipeline that would actually run at real per-frame
volume carries no per-pixel unsafe at all, migrated or not. `compositor.rs`'s `paint`/`serve_frame`
call `compositor::composite(screen(), &srcs[..n], n, damage)`, the crate's host-tested pure logic.
That runs over ordinary safe `&mut [u32]`/`&[u32]` slices obtained by exactly one `unsafe` call per
frame (`screen()`, `source(i)`), not one per pixel. The compositor's hot loop already is the "assert
the invariant once, then use safe indexing" idiom `MappedWindow` generalizes, just spelled as a
slice rather than as that type. This is the decisive part of round 2's "is a bounds check per pixel
write a real performance question" framing. The answer is: not here, since there is no per-pixel
check (bounds or otherwise) on this path today.

What remains is genuinely per-pixel `unsafe`:

- `painter.rs`'s and `window.rs`'s `px_write`/`px_read`: a client painting and then digesting its
  own surface. That is `graphics_protocol::PIXELS` = 8,192 accesses per run for `painter.rs`, and up
  to 2,048 for the largest window in `compositor::SCENE`.
- `display.rs`'s `surface_pixel`: the driver's own post-flush digest, also 8,192 accesses per run.
  It shares its `dma_read`/`dma_write` pair with a few dozen one-off virtqueue-field writes that are
  not the performance question.
- `display_terminal.rs`'s `paint`, the one file among the five where the question is live rather
  than one-shot. It repaints a damaged rectangle on every terminal update, driven by keystrokes
  rather than a fixed frame rate. Its volume is bounded by typing speed and paste size rather than
  by a boot-time test, unlike the other four.

None of these is the "thousands of writes per frame, sixty times a second" sustained path round 2's
handoff named; that path turned out not to exist in this file set. But "bounded and one-shot" or
"bounded by typing speed" is a structural characterization, not a measurement. This round did not
obtain an actual instruction-count or cycle number for a `MappedWindow`-checked pixel write versus
the current raw one. No such micro-benchmark exists yet, and building one was out of this round's
reasonable scope alongside the three migrations above. Left as a narrowed follow-on rather than a
migration on a guess. The question a next lane needs to answer is no longer "does a bounds check
survive 60fps". It does not need to, because nothing here runs at 60fps. The question is "does a
bounds check cost enough at 2,048-8,192 accesses per one-shot run, or per keystroke-driven repaint,
to matter". That is a much smaller question, worth `script/icount` or `script/bench` rather than
reasoning about it further.
