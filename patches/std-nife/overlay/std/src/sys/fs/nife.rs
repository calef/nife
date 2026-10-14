//! `std::fs` for nife (milestone 27 phase two): `File` bound to the FS-service contract
//! (DECISIONS §27, notes/fs-server.md, `crates/filesystem_protocol`).
//!
//! # There is no global namespace
//!
//! Per §27, open-by-path exists **only inside the FS server**, resolved relative to the one
//! directory node the client's endpoint is bound to. A std program holds a **directory capability**
//! (the FS-service endpoint, slot 4 of the std slot convention in `pal/nife/rt.rs`), and
//! `File::open("foo")` means *"foo, under the directory I was granted"*. Consequences:
//!
//! 1. **A path that tries to leave the granted directory is refused here, before the wire.** Any
//!    `..` names something this process holds no capability for: [`io::ErrorKind::InvalidFilename`],
//!    not `PermissionDenied`, because nothing checked a permission. A leading `/` is the granted
//!    directory itself, so `/motd` and `motd` name one file (`walk::count_names`).
//! 2. **A program granted no directory capability gets `Unsupported` from everything**, as
//!    `std::net` does when the `Stack` endpoint is absent.
//! 3. **A nested path is a chain of descents, not a lookup** (milestone 122). `File::open("a/b/c")`
//!    is `OPENDIR a`, `OPENDIR b`, then `OPEN c` against `b`'s handle. Every hop resolves under the
//!    capability this process holds, rights only narrow, and there are no symlinks to escape through.
//!

//! # What binds, and what stays Unsupported
//!
//! Bound: [`File::open`] (`OPEN`), `read` (`READ`), `write` (`WRITE`), `seek`/`tell` (an offset
//! this side keeps, since the contract's read and write are both positional), `file_attr`/`size`
//! (`FSTAT`), close on `Drop` (`CLOSE`), [`stat`]/[`exists`] built from open + fstat + close,
//! `create` and `create_new` (`CREATE`), `truncate` and `File::set_len` (`TRUNCATE`), `read_dir`,
//! `create_dir`, `remove_file`, `remove_dir` and `rename` (`OPENDIR`, `READDIR`, `MKDIR`, `UNLINK`,
//! `RMDIR`, `RENAME`), [`copy`] (open, a read/write loop, two closes), [`Dir`] (std's
//! `openat`-shaped API, `#![feature(dirfd)]`), `remove_dir_all` (std's generic implementation over
//! `read_dir` and `remove_file`), `Metadata::modified` (`GETMTIME`) and
//! `std::fs::set_times`/`set_times_nofollow` (`SETMTIME_AT`, DECISIONS §112).
//!
//! The mtime verbs take a **name under a directory handle**, not an open file, so a path's
//! metadata carries an mtime and an open `File`'s does not. See [`Mtime`] for why the PAL refuses
//! rather than re-resolving a name it no longer holds.
//!
//! Still Unsupported, each because no verb in the contract backs it: symlinks and hard links,
//! `canonicalize`, permissions, access and creation times, setting a time through an open `File`,
//! locks, and duplicating a handle (`File::try_clone`, and `Dir::try_clone` for anything but
//! the granted directory: a handle is a token the server minted; there is no dup verb). Before
//! adding a refusal here, check the contract: several earlier ones outlived the verb that ended
//! them. See notes/std.md for the full list with reasons.

use crate::ffi::OsString;
use crate::fmt;
use crate::fs::TryLockError;
use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut, SeekFrom};
use crate::path::{Component, Path, PathBuf};
use crate::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use crate::sys::pal::nife::fsproto::{self, fs as proto};
use crate::sys::pal::nife::rt;
use crate::sys::time::{SystemTime, UNIX_EPOCH};
use crate::sys::{unsupported, unsupported_err};

