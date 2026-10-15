//! **Sequential and random throughput** through the FS server (`ROLE_THROUGHPUT`, milestone 38
//! (filesystem throughput, and the comparison)).
//!
//! Cut out of `fs_test_client.rs` unchanged on 2026-10-12 (UTC) by lane
//! maintainer/files-over-1500, for §282 (when a file is split, the parent becomes an index). What
//! another role uses is `pub(crate)` here.
//!
//! Name: provisional (lane maintainer/files-over-1500, 2026-10-12 UTC), for the module and the
//! file.

use filesystem_protocol::{fs};
use filesystem_fixture as fixture;
use user_mode_runtime::{call, exit, now, send};

use crate::channel::{FILE, REPORT, STAGE_WRITE, WINDOW, check, fail, open, put_page, read};

/// **The filesystem throughput benchmark** (milestone 38, DECISIONS §34 condition 2). Six phases
/// over one file, each self-timed and reported home as `[ticks, transfers, phase]`: sequential
/// write, sequential read, random read, a record-aligned read, random write, and the client's own
/// payload cost. The bench boot turns those into the `fs_seq_write` / `fs_seq_read` /
/// `fs_rand_read` / `fs_record_read` / `fs_rand_write` / `fs_payload_fill` lines and into MiB/s.
///
/// **What one transfer is, and why that is the whole story.** One `filesystem_protocol` request moves at most
/// `fs::TRANSFER_MAX` bytes, because the payload travels through the region the client shares with
/// the server. That was one page until milestone 138 step 3 and is sixteen now, so a phase here is
/// `UNIT` per request and the throughput is the request rate times `UNIT`. A Linux program reading
/// the same file would use a 64 KiB or 1 MiB buffer and pay one syscall for it; **that difference
/// is a property of this protocol, not of the measurement**, and it is still the first thing
/// notes/benchmarks.md says next to the numbers. It got smaller; it did not go away.
///
/// **The bytes moved per phase are held constant** (`tp::TOTAL`, 1 MiB) rather than the transfer
/// count, so a run at one page and a run at sixteen move the same file and their MiB/s figures are
/// a before and an after rather than two unrelated numbers.
///
/// **Why the write phase comes first.** It creates the file, so it is the only phase that pays
/// allocation, and the read phases have something laid out by the server under test rather than by
/// the host tool. It is also the reason there is no separate "create" number.
///
/// **What a write here promises, which decides what it compares against.** Every `filesystem_protocol` write
/// goes through one RedoxFS transaction that commits to the header ring before the reply, so the
/// filesystem's own state is durable per request the way `O_DSYNC` makes ext4's; but no device
/// flush is issued unless a client asks for one (`filesystem_protocol::fs::SYNC`), so the bytes sit where
/// `O_DIRECT` alone leaves them. This sits **between** Linux's two, and notes/benchmarks.md prints
/// both rather than picking one.
///
/// **The offsets are drawn from a fixed seed**, so "random" means the same sequence on every run
/// and two runs are comparable. They are page-aligned and inside the file, so a random read never
/// hits EOF and a random write never extends the file: the random phases measure placement, not
/// growth.
///
/// A read phase's timed loop issues nothing but the `CALL`: the bytes are never copied out, because
/// a client that shares a page with its server can use them where they land, which is one copy
/// fewer than a Unix `read(2)`. A write phase's loop repaints the page first, and has to; see the
/// comment on the payload below.
pub(crate) fn throughput() -> ! {
    use fixture::throughput as tp;

    let handle = throughput_file();

    // **Every write gets a fresh, incompressible page, and that is not fussiness.** RedoxFS stores
    // a file in records (8 KiB since milestone 138 step 1, 128 KiB before it), compresses each with
    // lz4 when it is larger than one block, and skips a write outright when the record it would
    // produce is byte-identical to the one already there. So a benchmark that sends one constant
    // page measures the short circuit on every repeat, and one that sends several copies of an
    // incompressible page into a single record measures lz4 finding the copies. The first draft
    // of this bench did both, and reported random writes as fast as random reads because none of
    // them wrote anything. See notes/benchmarks.md.
    let mut payload = 0x9E37_79B9_7F4A_7C15_u64;

    // What that costs, measured on its own so a reader can take it back out of the write phases,
    // which pay it inside their window because a client that writes data has to produce the data.
    let start = now();
    for _ in 0..tp::BLOCKS {
        paint_page(&mut payload);
    }
    let ticks = now().wrapping_sub(start);
    send(REPORT, ticks, tp::BLOCKS, tp::PAYLOAD_FILL);

    // --- Phase 1: sequential write. This is also the file's creation. ---
    // **No warmup here, on purpose.** Every other phase warms up to keep a cold first touch out of
    // the window; this phase's whole subject is the cold path, because writing a file that did not
    // exist is what allocates its blocks. Warming it would mean overwriting the first few blocks
    // before timing them, which measures a different operation than the rest of the loop.
    let start = now();
    for k in 0..tp::BLOCKS {
        paint_page(&mut payload);
        write_unit(handle, k * tp::UNIT as u64);
    }
    let ticks = now().wrapping_sub(start);
    // The file is exactly as long as the phase claims to have written it. Without this a server
    // that answered every write with a short count would still produce a throughput number.
    let (size, _) = call(FILE, fs::req(fs::FSTAT, handle, 0), 0);
    check(size == tp::BLOCKS * tp::UNIT as u64);
    send(REPORT, ticks, tp::BLOCKS, tp::SEQ_WRITE);

    // --- Phase 2: sequential read of what phase 1 left behind. ---
    for _ in 0..tp::WARMUP {
        check(read(handle, 0, tp::UNIT) == tp::UNIT);
    }
    let start = now();
    let mut got = 0u64;
    for k in 0..tp::BLOCKS {
        got += read(handle, k * tp::UNIT as u64, tp::UNIT) as u64;
    }
    let ticks = now().wrapping_sub(start);
    check(got == tp::BLOCKS * tp::UNIT as u64);
    send(REPORT, ticks, tp::BLOCKS, tp::SEQ_READ);

    // --- Phase 3: random read, page-aligned, inside the file. ---
    let mut rng = 0x38F5_7042_1DEA_u64;
    for _ in 0..tp::WARMUP {
        check(read(handle, next_offset(&mut rng), tp::UNIT) == tp::UNIT);
    }
    let start = now();
    let mut got = 0u64;
    for _ in 0..tp::BLOCKS {
        got += read(handle, next_offset(&mut rng), tp::UNIT) as u64;
    }
    let ticks = now().wrapping_sub(start);
    check(got == tp::BLOCKS * tp::UNIT as u64);
    send(REPORT, ticks, tp::BLOCKS, tp::RAND_READ);

    // --- Phase 5, out of order on purpose: the cheapest 4 KiB read there is. ---
    // Record-aligned, so `read_record` fetches one block instead of averaging over a record's
    // worth. It runs before the second write phase so that it reads a file no write has touched
    // since the read phases above, and its gap to the sequential read is the record geometry with
    // everything else held identical. See `fixture::throughput::RECORD_READ`.
    let records = (tp::BLOCKS * tp::UNIT as u64) / tp::RECORD;
    for k in 0..tp::WARMUP {
        check(read(handle, (k % records) * tp::RECORD, tp::UNIT) == tp::UNIT);
    }
    let start = now();
    let mut got = 0u64;
    for k in 0..tp::BLOCKS {
        got += read(handle, (k % records) * tp::RECORD, tp::UNIT) as u64;
    }
    let ticks = now().wrapping_sub(start);
    check(got == tp::BLOCKS * tp::UNIT as u64);
    send(REPORT, ticks, tp::BLOCKS, tp::RECORD_READ);

    // --- Phase 4: random write, in place, over blocks that already exist. The offset stream runs
    // on from phase 3 rather than restarting, so no timed write lands on a block the warmup just
    // touched. ---
    for _ in 0..tp::WARMUP {
        paint_page(&mut payload);
        write_unit(handle, next_offset(&mut rng));
    }
    let start = now();
    for _ in 0..tp::BLOCKS {
        paint_page(&mut payload);
        write_unit(handle, next_offset(&mut rng));
    }
    let ticks = now().wrapping_sub(start);
    // The file did not grow: a random write that appended would be measuring allocation.
    let (size, _) = call(FILE, fs::req(fs::FSTAT, handle, 0), 0);
    check(size == tp::BLOCKS * tp::UNIT as u64);
    send(REPORT, ticks, tp::BLOCKS, tp::RAND_WRITE);

    exit();
}

