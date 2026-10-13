// Seeded from relibc (MIT, vendor/relibc/LICENSE) at 893a3b9133ac, 2026-10-10 (UTC), for milestone 835; nife owns it from here, and its edits say `nife:` where they are (vendor/README.md).
use crate::platform::types::c_int;

/// Open for reading only.
pub const O_RDONLY: c_int = 0x0000;
/// Open for writing only.
pub const O_WRONLY: c_int = 0x0001;
/// Open for reading and writing.
pub const O_RDWR: c_int = 0x0002;
/// Mask for file access modes.
pub const O_ACCMODE: c_int = 0x0003;
/// Create file if it does not exist.
pub const O_CREAT: c_int = 0x0040;
/// Exclusive use flag.
pub const O_EXCL: c_int = 0x0080;
/// Do not assign controlling terminal.
pub const O_NOCTTY: c_int = 0x0100;
/// Truncate flag.
pub const O_TRUNC: c_int = 0x0200;
/// Set append mode.
pub const O_APPEND: c_int = 0x0400;
/// Non-blocking mode.
pub const O_NONBLOCK: c_int = 0x0800;
/// Fail if file is a non-directory file.
pub const O_DIRECTORY: c_int = 0x1_0000;
/// Do not follow symbolic links.
pub const O_NOFOLLOW: c_int = 0x2_0000;
/// Write according to synchronized I/O data integrity completion.
///
/// nife: added 2026-10-12 (UTC) for fio (milestone 833), with Linux's generic values, which relibc's
/// Linux arm lacked. A write on a descriptor opened with this or [`O_SYNC`] returns after the file
/// server has the data on its disk (`c_library/src/platform/nife.rs`).
pub const O_DSYNC: c_int = 0x1000;
/// Write according to synchronized I/O file integrity completion.
pub const O_SYNC: c_int = 0x10_1000;
/// Synchronized read I/O operations. Linux makes it [`O_SYNC`]; a read on nife never waits on a
/// write, so it changes nothing.
pub const O_RSYNC: c_int = O_SYNC;
/// Atomically set the `FD_CLOEXEC` flag on the new file desciptor.
pub const O_CLOEXEC: c_int = 0x8_0000;
/// Non-POSIX, see <https://www.man7.org/linux/man-pages/man2/open.2.html>.
///
/// Get a file descriptor to indicate a location in the filesystem tree and
/// to perform operations that act purely at the file descriptor level.
pub const O_PATH: c_int = 0x20_0000;

/// Non-POSIX, see <https://www.man7.org/linux/man-pages/man2/open.2.html>.
///
/// Alternative name for `O_NONBLOCK`.
pub const O_NDELAY: c_int = O_NONBLOCK;