// The pieces of the phase-one backend that stay exactly as honest as they were: nothing in the
// contract links or canonicalizes, so those keep the `unsupported` implementations rather than
// gaining a nife-shaped copy of the same refusal. `FileTimes` used to come from there too, and its
// setters discarded what they were given; it is this module's own now, because `set_times` has a
// verb to carry a modification time to.
#[expect(dead_code)]
#[path = "unsupported.rs"]
mod unsupported_fs;
pub use unsupported_fs::{canonicalize, link, readlink, symlink};

/// **`std::fs::rename_noreplace` stays Unsupported** (added upstream in the 2026-10-13 nightly).
///
/// `RENAME` replaces an existing destination and has no flag to refuse one. DECISIONS §129 (whether
/// `RENAME` grows a `NOREPLACE` flag) holds that flag until a customer needs it, while recording
/// that `redoxfs_server` could check it atomically for one branch and a spare bit in `rename_dst`'s
/// word. Building it here instead, as a `stat` of the destination followed by `rename`, would be
/// two requests with another client free to create the name between them: the racy emulation §42
/// (a filesystem declares what it offers and must be truthful) refuses. So this is upstream's own
/// `unsupported` version, and wiring it is one line here once the verb grows the flag.
pub use unsupported_fs::rename_noreplace;

/// **`remove_dir_all` needed no code** (milestone 122), and that is the whole finding.
///
/// It used to come from `unsupported.rs` with a note saying the recursion has to descend and a
/// nested path is refused, so the loop belonged in `components/src/rm.rs` where a program can hold a
/// directory capability per level. The second half of that was right and is now this module's
/// business: [`walk`] holds one per level. std's own generic implementation is written entirely in
/// terms of `read_dir`, `remove_file` and `remove_dir` on paths it composes with `DirEntry::path`,
/// which is exactly the shape that started working when descent did.
///
/// It is the same finding milestone 64 recorded twice and this module records a fourth time: **a
/// refusal outlived the reason for it**, and looked correct the whole while. The rights it needs at
/// every level are `dir::REMOVE_TREE`'s (`ENUMERATE` to see, `DESCEND` to walk in, `REMOVE` to take
/// the name out), asked for one verb at a time by the walk rather than all at once.
pub use crate::sys::fs::common::remove_dir_all;

/// No nife-specific fields behind `std::fs::{HomeDirs, MediaDirs}` (rust-lang/rust#158936): the
/// empty structs every non-Unix platform takes. Nothing in this system names a home or a media
/// directory, so there is nothing for an extension trait to carry; `home_dir` is `None` for the
/// same reason (`sys/paths/nife.rs`).
pub use crate::sys::fs::common::{ExtraHomeDirs, ExtraMediaDirs};

/// The FS-service endpoint: this process's entire authority over files. Naming a file over it is a
/// request the server resolves under the one directory the endpoint is bound to.
const FS: u64 = rt::FS_DIR_SLOT;

/// The page shared with the FS server, mapped by the loader alongside the grant.
const PAGE_VA: u64 = rt::FS_PAGE;

/// The contract's transfer unit: one page, so one request never moves more than this.
const PAGE: usize = fsproto::PAGE;

// **The module's seams, one child each** (2026-10-12 UTC, for §266 (a Rust source file stays under
// 2,000 lines) and §282 (when a file is split, the parent becomes an index)). Each child takes
// `use super::*` and is re-exported whole, the shape `kernel/src/sched.rs` uses, because it is part
// of this module split across files rather than a client of it: `sys/fs/mod.rs` names `imp::File`,
// `imp::stat` and the rest exactly as before. The three constants above are the handful every
// child reads.
mod dir;
mod errors;
mod file;
mod metadata;
mod open_options;
mod page;
mod path_operations;
mod path_request;
mod reachable;
mod read_dir;
mod walk;

pub use dir::*;
pub use errors::*;
pub use file::*;
pub use metadata::*;
pub use open_options::*;
pub use page::*;
pub use path_operations::*;
pub use path_request::*;
pub use reachable::*;
pub use read_dir::*;
pub use walk::*;
