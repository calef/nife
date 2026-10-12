# §27 appendix: the 2026-07-29 amendment's account, and the first superseded one

Most of the 2026-07-29 amendment of [§27 (the filesystem service)](../0027-filesystem-service.md),
which opens on that page and continues there after this. Moved out of that page on 2026-10-12 (UTC)
under [§212 (a prose budget)](../0212-a-prose-budget-for-every-document.md). The main page is
complete without this one, which exists to verify or challenge it. calef asked on 2026-10-12 (UTC)
that §27 meet the cap. Each sentence over the 40 words of [§213 (writing
standards)](../0213-writing-standards.md) was split, changing as few words as it could, and two byte
counts lost their bold. Nothing else changed. "Above" and "below" mean the original page.

## The 2026-07-29 amendment, continued


The cause is the missing `TRUNCATE` verb meeting a whole-file comparison. A write shorter than the
file does not truncate it. One boot's FS client left a 64-byte payload in `scratch`. The next boot's
`std::fs` test wrote its 61-byte pattern, asserted the whole file equaled it, got 64 bytes back (61
new plus the old three-byte tail), and panicked *inside its write block*. That panic, read as "the
server refused the write," is the entire bug. No allocator loop, no heap exhaustion, no accumulated
mount state, no device-only defect, and no error reply, which is why nobody ever found the errno:
**there was none to find.**

That also explains why three investigations produced three incompatible answers while every one of
them reported honestly. The symptom depended on what the *previous boot's* client happened to leave
behind, and that changed as the client changed, so each round measured a genuinely different thing.
Two lessons worth carrying off, because neither is about filesystems:

- An order-coupled gate manufactures facts. `mkredoxfs` ran once for both ISA legs and the
  aarch64 leg mutates the image, so whichever leg ran second failed and neither was reproducible
  alone. Each leg now regenerates its own fixture, and `CRICKER_KEEP_REDOXFS=1` makes the cross-boot
  case *deliberate* rather than an accident of ordering.
- **A test that asserts on whole-file equality asserts on history it did not write.** The fix is at
  that layer, not in the engine: the client restores the fixture as its last write, all its payloads
  are one length, and the post-run host check compares content **and length**. So a future client
  leaving a longer file fails the gate instead of corrupting a later boot's assertion. Pinned by a
  millisecond host test carrying the real 64/61/3 byte counts, so if it ever fails, the contract
  grew a verb and that was a decision.

Two hypotheses died by measurement, and both are recorded as dead rather than left looking
plausible. Heap exhaustion and accumulated mount state: the real engine under the FS server's own
allocator, capped identically, image in a `static` so it stays off the heap exactly as a real disk
does. It runs 30 mount-and-write cycles with the high-water **flat at 352 KiB**, four percent of the
8 MiB budget, the cap never once refusing a growth. Raising `FS_BUDGET_PAGES` would have fixed
nothing, and a number chosen to make a test pass would have been a coincidence rather than an
argument.

The errno plumbing built to chase this stays, because the reason it was unreadable was real. The
client routed every failed reply through a panicking `check`, so a trapped client told the waiting
test only that something went wrong while the server's reason died with the process. A negative
reply is now sent, carrying the raw reply word alongside the decoded errno rather than instead of
it. That is because the wire's negated errnos overlap the kernel's own `invoke` errors at −1..−8
(the notes/std.md wart) and a small value is otherwise ambiguous between "the server returned this
errno" and "the IPC itself failed."

*Superseded (2026-07-29), kept for the record.* The previous settled account said there was no
allocator loop but that a second mount of a used image failed its write for an unrelated reason. The
first half was right. The second half named a real symptom and mislocated it: the mount was fine and
the *assertion* was wrong.

Measured on a clean build: the FS client writes the same block three times in one run and passes
on both ISAs, and the image afterwards carries the third payload, so the repeat write reached the
disk. A `VERIFY_WRITES` switch that reads every written block back through `IpcDisk` and compares
never fired, so the blk IPC transport is faithful (nothing lost, nothing misdirected, no stale read).
And the observed failure was never a spin: the std program's own `expect` panics, which is what
truncated the transcript that got read as a hang. The "400% CPU looping in
`Transaction::sync_allocator`" reading does not survive a correct build.

What was actually broken: `mkredoxfs` ran once for both ISA legs, and the aarch64 leg writes the
image, so the riscv leg mounted an image a previous *boot* had mutated. Whichever leg ran second
failed, and neither leg was reproducible on its own. That is why three separate investigations,
each measuring a differently-broken setup, produced three incompatible answers, and it is worth
naming as a failure mode: an order-coupled gate manufactures facts. Each ISA leg now regenerates
the same known-good fixture.

That is determinism, not a fix. A second mount of a *used* image still fails its write, and the
recipe is recorded in notes/fs-server.md (generate once, run one leg, then the other without
regenerating) along with the leading hypothesis: accumulated mount state rather than bad data. A
used image carries a higher header generation, a longer allocator log and more live tree blocks. So
the second mount allocates more heap (capped at 8 MiB in `fs_server.rs`, bounded by
`FS_BUDGET_PAGES`) and may reach an allocator squash path a pristine mount never does. The next step
is reading the errno the server returns, which nothing currently surfaces. Note the cost of the fix:
the gate no longer exercises the cross-boot case at all, so this bug is now known-and-untested,
which is the same shape of invisibility that hid it in the first place.

The missing test layer, which should have existed from the start, now does: the EL0 binary's chunking
was extracted into a host-testable `BlockDisk`/`BlockIo`, because chunking that lives only in the EL0
binary is chunking no host test can reach. Ten host tests run in milliseconds (repeat writes,
record-sized writes across the multi-block and compressed-tail paths, and write then drop the mount
with no unmount then reopen and write again) and all pass. That is the decisive comparison: the host
does not loop, so there is no upstream RedoxFS bug and no vendored patch to offer.
