# Steps 4 and 2: the 16-block blk transfer and the metadata cache

This appendix belongs to [milestone 138 (close the read gap)](../0138-file-io-throughput.md). It was
moved out of that block on 2026-10-12 (UTC) under [§212 (a prose
budget)](../../decisions/0212-a-prose-budget-for-every-document.md), and the block is complete
without it. It holds the measurement and reasoning behind the last two steps built, both on
2026-08-19. The full accounts are in
[`notes/benchmarks/read-path-block-contract-and-metadata-cache.md`](../../../notes/benchmarks/read-path-block-contract-and-metadata-cache.md).

## Step 4: the blk contract carries 16 blocks, and the win is smaller than the block count predicts

Step 3's own residual pointed here. After it, a read was 74% single-block trips through
`fs_proto::blk`, one per filesystem block. notes/fs-server.md's `BUGS` section had already named the
~100 MiB/s ceiling that imposes. `fs_proto::blk::TRANSFER_BLOCKS` goes from an unwritten 1 to 16
(the same number as `fs::TRANSFER_PAGES`, for the same reason). The region the FS server and the
block server share grows to 64 KiB of contiguous pages. `IpcDisk` batches contiguous whole-block runs
into one blk `CALL`, and the block server issues one virtio descriptor for the whole batch instead
of one per block. Nothing in the request word's packing changed, the same shape step 3's change had:
56 bits below the opcode had never been used, and a block count fits in a handful of them.

### The measurement

10 interleaved rounds each (`sh bench/blk-transfer-sweep.sh 10 1 16`), on a shared, noisy machine.
Other lanes were building and testing concurrently, and `uptime` load was 15 to 21 throughout, well
above earlier sweeps' 3.6 to 9. The figure is the median of 10 rounds, because 8 of 10 landed within
6% of the established quiet `fs_read` baseline. The discipline is to discard load rather than
average it in, and with mostly-quiet rounds the median already does that job:

| phase | 1 block/CALL | 16 blocks/CALL | speedup |
|---|---|---|---|
| `fs_seq_write` | 1,544,228 ns | 1,335,376 | 1.16x |
| `fs_rand_write` | 2,101,313 | 1,539,488 | 1.37x |
| `fs_seq_read` | 811,724 | 527,420 | 1.54x |
| `fs_rand_read` | 848,706 | 546,541 | 1.55x |
| `fs_record_read` | 843,448 | 545,040 | 1.55x |

### Why this is nowhere near 16x

It is a finding about steps 1 and 3, not a flaw here. The record level is 1 (step 1): an 8 KiB
record, two blocks. RedoxFS walks its tree once per file-level `Server` call. It then loops over
however many records that call's transfer spans, each with its own `Disk::read_at`. A 64 KiB request
(step 3) spans 8 records. So step 4 batches each record's own body (2 blocks, one call instead of
two) but cannot batch across records, because each record's body is a separate call the engine
issues on its own.

Per record: 5 unbatchable metadata reads + 1 data call (was 2). That is 8 x 6 = 48 calls per
request, down from 8 x 7 = 56, a 1.17x call-count ratio that lines up with the measured write
speedups almost exactly. Reads show a larger ratio because a read's cost is almost entirely blk
calls, so the same eight eliminated round trips are a bigger fraction of it. The majority of what
remains, 5 of 6 to 7 calls per record, is the tree walk. That is exactly step 2's target and is why
it was taken next rather than left for later. The notes appendix has the arithmetic that predicts
the absolute savings from the per-block cost this block already measured twice.

### Crash consistency, step 4

Re-run and unchanged, 0 silently wrong, for the structural reason step 3's re-run gave (the
host-side model is below `IpcDisk`'s batching). The device-level crash injector keeps its own
one-block-per-`CALL` path unconditionally. So the real device test's coverage is unchanged by this
step rather than merely re-run.

## Step 2: a 64-block cache over the tree walk, the largest number in this milestone

