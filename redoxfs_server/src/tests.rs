use redoxfs::DiskMemory;

use super::*;

mod directory_capability;
mod entries;
mod extended_attributes;

/// A [`BlockIo`] over an in-memory image, so the exact chunking the EL0 binary uses is exercised
/// on the host. Byte for byte it must behave like `DiskMemory`.
pub(crate) struct VecIo(pub(crate) Vec<u8>);
impl BlockIo for VecIo {
    fn read_block(&mut self, block: u64, buf: &mut [u8; BLOCK]) -> Result<()> {
        let off = block as usize * BLOCK;
        buf.copy_from_slice(&self.0[off..off + BLOCK]);
        Ok(())
    }
    fn write_block(&mut self, block: u64, buf: &[u8; BLOCK]) -> Result<()> {
        let off = block as usize * BLOCK;
        self.0[off..off + BLOCK].copy_from_slice(buf);
        Ok(())
    }
    fn size_bytes(&mut self) -> Result<u64> {
        Ok(self.0.len() as u64)
    }
}

/// **A sync is a write-side authority, and a read-only capability may not ask for one**
/// (milestone 55).
///
/// The rights question here is not obvious and the answer is argued at `filesystem_protocol::fs::SYNC`,
/// so what this pins is the choice rather than a mechanism: a sync is an *action on the
/// device*, and a device flush stalls the queue for every client of the image. Letting a
/// read-only holder ask would hand it a lever on everybody else's write latency for no
/// purpose, because a holder with nothing written has nothing to make durable.
///
/// It also pins the shape: **either kind of handle**, file or directory, because the answer is
/// about the storage rather than about the node, which is `STATFS`'s rule and the reason this
/// goes through `node_at` rather than `file_at`.
#[test]
fn a_sync_needs_write_and_takes_a_handle_of_either_kind() {
    let mut fs = FileSystem::create(BlockDisk(VecIo(vec![0u8; 16 * 1024 * 1024])), None, 0, 0)
        .expect("mkfs through BlockDisk");
    fs.tx(|tx| {
        tx.create_node(TreePtr::root(), "scratch", Node::MODE_FILE | 0o644, 0, 0)?;
        tx.create_node(TreePtr::root(), "sub", Node::MODE_DIR | 0o755, 0, 0)?;
        Ok(())
    })
    .expect("populate");
    let image = fs.disk.0.0;
    let mut srv = Server::open(BlockDisk(VecIo(image))).expect("Server::open");

    // The bound directory carries every right, so it may ask.
    assert!(
        srv.sync_permitted(filesystem_protocol::fs::ROOT as u32)
            .is_ok()
    );

    // A file handle opened through it inherits READ | WRITE, so it may too: the verb is not
    // about the file, and refusing a file handle would be inventing a distinction.
    let file = srv.open_file("scratch").expect("open scratch");
    assert!(srv.sync_permitted(file).is_ok());

    // A directory capability narrowed to reading may not. EROFS rather than EACCES, because
    // through this capability the storage takes no writes and there is no policy that could
    // have said yes.
    let ro = srv
        .open_dir(
            filesystem_protocol::fs::ROOT as u32,
            "sub",
            dir::ENUMERATE | dir::READ | dir::DESCEND,
        )
        .expect("open sub read-only");
    assert_eq!(
        srv.sync_permitted(ro).unwrap_err().errno,
        dir::EROFS,
        "a read-only capability must not be able to make the whole device flush"
    );

    // And a handle nobody minted is EBADF, which is the check that makes the right meaningful:
    // a forged handle must not be a way around it.
    assert_eq!(srv.sync_permitted(9999).unwrap_err().errno, EBADF);
}

