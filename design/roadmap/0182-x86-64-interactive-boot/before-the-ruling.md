# Before §149 was ruled: an appendix to milestone 182

[Milestone 182 (x86_64's own interactive-boot entry point)](../0182-x86-64-interactive-boot.md)
keeps what was built, the third `script/swish-check` leg, the `BUGS` and the follow-on work. This
appendix holds parts of the record that two things later overtook. One is the ruling on §149 (may
the kernel answer on an endpoint, where §121 leaves no userspace holder?). The other is milestone
299 (the x86 port-range capability). The first part is the superseded claim of milestone 177 (wire the
graphical terminal stack into the real interactive boot). The others are the two faults in the
2026-09-14 transcript, the empty-slot correction, and what each of §149's options would have taken
from the tree as it stood.

## The only route, as milestone 177 first believed it

Milestone 177 corrected its own earlier claim that this piece was independent of the others.
x86_64 has no fallback UART path at all. Under DECISIONS §121 (what a device capability is when the device has no page) it is
permanently kernel-resident. So this milestone's only possible route to an interactive shell is through the graphical stack milestone
177 builds, not a plain-console alternative the way aarch64/riscv64 each have one. *(Superseded by
the 2026-09-09 amendment above; kept as the record of what was believed.)*

## The two faults in the 2026-09-14 transcript

The parent block carries the transcript of the first x86_64 hand-over, QEMU `q35` with one core.
What those two faults are, identified rather than assumed. A temporary dump of the bytes at
each faulting `rip` and of the user stack, symbolized against the x86 ELFs: both are
`user_mode_runtime::trap`. Thread 6 is `input` (`rip` is `trap`, called from `input::uart::rx_pending`
via `input::drain`), and the other is `console` (`trap` at `0x400398` in that binary, its
`uart_put`). Both are the `x86_64` arms those programs have carried since milestone 161, which trap
on first use because a ring-3 process cannot reach port I/O (DECISIONS §121). They are the
progenitor's children, so the progenitor built the console server, the line discipline and the
input driver before they stopped, and it is still alive after the bound. Under OVMF with two cores
(`cargo xtask uefi-boot`) the same boot also prints the kernel's `capability slots: 13 of 24 at
peak`, which is the progenitor's own table gauge.

## A correction found by measurement: the empty slot

The first version left slot 1 empty on x86_64, because slot 1 is the UART page on RISC-V and
there is no page to grant here. The boot then failed in a way that had nothing to do with §149:
the progenitor trapped in `must(build_child(swish))` (`crates/system_initializer/src/lib.rs:1374`).
A temporary print of every syscall error named it: `MAP_INTO` of the shell's output page returned
`WrongObject`. The progenitor's first-free retype had put `term_out` into the empty slot 1, and once
the drivers existed it deleted slot 1 as `uart_dev`, deleting its own output page.

So slot 1 holds a placeholder on x86_64: a freshly zeroed frame, `READ | GRANT`, which nothing
else owns and which the console and input drivers' x86 arms trap before touching. It is marked in
the code as the foot gun it is. A device capability over physical page zero, which is what
`spawn_progenitor`'s aarch64-shaped path grants on x86, was refused for this: page zero is real
memory, and the console server would have written to it.

## What §149 would take from here, per option

Priced against the tree as it stands, so the ruling can be made without reading the diff.

- Option 1, a kernel thread answering on an endpoint (recommended in §149). Smaller than §149
  reads, because the progenitor already has the shape. Its graphical branch (milestone 177,
  `has_graphical`) takes three kernel-created capabilities at slots 10 to 12 and builds no console
  and no input driver at all. They are `disp_term_ep`, which `line_editor` `CALL`s with `OP_WRITE` after
  filling `disp_term_page`, and `kbd_ep`, which a keystroke source `CALL`s with `OP_BYTES`. Under
  option 1 the kernel would run two kernel threads and grant the three slots. One would serve
  `OP_WRITE` by copying the page to COM1 through the direct map. The other would take COM1's receive
  interrupt and `CALL` `kbd_ep`. The kernel already has the IPC for a kernel-side server
  (`sched::ipc_recv_cap`, `sched::ipc_reply`, `sched::ipc_call`). No change to the progenitor,
  `line_editor` or `swish`, and no syscall surface. What it costs is §149's own objection, a
  message parser in the kernel, and one naming wrinkle: the slots are called `disp_term_*` and would
  hold a serial terminal.
- Option 2, a console syscall. One or two new syscall numbers (the surface is four today), the
  `x86_64` arms of `components/src/console.rs` and `components/src/input.rs` rewritten to call
  them, and a blocking read, which needs a kernel wait queue for keystrokes. The progenitor would
  be unchanged. Every program on every architecture is then written against a surface that exists
  for one of them.
- Option 3, the graphical stack. `boot_graphical_terminal` is already portable and
  `pci::find_gpu_device` is PCI, which x86 has. The q35 runner would need a `NIFE_GPU` block
  (`virtio-gpu-pci` behind VT-d) and a keystroke source: the UART fallback in
  `boot_graphical_terminal` is `input`, which traps on x86, so a virtio keyboard is required too.
  xenon has neither device, and `board_console` cannot read a monitor, so the bench loses its
  serial path.
