# The three options, and how they were priced (2026-08-18)

This appendix belongs to [milestone 138 (close the read gap)](../0138-file-io-throughput.md). It was
moved out of that block on 2026-10-12 (UTC) under [§212 (a prose
budget)](../../decisions/0212-a-prose-budget-for-every-document.md), and the block is complete
without it. It keeps the options as first written, the sweep that priced them, and the count that
ruled out the third. calef's decision to take all four steps settled every option here.

## Why the options had to be priced first

This milestone's gate read `DECISION` while the options were unpriced. Both candidate fixes are
things two programs agree on, which the *move fast on what can be undone* tenet puts in the
irreversible column. [Milestone 38 (filesystem throughput)](../0038-filesystem-throughput.md)'s own
`BUGS` entry says so: *"the fix is not in this server: it is either a multi-page transfer on the
contract, or a record level chosen to match the transfer unit, and both are decisions rather than
patches."* The two had to be priced against each other before either was built.

## The candidate fixes, none priced against the others

1. A multi-page transfer on the file contract. `fs_proto`'s transfer unit is one page because
   that is what a request can carry. Milestone 38 measured the neighboring case: ext4 moves 64 KiB
   for about what it charges for 4 KiB. So sixteen times the payload arrives at the same price,
   600-900 MiB/s against 40-80. This is a wire change.
2. A record level matched to the transfer unit. Leaves the contract alone and changes what the
   store does. Cheaper to reach and it gives up compression's current terms: RedoxFS compresses a
   record with lz4 only when the record exceeds one block, which is always true today.

All three must arrive priced together, per `AGENTS.md`'s rule that an irreversible fork gets
options and their costs rather than a recommendation.

3. Replace the store. calef raised it on 2026-08-18, on the pull request that minted this block:
   *"we should consider if redoxfs is the problem and we should move to a different
   implementation"*. It belongs in the list, and the first draft of this block was wrong to name two
   options that both keep RedoxFS without saying that a third existed.

   What the code says, read rather than assumed. `RECORD_LEVEL` is 5 and `BLOCK_SIZE` is 4096, so
   `RECORD_SIZE` is 128 KiB. But the record level is a per-node field in the on-disk format
   (`node.rs`: `pub record_level: Le<u32>`), set once at file creation from that constant. Every
   read and write path honors the node's value rather than the constant (`transaction.rs`:
   `let record_level = node.data().record_level();`). Directories already get 0.

   So a smaller record for a file is a creation-time choice the format already supports, not a
   format change and not a fork of the vendored crate. That does not make it free, and the costs are
   named in option 2. But it means the 32x is a parameter this store exposes rather than a property
   it imposes. On the evidence available today, RedoxFS is not structurally the cause.

   What would actually justify replacing it is therefore something this milestone has not
   measured: a cost that survives after the record level is tuned. [§46 (thin primitives or whole
   subsystems)](../../decisions/0046-dependency-rule.md) puts a dependency of this size in the
   expensive column (*"adding one is a morning; removing one after a subsystem is built on it is a
   project"*). RedoxFS is the case §46 (thin primitives or whole subsystems) itself cites for
   vendoring, where correctness is won by exposure rather than by reading a spec.

## The question the block originally asked

Nobody has measured throughput against record level, and it is the cheap experiment that decides
between all three options. Sweep the per-file record level against the transfer size and the access
pattern, on the harness milestone 38 already built. If a lower level closes most of the 32x, option 2
is a small change to a constant at creation and options 1 and 3 are unnecessary. If it does not, the
number that survives is the argument for one of the others, and it is an argument nobody can make
today.

And it should re-ask which workload this milestone is optimizing, because milestone 38 measured
4 KiB by convention rather than by need. A Time Machine backup writes band files, which are large
and sequential, and a 128 KiB record is plausibly right for those. It is possible that the customer
path wants the current setting and that the 4 KiB figure is the atypical case. That would not make
the gap uninteresting. It would change what "close the gap" means and which milestone owns it.

## The measurement was taken, and it moved two of the three

The section above asked for a record-level sweep before any option was
chosen. It was run (PR #338) and the result is in notes/benchmarks.md. The question is kept because
it is why the answer is trustworthy, so a reader can see what was asked.

What it found, all measured on milestone 38's own harness across twenty interleaved passes:
`cost = 208 us + 39.0 us x 2^level`, read residuals within 5% at every level. So a one-block record
buys 5.6x on reads and 3.0 to 3.8x on writes, not 32x, because the record is only one of two
terms. Option 1 is worth more (16x) because it amortizes both. Both together are 28x.

And it corrected two things this block asserted. Milestone 38's 46.2 us per block was an average
that charged the per-request walk to the blocks; the marginal cost is 39.0 us and the walk is a
separate 208 us. And this block said option 2 is "not a fork of the vendored crate": it is one.
`Node::new` takes no level and has no setter, and three call sites gate on `RECORD_LEVEL`. So
lowering the constant makes every record already stored at a higher level unreadable.

## The 208 us is the absence of a cache, counted rather than argued

The record-level sweep (notes/benchmarks.md) fit every read to `cost = 208 us + 39.0 us x 2^level`.
It attributed the fixed term to "RedoxFS re-reading its own metadata, about 5.3 block reads". calef
asked the question that attribution left open: is the 208 us inherent to RedoxFS's design, or is it
the absence of a cache that any store would also need? PR #348 answered it.

It is the absence of a cache. Every 4 KiB read of an ordinary file makes exactly five single-block
reads below the record, and they are the same five block numbers on every request. They are
`Transaction::read_tree_and_addr`'s walk of the node tree (L3 root, L2, L1, L0) and then the file's
own `Node` block. Measured at 5.00 per request over 256 requests per phase, sequential and random.
99.6% of them were a block already read in the same phase, and the count was unchanged at record
levels 1, 2 and 5. The first four are the tree spine and are shared by every file whose node id
falls in the same 256, so two different files differ in one block out of five.

At the sweep's measured 39.0 us per block-server round trip that is 195 us of the 208, 94% of it.
The remaining ~13 us is the file-IPC round trip and the server's own work. The same number arrives
independently from a measurement already on the page: `fs_read` reads an inline `motd`, does exactly
these five reads and no record read at all, and costs 203 to 208 us.

What it means for option 3. It rules it out on this evidence, and now for a measured reason. The
walk is structural only in that the format fixes its depth. What makes it cost 195 us is that the
same five blocks are fetched off the device 256 times in a row. Every store that maps an id to a node
has a root-to-node path it would fetch too, so replacing RedoxFS buys a rewrite and arrives needing
the identical cache. The cache is small: the spine of a 65,536-node filesystem is 259 blocks, about
1 MiB, and the fifth block is a node the server could hold per open handle.

What it did not settle: that a cache is cheap to build. It said which milestone owns the 208 us,
not how to close it. calef's decision the same day made the cache step 2.
