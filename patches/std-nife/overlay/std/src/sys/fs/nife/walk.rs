//! **Paths: a chain of names under the granted directory**.
//!
//! Cut out of `sys/fs/nife.rs` unchanged on 2026-10-12 (UTC) by lane
//! maintainer/near-ceiling-splits, for §266 (a Rust source file stays under 2,000 lines) and §282
//! (when a file is split, the parent becomes an index). What was private to `nife` is `pub(super)`
//! here, which reaches the same modules: `nife` and its children.
//!
//! Name: provisional (lane maintainer/near-ceiling-splits, 2026-10-12 UTC), for the module and the
//! file.

use super::*;

/// Check every component of `path` and count the names that survive, or refuse the path.
///
/// This is where "no global namespace" is enforced, on the client side, before a byte reaches the
/// server. The server enforces it again (it resolves a single component under the handle it is
/// given and nothing else); doing it here as well is not redundant, it is what turns a would-be
/// escape into a legible `io::Error` instead of an `ENOENT` that reads like a missing file.
///
/// Every refusal is [`io::ErrorKind::InvalidFilename`], never `PermissionDenied`: no permission was
/// consulted, and there is no name for what was asked, because this process holds no capability
/// that designates it. The message says which case it was.
///
/// **A nested path is no longer one of them** (milestone 122). It is a chain of names, and [`walk`]
/// spends one attenuated `OPENDIR` per link to reach the last one. **Nor is a leading `/`**
/// (milestone 47's namespace half, 2026-08-18): `/` is the root of *this process's* namespace,
/// which is the directory it was granted, so `/motd` and `motd` name one file and neither can
/// reach anything the grant does not. That is Plan 9's answer rather than DOS's, and here it is
/// less than Plan 9's: a nife process has exactly one root, so there is nothing for the leading
/// slash to select between. What is still refused is what still names nothing: `..` at any
/// position, and a Windows-shaped prefix.
pub(super) fn count_names(path: &Path) -> io::Result<usize> {
    let mut n = 0;
    for c in path.components() {
        match c {
            // "./motd" is "motd", and "a/./b" is "a/b": the current directory IS the one the walk
            // has reached.
            Component::CurDir => {}
            // `/motd` is `motd`, for the same reason `./motd` is: this process has exactly one
            // root and it is the directory it was granted. The slash selects nothing, because
            // there is nothing else to select; what it buys is that a path a program built from
            // `current_dir()` resolves instead of being refused.
            //
            // **A leading slash is relative to the base the walk was given, and that is a
            // deliberate divergence from `openat`.** POSIX makes an absolute path ignore the
            // `dirfd`; here `Dir::open_file("/x")` resolves under that `Dir` rather than jumping
            // back to the granted root. The Unix rule exists because `/` names one global thing
            // and a `dirfd` is a shortcut into it. Neither half is true here: a handle is a
            // namespace, `/` is the root of whichever one you are resolving in, and a rule that
            // let a name climb out of the handle it was asked of would make a `Dir` useless for
            // the one thing programs reach for it for. It also cannot widen anything either way,
            // since a process that holds a `Dir` holds the root it descended from.
            Component::RootDir => {}
            // A Windows-shaped prefix (`C:`, `\\?\`) is still nothing here, and unlike a leading
            // slash there is no root it could name.
            Component::Prefix(_) => {
                return Err(io::const_error!(
                    io::ErrorKind::InvalidFilename,
                    "a path prefix names nothing here: this process holds a directory \
                     capability, not a drive or a share"
                ));
            }
            // Refused at every position, not just the first. `sub/../other` would be a descent
            // followed by an ascent, and there is no verb for the ascent and no capability that
            // would designate what it reached: a handle names a directory, and nothing on the wire
            // names its parent. This is the property that makes the walk safe by construction
            // rather than by checking, and it is why `cap-primitives` has to work so much harder on
            // Unix (see notes/std/fs-descent.md).
            Component::ParentDir => {
                return Err(io::const_error!(
                    io::ErrorKind::InvalidFilename,
                    "`..` would leave the granted directory, which no capability designates"
                ));
            }
            Component::Normal(part) => {
                let name = part.to_str().ok_or_else(|| {
                    io::const_error!(
                        io::ErrorKind::InvalidFilename,
                        "a file name must be UTF-8 to cross this contract"
                    )
                })?;
                if name.len() > PAGE {
                    return Err(io::const_error!(
                        io::ErrorKind::InvalidFilename,
                        "a name is longer than the page shared with the FS server"
                    ));
                }
                n += 1;
            }
        }
    }
    Ok(n)
}

/// The names in `path`, in order. Only ever called after [`count_names`] has agreed there is
/// nothing else in there, so the `to_str` that cannot fail is dropped rather than unwrapped.
pub(super) fn names(path: &Path) -> impl Iterator<Item = &str> {
    path.components().filter_map(|c| match c {
        Component::Normal(part) => part.to_str(),
        _ => None,
    })
}

