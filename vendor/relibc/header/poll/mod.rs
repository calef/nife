// Seeded from relibc (MIT, vendor/relibc/LICENSE) at 893a3b9133ac, 2026-10-12 (UTC), for milestone 836; nife owns it from here, and its edits say `nife:` where they are (vendor/README.md).
//! `poll.h` implementation.
//!
//! See <https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/poll.h.html>.
//!
//! nife: relibc implements `poll` on epoll, which is not seeded. Here `poll` and `ppoll` hand the
//! array to the platform layer's own (`c_library/src/platform/stream.rs`), which knows what each
//! descriptor in its table is. The constants and `pollfd` are relibc's.

use core::slice;

use crate::{
    error::ResultExt,
    header::{bits_sigset_t::sigset_t, time::timespec},
    platform::types::{c_int, c_short, c_ulong},
};

/// Data other than high-priority data may be read without blocking.
pub const POLLIN: c_short = 0x001;
/// High-priority data may be read without blocking.
pub const POLLPRI: c_short = 0x002;
/// Normal data may be written without blocking.
pub const POLLOUT: c_short = 0x004;
/// An error has occurred (revents only).
pub const POLLERR: c_short = 0x008;
/// Device has been disconnected (revents only).
pub const POLLHUP: c_short = 0x010;
/// Invalid fd member (revents only).
pub const POLLNVAL: c_short = 0x020;
/// Normal data may be read without blocking.
pub const POLLRDNORM: c_short = 0x040;
/// Priority data may be read without blocking.
pub const POLLRDBAND: c_short = 0x080;
/// Equivalent to POLLOUT.
pub const POLLWRNORM: c_short = 0x100;
/// Priority data may be written.
pub const POLLWRBAND: c_short = 0x200;

/// An unsigned integer type used for the number of file descriptors.
#[allow(non_camel_case_types)]
pub type nfds_t = c_ulong;

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/poll.h.html>.
///
/// A structure storing a file descriptor along with input and output flags for poll operations.
#[allow(non_camel_case_types)]
#[repr(C)]
pub struct pollfd {
    /// The following descriptor being polled.
    pub fd: c_int,
    /// The input event flags.
    pub events: c_short,
    /// The output event flags.
    pub revents: c_short,
}

impl core::fmt::Debug for pollfd {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.fd)
    }
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/poll.html>.
///
/// Provides applications with a mechanism for multiplexing input/output
/// over a set of file descriptors.
///
/// - The `timeout` parameter represents milliseconds.
/// - A `timeout` of `-1` is equivalent to passing a null pointer for `tmo_p` to `ppoll`.
/// - `poll` should behave equivalent to `ppoll` with a null pointer for `sigmask`.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn poll(fds: *mut pollfd, nfds: nfds_t, timeout: c_int) -> c_int {
    let fd_buf = unsafe { slice::from_raw_parts_mut(fds, nfds as usize) };
    crate::platform::sys::poll(fd_buf, timeout).or_minus_one_errno()
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/ppoll.html>.
///
/// Provides applications with a mechanism for multiplexing input/output
/// over a set of file descriptors.
///
/// - The `tmo_p` parameter is the timeout as represented by a `timespec` struct.
/// - Passing a null pointer for timeout is equivalent to `-1` for `timeout` to `poll`.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ppoll(
    fds: *mut pollfd,
    nfds: nfds_t,
    tmo_p: *const timespec,
    sigmask: *const sigset_t,
) -> c_int {
    let timeout = if tmo_p.is_null() {
        -1
    } else {
        let tmo = unsafe { &*tmo_p };
        if tmo.tv_sec > (c_int::MAX / 1000).into() {
            c_int::MAX
        } else {
            ((tmo.tv_sec as c_int) * 1000) + ((tmo.tv_nsec as c_int) / 1000000)
        }
    };
    let fd_buf = unsafe { slice::from_raw_parts_mut(fds, nfds as usize) };
    // nife: no signal arrives from outside a process (c_library/README.md), so the mask a
    // `ppoll` installs for its wait could not change what wakes it, and is not installed.
    let _ = sigmask;
    crate::platform::sys::poll(fd_buf, timeout).or_minus_one_errno()
}
