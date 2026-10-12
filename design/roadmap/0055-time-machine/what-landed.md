# Milestone 55 appendix: what landed before the retirement

An appendix to [milestone 55 (Time Machine: SMB3 with Apple's extensions, and
mDNS)](../0055-time-machine.md), moved out of that page on 2026-10-12 (UTC) under [§212 (a prose
budget)](../../decisions/0212-a-prose-budget-for-every-document.md). The main page is complete
without it. This is the block's record of the SMB half as it was built between 2026-08-17 and
2026-08-19, before the premise was retired on 2026-08-30. All of the code described here went with
the SMB implementation; notes/smb.md records it.

## The `AAPL` create context and the Time Machine flag

The `AAPL` create context and the Time Machine flag landed 2026-08-17 (pull request #292). It is the
piece macOS refuses the share without. `crates/smb_proto` grew the create-context chain ([MS-SMB2]
section 2.2.13.2) and the `AAPL` tag's meaning as two modules, host-tested. The QEMU gate's prober
now hung the context off its first CREATE the way a Mac does, and checked each claim separately, on
both ISAs.

What the server claimed:

- `UNIX_BASED`.
- `FULL_SYNC`, which is `fruit:time machine = yes`, the one bit that makes macOS willing to hold a
  backup.
- The model string `TimeCapsule`.

What it declined to claim, with a reason each: `READ_DIR_ATTR`, `OSX_COPYFILE`, `NFS_ACE`,
`CASE_SENSITIVE` and `RESOLVE_ID`. notes/smb.md's Apple section is the table.

## Two items that changed shape on inspection

Two of the block's five remaining items changed shape on inspection, which was worth more than the
code.

`posix_rename` is not work. Samba's `fruit:posix_rename` switches on two behaviors: renaming onto an
existing name, and renaming a file that is open. The first is already `fs_proto::fs::RENAME`'s
documented semantics. The second cannot fail here, because this server enforces no share modes at
all, so there is no sharing violation for POSIX semantics to be an exception to. The real defect next
door is that `ReplaceIfExists = 0` is ignored: a rename that clobbers when the client asked it not to.
[path-to-built.md](path-to-built.md) follows that defect to §129 (whether `RENAME` grows a
`NOREPLACE` flag).

The metadata fork was a smaller question than the block assumed. "We have no extended attributes at
all" was true when it was written and was not by then. Milestone 57 (partitioning and formatting a
real drive, and extended attributes) built xattrs into `fs_proto` (ops 14-17), the FS server and the
store. So the layer that made `streams_xattr` look expensive existed. What was missing was the SMB
half of alternate data streams: a stream name in a CREATE path, `FileStreamInformation`, and
`FILE_NAMED_STREAMS` in the volume attributes.

The stream-versus-sidecar choice was still open, and still a decision about what lands on disk; it
was just no longer a stack-deep one. It was written up as §99 (where Apple's metadata lands: stream
or sidecar) and waited on calef from 2026-08-18, with four options and their measured costs.

The finding that block's reader most needed: none of it is on the Time Machine path. A backup writes
a sparse bundle, which is directories and band files with no extended attributes and no forks. So
this is a file-server feature milestone 55 inherited by proximity rather than a Time Machine
requirement. The second finding is that the sidecar option is not something this server implements.
macOS's own VFS writes `._name` files when a share does not claim `FILE_NAMED_STREAMS`, so the tree
was already running it, at zero lines.

The status did not move. That lane answered the fork instead of building it, which is what it was
asked for. 55 stayed PARTIAL until a decision landed and something was built on it.

What then remained of the milestone: Apple metadata (the choice above, then the SMB stream surface)
and the first contact with a real Mac.

## Durability: `FULL_SYNC` made true

