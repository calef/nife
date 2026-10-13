# The fleet this unlocks: an appendix to milestone 243

[Milestone 243 (a machine with no serial port has no way to say anything, and no gate can read
it)](../0243-a-machine-with-no-serial-port-cannot-say-anything.md) keeps what was built and what is
open. This appendix holds the household fleet calef named the day the block was minted, why it was
already reachable, the two qualifications, and the cheap experiment it makes available.

> if I can boot a nife system off of a USB drive, then that opens up a lot of different hardware in
> our house that I can use for testing: [a family member's] laptop, [their] desktop, cordoba, two
> MacBooks, [another family member's] desktop
>
> -- calef, 2026-09-03

That fleet is already reachable and nobody had noticed. Milestone 87 (the x86_64 bare-metal
machine) boots from a FAT32 stick at `\EFI\BOOT\BOOTX64.EFI`. That is the removable-media
fallback every UEFI firmware looks for with no configuration. Nothing about it is specific to
xenon.

And `uefi_loader` already writes to the firmware's `ConOut`. So on any UEFI x86_64 machine a nife
stick prints to the monitor with no driver of ours, right up to `ExitBootServices`. After that the
kernel takes over and the machine goes silent, which is exactly this milestone.

So the blocker on using those machines is this block and not milestone 242 (USB host and HID,
because on commodity hardware the keyboard is not a UART). A boot test needs output, not input.

Two qualifications, both worth stating rather than discovering:

- The Apple Silicon MacBooks are not in this fleet. They boot non-Apple kernels legitimately, and
  `notes/target-hardware.md` records that Asahi Linux is built on a documented permissive-security
  mode with an Apple-signed `m1n1` payload. But they have no UEFI, so a `BOOTX64.EFI` stick will not
  start one. That is a port, filed in that note as "possibly the fifth", not a boot. An Intel
  MacBook is in the fleet. Worth knowing for later: m1n1 offers a serial console over USB-C. That is
  more than any commodity x86 machine here offers.
- Milestone 195 (finish the UEFI boot path) created a question each machine answers for itself.
  `PHYS_START` is 32 MiB, not 1 MiB, because OVMF holds ACPI NVS and its own allocations across
  the low range. Whether a given machine leaves 32 MiB free is that machine's answer. The loader
  prints which range it wanted and which descriptors are in the way, rather than `Load Error`.

The cheap experiment this makes available: build a stick, boot one of those machines, and read
what the loader says on the screen. Even with the kernel silent afterwards, that establishes three
things. Does the firmware find the stick? Does Secure Boot refuse it? And what does the memory map
look like away from OVMF? A loader that prints and a kernel that then goes quiet is this milestone confirmed by
evidence rather than by reasoning.

That experiment is now worth more than it was when this was written. The procedure for it is
`notes/serial-less-output.md`'s bench section. The kernel no longer goes quiet: it clears the screen
and prints its whole tour there. So the same stick, on the same machine, now distinguishes four
outcomes rather than two, and each is a different bug. The firmware does not find the stick. The
loader refuses and says why. The screen clears and stays black (the kernel armed its console and
died after). Or the tour prints.
