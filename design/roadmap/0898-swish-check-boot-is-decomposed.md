---
status: BUILT
raised: 2026-10-12
built: 2026-10-15
promoted_from: swish-check-boot-is-decomposed
milestone_dependencies: 847
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 898. `swish_check_boot` is decomposed into its phases

*(The title and slug are provisional.)*

`swish_check_boot`, in `xtask/src/swish_check/boot.rs` since milestone 847 (the swish-check harness
is split along its seams), is 1,051 lines in one body.

The body already has phases a reader can see. It builds the images and the disk. It chooses the
devices each leg's runner attaches. It starts QEMU and the transcript reader. It types each line
and waits for its echo and prompt. Then it reads the transcript's verdicts. Each phase could be a
function, with the state they share made a struct.

This is a refactor, not a move. Only `script/swish-check` on all three architectures proves it,
which is why milestone 847 left it alone. Milestone 843 (the system-initializer crate root is split
along its seams) gives the same reason for its own `boot`.

Reuse: not applicable; this restructures one function.

Done when no function in `boot.rs` is over 300 lines. `script/swish-check` must pass on aarch64,
riscv64 and x86_64, and both falsification records under `boot` must still turn their lines red.

## What was built

The decomposition landed 2026-10-13 in #2060 (`maintainer/long-function-paydown-xtask`), as the
work of milestone 888 (a ratchet on function length, beside the one on file length) outside that
milestone's scope. `swish_check_boot` went from 761 code lines to 79 and `usb_keyboard_boot` from
159 to 44, each phase its own function with the shared state on `Leg` and `Console`, the shape
this block proposed.

This lane proved the rest of the done-when on `0f22bc9a4`, 2026-10-15 (UTC). 38 functions
measured in `boot.rs`, the largest 82 code lines, none over 300. `script/swish-check` green on
aarch64, riscv64 and x86_64. `script/falsifications --sweep xtask` found the `caps` record red
at its line and the `installed/unvouched` record stale: its patch no longer compiled, because
the vouch decision had moved out of `spawn_service` into `resolve`, where `Wants::of` has no
`unvouched` in scope. The lane re-cut the patch to the same defect threaded through today's
shape, and the sweep then turned both records red at the lines their patches name.

## Follow-on

- **None.**

## Index row

`swish_check_boot`, 1,051 lines in one body, becomes one function per phase, with no function in
`boot.rs` over 300 lines and `script/swish-check` passing on all three architectures.
