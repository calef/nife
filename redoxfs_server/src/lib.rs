//! **The FS-server core: RedoxFS behind a capability-shaped file service** (milestone 32 phase 2).
//!
//! This is the sans-IO heart of the FS server, in the `line_editor` spirit: the filesystem logic
//! with no IPC and no device in it, so it is host-tested in milliseconds against a real RedoxFS
//! image and only the wiring needs QEMU. The EL0 binary (this crate's `el0` build) wraps a
//! [`Server`] in the block-IPC [`redoxfs::Disk`] and the file-service serve loop; everything the
//! filesystem actually *does* lives here.
//!
//! # The contract, capability-shaped from birth
//!
//! A [`Server`] is opened **bound to one directory** ([`Server::open`] binds the RedoxFS root).
//! Every name a client presents is resolved *under a directory handle*: there is no absolute path,
//! no `..` escape, no global namespace. In the running system the client reaches this server only by
//! holding an endpoint the server reads on, and that endpoint IS the directory capability; a client
//! without it can open nothing. A successful open returns a **handle**, a small integer this server
//! minted and will validate against its own table, which is itself a capability: forging one is
//! meaningless because the server honors only the handles it issued. See notes/fs-server.md.
//!
//! # The directory capability (milestone 47)
//!
//! The bound directory is [`filesystem_protocol::fs::ROOT`], handle 0, an ordinary entry in the same table, so
//! every name-taking verb names a directory rather than resolving against a hidden field. A
//! directory handle carries a [`filesystem_protocol::dir::Rights`] set, [`Server::open_dir`] hands back a
//! handle to a child, and a child's rights are the parent's intersected with what was asked for.
//! **That intersection is the only constructor for a non-root rights set**, so no descendant can
//! carry a right its ancestor lacked, at any depth, without a check anyone could forget. A file
//! handle inherits its directory's read and write bits, so what may be done to a file was decided
//! when the directory was granted. See notes/dir-capability.md.
//!
//! **What this table is not.** It is per *server*, not per client, so two clients sharing one
//! endpoint share these handles and a rights-carrying handle attenuates only its holder. The handle
//! is the authority; the **endpoint** is the boundary. Confining a program to a subtree is
//! therefore still a caretaker process holding the wider endpoint
//! (`components/src/fs_subtree_caretaker.rs`), for exactly the reason `fs_file_caretaker` is one.
//!

//! # The error boundary
//!
//! Every method here returns `syscall::error::Result`, RedoxFS's own error type, unmapped. The
//! translation to the wire (a negated errno, `filesystem_protocol::reply_err`) happens once, in the serve loop
//! at the process boundary, and nowhere else. Keeping the core in RedoxFS's error vocabulary is what
//! makes that rule enforceable: there is no ABI type in here to leak.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

// Crash injection (milestone 37). Host-only: it exists to damage an in-memory image the way a dying
// machine damages a platter, and the EL0 binary has no business carrying it. Gated on `hosttest`
// rather than `test` so an integration test in `tests/` can reach it.
#[cfg(feature = "hosttest")]
pub mod crash;

// Milestone 606 (a directory walk costs what it does on Linux)'s instrument: the walk `std` sends,
// replayed against the core with the device reads counted. Host-only for the same reason.
#[cfg(feature = "hosttest")]
pub mod walk_model;

mod block;
mod memo;

// The serve loop's request dispatch, moved out of the EL0 binary so a host fuzz target and host
// tests drive the same code the running server does.
mod dispatch;
use alloc::vec::Vec;

pub use block::{BLOCK, BlockDisk, BlockIo, CACHE_SLOTS, CachedDisk};
pub use dispatch::ServeEdges;
use filesystem_protocol::dir::{self, Rights};
use filesystem_protocol::xattr;
use redoxfs::{Disk, FileSystem, Node, Transaction, TreePtr};
use subtree_scope::Kind;
/// **The engine's error type, for a host caller that implements [`redoxfs::Disk`]** (a fuzz target's
/// in-memory disk). Re-exported so such a caller spells it through this crate rather than naming
/// `redox_syscall` itself; it is the type every [`Server`] method returns. Host-only, because
/// nothing in the EL0 build implements a `Disk` from outside. Name: provisional
/// (`lane/fuzz-service-handlers`, 2026-10-04 UTC).
#[cfg(feature = "hosttest")]
pub use syscall::error as engine_error;
use syscall::error::{
    EBADF, EEXIST, EFBIG, EINVAL, EIO, EISDIR, ENOENT, ENOTDIR, ENOTEMPTY, EPERM, EROFS, Error,
    Result,
};

use crate::memo::Memo;

/// **The largest end a file may have, in bytes**: every record the engine's node tree can address,
/// at the smallest record (one block). A write ending past it, or a truncate growing past it, is
/// refused `EFBIG` before the engine sees it.
///
/// # BUGS
///
/// **This bound was written for an upstream defect.** `redoxfs::NodeLevel::new` admitted
/// `12 * 256^4` level-4 records where `NodeLevelData::level4` holds 8 pointers, so a record offset
/// in that band indexed past the array and the engine panicked, which in this server is a dead
/// server (found by the `redoxfs_server_session` fuzz target, 2026-10-04 UTC). The root fix, `12`
/// to `8` in `vendor/redoxfs/src/node.rs`, is a pin divergence taken by #1606
/// (`lane/redoxfs-level4`); this bound is kept beside it because it answers `EFBIG` at the
/// boundary rather than an engine `ERANGE` deep in a transaction. It is conservative: a file with
/// 8 KiB records (every file this build creates) could address twice this, and no image this
/// system serves comes within a factor of a million of either.
pub const MAX_FILE_END: u64 =
    (128 + 64 * 256 + 32 * 256 * 256 + 16 * 256 * 256 * 256 + 8 * 256 * 256 * 256 * 256)
        * BLOCK as u64;

#[cfg(test)]
mod tests;

mod attribute_store;
mod listing;
mod names;
mod server;

use attribute_store::*;
use listing::*;
pub use names::NameCopy;
use names::*;
pub use server::Server;
