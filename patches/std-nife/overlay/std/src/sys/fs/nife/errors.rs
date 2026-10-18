//! **Errors, mapped by meaning**.
//!
//! Cut out of `sys/fs/nife.rs` unchanged on 2026-10-12 (UTC) by lane
//! maintainer/near-ceiling-splits, for §266 (a Rust source file stays under 2,000 lines) and §282
//! (when a file is split, the parent becomes an index). What was private to `nife` is `pub(super)`
//! here, which reaches the same modules: `nife` and its children.
//!
//! Name: provisional (lane maintainer/near-ceiling-splits, 2026-10-12 UTC), for the module and the
//! file.

use super::*;

/// One request on the FS service. Returns the reply's first word as a count/handle, or an
/// `io::Error`. The wire convention is `filesystem_contract`'s: non-negative is a result, negative is a
/// negated errno ([`fsproto::reply_errno`]).
pub(super) fn request(w0: u64, w1: u64) -> io::Result<u64> {
    let (r0, _) = rt::call(FS, w0, w1);
    reply(r0)
}

/// A reply word as a result: [`request`]'s mapping, split out so a reply can be kept and turned into
/// an `io::Error` later. [`FileAttr`] needs that, because `Metadata` is a value std hands back
/// whole and `modified()` is where a refused `GETMTIME` has to surface, not `metadata()`.
pub(super) fn reply(r0: u64) -> io::Result<u64> {
    match fsproto::reply_errno(r0 as i64) {
        None => Ok(r0),
        // The kernel refusing the invoke (a revoked or missing endpoint) is the same answer a
        // program with no directory capability gets: the platform cannot do this.
        Some(_) if no_capability(r0) => Err(unsupported_err()),
        Some(errno) => Err(from_errno(errno)),
    }
}

/// The server's errno into an `io::ErrorKind`, by meaning. There is no errno anywhere else in this
/// PAL: the FS service is the one place nife speaks one, because the component behind it
/// (RedoxFS) does, and §27 maps it at the server boundary and nowhere deeper.
pub(super) fn from_errno(errno: i32) -> io::Error {
    match errno {
        // `EPERM` is milestone 47's, and milestone 122 is what made it reachable from here: a
        // directory capability that does not carry `ENUMERATE`, and a descent that asked for more
        // than the parent holds, both answer it. It is the second errno in this map that is about
        // *you* rather than about the name, so it gets the kind that says so.
        //
        // **`PermissionDenied` here is not the EPERM fiction `count_names`' refusals avoid**, and the
        // difference is which side of the wire decided. Those refusals are this client saying "no
        // capability designates that name", where no permission was ever consulted. This is the
        // server saying "the capability you hold lacks the right that verb needs", which is a fact
        // about authority and is exactly what `dir::EPERM` was specified to mean loudly rather than
        // conceal. `ReadOnlyFilesystem` is its sibling for the mutating rights; std has no third
        // kind that would fit better.
        1 => io::const_error!(
            io::ErrorKind::PermissionDenied,
            "this directory capability does not carry the right that verb needs"
        ),
        2 => io::const_error!(io::ErrorKind::NotFound, "no such name in the granted directory"),
        9 => io::const_error!(
            io::ErrorKind::InvalidInput,
            "the FS server does not honor that handle"
        ),
        // The four below arrived with milestone 64's bindings, and each one exists because the
        // verbs it belongs to distinguish cases the read/write half never had to. Mapping them all
        // to `Other` would have made `remove_dir` on a full directory indistinguishable from a disk
        // error, which is exactly what a caller loops on.
        17 => io::const_error!(io::ErrorKind::AlreadyExists, "that name is already taken"),
        20 => io::const_error!(io::ErrorKind::NotADirectory, "that name is not a directory"),
        21 => io::const_error!(io::ErrorKind::IsADirectory, "that name is a directory"),
        22 => io::const_error!(io::ErrorKind::InvalidInput, "the FS server refused the request"),
        28 => io::const_error!(io::ErrorKind::StorageFull, "the filesystem is full"),
        // `EROFS` here is a **capability** answer, not a mode bit: the directory capability this
        // process holds does not carry the right this verb needs (§47's ladder). It is the one
        // errno in this map that means "you", rather than "that name".
        30 => io::const_error!(
            io::ErrorKind::ReadOnlyFilesystem,
            "this directory capability does not carry the right that verb needs"
        ),
        39 => io::const_error!(io::ErrorKind::DirectoryNotEmpty, "that directory is not empty"),
        _ => io::const_error!(io::ErrorKind::Other, "the FS server reported a failure"),
    }
}
