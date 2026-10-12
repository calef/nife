//! **The binaries the service runs**, looked up in the boot archive, and the reasons a test
//! gives when one is missing.
//!
//! Cut out of `fs_service.rs` unchanged by lane `maintainer/last-three-over-2000`, which left
//! the parent an index (§282 (when a file is split, the parent becomes an index)) and under §266
//! (a Rust source file stays under 2,000 lines).
//!
//! Name: provisional (lane `maintainer/last-three-over-2000`, 2026-10-12 UTC), for the module and
//! the file.

use super::*;

/// **The FS server's binary, or `None` because nothing packed one into this archive** (milestone
/// 161; the reason changed under milestone 164 and this comment with it).
///
/// It exists as a named helper rather than an `.expect` somebody reads once because for two
/// months this was a **toolchain** fact rather than a machine one, unlike every other missing
/// fixture in this tree (no disk attached, no virtio-rng on the bus, no second core). The engine
/// this server links pulls in `aes` unconditionally, and `aes` would not codegen for
/// `x86_64-unknown-none` at any optimisation level: that target is `-mmx,-sse,+soft-float`, so
/// there was no 128-bit vector register for LLVM to legalise an AES block into.
///
/// **That is fixed** (milestone 164): `--cfg aes_force_soft` in `.cargo/config.toml` selects the
/// portable software backend `aes` already ships, and all three architectures build the server
/// and pack it. So `None` is once again an ordinary build fact: nothing built the binary before
/// the archive was packed. A test that needs a filesystem still skips with [`NO_FS_SERVER`]
/// rather than panicking on a lookup, and on `x86_64` it now more often skips one step later,
/// inside [`start`], for want of a disk this machine's runner does not attach yet
/// (design/roadmap/0164-x86-64-fs-server-aes.md).
///
/// **The archive entry is `redoxfs_server`, not `fs_server`, on every architecture that has one**
/// (milestone 140 increment zero renamed the crate and its packed name together). A lookup for the
/// old name returns `None` everywhere, which reads exactly like "no fixture on this build" and
/// skips 30 tests silently rather than failing loudly -- caught only because one of those thirty is
/// `kernel::user::tests::a_host_process_connects_to_the_guest_and_is_answered`, whose absence took
/// the mDNS responder and the inbound TCP listener down with it, which is what a
/// bisection of the resulting host-side network-check failures actually found.
pub fn fs_server_image() -> Option<&'static [u8]> {
    program("redoxfs_server")
}

/// The reason a test gives when there is no FS service to talk to. One string, because a dozen
/// files share one cause and a reader comparing two runs should not have to decide whether two
/// wordings mean the same thing.
///
/// **It names two causes, and that is deliberate rather than vague** (milestone 164). Most callers
/// reach it from [`fs_server_image`] being `None`, which is only the first; `rmle_tests` reaches
/// it from `rmle_service::start` returning `None`, which on a machine whose archive *does* carry
/// the binary means the second. Until milestone 164 the first cause was the only one that ever
/// fired on `x86_64` and the string named only it, which would now be a false statement in two
/// tests rather than an imprecise one in none.
pub const NO_FS_SERVER: &str = "no RedoxFS service on this boot: either nothing packed a \
                                redoxfs_server into this archive, or this machine has no RedoxFS \
                                disk for one to serve";

/// **`mkfs`'s binary, or `None`** (milestone 161). Same package as [`fs_server_image`], same
/// vendored engine underneath it, so it is absent for exactly the same reason and on exactly the
/// same targets; a separate accessor only so a skipped test names the program it actually wanted.
pub fn mkfs_image() -> Option<&'static [u8]> {
    program("mkfs")
}

/// The reason a test gives when [`mkfs_image`] is `None`. [`NO_FS_SERVER`]'s cause, one binary over.
pub const NO_MKFS: &str = "no mkfs in this archive (nothing built one for this target before \
                           the archive was packed)";

/// **The binary carrying the block server's role**, which is now the same one everywhere.
///
/// **Three `cfg` arms stood here until milestone 291**, because aarch64 reached this role through
/// the `hello` multiplexer while the other two boards had the dedicated `block_driver`. The two
/// shapes always ran the same code (`crates/virtio` is the driver; both binaries were dispatch
/// tables in front of it), so the disagreement bought nothing and cost a fact every caller of this
/// function had to be routed around. 291 packed `block_driver` into the aarch64 archive too and
/// the arms collapsed.
///
/// It panics rather than returning `None` because a boot archive without it is a build that did
/// not finish, not a machine without a disk; the disk's absence is [`root_directory`]'s `None`.
pub fn blk_server_image() -> &'static [u8] {
    program("block_driver").expect("no block_driver program in the initial ramdisk")
}
