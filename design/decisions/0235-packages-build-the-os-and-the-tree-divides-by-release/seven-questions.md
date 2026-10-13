# §235's seven questions

Moved verbatim from [`0235-packages-build-the-os-and-the-tree-divides-by-release.md`](../0235-packages-build-the-os-and-the-tree-divides-by-release.md) on 2026-10-13 (UTC) to bring that
page under §212 (a prose budget)'s 3,000-word cap by the whole-file count. The page keeps the
heading, and the text below is unchanged.

## The seven questions

1. What else was considered? Each fork's table is the answer. Also refused: proofs as a division,
   because it separates a proof from its subject. Refused: a shared blob store built ahead of a ruling,
   because U2's collection rule is the one mistake that bricks both slots.
2. What does the tree already do? The image is a closure pinned by `TRUST_ROOT` (milestone 104).
   User packages are archives, activated by grant (§197, §208). One out-of-tree build exists,
   `helpers/build-ripgrep.sh`.
3. Prior art: Fuchsia, Nix, FreeBSD pkgbase, ChromeOS, OSTree and bootc, and Genode's depot, read
   2026-09-26. What to take and refuse is in the note.
4. Is the premise true? Partly, and three facts above qualify it. The kernel cannot release
   independently under T1, whatever the repository shape.
5. What does each option cost? Measured where a command could: image size, edges, commit coupling
   and the SDK closure. The rest is named as unmeasured.
6. How reversible, and who has acted on it? Nobody outside this machine has fetched a package,
   installed a slot, or built against the ABI. Each ruling becomes expensive at its own first
   outside consumer.
7. Would we choose the same if every option cost the same? The recommendations are about order, not
   effort. P1 goes first because every Fork 1 option needs it, not because it is small.
