//! **Path-level operations**.
//!
//! Cut out of `sys/fs/nife.rs` unchanged on 2026-10-12 (UTC) by lane
//! maintainer/near-ceiling-splits, for §266 (a Rust source file stays under 2,000 lines) and §282
//! (when a file is split, the parent becomes an index). What was private to `nife` is `pub(super)`
//! here, which reaches the same modules: `nife` and its children.
//!
//! Name: provisional (lane maintainer/near-ceiling-splits, 2026-10-12 UTC), for the module and the
//! file.

use super::*;

/// A name's metadata: open, `FSTAT`, close, then `GETMTIME`. Four messages instead of one, because
/// the contract has no stat-by-name verb; the effect is the same and the authority is identical (the
/// name still resolves only under the granted directory, and every one of the four needs the same
/// `dir::READ` the walk asks for).
///
/// **A directory answers too** (milestone 64). `OPEN` refuses a directory with `EISDIR`, and that
/// refusal *is* the answer to "what kind of thing is this name", so it is read as one rather than
/// propagated. Without this, `Path::is_dir()` was false for every directory, and
/// `std::fs::create_dir_all` was not idempotent: it recovers from `AlreadyExists` by asking whether
/// the name is already a directory, and got told no.
///
/// **The mtime is asked for by name, after the kind is known** (milestone 64's last pass). The walk
/// has already reached the directory holding the name, so `GETMTIME` against that handle is one
/// more message and needs no right the walk did not already ask for. It is issued after `OPEN`
/// rather than before so that every *error* this function returns is the one it returned before
/// the mtime existed: a refused `GETMTIME` is kept in the [`FileAttr`] and surfaces from
/// `modified()`, never from here (see [`Mtime`]). The four messages are not atomic, so a name
/// replaced between `OPEN` and `GETMTIME` reports the new node's time with the old node's size,
/// which is the same window a Unix program that calls `stat` twice has.
///
/// The size it reports for a directory is 0 and that is a placeholder rather than a measurement,
/// for the reason [`FileAttr`] gives. `accessed`/`created` still refuse, so nothing here invents a
/// fact the contract does not carry.
pub fn stat(path: &Path) -> io::Result<FileAttr> {
    if !is_reachable() {
        return Err(unsupported_err());
    }
    // ``, `.` and `/` name the granted directory, which no `OPEN` can reach: it is the directory
    // the endpoint is bound to rather than a name inside it. Holding the capability at all is the
    // whole of what there is to know about it, and it has no name to ask an mtime by. (`/` used
    // to fall through to the walk and come back `InvalidFilename`, a leftover from before a
    // leading slash named this process's root; `count_names` is the same test `readdir` uses.)
    if count_names(path)? == 0 {
        return Ok(FileAttr { size: 0, dir: true, mtime: Mtime::Unnamed });
    }
    stat_under(proto::ROOT, path)
}

/// The name-shaped stat against whichever directory handle the caller holds: the root endpoint for
/// the free functions above, a [`Dir`]'s handle for the dirfd-shaped ones below. One mechanism,
/// because upstream's `fstatat(dirfd, ...)` and `stat(path)` differ in exactly the starting handle
/// and nothing else, and two copies of the four-message dance would be one too many places to
/// learn that `EISDIR` is an answer.
pub(super) fn stat_under(at: u64, path: &Path) -> io::Result<FileAttr> {
    let mut p = page();
    let (at, name) = walk(&mut p, at, path, fsproto::dir::READ)?;
    p.put(name.as_bytes());
    let (size, dir) = match request(proto::req(proto::OPEN, at.0, name.len() as u64), 0) {
        Ok(handle) => {
            let size = request(proto::req(proto::FSTAT, handle, 0), 0);
            // Closed before `size?` can return, so an `FSTAT` failure does not leak the handle.
            let _ = request(proto::req(proto::CLOSE, handle, 0), 0);
            (size?, false)
        }
        Err(e) if e.kind() == io::ErrorKind::IsADirectory => (0, true),
        Err(e) => return Err(e),
    };
    // The name again: nothing promises `OPEN`'s reply left the page as it found it.
    p.put(name.as_bytes());
    let (r0, _) = rt::call(FS, proto::req(proto::GETMTIME, at.0, name.len() as u64), 0);
    let mtime = match fsproto::reply_errno(r0 as i64) {
        None => Mtime::Seconds(r0),
        Some(_) => Mtime::Refused(r0),
    };
    Ok(FileAttr { size, dir, mtime })
}

/// No symlinks cross this contract, so following one and not following one are the same thing.
pub fn lstat(path: &Path) -> io::Result<FileAttr> {
    stat(path)
}

