# §34 appendix: ZFS and XFS, and why RedoxFS is better-shaped than its size suggests

The survey behind [§34 (RedoxFS is the primary filesystem, on three conditions)](../0034-redoxfs-primary.md)'s 2026-07-30
amendment on ZFS and XFS. Moved out of that page on 2026-10-12 (UTC) under [§212 (a prose
budget)](../0212-a-prose-budget-for-every-document.md). The main page is complete without this one,
which exists to verify or challenge it. No ruled word changed.

## Amendment (2026-07-30): ZFS and XFS, and why RedoxFS is better-shaped than its size suggests

calef asked whether OpenZFS is best in class. For a backup server, yes: end-to-end checksums with
*repair*, snapshots, `send`/`recv`, scrub, no RAID write hole. And it is unavailable to us, for a
reason worth distinguishing from "too big": at roughly 400k lines of C it would *be* the project, but
more decisively **it is not a component you confine, it is a subsystem you host.** OpenZFS needs a
Solaris Porting Layer (kmem, mutexes, condvars, taskqs, VFS integration, page cache); §31's seam
confines a narrow interface, and ZFS's interface to its host kernel is enormous. Its ARC also expects
gigabytes where the VisionFive 2 has 4 to 8, and CDDL is worth checking against our license posture
before publication.

XFS: excellent, aimed at another problem, and weakest exactly where this application cares. XFS v5
checksums metadata only; file data is unprotected, so bit rot in a backup is invisible until
restore. Its strengths (allocation groups for parallel metadata, extents, large-file throughput) target
a workload we do not have. It is journaling rather than copy-on-write, has no snapshots, cannot shrink,
and is ~100k lines of C more entangled with Linux VFS than ext4.

The finding that reframes the comparison. RedoxFS is architecturally ZFS-shaped where it counts:
copy-on-write, transactions, and checksums stored in the parent `BlockPtr` and verified on every
`read_block` (seahash recomputed, `EIO` on mismatch). The hash living in the *pointer* rather than
the block makes it a Merkle tree, which is ZFS's design and is strictly stronger than a header
checksum, because a header checksum cannot catch a misdirected write. It also encrypts. So RedoxFS
beats XFS on data integrity and matches ZFS's integrity architecture, at a fraction of the size.

What it genuinely lacks: snapshots, self-healing, scrub, compression, RAID-Z.

And on a single disk, ZFS's headline advantage largely evaporates, which is the observation that
should settle the anxiety. Self-healing requires redundancy; with one drive ZFS also only *detects*
corruption. calef's topology is a single USB drive, so the real gap is snapshots, scrub and
compression rather than integrity. Scrub is buildable on what exists today: read every block, let the
checksums verify themselves, report failures. That is a small program, not a filesystem feature.

**So the direction is snapshots, not migration.** Copy-on-write makes them tractable, and it is the one
missing property that matters for backups. If ZFS-class *redundancy* is ever wanted, that is a
multi-disk story and nothing within reach provides it.

The pattern across the whole survey, now covering ext2, ext4, XFS, btrfs, ZFS, F2FS, FAT and
littlefs: every mature filesystem with the properties we want is a large C codebase entangled with a
Unix kernel. That is not a coincidence to route around; it is why the choice was a small Rust
filesystem with the right architecture rather than a large one with more features.
