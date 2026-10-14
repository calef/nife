---
status: BUILT
raised: 2026-07-30
built: 2026-07-30
---
# 37. Prove RedoxFS's crash consistency (DECISIONS §34, condition 1)

Built 2026-07-30, both ISAs. Condition 1 of §34 (RedoxFS is the primary filesystem) is met. The
claim it earns is narrower and sharper
than the one it replaces (DECISIONS §34's amendment carries the full statement).

What is measured, on the host, exhaustively: 93 fault points across a seven-operation workload. Each
one is a power cut with the process gone, and the recovery a fresh mount. Every one recovers a state
that really existed. The prefix never goes backwards as the cut advances, and at the last cut point
nothing is lost. The same sweep with the interrupted write torn at four offsets: 372 points, same
result. A separate sweep models a device that *lies* (acknowledges a write it never persists, then
carries on). It gives 186 damages, 112 recovered, 74 refused at the mount or the read, and zero
silently wrong. That is the honest limit and the honest guarantee in one number.

The controls, which is what makes the rest mean anything. With the header ring's older generations
removed, **92 of the 93 fault points do not mount at all**. A commit torn at 2048 bytes fails
`Header::valid()` while the previous generation's slot stays valid and older. And the injector
caught a bug in the *harness* first, which is the best evidence it bites. Nine fault points looked like
filesystems that never existed until it turned out `snapshot` was reading `EIO` as "the name is
absent".

On device, on its own disk on both ISAs: the FS server is killed one block write into its second
transaction. That block is torn in half by a real virtio write. A different FS-server process mounts
what it left behind through the same block server and reads the file back whole.

In brief. Inject the failure a copy-on-write filesystem exists to survive, and measure whether it
does. Torn writes (a block partially written), dropped writes (a write the device acknowledged and
did not persist), and a kill mid-transaction. Then reopen with the same `cleanup: true` header-ring
replay the FS server always mounts with. Assert the filesystem is consistent and every acknowledged
write is either wholly present or wholly absent. The seam is `IpcDisk` and the block server, which
sit between the engine and the device and can drop or truncate a write deliberately. The sans-IO
core already runs on the host against a real image, so most of this is host-testable in milliseconds
and only the device-level kill needs QEMU. Includes the negative control that makes the rest mean
anything. The injector must be shown to actually corrupt something when the replay is disabled

Why it matters. the condition that decides whether §34's label is earned. Crash consistency is
RedoxFS's central selling point and the reason it beat ext2. We currently assert it on the strength
of the upstream design description rather than any measurement. That is a claim of exactly the kind
this project's rules forbid. It is the first thing a skeptic asks a filesystem. Until it passes,
the docs say "designed for crash consistency" and never "crash consistent". Note this is a gap in
**our harness, not in RedoxFS**. No candidate engine's crash consistency is tested here, so
switching engines would not address it

## Follow-on

- **Recorded.** In `design/decisions/0034-redoxfs-primary.md`, in the amendment this milestone earned:
  RedoxFS's `Disk` trait has no flush and no barrier, so write ordering is the device's job. Our
  block server issues no `VIRTIO_BLK_T_FLUSH`. On real hardware with a volatile write cache the
  durability of the last acknowledged write is the device's word rather than ours. Every block
  pointer carries a checksum, so the failure is an error and never a wrong answer. That is what the
  74 refusals and the zero silently-wrong count measure.

## Index row

decides whether §34's "primary filesystem" label is earned