`fs_server::CachedDisk` wraps `IpcDisk` in a small direct-mapped, write-through cache of
single-block reads: 64 slots (`CACHE_SLOTS`), about 257 KiB. Only a `buffer.len() == BLOCK` read
consults it; a record body (already batched by step 4) bypasses it. A write updates or invalidates
the written block's slot, only after the inner disk confirms the write landed.

RedoxFS's copy-on-write allocator never rewrites a live address in place. So the only way a cached
address's content can change is through that same write path, which is what makes a bare
write-through cache correct with no generation counter or fencing scheme. Six host tests cover
hit/miss, write-through freshness, short-write invalidation, slot-collision safety and multi-block
bypass, in milliseconds, no emulator.

### The measurement

8 interleaved rounds each (`sh bench/cache-slots-sweep.sh 8 1 64`), same machine, `uptime` load 15
to 19. Capacity 1 approximates "off" rather than being a true zero. The tree walk touches five
*different* blocks per call, so a one-slot cache thrashes and rarely survives to the next call. Both
`fs::TRANSFER_PAGES` and `blk::TRANSFER_BLOCKS` are at their shipped settings for both points, so
this isolates the cache's own marginal contribution:

| phase | 1 slot (~off) | 64 slots | speedup |
|---|---|---|---|
| `fs_read` (repeated inline read) | 210,490 ns | 9,474 | 22.2x |
| `fs_seq_write` | 1,387,898 | 936,583 | 1.48x |
| `fs_rand_write` | 1,487,736 | 1,087,904 | 1.37x |
| `fs_seq_read` | 514,419 | 329,168 | 1.56x |
| `fs_rand_read` | 542,605 | 331,732 | 1.64x |
| `fs_record_read` | 560,088 | 341,504 | 1.64x |

### Why the models were all low

The block's 4.7x model and its two re-pricings (3.2x after step 1, 1.33x after step 3) all assumed
the walk repeated *within* one file-level request. It does not: RedoxFS walks the tree once per
`Server` call, not once per record. So the real payoff is across separate requests to the same
handle. Every phase of the throughput bench makes those, and the measured numbers price them
directly.

### The "no cache anywhere" claim is retired

`fs_read`'s 22.2x retires the claim milestone 38 (filesystem throughput) demonstrated and both this
block and notes/fs-server.md stated as an architectural property. It was true of the build measured
then and it is not true of the build this tree ships now. `motd` lives inline in its node, so its
whole content rides on the tree walk with no separate record read. A warm cache answers it close to
the bare IPC floor this block already estimated at ~13 us. See notes/fs-server.md's own correction
for what still holds: a different file's first access, or any file's first access in a fresh
session, stays fully uncached.

`fs_test_client`'s "warm read" claim was wrong when milestone 38 corrected it and is right again
now, for a different, stated reason. See `fs_test_client.rs`'s own updated comment and
notes/fs-server.md's correction.

### All four steps combined

Measured head to head against milestone 38's original baseline rather than by multiplying
separately-measured ratios. `fs_seq_read` went from 1,509,270 ns per 4 KiB (2.68 MiB/s) to 329,168 ns
per 64 KiB (189.9 MiB/s): 70.9x. The remaining gap to buffered Linux (7,141 MiB/s) is the page-cache
gap this milestone was never scoped to close; see the block's frontier section.

### Crash consistency, step 2

Re-run and unchanged, 0 silently wrong. The host-side model is below `CachedDisk` too (it drives
`BlockDisk<Recording>` directly). At the device level the property that matters is argued rather
than merely re-run: [milestone 37 (RedoxFS crash
consistency)](../0037-redoxfs-crash-consistency.md)'s recovery mount is a fresh process. It builds a
fresh, cold cache and can never observe anything the killed process's cache held.

### What was not measured

The two caches (`blk::TRANSFER_BLOCKS` and `CACHE_SLOTS`) were swept independently, each at the
other's shipped value, not against each other. And 64 slots was sized against the tree spine and the
crash-test heap budget, not against a sweep of the capacity itself. notes/benchmarks.md's `BUGS`
entries for both steps name these as the first things to run next.
