// Seeded from relibc (MIT, vendor/relibc/LICENSE) at 893a3b9133ac, 2026-10-10 (UTC), for milestone 835; nife owns it from here, and its edits say `nife:` where they are (vendor/README.md).
//! `sys/select.h` implementation.
//!
//! See <https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sys_select.h.html>.
//!
//! nife: relibc implements `select` and `pselect` on epoll, which is not seeded. Stage 1 seeded the
//! types alone. `select` and `pselect` were added on 2026-10-12 (UTC) for fio (milestone 833 (the
//! same storage benchmark on nife and Linux)), whose helper thread waits in `select` on a pipe
//! inside its own process. Each turns its three sets into a `pollfd` array and hands it to the
//! platform layer's `poll` (`c_library/src/platform/stream.rs`), which is what musl does on a
//! kernel with no `select` of its own. The readiness each set reports is Linux's mapping from
//! `poll` (`fs/select.c`): readable on `POLLIN` or a hang-up or an error, writable on `POLLOUT` or
//! an error, exceptional on `POLLPRI`.

use alloc::vec::Vec;
use core::time::Duration;

use crate::{
    header::{
        bits_sigset_t::sigset_t,
        errno::{EBADF, EINVAL},
        poll::{POLLERR, POLLHUP, POLLIN, POLLNVAL, POLLOUT, POLLPRI, pollfd},
        time::timespec,
    },
    platform::{
        self,
        types::{c_int, c_short, c_ulong},
    },
};

pub use crate::header::bits_timeval::timeval;

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sys_select.h.html>.
/// cbindgen:ignore
pub const FD_SETSIZE: usize = 1024;

/// Bits in one word of an [`fd_set`].
const WORD_BITS: usize = 8 * core::mem::size_of::<c_ulong>();

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sys_select.h.html>.
/// cbindgen:ignore
#[allow(non_camel_case_types)]
#[repr(C)]
pub struct fd_set {
    pub fds_bits: [c_ulong; FD_SETSIZE / WORD_BITS],
}

impl fd_set {
    fn has(&self, fd: usize) -> bool {
        self.fds_bits[fd / WORD_BITS] & (1 << (fd % WORD_BITS)) != 0
    }

    fn add(&mut self, fd: usize) {
        self.fds_bits[fd / WORD_BITS] |= 1 << (fd % WORD_BITS);
    }

    fn clear(&mut self) {
        self.fds_bits = [0; FD_SETSIZE / WORD_BITS];
    }
}

/// What a descriptor reports in each set, as Linux maps `poll`'s events (`fs/select.c`).
const READ_SET: c_short = POLLIN | POLLHUP | POLLERR;
const WRITE_SET: c_short = POLLOUT | POLLERR;
const EXCEPT_SET: c_short = POLLPRI;

