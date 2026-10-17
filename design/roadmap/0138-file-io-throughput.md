---
status: BUILT
raised: 2026-08-18
built: 2026-08-22
---
# 138. Close the read gap: a 4 KiB request must stop moving 128 KiB

All four steps are built and measured, and together they make sequential read throughput 70.9x
faster than [milestone 38 (filesystem throughput)](0038-filesystem-throughput.md)'s baseline. That
figure was measured head to head, not by multiplying separately-measured ratios. Step 1 (2026-08-18)
set the record level to 1: a 4 KiB read is 5.13x faster and a write 3.01x. Step 3 (2026-08-19) was
taken before step 2 because step 1's residual said to: a request carries 64 KiB, a sequential write
is 8.02x and a read 5.67x. Step 4 (2026-08-19) made the blk contract carry 16 blocks, for 1.16x to
1.55x. Step 2 (2026-08-19), last and largest, put a 64-block cache over the tree walk: 22.2x on a
repeated inline read and 1.37x to 1.64x on the throughput phases. See notes/benchmarks.md,
notes/fs-server.md and vendor/README.md divergence 5.

Minted 2026-08-18 by calef, on milestone 38's measurement: *"Against buffered Linux we are three
orders of magnitude behind on reads. We need a milestone to optimize and close the gap."*

In brief, every 4 KiB file request moved 128 KiB, in both directions. A read fetched a whole RedoxFS
record (32 blocks). A write read the record, changed 4 KiB and wrote a new copy, because the store
is copy-on-write. This milestone closes that, and only that.

The working behind this page is in four appendices, listed in [their
README](0138-file-io-throughput/README.md):

- [`options.md`](0138-file-io-throughput/options.md): the three options as first written, the sweep
  that priced them, and the count that ruled out replacing RedoxFS.
- [`record-and-transfer.md`](0138-file-io-throughput/record-and-transfer.md): steps 1 and 3 in full.
- [`batching-and-cache.md`](0138-file-io-throughput/batching-and-cache.md): steps 4 and 2 in full,
  with both tables.
- [`frontier.md`](0138-file-io-throughput/frontier.md): the 13 us residual, and why a data cache
  stayed out of scope.

## The measurement this exists to move

From milestone 38, all medians, ns per 4 KiB, at a matched virtualization tier:

| | nife | ext4 `O_DIRECT` | ext4 buffered | raw virtio |
|---|---|---|---|---|
| sequential read | 1,509,270 | 91,694 | 547 | 53,296 |
| sequential write | 2,566,304 | 63,688 | 2,068 | 42,104 |

### The architecture is not the problem

That is the finding that makes this milestone tractable. The confined-server tax is about 1 us per
request against a 1.5 to 3.4 ms operation: 0.07% of the measurement. The confined userspace block
server is at parity with Linux's block layer (39.0 us per 4 KiB against 39 to 53 us for Linux's own
raw virtio reads on the same device). Every nife figure is 39.0 us times a small integer plus a
fixed walk, and nothing was fitted. The 32x is most of the remaining gap, and it belongs to the
vendored store's record size, not to the microkernel.

## The options, and how they were priced

The block first named three fixes. A multi-page transfer on the file contract is a wire change. A
record level matched to the transfer unit changes what the store does. Replacing the store was
raised by calef on 2026-08-18. Both of the first two are things two programs agree on, so the gate
read `DECISION` until they were priced against each other.

