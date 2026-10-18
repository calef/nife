extern crate std;

use super::*;

const SIZE: u64 = 256 << 20;
const BS: usize = blk::BLOCK_SIZE;

fn call(t: &mut Tree, code: u64, handle: u64, name: &[u8], w1: u64) -> Answer {
    let mut page = [0u8; fs::TRANSFER_MAX];
    page[..name.len()].copy_from_slice(name);
    t.handle(fs::req(code, handle, name.len() as u64), w1, &mut page)
}

fn read(t: &mut Tree, handle: u64, len: u64, offset: u64) -> Answer {
    let mut page = [0u8; fs::TRANSFER_MAX];
    t.handle(fs::req(fs::READ, handle, len), offset, &mut page)
}

fn handle_of(a: Answer) -> u64 {
    match a {
        Answer::Reply(h, _) if h >= 0 => h as u64,
        other => panic!("expected a handle, got {other:?}"),
    }
}

/// The walk a C program behind a directory grant of [`DIRECTORY`] makes: its caretaker's one
/// `OPENDIR`, then `OPEN` of the file, which answers the disk's size in the second word.
fn opened(t: &mut Tree) -> u64 {
    let d = handle_of(call(t, fs::OPENDIR, fs::ROOT, DIRECTORY, dir::READ));
    match call(t, fs::OPEN, d, FILE, 0) {
        Answer::Reply(h, size) if h >= 0 => {
            assert_eq!(size, SIZE);
            h as u64
        }
        other => panic!("OPEN refused: {other:?}"),
    }
}

#[test]
fn ioping_at_one_mebibyte_reads_block_256_from_the_device() {
    let mut t = Tree::new(SIZE);
    let f = opened(&mut t);
    assert_eq!(
        call(&mut t, fs::FSTAT, f, b"", 0),
        Answer::Reply(SIZE as i64, 0)
    );
    assert_eq!(
        read(&mut t, f, BS as u64, 1 << 20),
        Answer::Read(Read {
            block: 256,
            blocks: 1,
            skip: 0,
            len: BS
        })
    );
}

/// The property the whole tree exists for: nothing answers a read from memory. The same read
/// twice is two device reads, where `redoxfs_server` would answer the second from its memo.
#[test]
fn the_same_read_twice_goes_to_the_device_twice() {
    let mut t = Tree::new(SIZE);
    let f = opened(&mut t);
    let first = read(&mut t, f, BS as u64, 8 * BS as u64);
    assert!(matches!(first, Answer::Read(_)));
    assert_eq!(read(&mut t, f, BS as u64, 8 * BS as u64), first);
}

#[test]
fn an_unaligned_read_spans_the_blocks_it_touches() {
    let mut t = Tree::new(SIZE);
    let f = opened(&mut t);
    assert_eq!(
        read(&mut t, f, BS as u64, 3 * BS as u64 + 100),
        Answer::Read(Read {
            block: 3,
            blocks: 2,
            skip: 100,
            len: BS
        })
    );
}

#[test]
fn a_read_comes_back_short_at_the_window_and_at_the_end() {
    let mut t = Tree::new(SIZE);
    let f = opened(&mut t);
    let window = blk::TRANSFER_BLOCKS * BS;
    assert_eq!(
        read(&mut t, f, fs::TRANSFER_MAX as u64, 10),
        Answer::Read(Read {
            block: 0,
            blocks: blk::TRANSFER_BLOCKS,
            skip: 10,
            len: window - 10
        })
    );
    assert_eq!(
        read(&mut t, f, BS as u64, SIZE - 10),
        Answer::Read(Read {
            block: SIZE / BS as u64 - 1,
            blocks: 1,
            skip: BS - 10,
            len: 10
        })
    );
    assert_eq!(read(&mut t, f, BS as u64, SIZE), Answer::Reply(0, 0));
}

#[test]
fn a_path_reaches_the_file_from_the_root() {
    let mut t = Tree::new(SIZE);
    let path = [DIRECTORY, b"/", FILE].concat();
    assert!(
        matches!(call(&mut t, fs::OPEN, fs::ROOT, &path, dir::READ), Answer::Reply(h, SIZE) if h > 0)
    );
    assert_eq!(
        call(&mut t, fs::OPEN, fs::ROOT, b"nvme0/../n1", dir::READ),
        err(dir::EINVAL)
    );
}

#[test]
fn nothing_writes() {
    let mut t = Tree::new(SIZE);
    let f = opened(&mut t);
    for code in [fs::WRITE, fs::TRUNCATE] {
        assert_eq!(call(&mut t, code, f, b"", 0), err(dir::EROFS));
    }
    for code in [fs::CREATE, fs::MKDIR, fs::UNLINK, fs::RMDIR, fs::SETMTIME] {
        assert_eq!(call(&mut t, code, fs::ROOT, b"x", 0), err(dir::EROFS));
    }
    // A descent cannot ask for a right the root never had.
    assert_eq!(
        call(&mut t, fs::OPENDIR, fs::ROOT, DIRECTORY, dir::WRITE),
        err(dir::EPERM)
    );
}

#[test]
fn a_grant_without_read_cannot_open_the_file() {
    let mut t = Tree::new(SIZE);
    let d = handle_of(call(
        &mut t,
        fs::OPENDIR,
        fs::ROOT,
        DIRECTORY,
        dir::ENUMERATE,
    ));
    assert_eq!(call(&mut t, fs::OPEN, d, FILE, 0), err(ENOENT));
}

#[test]
fn listings_name_what_is_there() {
    let mut t = Tree::new(SIZE);
    let mut page = [0u8; fs::TRANSFER_MAX];
    let Answer::Reply(n, _) = t.handle(fs::req(fs::READDIR, fs::ROOT, 0), 0, &mut page) else {
        panic!()
    };
    let got: std::vec::Vec<_> = dirent::iter(&page[..n as usize]).collect();
    assert_eq!(got, [(DIRECTORY, true)]);
    assert_eq!(
        t.handle(fs::req(fs::READDIR, fs::ROOT, 0), 1, &mut page),
        Answer::Reply(0, 0)
    );
}

/// The C library's reachability probe must get a server's refusal, not one of the kernel's
/// "not granted" errors (-1, -2, -3), or the program concludes it holds no directory.
#[test]
fn the_reachability_probe_is_refused_as_a_bad_handle() {
    let mut t = Tree::new(SIZE);
    assert_eq!(call(&mut t, fs::FSTAT, fs::MAX_HANDLE, b"", 0), err(EBADF));
    assert_eq!(call(&mut t, fs::CLOSE, fs::ROOT, b"", 0), err(dir::EINVAL));
}