/// nife: `select` and `pselect` both come here, with the wait as a `Duration` (`None` for no
/// limit), so neither rounds a short wait down to none.
///
/// # Safety
///
/// Each set pointer is null or names a writable `fd_set`.
unsafe fn select_on_poll(
    nfds: c_int,
    readfds: *mut fd_set,
    writefds: *mut fd_set,
    exceptfds: *mut fd_set,
    timeout: Option<Duration>,
) -> c_int {
    let Some(nfds) = usize::try_from(nfds).ok().filter(|&n| n <= FD_SETSIZE) else {
        platform::ERRNO.set(EINVAL);
        return -1;
    };
    // SAFETY: the caller passes null or a valid `fd_set` for each, as POSIX requires.
    let (mut r, mut w, mut e) = unsafe { (readfds.as_mut(), writefds.as_mut(), exceptfds.as_mut()) };
    let mut fds = Vec::new();
    for fd in 0..nfds {
        let mut events = 0;
        if r.as_deref().is_some_and(|s| s.has(fd)) {
            events |= POLLIN;
        }
        if w.as_deref().is_some_and(|s| s.has(fd)) {
            events |= POLLOUT;
        }
        if e.as_deref().is_some_and(|s| s.has(fd)) {
            events |= POLLPRI;
        }
        if events != 0 {
            fds.push(pollfd {
                fd: fd as c_int,
                events,
                revents: 0,
            });
        }
    }
    if let Err(err) = platform::sys::poll(&mut fds, timeout) {
        platform::ERRNO.set(err.0);
        return -1;
    }
    // A descriptor that is not open is an error for `select`, where `poll` reports it in place.
    if fds.iter().any(|p| p.revents & POLLNVAL != 0) {
        platform::ERRNO.set(EBADF);
        return -1;
    }
    for set in [&mut r, &mut w, &mut e].into_iter().flatten() {
        set.clear();
    }
    let mut count = 0;
    for p in &fds {
        let fd = p.fd as usize;
        for (set, mask, asked) in [
            (&mut r, READ_SET, POLLIN),
            (&mut w, WRITE_SET, POLLOUT),
            (&mut e, EXCEPT_SET, POLLPRI),
        ] {
            if p.events & asked != 0
                && p.revents & mask != 0
                && let Some(s) = set.as_deref_mut()
            {
                s.add(fd);
                count += 1;
            }
        }
    }
    count
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/select.html>.
///
/// Examines the file descriptor sets whose addresses are passed in the
/// `readfds`, `writefds` and `errorfds` parameters to see whether some of
/// their descriptors are ready for reading, ready for writing, or have an
/// exceptional condition pending, respectively.
///
/// `timeout` is given in seconds and microseconds as represented by `timeval`.
/// Behaves as `pselect()` does when `sigmask` is a null pointer.
///
/// nife: `timeout` is left as it was given. POSIX allows either; Linux writes back the time that
/// was left, so a program that reuses the `timeval` without resetting it waits longer here.
///
/// Upon success, returns the total number of bits set in the bit masks. Upon
/// failure, returns `-1` and sets errno to indicate the error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn select(
    nfds: c_int,
    readfds: *mut fd_set,
    writefds: *mut fd_set,
    exceptfds: *mut fd_set,
    timeout: *mut timeval,
) -> c_int {
    let wait = if timeout.is_null() {
        None
    } else {
        // SAFETY: a non-null `timeout` names a `timeval`, as POSIX requires.
        let t = unsafe { &*timeout };
        match (u64::try_from(t.tv_sec), u64::try_from(t.tv_usec)) {
            (Ok(s), Ok(us)) if us < 1_000_000 => Some(Duration::from_secs(s) + Duration::from_micros(us)),
            _ => {
                platform::ERRNO.set(EINVAL);
                return -1;
            }
        }
    };
    // SAFETY: the sets are passed through under this function's own contract.
    unsafe { select_on_poll(nfds, readfds, writefds, exceptfds, wait) }
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/pselect.html>.
///
/// Examines the file descriptor sets whose addresses are passed in the
/// `readfds`, `writefds` and `errorfds` parameters to see whether some of
/// their descriptors are ready for reading, ready for writing, or have an
/// exceptional condition pending, respectively.
///
/// `timeout` is given in seconds and nanoseconds as represented by `timespec`.
///
/// nife: `sigmask` is not installed for the wait, as `ppoll`'s is not: no signal arrives from
/// outside a process (c_library/README.md), so the mask could not change what wakes it.
///
/// Upon success, returns the total number of bits set in the bit masks. Upon
/// failure, returns `-1` and sets errno to indicate the error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pselect(
    nfds: c_int,
    readfds: *mut fd_set,
    writefds: *mut fd_set,
    exceptfds: *mut fd_set,
    timeout: *const timespec,
    sigmask: *const sigset_t,
) -> c_int {
    let _ = sigmask;
    let wait = if timeout.is_null() {
        None
    } else {
        // SAFETY: a non-null `timeout` names a `timespec`, as POSIX requires.
        let t = unsafe { &*timeout };
        match (u64::try_from(t.tv_sec), u32::try_from(t.tv_nsec)) {
            (Ok(s), Ok(ns)) if ns < 1_000_000_000 => Some(Duration::new(s, ns)),
            _ => {
                platform::ERRNO.set(EINVAL);
                return -1;
            }
        }
    };
    // SAFETY: the sets are passed through under this function's own contract.
    unsafe { select_on_poll(nfds, readfds, writefds, exceptfds, wait) }
}
