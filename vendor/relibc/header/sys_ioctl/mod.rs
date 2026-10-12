// Seeded from relibc (MIT, vendor/relibc/LICENSE) at 893a3b9133ac, 2026-10-10 (UTC), for milestone 835; nife owns it from here, and its edits say `nife:` where they are (vendor/README.md).
//! `sys/ioctl.h`: `ioctl`, which on nife controls nothing.
//!
//! nife: relibc's module is Redox's and Linux's terminal and device controls, none of which nife
//! has. A device on nife is a capability whose protocol says what it does, so there is no
//! untyped side channel to it. SQLite's unix VFS includes this header unconditionally and calls
//! `ioctl` only on Linux (`F2FS_IOC_*`), so the function exists and answers `ENOTTY`, POSIX's
//! "this descriptor does not accept that request".
//!
//! nife: one request is kept, `FIONBIO` (stage 2, milestone 836), which is `fcntl`'s
//! `O_NONBLOCK` under an older name and the one hackbench sets on its sockets. relibc's value.

use crate::{
    error::ResultExt,
    header::{
        errno::ENOTTY,
        fcntl::{F_GETFL, F_SETFL, O_NONBLOCK},
    },
    platform::{
        self, Pal, Sys,
        types::{c_int, c_ulong, c_ulonglong},
    },
};

/// Not recommended, should use POSIX `O_NONBLOCK` instead.
pub const FIONBIO: c_ulong = 0x5421;

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/ioctl.html>.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ioctl(fd: c_int, request: c_ulong, mut args: ...) -> c_int {
    if request == FIONBIO {
        // SAFETY: `FIONBIO`'s argument is a pointer to an `int`, as on Linux.
        let on = unsafe { args.next_arg::<*const c_int>().as_ref() }.is_some_and(|v| *v != 0);
        return Sys::fcntl(fd, F_GETFL, 0)
            .and_then(|flags| {
                let flags = if on { flags | O_NONBLOCK } else { flags & !O_NONBLOCK };
                Sys::fcntl(fd, F_SETFL, flags as c_ulonglong)
            })
            .map(|_| 0)
            .or_minus_one_errno();
    }
    platform::ERRNO.set(ENOTTY);
    -1
}