/// **`touch`'s bare form observes a value the server chose, and it moves forward** (milestone
/// 47, DECISIONS §112). Two `set_mtime_now` calls on the same name must not read back equal:
/// if they did, `touch` could not be used for a staleness check, which is the property every
/// caller of bare `touch` actually depends on.
#[test]
fn setting_mtime_to_now_twice_observes_two_different_times_and_neither_is_the_callers_choice() {
    let mut srv = server_with_tree();
    let root = filesystem_protocol::fs::ROOT as u32;

    let before = srv.mtime(root, "motd").expect("read the seeded mtime");
    srv.set_mtime_now(root, "motd").expect("touch, bare");
    let first = srv.mtime(root, "motd").expect("read back the first touch");
    srv.set_mtime_now(root, "motd").expect("touch again");
    let second = srv.mtime(root, "motd").expect("read back the second touch");

    assert!(
        first > before,
        "a bare touch must move the mtime forward from what the file was seeded with"
    );
    assert!(
        second > first,
        "two bare touches in sequence must not observe the same mtime, or a make-style \
         staleness check could not tell them apart"
    );
}

/// **`touch -t`'s round trip: the value read back is exactly the value asserted, not "now"**
/// (milestone 47, DECISIONS §112). This is the test that makes the two-right split real rather
/// than declared: if `set_mtime_at` silently fell back to the server's own clock, every rights
/// check in [`each_rung_of_the_ladder_gates_exactly_its_own_verb`] would still pass and this is
/// the only thing that would catch it.
#[test]
fn setting_an_arbitrary_mtime_round_trips_exactly_and_is_not_now() {
    let mut srv = server_with_tree();
    let root = filesystem_protocol::fs::ROOT as u32;

    // The same instant the kernel-booted witness asserts with a real `touch -t` command line
    // (`filesystem_protocol::fixture::tree::NAV_TOUCH_AT_UNIX`), reused here rather than a second
    // literal so the two round-trip checks (this one in-process, that one over the real wire)
    // are provably checking the same claim. Nowhere near this server's own advancing clock
    // (which starts at 1 and has moved by single digits by the time this test runs), so a
    // fallback to "now" cannot coincidentally pass.
    const ASSERTED: u64 = filesystem_protocol::fixture::tree::NAV_TOUCH_AT_UNIX;
    srv.set_mtime_at(root, "motd", ASSERTED)
        .expect("write an arbitrary mtime with full rights");
    assert_eq!(
        srv.mtime(root, "motd").expect("read it back"),
        ASSERTED,
        "the round trip must return exactly the asserted value"
    );

    // And a second, ordinary touch does NOT preserve it: `set_mtime_now` is unconditional, so
    // an arbitrary value set with -t is exactly as overwritable by a bare touch as any other
    // mtime, which is the honest answer for "how long does a lie about history last".
    srv.set_mtime_now(root, "motd").expect("touch, bare");
    assert_ne!(
        srv.mtime(root, "motd").expect("read it back again"),
        ASSERTED,
        "a bare touch after an arbitrary one must move the mtime again, not preserve the lie"
    );
}

/// **Repeat writes through the EL0 binary's exact chunking.** The device's `IpcDisk` and this
/// `BlockDisk` split `Disk` calls the same way, so if the chunking is what breaks a repeat write
/// (the partial-block read-modify-write is the suspicious part, and compression plus a growing
/// allocator log exercise it more on later writes), it breaks here, on the host, in milliseconds.
#[test]
fn repeat_writes_through_the_block_chunking_do_not_loop() {
    let mut fs = FileSystem::create(BlockDisk(VecIo(vec![0u8; 16 * 1024 * 1024])), None, 0, 0)
        .expect("mkfs through BlockDisk");
    fs.tx(|tx| {
        let ptr = tx
            .create_node(TreePtr::root(), "scratch", Node::MODE_FILE | 0o644, 0, 0)?
            .ptr();
        tx.write_node(ptr, 0, b"(placeholder)", 0, 0)?;
        Ok(())
    })
    .expect("populate");
    let image = fs.disk.0.0;

    let mut srv = Server::open(BlockDisk(VecIo(image))).expect("Server::open");
    let h = srv.open_file("scratch").expect("open scratch");
    let mut buf = [0u8; 128];
    for pass in 1..=3u8 {
        let payload = repeat_write_payload(pass);
        assert_eq!(srv.write(h, 0, &payload).unwrap(), payload.len());
        let n = srv.read(h, 0, &mut buf).unwrap();
        assert_eq!(&buf[..n], &payload[..], "pass {pass} through the chunking");
    }
}

