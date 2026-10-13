# Milestone 523: what provoked it, and the cause measured

An appendix to [milestone 523 (moving the job-mix supervisor into
userspace)](../0523-the-job-mix-supervisor-in-userspace.md): what provoked it, and the cause,
measured rather than inferred. The page holds the investigation of the move itself.

`script/board-image --job-mix` builds a card whose kernel and archive are packed by one command, in
the order that exists so they cannot disagree, and `sealed_pair` then reported the pair unsealed.
`cargo clean -p kernel` did not change it, and neither did deleting the build directory.

The kernel compiles the right trust root and the linker drops it. Measured in this lane's
worktree, deterministic across three builds alternating the feature:

```text
board          -> 3 occurrences of `MEASURED BOOT REFUSED`
board,job_mix  -> 0
board          -> 3
```

The reason is one `cfg` and it is not a bug in the seal at all. Since milestone 268 (every
architecture boots the same way: describe the machine, test yourself, hand over) the measured-boot
refusal lives in `user::boot_progenitor`. `kernel/src/main.rs` reaches that call only under
`#[cfg(not(any(feature = "soak_test", feature = "job_mix")))]` (`:1640`, `:2018`, and
`riscv_hand_over`'s own `cfg_attr` at `:2195`). A job-mix build never calls it, so nothing
references the refusal, so the trust root and its message are dead code and the linker removes them.

`sealed_pair` then asks the only question it can ask, which is whether the kernel image *carries*
the digest (`haystack.windows(needle.len()).any(..)`, `crates/sealed_pair/src/lib.rs:445`). It finds
it absent, and prints a sentence that is false twice over: that the two files are from different
builds, and that the pair will halt at `MEASURED BOOT REFUSED` after the power cycle. The pair is
from one build and the kernel will halt at nothing, because it verifies nothing.

So the failure was never about job-mix specifically. `--soak` and `--bench` cards divert the
tour at the same three sites and will read `NOT SEALED` for the same reason. That finding is
separable from this milestone and is raised on its own as milestone 563 (a seal check that reads
bytes cannot see a check that was dropped),
`design/roadmap/0563-a-seal-check-that-reads-bytes-cannot-see-a-check-that-was-dropped.md`. It
is also recorded where a bench operator meets it, in `notes/job-mix.md`'s `BUGS` and in
`crates/sealed_pair`'s.

It is worth separating the two, because only one of them is urgent. A bench evening is blocked
by a tool that cries wolf, and that is an hour's work on the tool. A userspace supervisor is a
different and much larger claim, and the argument for it survives whether or not the tool is fixed.
