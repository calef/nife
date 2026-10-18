# Milestone 20 appendix: name the seams (HAL-prep without the HAL)

An appendix to [milestone 20 (a portable HAL, proven on a second architecture)](../0020-portable-hal.md),
under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md). It is the
part of milestone 20 that was safe to do before a second architecture existed. Milestone 20 and
milestone 161 (x86_64) built the split it prepared, so it is a record and plans nothing. DECISIONS warns against speculatively trait-ifying subsystems, because you build the wrong
abstraction before the requirements are known. That is squarely this: the generic/arch boundary in
`paging`, a device-discovery interface, and any HAL trait can only be shaped once RISC-V shows where
aarch64 was accidentally load-bearing. So this step does the subset that needs no guessing. It
*names and isolates* the seams; it does not *abstract across* them.

Deliverable, with where each item stands.

1. A concrete arch-boundary audit. Make notes/portability.md nife-specific: the exact
   files (`arch/aarch64/*`), the crates that are secretly machine-dependent (`paging` carries the
   aarch64 descriptor format; `dtb` is the device-tree discovery path), and the driver assumptions
   (`pl011`, `gic`, virtio-mmio are MMIO; semihosting is the test-exit). This is the map milestone 20
   executes against, and it is useful on its own as "what a port actually touches."
   *Done: notes/portability.md measures nife's own tree.*
2. The arch contract, written down. `arch/mod.rs` enforces its surface only by failing to
   compile. Document the required surface as a doc comment: the functions and types every arch module
   must provide. A list, not a trait, naming the seam without shaping the abstraction across it.
   *Not written as a list. `arch/mod.rs` states the boundary, and three architectures compiling
   against one re-exported surface is still what enforces it.*
3. Isolate the aarch64 format inside `paging`. Group the descriptor-bit encoding and the `Flags`
   constructors into one clearly-labeled module ("this is the aarch64 format; a second arch replaces
   this file"), beside the table/index/walk code. One crate, one arch, no generic interface yet: a
   clean, visible line for the eventual split, not the split itself.
   *Overtaken: `paging` holds the split itself, a `PageFormat` seam (`page_format.rs`) with
   `aarch64.rs`, `sv39.rs` and `x86_64.rs` behind it.*

Deferred to arch #2 (RISC-V): the generic-level-walk / per-arch-codec interface, a
device-discovery abstraction, and any HAL trait. Each needs the second implementation to avoid
encoding aarch64's accidents as "generic." The codec interface is the `PageFormat` seam above; the
device-discovery seam is milestone 176 (the x86_64 discovery seam's wide half).

**Why it was worth doing.** Modestly, and honestly. It is mostly documentation plus one clarity refactor, so it
will not feel like much. What it buys: the port map is written down, the arch surface is explicit
rather than discovered by compile error, and the `paging` split becomes mechanical when RISC-V lands.
It also makes the aarch64 code clearer today, which is its own small return even if no port ever
happens.