pub fn exists(path: &Path) -> io::Result<bool> {
    match stat(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

/// **List a directory** (`OPENDIR` + `READDIR`, milestone 64).
///
/// Two shapes, and the difference is which directory capability answers:
///
/// - `read_dir(".")` (or `read_dir("")`) lists **the granted directory itself**, which is handle
///   `fs::ROOT` and costs no `OPENDIR`.
/// - `read_dir("sub")` asks the server for a capability to `sub` first, lists through it, and
///   closes it. That is descend-then-enumerate, and it is the only way to reach `sub`: this
///   process holds no name for it other than through its parent.
///
/// The listing is drained here rather than streamed; [`ReadDir`] says why.
pub fn readdir(p: &Path) -> io::Result<ReadDir> {
    if !is_reachable() {
        return Err(unsupported_err());
    }
    // One guard for the whole exchange, because `page()` is not reentrant: every OPENDIR name,
    // every READDIR page, and the CLOSE all run under it. That also makes the listing a snapshot
    // with no other operation of ours interleaved.
    let mut page = page();

    // `ENUMERATE` and nothing more, for the reason `DirBuilder::mkdir` spells out: over-asking is
    // `EPERM`, not attenuation, so a PAL that asked for `dir::ALL` here would work through the
    // test's full-rights grant and fail through every narrowed one. Every hop on the way to a
    // nested path asks for `DESCEND | ENUMERATE`, which is the same minimum one level up.
    let at = dir_at(&mut page, p, fsproto::dir::ENUMERATE)?;
    let handle = at.0;

    let mut entries: Vec<(OsString, bool)> = Vec::new();
    let mut buf = [0u8; PAGE];
    let mut cursor: u64 = 0;
    let result = loop {
        // `req_len` is ignored by READDIR; the cursor rides in the second word.
        let filled = match request(proto::req(proto::READDIR, handle, 0), cursor) {
            Ok(n) => n as usize,
            Err(e) => break Err(e),
        };
        // Zero bytes means the cursor is past the end, which is how the contract tells "the
        // directory ended" apart from "the page was full" without the client guessing.
        if filled == 0 {
            break Ok(());
        }
        let filled = filled.min(PAGE);
        page.get(&mut buf[..filled]);
        let mut this_page = 0u64;
        for (name, dir) in fsproto::dirent::iter(&buf[..filled]) {
            // A name is bytes on the wire and an `OsString` in std, and on this target an
            // `OsString` is UTF-8. A name the FS server holds that is not UTF-8 cannot be
            // represented, so it is skipped rather than lossily renamed: handing back a name that
            // does not open is worse than not listing it.
            if let Ok(s) = core::str::from_utf8(name) {
                entries.push((OsString::from(s), dir));
            }
            this_page += 1;
        }
        if this_page == 0 {
            // A non-empty reply that decoded to no records is a malformed page, and looping on it
            // would spin forever against a server that keeps sending it.
            break Err(io::const_error!(
                io::ErrorKind::InvalidData,
                "the FS server returned a directory page with no readable entries"
            ));
        }
        cursor += this_page;
    };

    // `at` closes what it minted, and never closes `ROOT`.
    drop(at);
    result?;

    Ok(ReadDir { root: p.to_path_buf(), entries: entries.into_iter() })
}

/// **Remove a name** (`UNLINK`, milestone 64). Not revocation: a handle already open on the file
/// keeps reading, exactly as POSIX promises, and `filesystem_contract::fs::UNLINK` records why the contract
/// draws that line. A directory is refused (`IsADirectory`); [`rmdir`] is its verb.
pub fn unlink(p: &Path) -> io::Result<()> {
    if !is_reachable() {
        return Err(unsupported_err());
    }
    let mut page = page();
    let (at, name) = walk(&mut page, proto::ROOT, p, fsproto::dir::REMOVE)?;
    page.put(name.as_bytes());
    request(proto::req(proto::UNLINK, at.0, name.len() as u64), 0)?;
    Ok(())
}

/// **Remove an empty directory** (`RMDIR`, milestone 64). Empty-only is the safety property, not a
/// missing feature: `DirectoryNotEmpty` is the answer for anything else, and the recursion that
/// Unix's `rm -r` does belongs in userspace where each step can be checked against the capability
/// for that level. A file is refused (`NotADirectory`), the mirror of [`unlink`]'s refusal.
pub fn rmdir(p: &Path) -> io::Result<()> {
    if !is_reachable() {
        return Err(unsupported_err());
    }
    let mut page = page();
    let (at, name) = walk(&mut page, proto::ROOT, p, fsproto::dir::REMOVE)?;
    page.put(name.as_bytes());
    request(proto::req(proto::RMDIR, at.0, name.len() as u64), 0)?;
    Ok(())
}

/// **Move a name** (`RENAME`, milestone 64), and the reason this one matters out of proportion to
/// how often crates name it: write-a-temp-then-rename is how every careful program replaces a file,
/// and `filesystem_contract::fs::RENAME` is both concurrency-atomic and crash-atomic, which is what that idiom
/// actually needs.
///
/// The two names travel in the shared page back to back, source first, because one request word
/// cannot carry two lengths.
///
/// **`RENAME` is the one verb that names two directories**, and since milestone 122 both of them
/// can be somewhere other than the granted root: each side is walked separately and its handle goes
/// into the request, which is what makes `rename("a/tmp", "b/final")` a single atomic message
/// rather than something a caller has to build out of copy and unlink. The rights asked for differ
/// by side, and that is the contract's rather than a nicety: the source directory loses a name
/// (`REMOVE`) and the destination gains one (`CREATE`).
pub fn rename(old: &Path, new: &Path) -> io::Result<()> {
    if !is_reachable() {
        return Err(unsupported_err());
    }
    let mut page = page();
    let (src_at, src) = walk(&mut page, proto::ROOT, old, fsproto::dir::REMOVE)?;
    let (dst_at, dst) = walk(&mut page, proto::ROOT, new, fsproto::dir::CREATE)?;
    if src.len() + dst.len() > PAGE {
        return Err(io::const_error!(
            io::ErrorKind::InvalidFilename,
            "the two names together are longer than the page shared with the FS server"
        ));
    }
    page.put(src.as_bytes());
    page.put_at(src.len(), dst.as_bytes());
    request(
        proto::req(proto::RENAME, src_at.0, src.len() as u64),
        proto::rename_dst(dst_at.0, dst.len() as u64),
    )?;
    Ok(())
}

/// **Copy a file's contents** (milestone 64, rank 26), as an open, a read/write loop, and two
/// closes. No verb backs this and none needs to: unlike `rename`, which has to be atomic and
/// therefore has to be the server's job, a copy is exactly "read all of one, write all of the
/// other", and doing it here costs the same messages doing it in the caller would.
///
/// Both names resolve under the granted directory, so this cannot copy across a capability boundary;
/// a program holding two directories needs the namespace milestone 47 owns.
///
/// **What it does not copy is permissions**, which every other platform's `fs::copy` does. There are
/// none to copy (§27 has no mode bits), so the usual caveat is inverted: nothing is silently lost,
/// because there was never anything there.
pub fn copy(from: &Path, to: &Path) -> io::Result<u64> {
    let mut reader = OpenOptions::new();
    reader.read(true);
    let src = File::open(from, &reader)?;

    // Refuse a directory before creating anything, so a mistaken `copy("dir", "x")` does not leave
    // an empty `x` behind. `File::open` already answers `EISDIR` for a directory source, so reaching
    // here means the source is a file.
    let mut writer = OpenOptions::new();
    writer.write(true);
    writer.create(true);
    writer.truncate(true);
    let dst = File::open(to, &writer)?;

    let mut buf = [0u8; PAGE];
    let mut copied: u64 = 0;
    loop {
        let n = src.read(&mut buf)?;
        if n == 0 {
            break;
        }
        let mut written = 0;
        while written < n {
            // A short write is not an error and not a loop-exit: the contract's `WRITE` is capped at
            // one page and may return less, so the only correct read of `Ok(0)` here is a full disk,
            // which would otherwise spin.
            match dst.write(&buf[written..n])? {
                0 => {
                    return Err(io::const_error!(
                        io::ErrorKind::WriteZero,
                        "the FS server accepted no bytes; the destination cannot grow"
                    ));
                }
                k => written += k,
            }
        }
        copied += n as u64;
    }
    Ok(copied)
}

pub fn set_perm(_p: &Path, _perm: FilePermissions) -> io::Result<()> {
    unsupported()
}

/// The `nofollow` twin of [`set_perm`], added upstream in the 2026-07-31 nightly.
///
/// Cannot be re-exported from `unsupported_fs` the way `rename` and friends are: that version takes
/// `unsupported_fs::FilePermissions`, and this module defines its own, so the signature does not
/// match. It refuses for the same reason `set_perm` does — permissions are not a thing this contract
/// has (§27), so there is nothing to set and nothing to follow or not follow.
pub fn set_perm_nofollow(_p: &Path, _perm: FilePermissions) -> io::Result<()> {
    unsupported()
}

/// **`std::fs::set_times`** (milestone 64, rank 28): `SETMTIME_AT`, the caller's time, by name.
///
/// **Always `SETMTIME_AT`, never `SETMTIME`, and the rights follow from that.** std hands this
/// function a `SystemTime` the caller chose, which is an assertion about history even when the
/// caller computed it from `SystemTime::now()`; DECISIONS §112 put exactly that authority behind
/// `dir::SETTIME`, separate from `dir::WRITE`. So the walk asks for `WRITE | SETTIME` and a grant
/// without `SETTIME` answers `ReadOnlyFilesystem`. Falling back to `SETMTIME` there would write the
/// server's own "now" in place of the time the caller asked for and report success, which is the
/// one thing this function must not do; and the server's "now" is a logical counter, not a time
/// (see `FileAttr::modified`).
///
/// Three things are refused before any message is sent, so a refusal never leaves half a change
/// behind:
///
/// - **An access time.** No verb sets one. Refusing the whole call is the honest answer to a
///   request this contract can only half carry; setting the mtime and dropping the atime would
///   report success for something that did not happen.
/// - **A time past `i64::MAX` seconds.** The contract's reply word is signed, so `GETMTIME` could
///   never report such a value back; it would read as an error. The server accepts it (§112 does
///   not referee plausibility), and this PAL declines to write a time it cannot read.
/// - **The granted directory itself** (``, `.`, `/`), which has no name to ask by.
///
/// **Whole seconds.** The wire carries seconds and the server stores a zero nanosecond part, so a
/// sub-second part is truncated. That is the same answer a second-granularity filesystem gives on
/// Unix, and std documents timestamp precision as platform-dependent; it is recorded in
/// notes/std/file-times.md's BUGS rather than refused, because refusing every `SystemTime::now()` would make
/// the call unusable for its commonest caller.
///
/// A `FileTimes` with nothing set changes nothing, and still resolves the name, so a missing file
/// is `NotFound` as it is on Unix rather than a vacuous success.
pub fn set_times(p: &Path, times: FileTimes) -> io::Result<()> {
    if !is_reachable() {
        return Err(unsupported_err());
    }
    if times.accessed.is_some() {
        return Err(io::const_error!(
            io::ErrorKind::Unsupported,
            "no verb in the file contract sets an access time; nothing was changed"
        ));
    }
    let Some(modified) = times.modified else {
        return stat(p).map(|_| ());
    };
    if count_names(p)? == 0 {
        return Err(io::const_error!(
            io::ErrorKind::Unsupported,
            "the granted directory has no name to set its mtime by"
        ));
    }
    // `SystemTime` on this target is a `Duration` since the epoch, so this cannot fail; a time
    // before 1970 is not representable here at all, which is the time PAL's own limit.
    let since = modified.sub_time(&UNIX_EPOCH).map_err(|_| {
        io::const_error!(io::ErrorKind::InvalidInput, "an mtime before 1970 cannot cross this contract")
    })?;
    let secs = since.as_secs();
    if secs > i64::MAX as u64 {
        return Err(io::const_error!(
            io::ErrorKind::InvalidInput,
            "an mtime this far out could be written but never read back over this contract"
        ));
    }
    let mut page = page();
    let (at, name) =
        walk(&mut page, proto::ROOT, p, fsproto::dir::WRITE | fsproto::dir::SETTIME)?;
    page.put(name.as_bytes());
    request(proto::req(proto::SETMTIME_AT, at.0, name.len() as u64), secs)?;
    Ok(())
}

/// No symlinks cross this contract, so not following one is [`set_times`] itself.
pub fn set_times_nofollow(p: &Path, times: FileTimes) -> io::Result<()> {
    set_times(p, times)
}

/// Directories are created with `MKDIR`, which needs no options: the contract has no mode bits to
/// set, and `recursive` is std's own loop over this.
#[derive(Copy, Clone, Debug)]
pub struct DirBuilder {}

impl DirBuilder {
    pub fn new() -> DirBuilder {
        DirBuilder {}
    }

    /// `MKDIR` under the granted directory. The verb hands back a **capability to the new
    /// directory** (it is descend-with-creation, §47), and std's `create_dir` returns `()`, so the
    /// handle is closed immediately. Nothing is lost by that: a later `read_dir` of the same name
    /// mints another one, and holding this one would be a handle-table slot leaked per `mkdir`.
    pub fn mkdir(&self, p: &Path) -> io::Result<()> {
        if !is_reachable() {
            return Err(unsupported_err());
        }
        let mut page = page();
        let (at, name) = walk(&mut page, proto::ROOT, p, fsproto::dir::CREATE | fsproto::dir::DESCEND)?;
        page.put(name.as_bytes());
        // **The second word asks for rights on the child, and asking for too much is a refusal
        // rather than an attenuation.** The server intersects the request with the parent's rights
        // and answers `EPERM` if the result is smaller than what was asked (§47's monotonicity is
        // the intersection; the refusal is it telling the truth about it). A PAL cannot know what
        // its own directory capability carries, so it must ask for the *minimum the operation
        // needs* or it breaks under every narrowed grant. `create_dir` needs nothing from the
        // handle it gets back, because it closes it.
        let handle = request(proto::req(proto::MKDIR, at.0, name.len() as u64), 0)?;
        let _ = request(proto::req(proto::CLOSE, handle, 0), 0);
        Ok(())
    }
}
