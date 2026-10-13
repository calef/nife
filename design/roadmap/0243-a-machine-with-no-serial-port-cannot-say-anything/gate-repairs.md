# Two repairs to the screen gate: an appendix to milestone 243

[Milestone 243 (a machine with no serial port has no way to say anything, and no gate can read
it)](../0243-a-machine-with-no-serial-port-cannot-say-anything.md) keeps what was built and the
early-boot bound. It also keeps the `ramfb` screens, the follow-on work and the `BUGS`. This appendix holds two repairs the
lane made to its gates after they were built: the x86_64 marker that asserted a race, and the
riscv64 leg's first red run in CI.

## The x86_64 gate was asserting a race, and a measured premise was wrong

Found 2026-09-20 while re-verifying after a rebase, and it is not this lane's code.
`cargo xtask uefi-boot`'s tour stage failed intermittently on a loaded dev Mac: four consecutive
runs read 19, 27, 40 and 0 rows. Milestone 400's own BUGS predicted it in those words ("a much
faster guest or a slower screendump could miss it").

The lane's first instinct was that its own loader banner had caused it, and that was tested rather
than assumed. With the banner's paint disabled and nothing else changed, the same gate failed
two runs in four, which is worse. The banner is not the cause, and the ninety seconds that test
cost were the cheapest ninety seconds in this lane.

The marker is the self-test verdict, `boot_ladder::SELF_TEST` (`UEFI_SCREEN_MARKER` in
`xtask/src/uefi.rs`). The tour scrolls on OVMF, so a first-line marker is gone by the end, and the
last line before the handover's clear is on screen for only a few hundred milliseconds. Milestone
445 (the screen check stops sampling and starts asking) holds the boot at the handover and reads the
marker behind a handshake, so the check asks rather than races.

The total claim is unchanged, which is the thing to check before believing any gate got easier.
The screen's job is to prove the *pixels*: `LocateProtocol`, the byte order, the stride, the mapping
surviving `mmu::init`, and the glyphs. Any decoded row proves all five. That the boot reached the
self-test is asserted separately and unconditionally on the serial transcript a few dozen lines
above. So the change trades nothing away; it stops the gate asserting something it never meant to.
How deep the dump caught the tour is now *reported* rather than required.

This edits the gate of milestone 400 (the shell on the firmware's screen), not this one's. 400's
`BUGS` entry about that window is now stale in its last clause. A lane may not edit another
milestone's block, so it is named here and in this lane's report for the integrator to strike.
(Struck 2026-10-12, UTC.)

## The gate's first red run, and the line that made it readable

CI found a wiring bug this lane had already been told about and had dismissed. It is worth
recording at more length than the fix deserves, because the dismissal is the reusable part.

`cargo xtask test`'s riscv64 leg exports `NIFE_INITIAL_RAMDISK` pointing at the riscv64 archive, explicitly,
with a comment saying it has to. `cargo()` exports the aarch64 one, because it is the aarch64
path's helper. `screen_boot` went through `cargo()` and never overrode it, so the riscv64 leg handed
a riscv kernel an aarch64 archive.

That bug had two faces and only the harmless one was visible locally. With the aarch64 archive on
disk (which it always is, after any other leg has run) the boot carried on and printed
`MEASURED BOOT REFUSED: 'progenitor' is not what this kernel image was built against`. The lane read
that as noise, because the gate's own assertion still passed: the tour was on the screen, which is
what the gate checks. In the `cpu matrix (riscv64)` job, which builds no aarch64 archive, the second
face appeared and QEMU would not start at all: `could not load ramdisk`. Five models, one cause, none
of it about a CPU model.

The lesson is the one this tree keeps relearning. A line the machine printed outranks a gate that
went green, and a refusal message inside a passing run is still a refusal. `MEASURED BOOT REFUSED`
was telling the truth an hour before CI repeated it more loudly.

What the fix is: the archive is chosen per architecture at the spawn, the way every other riscv64
caller in `xtask` already does it. Verified by reproducing CI's condition rather than by reasoning,
with `target/initrd.img` moved out of the tree. `cargo xtask screen-boot riscv64` then exits 0 and
reads 45 rows off the screen. The tour reaches further than before, because the progenitor now
composes userspace instead of refusing the archive.

And the screen legs no longer run under `--cpu`. That is a cost decision made on its own merits
rather than a way around the failure (the failure reproduces on `rv64` too, and is fixed). The matrix
runs this suite five times to narrow the ISA; nothing on the screen path varies with `-cpu`. The
`fw_cfg` conversation is byte moves and MMIO stores, and `screen_console` is integer arithmetic and
byte stores. Every instruction either one uses has already been executed on that same model by
the three hundred tests above it. Five more emulated boots would buy a claim that cannot differ
between models. The leg still runs on every ordinary `script/test`, `--arch riscv64` included.

The gate's own second diagnostic line is why this was quick to read, and it is the shape worth
copying into other gates:

```text
screen-boot: nothing decodable was ever on the screen (did QEMU get a ramfb and a monitor?)
screen-boot: the tour never reached the SERIAL line either, so this is a boot failure and not a screen one
```

The second line is a control, not a detail. A screen gate that only says "the screen was blank"
sends the next reader into the framebuffer path, which is where this lane would have gone and where
nothing was wrong. Checking the serial transcript for the same marker costs one `contains` and
separates "this milestone's mechanism is broken" from "the machine did not boot", which are different
people's afternoons. Any gate that reads a machine through one channel should assert the same fact
through the channel it is replacing, and say which one failed.
