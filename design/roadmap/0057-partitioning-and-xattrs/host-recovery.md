# Milestone 57 appendix: reading the drive from a host, and no encryption

An appendix to [milestone 57 (partitioning and formatting a real drive, and extended
attributes)](../0057-partitioning-and-xattrs.md), moved out of that page on 2026-10-12 (UTC) under
[§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md). The main page is
complete without it. This is the block's account of the host recovery tool, built 2026-07-30, and of
calef's decision of the same day against filesystem-level encryption on the backup volume.

## Reading the drive from a MacBook or a Linux host: BUILT 2026-07-30

The question that makes a backup credible rather than merely functional: the board is dead, can I
get my data? calef asked it, and the answer turns out to be that we disabled the feature.

Correction to this section's original heading, which said "which upstream already solved". It
half did. Upstream solved *mounting* (FUSE), and that is the path we deliberately do not take.
Nothing upstream ships extracts. `redoxfs-ar` is an archiver that only writes, and it creates the
filesystem as it goes, so it cannot even be pointed at an existing image. `redoxfs-clone` copies an
image to another image, and `redoxfs-mkfs` and `redoxfs-resize` are what their names say. The
extraction verbs did not exist and are now ours. See notes/host-recovery.md.

`vendor/redoxfs` already ships `src/mount/fuse.rs`, a `redoxfs` mount binary, and `redoxfs-ar`,
`redoxfs-clone`, `redoxfs-resize`. Upstream's default features are `["std", "log", "fuse"]`. Our host
tool depends on it with `default-features = false, features = ["std"]`, so `fuse` is excluded by our
own choice and re-enabling it is a feature flag plus the `fuser` dependency.

What shipped: `redoxfs_host ls IMAGE [PATH]`, `cat IMAGE PATH`, `extract IMAGE PATH DEST`, plus
`import IMAGE HOST_DIR` on the write side (upstream's own `redoxfs::archive`, which is what makes
the round-trip test read something our writer did not produce). Paths resolve from the image root and
`..` is refused, the same rule the FS server enforces on the wire. `fuse` is still off and `fuser` is
still not a dependency.

Two things the build found that the plan did not predict. First, the recovery reads must not
write to the image, and the engine makes that easy to get wrong twice. `FileSystem::open`'s
`cleanup` pass tidies allocations. `Transaction::read_node` updates atime only when the last read
was more than an hour ago, which passes every test on a freshly made image and then dirties the
first real backup you touch. Read-only opens plus `read_node_inner` fix both, and the test hashes
the whole image across a read. Second, the operational rule can enforce itself. `Header::valid`
checks the format version first, so a mismatched reader sees no valid header anywhere. The engine
then says ENOENT, which reads as "no such file or directory" about a disk you are holding. The tool now
reads the signature and version straight off the disk when an open fails and names the mismatch,
with a test that forges it.

Three paths, and they are not equally good:

| Path | Cost | Verdict |
|---|---|---|
| Extend `tools/redoxfs_host` with `ls` / `cat` / `extract` | Small; the engine already links there with `std` | DONE 2026-07-30, plus `import`, `mkfs`, `put` and (2026-08-01) `xattr`. No FUSE, no kernel extension, no root, identical on macOS and Linux. The thing you want at 2am with a dead board. Upstream's `redoxfs-ar` did not cover it: it only writes |
| Linux mount via the `fuse` feature | A feature flag | Nearly free, and upstream maintains it: it is how Redox developers work with images |
| macOS mount via macFUSE | A third-party system extension plus reduced security mode on Apple Silicon | Works, genuinely awkward. Optional convenience, not the recovery story |

This removes the strongest argument for switching filesystems. Interop was the one thing ext4
genuinely bought that RedoxFS appeared not to; it turns out RedoxFS buys it too, with a tool instead
of a kernel driver.

The operational rule that follows: keep the recovery tool, or its exact source pin, with the
backup. We are pinned at 0.9.1 (on-disk format version 8) and a reader must match the on-disk
format version. A backup readable only by software you no longer have is not a backup. It is written up,
with what the off-site copy has to carry, in notes/host-recovery.md. That note also draws the
consequence for future pin bumps: a bump to a different on-disk format strands every image already written, so it
is a migration, not an upgrade.

The same-engine objection is weaker than it looks and is recorded so nobody relitigates it: yes, the
reader shares any bug the writer has, but that is true of every filesystem (`e2fsprogs` shares lineage
with the kernel driver). The real risk is an *undocumented* format, and RedoxFS is open source with
upstream tooling.

## Decided: no filesystem-level encryption on the backup volume

calef, 2026-07-30: "If I'm struggling to get the data off, I'm not all that worried about somebody
else getting it." RedoxFS supports encryption (`src/key.rs`, and the read path calls `decrypt`), and
we are deliberately not using it here.

It is the right call, and for a stronger reason than the one given. Encryption belongs at the Time
Machine layer, and calef's own setup instructions already offer it ("Optionally enable Encrypted
Backups"). The Mac encrypts before anything is sent, so the server never holds plaintext and recovery
uses the client's key rather than the server's. Filesystem encryption underneath would be
redundant, while putting a key on the machine most likely to be compromised. It also strengthens
milestone 55's claim: a compromised SMB adapter leaks ciphertext.

Two consequences. The recovery tool needs no key handling at all, which is a real simplification.
And if Time Machine encryption *is* enabled, recovery then depends on that password, which relocates
the "can I get my data" risk rather than removing it, so the password belongs wherever the family's
other credentials live rather than only in one Keychain.
