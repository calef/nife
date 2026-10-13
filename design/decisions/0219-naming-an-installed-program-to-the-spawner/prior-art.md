# §219's prior art, read 2026-09-26

An appendix of [`0219-naming-an-installed-program-to-the-spawner.md`](../0219-naming-an-installed-program-to-the-spawner.md), which keeps the heading.

## Prior art, read 2026-09-26

- Fuchsia, `fuchsia.dev/reference/fidl/fuchsia.process`. `LaunchInfo.executable` is a VMO handle,
  and the child gets exactly the arguments, namespace entries and handles the caller staged
  (`AddArgs`, `AddNames`, `AddHandles`). D is this shape, with frames standing in for a VMO.
- Fuchsia, `concepts/process/program_loading`. The loader needs `ZX_RIGHT_EXECUTE` on the VMO, and a
  `#!` interpreter is looked up through the loader service. Also
  `reference/syscalls/vmo_replace_as_executable` and `job_set_policy`
  (`ZX_POL_AMBIENT_MARK_VMO_EXEC`), which are D2's precedent.
- Genode Foundations 23.05, "Read-only memory (ROM)" and "The init component": a binary is a ROM
  dataspace routed like any session. `fs_rom`'s README and Sculpt's `depot_deploy/child.h` show a
  downloaded package reaching init as a ROM served from the depot. How a user-built component is
  launched was not found on a page read.
- Plan 9 `exec(2)`, `9p.io/magic/man2html/2/exec`: `exec` takes a file name, and a `#!` interpreter
  receives the file's name. Whether the kernel or a library parses it was not confirmed.
- Redox, `relibc/src/platform/redox/exec.rs` and `redox-rt/src/proc.rs`: exec runs in userspace
  and takes a path or an open file (`Executable::InFd`).
- Linux, `execveat(2)` with `AT_EMPTY_PATH` executes an open file, and its `BUGS` records that a
  script run that way either fails or leaks the descriptor into the script.