/// Repeat writes of a **multi-record** payload, so the chunking's multi-block path and the
/// compressed-record partial tail both get exercised across generations.
///
/// **The length is derived from the record size rather than written down**, and milestone 138 is
/// why. This used to say `160 * 1024`, chosen because it was 1.25 of the 128 KiB record of the
/// day; at the 8 KiB record that milestone moved to, the same constant is exactly twenty whole
/// records and the partial tail this test exists to reach **stops existing**, silently, with the
/// test still passing. Two whole records plus one block is that shape at any level, so the
/// coverage cannot be lost by a constant moving somewhere else.
#[test]
fn repeat_record_sized_writes_through_the_chunking_do_not_loop() {
    let mut fs = FileSystem::create(BlockDisk(VecIo(vec![0u8; 32 * 1024 * 1024])), None, 0, 0)
        .expect("mkfs");
    fs.tx(|tx| {
        tx.create_node(TreePtr::root(), "big", Node::MODE_FILE | 0o644, 0, 0)?;
        Ok(())
    })
    .expect("populate");
    let image = fs.disk.0.0;

    let mut srv = Server::open(BlockDisk(VecIo(image))).expect("open");
    let h = srv.open_file("big").expect("open big");
    let len = 2 * (redoxfs::BLOCK_SIZE << redoxfs::RECORD_LEVEL) as usize + BLOCK;
    for pass in 1..=3u8 {
        // Position-dependent and pass-dependent, and deliberately not compressible to a tiny
        // size, so the record path does real work each pass.
        let data: Vec<u8> = (0..len)
            .map(|i| ((i as u32).wrapping_mul(2_654_435_761) >> 16) as u8 ^ pass)
            .collect();
        assert_eq!(srv.write(h, 0, &data).unwrap(), len);
        let mut back = vec![0u8; len];
        let n = srv.read(h, 0, &mut back).unwrap();
        assert_eq!(n, len, "pass {pass} short read");
        assert!(back == data, "pass {pass} record data mismatch");
    }
}

/// Build a 16 MiB RedoxFS image in memory with the given files in its root (using the std
/// creation APIs the host tool uses), then reopen it through a fresh [`Server`], exactly as the
/// running system does: the host tool makes the image, the server only ever opens it.
fn server_with(files: &[(&str, &[u8])]) -> Server<DiskMemory> {
    Server::open(image_with(files)).expect("open")
}

/// The disk behind [`server_with`], as a standalone [`DiskMemory`], so a test can open it, drop
/// the server, and reopen the same disk to prove persistence across a close.
fn image_with(files: &[(&str, &[u8])]) -> DiskMemory {
    let disk = DiskMemory::new(16 * 1024 * 1024);
    let mut fs = FileSystem::create(disk, None, 0, 0).expect("create");
    fs.tx(|tx| {
        for (name, data) in files {
            let ptr = tx
                .create_node(TreePtr::root(), name, Node::MODE_FILE | 0o644, 0, 0)?
                .ptr();
            tx.write_node(ptr, 0, data, 0, 0)?;
        }
        Ok(())
    })
    .expect("populate");
    fs.disk
}

#[test]
fn opens_and_reads_a_file_the_host_tool_wrote() {
    let mut srv = server_with(&[("motd", b"hello from redoxfs\n")]);
    let h = srv.open_file("motd").expect("open motd");
    let mut buf = [0u8; 64];
    let n = srv.read(h, 0, &mut buf).expect("read");
    assert_eq!(&buf[..n], b"hello from redoxfs\n");
}