/// A directory handle a request is issued against: [`proto::ROOT`] for the granted directory, or
/// one a descent minted.
///
/// It closes what it minted and never closes `ROOT`, which is not this process's to close: `ROOT`
/// is the directory the endpoint is bound to rather than a handle anybody handed out. The server's
/// table has the bound directory in slot 0 and mints from 1 up, so `handle != ROOT` is exactly
/// "we made this".
pub(super) struct At(pub(super) u64);

impl Drop for At {
    fn drop(&mut self) {
        if self.0 != proto::ROOT {
            // A failure has nowhere to go: we are done with the handle either way, and a leaked
            // server-side slot is the cost of a reply we could not have acted on.
            let _ = request(proto::req(proto::CLOSE, self.0, 0), 0);
        }
    }
}

/// One hop: `OPENDIR name` under `parent`, asking for `want` on the child.
///
/// **`want` is the whole of the authority question** and the trap in this half of the PAL. The
/// server intersects the request with the parent's rights and answers `EPERM` when the result comes
/// up *short of the request* (§47's monotonicity is the intersection; the refusal is the server
/// refusing to hand back less than was asked for without saying so). A client cannot read its own
/// capability, so asking for more than the operation needs breaks under every narrowed grant while
/// passing every test that grants the root's full rights, which is the mistake `readdir` records
/// having nearly shipped.
pub(super) fn descend(p: &mut Page, parent: &At, name: &str, want: u64) -> io::Result<At> {
    p.put(name.as_bytes());
    let handle = request(proto::req(proto::OPENDIR, parent.0, name.len() as u64), want)?;
    Ok(At(handle))
}

/// **Resolve `path` to the directory its last name lives in, plus that name** (milestone 122).
///
/// `File::open("a/b/c")` is `OPENDIR a`, `OPENDIR b`, then `OPEN c` against `b`'s handle. This is
/// `openat` semantics, which is what the contract already was; what this function adds is that a
/// path-shaped standard library can sit on top of it, which is the whole of milestone 122's option
/// A. Unix answered the same question the same way and kept both interfaces, which is why [`Dir`]
/// exists beside this.
///
/// **The rights story, because it looks like a widening and is not.** Every hop asks for
/// `DESCEND | needs`, where `needs` is what the *final* verb requires on the directory it lands in
/// (`filesystem_contract::verb::TABLE` is the list, and `OPEN`'s "READ or WRITE" is the one row a caller has
/// to resolve from its own `OpenOptions`). Carrying `needs` down the whole chain rather than only
/// asking for it at the end costs nothing that was ever available: a child's rights are its
/// parent's intersected with the request, so a right an ancestor lacks is a right no descendant of
/// it could have had either. The walk therefore asks for exactly the maximum the grant could give
/// at the depth it is going to, and no more.
///
/// **At most two handles are open at once**, because the assignment in the loop drops the previous
/// hop, which closes it. A deep path costs round trips rather than handle-table slots.
pub(super) fn walk<'a>(p: &mut Page, from: u64, path: &'a Path, needs: u64) -> io::Result<(At, &'a str)> {
    let total = count_names(path)?;
    if total == 0 {
        return Err(io::const_error!(
            io::ErrorKind::InvalidFilename,
            "the granted directory itself is not a file this contract can open"
        ));
    }
    let mut at = At(from);
    let mut seen = 0;
    for name in names(path) {
        seen += 1;
        if seen == total {
            return Ok((at, name));
        }
        at = descend(p, &at, name, fsproto::dir::DESCEND | needs)?;
    }
    // `count_names` counted the same components this loop walks, so the return above always fires.
    Err(io::const_error!(
        io::ErrorKind::InvalidFilename,
        "the granted directory itself is not a file this contract can open"
    ))
}

/// Resolve `path` to a **directory** handle, asking for `want` on it.
///
/// The empty path and `"."` name the granted directory itself, which is [`proto::ROOT`] and costs
/// no message: it is the directory the endpoint is bound to rather than a name inside it. Anything
/// else is [`walk`] followed by one more descent, since the last component of a path a caller wants
/// listed is itself the directory rather than a name inside one.
pub(super) fn dir_at(p: &mut Page, path: &Path, want: u64) -> io::Result<At> {
    let names_in_path = count_names(path)?;
    if names_in_path == 0 {
        return Ok(At(proto::ROOT));
    }
    // One `OPENDIR` for the whole path when the server takes paths (milestone 606, ruling A); the
    // server descends each step asking for `DESCEND | want`, which is what the walk below sends.
    if names_in_path > 1 && paths_offered() {
        let joined = joined(path);
        match descend(p, &At(proto::ROOT), &joined, want) {
            Err(e) if refused_as_a_path(&e) => {}
            done => return done.inspect(|_| paths_seen()),
        }
        let (parent, name) = walk(p, proto::ROOT, path, want)?;
        let at = descend(p, &parent, name, want)?;
        paths_refused();
        return Ok(at);
    }
    let (parent, name) = walk(p, proto::ROOT, path, want)?;
    descend(p, &parent, name, want)
}