The durability gap closed 2026-08-18 (pull request #311), and the way it closed is worth more than the
code. The open question was whether to keep claiming `FULL_SYNC` when the stack did not back it. The
answer was to make the claim true instead of answering the question.

Two opcodes on two contracts:

- `fs_proto::blk::FLUSH` (op 4) is a real `VIRTIO_BLK_T_FLUSH` the block server waits for the device
  to complete. It is gated on `VIRTIO_BLK_F_FLUSH` being offered, so a device that cannot flush
  produces `EOPNOTSUPP` rather than a quiet success.
- `fs_proto::fs::SYNC` (op 19) is the file-service verb behind SMB2's `FLUSH`. It takes any handle the
  server minted, requires `dir::WRITE`, and is refused with `EROFS`.

What makes it a gate rather than a code path is where the witness stands. Both verbs answer with a
count of completed device flushes instead of a zero. The QEMU suite's in-guest verifier, which has
synced nothing itself, can therefore find the count already advanced when it first asks. Nothing but
the SMB server answering the host prober's `FLUSH` over TCP can have moved it. Then it syncs twice and
requires the count to move again, which separates a device round trip from a server answering a
constant. Both ISAs.

What was left is narrower, and is recorded where a reader meets the claim. The sync is device-wide
rather than per file, and nothing fences, because `fs_proto` has no ordering primitive. A device that
lies about its own flush is outside anything a protocol can check. notes/fs-server.md's
crash-injection table records the same limit from the other side.

## Throughput: the 64 KiB transfer reaches the mount

The 64 KiB transfer reached the mounted share 2026-08-19. It was the first thing in the block that
made a backup faster rather than possible.

Milestone 138 (close the read gap: a 4 KiB request must stop moving 128 KiB) step 3 grew the file
contract's transfer to 64 KiB and measured 8.02x on a sequential write. None of it reached a mount.
`smb_server` chunked every SMB read and write into `fs_proto::PAGE`-sized requests, and so turned a
Mac's 64 KiB write into sixteen 4 KiB ones. Both clamps then read `fs::TRANSFER_MAX` from the
contract. The kernel maps the whole channel at the adapter's `FS_VA`, because nothing checks that a
client asked for no more than it mapped.

Measured through the host SMB prober rather than through `fs_proto` (`bench/smb-throughput.sh`, two
rounds, aarch64):

| direction | before | after | ratio |
|---|---|---|---|
| write | 0.065 MiB/s | 0.31 MiB/s | 4.8x |
| read | 0.15 MiB/s | 0.36 MiB/s | 2.4x |

Most of the write speedup arrived and about half the read speedup did. Where the rest went is a
different contract. `socket_proto::DATA_MAX` is 4080 bytes, so every 64 KiB SMB message still crossed
the socket contract about seventeen times in each direction. That became the dominant cost of a
transfer. It is step 3's defect one contract over, and notes/smb.md's `BUGS` carries it with its
promotion trigger.

SMB's own ceiling binds too. `smb_proto::MAX_TRANSACT` is 64 KiB for `MaxRead`, `MaxWrite` and
`MaxTransact`. Raising `fs::TRANSFER_PAGES` past 16 buys this path nothing without raising that as
well.

What that is worth to a backup, stated at the depth it was measured. The write path alone goes from
the record-level sweep's 17.6 hours for a 100 GiB first backup to 40 minutes at step 3's 42.77
MiB/s. This change is what put a Mac's bytes on that path. No end-to-end hours figure is offered, on
purpose. The table above is a debug build under QEMU user-mode networking, the wrong instrument for a
wall clock; what transfers from it is the ratio rather than the rate.

The status did not move. This was throughput on a share that already worked. What remained was
unchanged, and the honest reading is that the milestone got cheaper to use rather than closer to
done.

## Identity, inherited from milestone 54

The identity substrate arrived 2026-08-17 (milestone 54 (a network file service a Mac can actually
mount), pull request #274). It was the block's other SMB-side prerequisite. It is also the reason a
Time Machine target could not have been serious before: a backup share that admits guests is a share
anyone on the segment can rewrite.

What this milestone inherited:

- a share that requires an NTLMv2 proof;
- an `Authenticator` seam in `Share`'s shape;
- a server that authenticates while holding no key.

What it inherited as a problem is that the boot a person runs still admitted guests, because nothing
could tell a running system a password. A Time Machine target is the first thing in this tree that
genuinely needs a provisioning path. That is the shape of milestone 56 (secrets, credentials, and the
entropy to make them safe), rather than either of these two blocks'. See notes/smb.md's BUGS.

## Nothing here has met a Mac

QEMU's user-mode networking cannot carry multicast to the host, so `dns-sd -B` finds nothing under
the emulator by construction. IGMP snooping, forwarding TTLs, a live segment's mDNS traffic and a real
querier all need hardware on the family network.

The lane's gate did accidentally prove the segment exists. Its injected query escaped slirp, reached
the real router, and came back NATed with the very records the test expected. That was a false green,
which the gate caught with a source-address filter and recorded.