#[test]
fn a_missing_name_is_enoent_and_names_do_not_walk() {
    let mut srv = server_with(&[("present", b"x")]);
    assert_eq!(
        srv.open_file("absent").unwrap_err().errno,
        syscall::error::ENOENT
    );
    // A name is one component resolved in the bound directory, never a path: a slash does not
    // descend, so "present/nope" finds nothing rather than walking anywhere.
    assert!(srv.open_file("present/nope").is_err());
}

#[test]
fn a_write_persists_and_reads_back() {
    let mut srv = server_with(&[("scratch", &[0u8; 32])]);
    let h = srv.open_file("scratch").unwrap();
    let payload = b"CRKWRIT1 and then some position-dependent bytes here";
    let n = srv.write(h, 0, payload).unwrap();
    assert_eq!(n, payload.len());
    let mut buf = [0u8; 128];
    let m = srv.read(h, 0, &mut buf).unwrap();
    assert_eq!(&buf[..m], payload);
    assert_eq!(srv.fstat(h).unwrap(), payload.len() as u64);
}

/// The payload for pass `n` of the repeat-write test: a fixed 64 bytes, tagged with the pass and
/// position-dependent after that, so every pass has the same length (no truncation confusion) and
/// a stale or shifted read cannot match. Shared with the on-device client through
/// `filesystem_protocol::fixture` where that matters.
fn repeat_write_payload(pass: u8) -> [u8; 64] {
    let mut p = [0u8; 64];
    p[..8].copy_from_slice(b"CRKRPT__");
    p[7] = b'0' + pass;
    for (i, b) in p.iter_mut().enumerate().skip(8) {
        *b = (i as u8).wrapping_mul(31) ^ pass;
    }
    p
}

/// **The repeat-write gate** (fix/redoxfs-repeat-write). A first write to a pristine block
/// worked all along; a write to a block that has ALREADY been written is what loops on device.
/// The old gate never saw it because `mkredoxfs` rewrites the target to a placeholder before
/// every run, so every gated write was a first write and the bug hid behind the harness.
///
/// This writes the same file TWICE in one run, which is the honest reproduction: it depends on
/// nothing left over from a previous invocation. It is also the decisive host-vs-device
/// comparison: if this loops, the bug is reachable with no nife runtime at all (upstream or
/// our chunking); if it passes, the divergence is ours, in the device I/O path.
#[test]
fn a_second_write_to_the_same_block_does_not_loop() {
    let mut srv = server_with(&[("scratch", b"(placeholder)")]);
    let h = srv.open_file("scratch").unwrap();

    // Equal-length, position-dependent payloads: each write fully replaces the last (a shorter
    // write at offset 0 does not truncate, which is correct filesystem behaviour but would muddy
    // the comparison), and a buffer that came back shifted or stale cannot match.
    let mut buf = [0u8; 128];
    for pass in 1..=3u8 {
        let payload = repeat_write_payload(pass);
        assert_eq!(
            srv.write(h, 0, &payload).unwrap(),
            payload.len(),
            "write {pass} was short"
        );
        let n = srv.read(h, 0, &mut buf).unwrap();
        assert_eq!(&buf[..n], &payload[..], "write {pass} did not read back");
    }
}