The record-level sweep (PR #338) priced them on milestone 38's harness. It fit every read to
`cost = 208 us + 39.0 us x 2^level`. A one-block record buys 5.6x on reads and 3.0 to 3.8x on
writes, not 32x, because the record is one of two terms. The multi-page transfer is worth 16x and
both together 28x. The sweep also showed that lowering the record level is a fork of the vendored
crate after all.

PR #348 then counted the 208 us fixed term. It is five single-block reads per request, the same
five blocks every time, at a 99.6% repeat rate: 94% of the term. That is the absence of a cache, not
a property of RedoxFS. Every store that maps an id to a node would fetch the same path, so replacing
RedoxFS buys a rewrite that needs the identical cache. That ruled out option 3. The full account,
with the code reading and the original question, is in
[options.md](0138-file-io-throughput/options.md).

## Decided 2026-08-18: all of them, then measure again

calef: *"it seems like we do them all. And then we measure and we figure out other optimization
options. Because disk performance is pretty critical to many real workloads."*

The sweep and the count had priced the options, so the decision needed no per-step gate. The
ordering is set by what each step unblocks rather than by size. Each step's model was checked
against a measurement before the next was taken.

| | what | measured effect |
|---|---|---|
| 1 | option 2, the record level | Done 2026-08-18. 4 KiB read 2.68 -> 13.76 MiB/s, write 1.63 -> 4.90. Shipped at level 1, which keeps lz4 and halves the space cost for 8.7% of the read speed. The created and largest readable levels are now separate constants, so nothing stored at levels 0 to 5 becomes unreadable |
| 3 | option 1, multi-page transfer on the file contract | Done 2026-08-19. Modeled 75 MiB/s; measured 80.30 on a read and 42.77 on a write, 5.67x and 8.02x. The wire change is one constant: the length field was always 40 bits and the shared page bounded a transfer. This is the customer path |
| 4 | the block contract, one request per 4 KiB | Done 2026-08-19. The blk channel carries 16 blocks, 1.16x to 1.55x. Steps 1 and 3 already shrank each record to 2 blocks, and step 4 cannot batch across the records one 64 KiB request spans |
| 2 | the metadata cache, the five blocks | Done 2026-08-19. A 64-slot write-through cache over the tree walk, 22.2x on a repeated inline read, 1.37x to 1.64x on the throughput phases |

The table's model was calibrated against the sweep (it reproduces the measured 837 us per 64 KiB as
832). It was not a prediction anybody should spend four milestones on without checking at each step.

## What each step found

The details are in [record-and-transfer.md](0138-file-io-throughput/record-and-transfer.md) and
[batching-and-cache.md](0138-file-io-throughput/batching-and-cache.md). The findings a reader needs:

- Step 1 did not shrink the per-request residual at all. A read's fixed term stayed at 205,698 ns
  and grew from 14% of a request to 72%. A write's was 690,085 ns, 87%, which is why step 3 went
  next.
- Step 1's "one-way door" was removed rather than walked through. `RECORD_LEVEL` is now the level a
  file is created at, and a new `RECORD_LEVEL_MAX` (still 5) is what the readers check against.
- Step 3's wire change is one constant, `fs_proto::fs::TRANSFER_PAGES = 16`. A single-page client is
  untouched because a length the server chooses stays inside the one page every client maps.
- Step 3 moved the residual's owner from the file contract to the block contract. The write
  transaction went from 690 us per 4 KiB to 43, and a read became 74% blk trips.
- The two-term model was fitted twice from different variables, and the fixed terms agree to 0.8%.
- Step 4's gain is small because RedoxFS reads each record with its own call. Per 64 KiB request,
  calls fell from 56 to 48 (1.17x), matching the write speedups almost exactly.
- Step 2 beat every model, because the tree walk repeats across requests to the same handle rather
  than within one. Its 22.2x on `fs_read` retires the "no cache anywhere" claim milestone 38 made.
- Crash consistency was re-run after every step: 0 silently wrong each time.

## The question underneath

calef asked it the same day: does this architecture have a disk-read liability that cannot be
overcome? The block layer is at parity, and the extra address-space crossing is not measurable
against the device round trip. What remains is a residual of about 13 us per request, the IPC round
trip plus the server's own work, which no step touched. It caps a cached 4 KiB read at roughly
300 MiB/s, against 7,141 MiB/s for ext4 buffered.

So against uncached Linux this design reaches parity, and against the page cache it does not, for a
structural reason. The way past it is capability-shaped: grant the client frames it can read
directly, as `mmap` over a page cache does. That is [milestone 337 (the capability-shaped way
past one IPC round trip)](0337-frames-a-client-reads-itself.md). A data cache stayed out of scope;
step 2 is a metadata cache inside one server, which raises no coherency question between processes.
The full argument is in [frontier.md](0138-file-io-throughput/frontier.md).

## Why it matters

