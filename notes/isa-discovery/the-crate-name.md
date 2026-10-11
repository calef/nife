# How `isa` became `machine_discovery`

An appendix of [ISA discovery](../isa-discovery.md), moved from its section "Naming" on 2026-10-11
(UTC) under §212 (a prose budget). The parent keeps the result: calef renamed the crate on
2026-08-23. This file keeps why the first name was protected, why the protection did not fit, and
the prior art that chose the second.

## The first name, and why its protection did not fit

The crate was `isa`, settled by calef on 2026-08-03 as an abbreviation in the group `DECISIONS.md`
§39 protects: a standard term of art a reader already knows from outside this project, like `elf`,
`dtb` and `pci`.

That protection turned out not to fit. `isa` collides with the well-known ISA bus, a 1980s PC
expansion standard. It also only partially described the crate's scope. The crate is a boot-time
"what machine is this?" record built from device-tree claims, targeted measurements, and
ID-register reads, not just instruction-set-extension discovery.

## The rename, and the prior art behind it

calef renamed it to `machine_discovery` on 2026-08-23, during a naming review of every crate the
kernel directly depends on. Verified prior art settled the new name. Linux ARM's
`setup_machine_fdt()`/`struct machine_desc`/`DT_MACHINE_START` and FreeBSD's `hw.machine` both use
"machine" for this same device-tree-plus-probe scope.

That is why `machine_discovery` won over two alternatives:

- `hardware_discovery`, too generic, which collides with `pci`'s and `dtb`'s own territory.
- `identcpu`, FreeBSD's actual name, but narrower: register-only, with no device-tree layer.
