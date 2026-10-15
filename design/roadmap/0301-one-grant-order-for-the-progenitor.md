---
status: SUPERSEDED
raised: 2026-09-08
superseded_by: 166
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 301. One grant order for the progenitor, on every board

Milestone 266 (one progenitor, on all three architectures) unified the first process into one program and could not unify the one thing left
under a `cfg` inside it.

Nothing outside this repository ever acted on the slot numbers, which is what made
this cheap to settle.

This is a numbered block rather than a proposal because `design/roadmap/proposals/` carries exactly
one status by design (`helpers/roadmap_proposals.py` matches `PROPOSED` and nothing else). A proposal
cannot be retired in place; the terminal vocabulary lives on numbered blocks. Its status is
`SUPERSEDED` rather than `REMOVED`, because this was never built and did not need to be: other work
answered it. Milestones 54 and 55 both record that the vocabulary has no word for their own case
("built, then removed"). That question is theirs and is deliberately left open here rather than
papered over with a shared word.

## What settled it

Milestone 166 did precisely what *What it would take* below asks for. It split milestone 19d's test
roles out of the boot path into `spawn_hello`. The kernel no longer hands the interactive system a
report endpoint nothing receives on and a test interrupt no component waits for. Both boards now
number the interactive capabilities identically from slot 0, `components/src/progenitor.rs`'s two
`BootEndowment` tables became one. The `cfg` this proposal was written about is gone.

The one genuine per-architecture difference left is the *shape* of the console authority, a device
page on aarch64 and riscv64 against an `x86_64` `PortRange` (milestone 299 (the x86 port-range
capability)). That is not a slot number, and it is not what this asked about.

The body below is the proposal as filed, an account of what was proposed and why rather than a
description of the tree. Its `spawn_progenitor` and `riscv_shell_boot` are `spawn_hello` and
`boot_progenitor` in the tree.

## In brief

`components/src/progenitor.rs` holds two `BootEndowment` tables. They differ because the two kernels grant
the boot capabilities in a different order:

| | aarch64 (`kernel::user::spawn_progenitor`) | riscv64 / x86_64 (`kernel::user::riscv_shell_boot`) |
|---|---|---|
| untyped | 0 | 0 |
| the kernel's report endpoint | 1 | absent |
| UART registers | 2 | 1 |
| the 19d.2b test interrupt | 3 | absent |
| UART receive interrupt | 4 | 2 |
| clock page | 5 | 3 |
| configuration page | 6 | 4 |
| file service, shared page | 7, 8 | 5, 6 |
| virtio-rng trio | 9, 10, 11 | 7, 8, 9 |
| graphical terminal trio | 12, 13, 14 | 10, 11, 12 |

Every row after the first is displaced by exactly the two capabilities aarch64 grants and the
interactive system never uses: a report endpoint nothing receives on, and a test interrupt no
component waits for. They are there because **aarch64's boot path is shared with milestone 19d's
test roles**, which enter `hello` through the same function with the same endowment. Their slot
numbering is written into `fixtures/src/hello.rs`'s role catalog and into six `spawn_hello` tests.

## What it would take

Grant the two test capabilities *after* the interactive set on aarch64, or grant them only for the
test roles, so both boards number the interactive capabilities identically from slot 0. Then one
`BootEndowment` serves every architecture and the `cfg` goes.

The work is not the kernel side. It is that 19d's roles read fixed slot numbers (`REPORT = 1`,
`TEST_IRQ = 3`, `UART_DEV = 2`) and each is named in `hello.rs` and asserted from the kernel's test
module. Renumbering them is mechanical and the blast radius is a whole milestone's worth of reading.
That is exactly why 266 did not fold it in: a boot failure and a renumbered test suite arriving in
one change is the ambiguity that milestone forbade itself.

## Why it is worth doing rather than living with

The `cfg` is data rather than code and it sits in one file beside the table it disagrees with, which
is the cheapest place to notice it. But it is still one architectural difference inside the first
process, in a milestone whose argument was that there should not be one. DECISIONS §19's failure
mode is precisely a fact that is true on one ISA and quietly not on another.

## What it is not

It is not a request to make the two kernels one function. `spawn_progenitor` and `riscv_shell_boot`
differ in what they *have* to do (a GIC and a PL011 device capability against a PLIC and an NS16550),
and that difference is rule 1 working. Only the order is arbitrary.

## Index row

`components/src/progenitor.rs` held two `BootEndowment` tables under a `cfg`. aarch64's boot
path was shared with milestone 19d (run a real workload)'s test roles and so granted a report
endpoint and a test interrupt the interactive system never used, displacing every slot after the
first. Milestone 166 (one boot loader, reached two inconsistent ways) settled it by splitting those
roles out into `spawn_hello`. Both boards number the interactive capabilities identically from
slot 0, the two tables are one, and the `cfg` is gone.