It is on the customer path. [Milestone 55 (Time Machine)](0055-time-machine.md)'s target is judged
against these numbers, and at 4 KiB they are not comfortable. [§34 (RedoxFS as the primary
filesystem)](../decisions/0034-redoxfs-primary.md)'s claim can now be argued from measurements
rather than asserted, and this is the measurement that most weakens it.

## BUGS

- Closing the 32x does not close the gap to buffered Linux. Buffered Linux is 547 ns against our
  starting 1,509,270. Removing the 32x leaves roughly two orders of magnitude, and the rest is page
  cache.
- The payload entropy caveat travels with every number here. RedoxFS lz4-compresses records, so an
  all-zero file reads and writes several times faster than an incompressible one. Milestone 38's
  figures use an incompressible payload, the conservative choice and the one a backup resembles. A
  re-measurement that quietly changes payload is not comparable.
- This block once asserted that RedoxFS is not the cause on a reading of the code. That is now
  measured twice over: the sweep held the prediction (5.6x on a read, 3.0x on a write), and the
  fixed term was counted as five repeated block reads. The block went on describing the sweep as
  unrun after it ran. The pre-sweep text lives in
  [options.md](0138-file-io-throughput/options.md), beside the sweep's
  correction of milestone 38's 46.2 us per block to a 39.0 us marginal cost plus a 208 us walk.
- `Transaction::write_node` compares before writing, so rewriting a block with identical contents
  costs a read and no write. A benchmark that sends one constant page repeatedly measures the
  comparison rather than the store.
- Step 1's record level was chosen on 4 KiB evidence, and step 3 made 4 KiB the atypical request.
  The sweep measured every level from 0 to 4 at the identical 837 us per 64 KiB, because the fixed
  term is per request and the block count is the same. So step 1's 5.13x is a ratio about a
  contract that no longer describes the default. Re-running the record sweep at
  `TRANSFER_PAGES = 16` is what would say so, and it is not optional. That run is
  [milestone 329 (the 5.13x record-level number)](0329-a-record-level-chosen-at-the-wrong-size.md).
- Nothing checks that a client asked for no more than it mapped (step 3). The channel is 64 KiB and
  a client maps as much as it uses. One that asks for more gets a data abort on its own unmapped
  page, after the server has done the work. This is rung four of `CLAUDE.md`'s ladder, marked where a
  reader meets the constant. Rung one is a channel size on the wire, a new concept that was not
  taken.
- The space cost of the 8 KiB record was not re-measured for step 1. The +19% is the sweep's, on
  text, the payload most favorable to lz4. A backup is the incompressible case and would show only
  the pointer half. Nobody has measured it.
- `RECORD_LEVEL_MAX` keeps old images readable and does not migrate them. A file created by an
  older build keeps its 128 KiB record forever: it reads correctly, at the old price. There is no
  rewrite path and no `fsck` to make one. That is right today because no such image exists outside a
  test, and wrong the day somebody upgrades a populated disk.
- The measurement conditions differ across the four steps. Steps 1 and 3 ran on a quiet machine.
  Steps 2 and 4 ran at `uptime` load 15 to 21 with other lanes building, against the earlier 3.6 to
  9. The `fs_read` control and the two-term model's internal agreement are the evidence these are
  real signals: step 4's predicted ~296 us saving matched a measured 284 to 302 us across three
  phases. Step 1's figures reproduce the earlier sweep's fit to within 3% on a different day. Every
  figure is one lane's own, on one machine, and no second machine has confirmed any of it.
- The two caches were never swept against each other, nor for where their own curves bend.
  `blk::TRANSFER_BLOCKS = 16` (step 4) and `CACHE_SLOTS = 64` (step 2) were each measured alone at
  the other's shipped value. Both were chosen by symmetry or by sizing against a known working set,
  as steps 1 and 3 were before being swept. Whether a smaller batch with a larger cache (or the
  reverse) reaches the same total for less DMA region or heap is open. `sh
  bench/blk-transfer-sweep.sh N 1 2 4 8 16` and `sh bench/cache-slots-sweep.sh N 1 4 16 64 256` are
  the experiments, one command each.