/// **The real trigger** (fix/redoxfs-repeat-write): write, drop the mount WITHOUT a clean
/// unmount, reopen, and write again. Every existing test stopped short of this. The one that
/// looked closest, `a_write_survives_a_full_close_and_reopen`, only READS after the reopen.
///
/// This is exactly what the gate does across its two ISA legs: `mkredoxfs` runs once, the aarch64
/// leg mounts and writes (a first write, which works), the process dies without unmounting, and
/// then the riscv leg mounts the same image and writes again. That second boot's write is what
/// fails, which is why the bug looked like "repeat write" and why one leg passing hid it.
#[test]
fn a_write_after_a_reopen_of_a_previously_written_image_does_not_loop() {
    let mut disk = image_with(&[("scratch", b"(placeholder)")]);

    // Boot 1: mount, write, and drop the mount the way a dying process does (no unmount).
    {
        let mut srv = Server::open(disk).expect("open 1");
        let h = srv.open_file("scratch").expect("open scratch 1");
        let p1 = repeat_write_payload(1);
        assert_eq!(srv.write(h, 0, &p1).unwrap(), p1.len());
        disk = srv.fs.disk; // take the disk back; no unmount, as on device
    }

    // Boot 2: mount the image boot 1 wrote, and write again.
    let mut srv = Server::open(disk).expect("open 2");
    let h = srv.open_file("scratch").expect("open scratch 2");
    let p2 = repeat_write_payload(2);
    assert_eq!(srv.write(h, 0, &p2).unwrap(), p2.len(), "boot-2 write");
    let mut buf = [0u8; 128];
    let n = srv.read(h, 0, &mut buf).unwrap();
    assert_eq!(&buf[..n], &p2[..], "boot-2 write did not read back");
}

/// **The mechanism behind the "cross-boot second-mount write failure", and it is not a filesystem
/// bug at all** (fix/redoxfs-second-mount). There is no TRUNCATE verb, so a write shorter than the
/// file leaves the previous write's tail in place. That is correct behaviour, and it is what the
/// contract offers today, but it is sharp: a test that writes N bytes and then compares a
/// *whole-file* read against those N bytes passes only if the file was not already longer.
///
/// That is exactly what happened. One boot's client left a 64-byte payload in `scratch`; the next
/// boot's `std::fs` test wrote its 61-byte pattern and asserted the whole file equalled it, got 64
/// bytes back, and panicked in the write block. It looked like "a second mount of a used image
/// fails its write", and three rounds chased a filesystem bug that was never there. The write
/// succeeded every time.
///
/// The byte counts here are the real ones so the arithmetic is legible rather than abstract.
#[test]
fn a_shorter_write_does_not_truncate_and_that_is_what_broke_across_boots() {
    let mut srv = server_with(&[("scratch", b"(placeholder)")]);
    let h = srv.open_file("scratch").unwrap();

    // Boot 1's client wrote 64 bytes.
    let long = [b'L'; 64];
    assert_eq!(srv.write(h, 0, &long).unwrap(), 64);
    assert_eq!(srv.fstat(h).unwrap(), 64);

    // Boot 2's std::fs wrote its 61-byte pattern over it. The write itself is fine.
    let short = [b'S'; 61];
    assert_eq!(srv.write(h, 0, &short).unwrap(), 61);

    // And the file is STILL 64 bytes, because nothing truncates it.
    assert_eq!(
        srv.fstat(h).unwrap(),
        64,
        "a shorter write must not truncate; TRUNCATE is the verb that fixes this, not WRITE"
    );
    let mut buf = [0u8; 128];
    let n = srv.read(h, 0, &mut buf).unwrap();
    assert_eq!(n, 64, "a whole-file read returns the OLD length");
    assert_eq!(&buf[..61], &short[..], "the new bytes landed");
    assert_eq!(
        &buf[61..64],
        b"LLL",
        "and the longer write's tail survives, which is what failed the comparison"
    );
}

#[test]
fn a_write_survives_a_full_close_and_reopen() {
    // Persistence across a full close/reopen is what the on-disk image buys, and it is the
    // property the kill-mid-write test leans on: a reopen sees the committed write.
    let disk = image_with(&[("f", b"")]);
    let mut srv = Server::open(disk).unwrap();
    let h = srv.open_file("f").unwrap();
    srv.write(h, 0, b"persisted").unwrap();
    let disk = srv.fs.disk; // the FileSystem committed on each tx; take the disk back
    let mut srv = Server::open(disk).unwrap();
    let h = srv.open_file("f").unwrap();
    let mut buf = [0u8; 16];
    let n = srv.read(h, 0, &mut buf).unwrap();
    assert_eq!(&buf[..n], b"persisted");
}