/// Open the throughput file, creating it if this is the first run against this image and emptying
/// it if it is not. `CREATE` answers `EEXIST` rather than opening (`filesystem_protocol`'s note says why), so
/// the fallback is an explicit open-and-truncate.
fn throughput_file() -> u64 {
    let name = fixture::THROUGHPUT_NAME;
    put_page(name.as_bytes());
    let (r0, _) = call(FILE, fs::req(fs::CREATE, 0, name.len() as u64), 0);
    if (r0 as i64) >= 0 {
        return r0;
    }
    let h = open(name);
    let (t, _) = call(FILE, fs::req(fs::TRUNCATE, h, 0), 0);
    check((t as i64) >= 0);
    h
}

/// One timed write: a page of the shared page's current contents at `offset`. No `put_page`, so the
/// measured window holds the request and not a 4 KiB byte-at-a-time fill.
fn write_unit(handle: u64, offset: u64) {
    let unit = fixture::throughput::UNIT;
    let (r0, _) = call(FILE, fs::req(fs::WRITE, handle, unit as u64), offset);
    if (r0 as i64) < 0 {
        fail(STAGE_WRITE, r0);
    }
    check(r0 as usize == unit);
}

/// **Fill the whole shared page with fresh, incompressible bytes**, advancing `state`. Eight bytes
/// at a time, so it is 512 stores rather than 4096; the cost is reported as its own row
/// (`fs_payload_fill`) because the write phases pay it inside their timed window.
fn paint_page(state: &mut u64) {
    let mut i = 0;
    while i < fixture::throughput::UNIT {
        let mut x = *state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *state = x;
        WINDOW.write::<u64>(i as u64, x);
        i += 8;
    }
}

/// The next page-aligned offset inside the throughput file, from a xorshift64 the seed makes
/// reproducible. A benchmark whose access pattern changes between runs cannot be compared with
/// itself, let alone with another system.
fn next_offset(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    (x % fixture::throughput::BLOCKS) * fixture::throughput::UNIT as u64
}