- `CachedDisk`'s 64 slots were sized against a small test fixture and milestone 37 (RedoxFS crash
  consistency)'s crash-test heap budget, not a real deployment's node count. A 65,536-node
  filesystem's full tree spine is 259 blocks. 64 slots holds one open file's spine comfortably and thrashes once enough distinct files
  are open to collide across the tree's shared upper levels. Nobody has measured a multi-file
  workload against it.

## Follow-on

- **Recorded.** `design/roadmap/0138-file-io-throughput.md` BUGS: closing the 32x does not close the
  gap to buffered Linux; the rest is page cache.
- **Recorded.** `notes/benchmarks.md` carries the payload entropy caveat that travels with every
  number here.
- **Recorded.** `design/roadmap/0138-file-io-throughput.md` BUGS: `Transaction::write_node` compares
  before writing, so a constant-page benchmark measures the comparison rather than the store.
- **Recorded.** `design/roadmap/0138-file-io-throughput.md` BUGS: `RECORD_LEVEL_MAX` keeps old
  images readable and does not migrate them.
- **Recorded.** `notes/benchmarks.md` names the two sweeps nobody has run as the first things to run
  next: `blk::TRANSFER_BLOCKS` against `CACHE_SLOTS`, and each for where its own curve bends.
- **Recorded.** `notes/fs-server.md` holds the cache-sizing limit: 64 slots against a 259-block
  spine, with no multi-file workload measured.
- **Recorded.** `design/roadmap/0138-file-io-throughput.md` BUGS: the space cost of the 8 KiB record
  was never re-measured for step 1 on an incompressible payload.
- **Recorded.** `design/roadmap/0138-file-io-throughput.md` BUGS: the measurement conditions differ
  across the four steps, and no second machine has confirmed any figure.
- **Refused.** A wire-level channel size for step 3's transfer. It is a new concept on a contract
  whose whole compatibility story is that nothing in the packed word changed. The unchecked client
  length stays rung four, marked where a reader meets the constant.
- **Refused.** A `READV`-shaped scatter list, a new opcode, and a negotiated channel size at bind
  time, all considered for step 3. The 40-bit length field already fits, so the page was the only
  limit. The others add a concept where every agreement between a client and this wiring is a
  compile-time constant both sides carry.
- **Refused.** Replacing RedoxFS, option 3, on measurement rather than preference. The 208 us fixed
  term is the absence of a cache, and a replacement would arrive needing the identical cache, against
  [§46 (thin primitives or whole subsystems)](../decisions/0046-dependency-rule.md)'s pricing of a
  dependency this size.
- **Refused.** A data cache, as opposed to step 2's metadata cache. It has coherency and
  confinement questions of its own, where a single-process metadata cache made correct by RedoxFS's
  copy-on-write allocator has none.
- **Milestone 329.** Re-run the record-level sweep at the shipped transfer size:
  `sh bench/record-level-sweep.sh 3 0 1 5` with `TRANSFER_PAGES = 16`. Until then a headline number
  here describes a configuration nothing ships.
- **Milestone 337.** Design the capability-shaped way past the ~13 us per-request residual: grant
  the client frames it can read directly. Frames are already capabilities here, so the primitive
  exists and nobody has drawn the design.

## Index row

Minted by calef on 2026-08-18 from milestone 38's measurement. Every 4 KiB file request moved 128
KiB in both directions, and that single term was the whole remaining gap: the confined-server tax
is 0.07% of the measurement and the block server is already at parity with Linux's block layer.
calef decided to take all four steps in order, measuring at each. All four are built and measured:
a smaller record (5.13x/3.01x), a 64 KiB file transfer (5.67x/8.02x on top). A 16-block blk
transfer (1.16x to 1.55x more, far below the naive 16x, because the first two steps already shrank
what one blk request can batch). And a 64-slot metadata cache over the tree walk the third step's
own residual pointed at (22.2x on a repeated inline read, 1.37x to 1.64x on the throughput
phases). Combined against milestone 38's original baseline, measured head to head: 70.9x on
sequential read throughput. The 13 us IPC-round-trip floor survives every step untouched and is
named as the frontier past this milestone, not a wall inside it