#[test]
fn a_forged_or_closed_handle_is_ebadf() {
    let mut srv = server_with(&[("f", b"data")]);
    let h = srv.open_file("f").unwrap();
    let mut buf = [0u8; 8];
    assert_eq!(srv.read(999, 0, &mut buf).unwrap_err().errno, EBADF);
    srv.close(h).unwrap();
    assert_eq!(srv.read(h, 0, &mut buf).unwrap_err().errno, EBADF);
    assert_eq!(srv.close(h).unwrap_err().errno, EBADF);
}

#[test]
fn handles_are_reused_after_close() {
    let mut srv = server_with(&[("a", b"1"), ("b", b"2")]);
    let h0 = srv.open_file("a").unwrap();
    assert_ne!(
        h0, 0,
        "slot 0 is the bound directory and is never handed out"
    );
    srv.close(h0).unwrap();
    let h1 = srv.open_file("b").unwrap();
    assert_eq!(h0, h1, "a freed slot is reused before growing the table");
}

/// Build an image with a subtree: files in the root, then `sub/` holding `inner` and `deeper/`,
/// and a sibling `other/` holding `secret`. The same shape `filesystem_protocol::fixture::tree` describes
/// and the device fixture carries, so the host tests and the guest tests attack one geometry.
fn image_with_tree() -> DiskMemory {
    let disk = DiskMemory::new(16 * 1024 * 1024);
    let mut fs = FileSystem::create(disk, None, 0, 0).expect("create");
    fs.tx(|tx| {
        let file = |tx: &mut redoxfs::Transaction<DiskMemory>,
                    parent: TreePtr<Node>,
                    name: &str,
                    body: &[u8]| {
            let ptr = tx
                .create_node(parent, name, Node::MODE_FILE | 0o644, 0, 0)?
                .ptr();
            tx.write_node(ptr, 0, body, 0, 0)?;
            Ok::<(), Error>(())
        };
        file(tx, TreePtr::root(), "motd", b"root motd")?;
        let sub = tx
            .create_node(TreePtr::root(), "sub", Node::MODE_DIR | 0o755, 0, 0)?
            .ptr();
        file(tx, sub, "inner", b"inside the grant")?;
        let deeper = tx
            .create_node(sub, "deeper", Node::MODE_DIR | 0o755, 0, 0)?
            .ptr();
        file(tx, deeper, "leaf", b"two levels down")?;
        let other = tx
            .create_node(TreePtr::root(), "other", Node::MODE_DIR | 0o755, 0, 0)?
            .ptr();
        file(tx, other, "secret", b"in the sibling")?;
        Ok(())
    })
    .expect("populate the tree");
    fs.disk
}

fn server_with_tree() -> Server<DiskMemory> {
    Server::open(image_with_tree()).expect("open")
}

/// Collect a whole directory listing through `read_dir`, driving the cursor the way a client
/// does, so the cursor and the "a record is never split" rule are exercised rather than assumed.
fn list(srv: &mut Server<DiskMemory>, handle: u32, buf_len: usize) -> Result<Vec<(String, bool)>> {
    let mut out = Vec::new();
    let mut buf = vec![0u8; buf_len];
    let mut cursor = 0u32;
    // Bounded so a cursor that fails to advance fails the test instead of hanging it.
    for _ in 0..64 {
        let n = srv.read_dir(handle, cursor, &mut buf)?;
        if n == 0 {
            return Ok(out);
        }
        for (name, is_dir) in filesystem_protocol::dirent::iter(&buf[..n]) {
            out.push((String::from_utf8(name.to_vec()).unwrap(), is_dir));
            cursor += 1;
        }
    }
    panic!("read_dir never reported the end of the directory");
}
