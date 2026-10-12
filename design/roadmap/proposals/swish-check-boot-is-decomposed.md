---
status: PROPOSED
raised: 2026-10-12
milestone_dependencies: 847
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# `swish_check_boot` is decomposed into its phases

Raised by the lane of milestone 847 (the swish-check harness is split along its seams) on
2026-10-12 (UTC). That milestone moved `swish_check_boot` whole into
`xtask/src/swish_check/boot.rs`. It is 1,051 lines in one body.

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
