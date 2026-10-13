# The question underneath, and what stayed out of scope

This appendix belongs to [milestone 138 (close the read gap)](../0138-file-io-throughput.md). It was
moved out of that block on 2026-10-12 (UTC) under [§212 (a prose
budget)](../../decisions/0212-a-prose-budget-for-every-document.md), and the block is complete
without it. It holds the full argument for why the 13 us residual is a frontier rather than a wall,
and why a data cache stayed out of scope when a metadata cache did not.

## Does this architecture have a disk-read liability that cannot be overcome?

calef asked it the same day, 2026-08-18, and it is a thesis question rather than an optimization
one. [§14 (project direction)](../../decisions/0014-project-direction.md) claims a capability
microkernel that runs real workloads. A structural inability to read a disk at speed would be the
strongest thing anyone could say against it.

### What is already answered

The confined userspace block server measured 39.0 us per 4 KiB against Linux's own raw virtio at
38.7 to 53.3 us on the same device at the same tier. The extra address-space crossing this design
pays, client to FS server to block server, is not measurable against the device round trip.
`relay_rtt` prices a two-hop confined relay at about 2 us. So the block layer is at parity and the
architecture is not the cost there.

### What is not answered, and is the number to watch

Every request pays a residual of about 13 us that no record level and no transfer size removes: the
IPC round trip plus the server's own work. That is the floor, and it puts a cached 4 KiB read at
roughly 300 MiB/s however good everything else gets. ext4 buffered is 7,141 MiB/s, because it is a
memcpy inside the kernel with no address-space crossing at all.

So the honest statement of the frontier: against uncached Linux this design reaches parity; against
the page cache it does not, and the reason is structural rather than lazy. Every gap found so far (a
128 KiB record, a page-sized file contract, a block-sized block contract, no metadata cache) was an
implementation choice Linux also had to solve. All four are now closed steps. The 13 us is the first
thing that is not, and it survives untouched by all four. It is the IPC round trip and the server's
own work, and no batching or caching of what travels over that round trip removes the round trip
itself.

The way past it, if it is ever worth taking, is capability-shaped. Stop doing a round trip per
request and grant the client frames it can read directly, which is what Linux's `mmap` over the page
cache is. Frames are already capabilities here. Nobody has designed it and it is not this milestone.
It is named so that the residual is understood as a frontier rather than a wall, and [milestone
337 (the capability-shaped way past one IPC round trip)](../0337-frames-a-client-reads-itself.md)
now holds it.

Each of the four steps was asked to report that residual, so the question gets answered by
accumulation rather than by one argument at the end.

## What is out of scope, deliberately

Step 2 is a metadata cache, by calef's 2026-08-18 decision, and step 4's measurement showed the
tree walk was most of what remained. What is built is narrower than what this section warns
against, and the distinction is the one worth keeping
precise.

A *data* cache is still not this milestone. Nothing here caches a record body or gives a client
`mmap`-shaped access to bytes the server already fetched. Every record read is still a round trip
(batched by step 4, never answered from memory), which is the property the section above argues is
the actual frontier past this milestone's four steps. That is the design with its own coherency and
confinement questions this paragraph warned about. It remains somebody else's milestone if it is
ever wanted.

A small, single-process *metadata* cache is step 2, and it does not raise those questions. It lives
entirely inside one FS server's address space; one open file's tree spine is five blocks and the
cache holds 64. RedoxFS's copy-on-write allocator is what makes a bare write-through cache correct
with no coherency protocol between processes to design.
