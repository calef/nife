// Seeded from relibc (MIT, vendor/relibc/LICENSE) at 893a3b9133ac, 2026-10-12 (UTC), for milestone 833; nife owns it from here, and its edits say `nife:` where they are (vendor/README.md).
//! `sys/resource.h` implementation.
//!
//! See <https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sys_resource.h.html>.
//!
//! nife: seeded for fio (milestone 833 (the same storage benchmark on nife and Linux)), which keeps
//! a `struct rusage` per job and calls `getrusage`. The resource limits (`getrlimit`, `setrlimit`,
//! `rlimit`, `rlim_t` and the `RLIMIT_*` and `RLIM_*` constants) were left behind: nife keeps no
//! per-process limit a program could read or lower (the bounds a process meets are the memory it
//! was granted and `std`'s sixteen threads), so a program that uses them fails to compile and says
//! why, rather than being told a limit that is not enforced.

use crate::{
    error::ResultExt,
    header::sys_select::timeval,
    out::Out,
    platform::{
        Pal, Sys,
        types::{c_int, c_long, id_t},
    },
};

/// Returns information about the current process.
pub const RUSAGE_SELF: c_int = 0;
/// Returns information about children of the current process.
pub const RUSAGE_CHILDREN: c_int = -1;
/// Non-POSIX.
///
/// Return resource consumption statistics for both the current process and
/// all of its terminated child processes that have been waited for.
pub const RUSAGE_BOTH: c_int = -2;
// TODO should be guarded by `_GNU_SOURCE`
/// Non-POSIX, see <https://www.man7.org/linux/man-pages/man2/getrusage.2.html>.
///
/// Return resource usage statistics for the calling thread.
pub const RUSAGE_THREAD: c_int = 1;

#[repr(C)]
pub struct rusage {
    /// User time used.
    pub ru_utime: timeval,
    /// System time used.
    pub ru_stime: timeval,
    /// Non-POSIX, see <https://www.man7.org/linux/man-pages/man2/getrusage.2.html>.
    ///
    /// Maximum resident set size.
    pub ru_maxrss: c_long,
    /// Non-POSIX, see <https://www.man7.org/linux/man-pages/man2/getrusage.2.html>.
    ///
    /// Integral shared memory size.
    pub ru_ixrss: c_long,
    /// Non-POSIX, see <https://www.man7.org/linux/man-pages/man2/getrusage.2.html>.
    ///
    /// Integral unshared data size.
    pub ru_idrss: c_long,
    /// Non-POSIX, see <https://www.man7.org/linux/man-pages/man2/getrusage.2.html>.
    ///
    /// Integral unshared stack size.
    pub ru_isrss: c_long,
    /// Non-POSIX, see <https://www.man7.org/linux/man-pages/man2/getrusage.2.html>.
    ///
    /// Page reclaims (soft page faults).
    pub ru_minflt: c_long,
    /// Non-POSIX, see <https://www.man7.org/linux/man-pages/man2/getrusage.2.html>.
    ///
    /// Page faults (hard page faults).
    pub ru_majflt: c_long,
    /// Non-POSIX, see <https://www.man7.org/linux/man-pages/man2/getrusage.2.html>.
    ///
    /// Swaps.
    pub ru_nswap: c_long,
    /// Non-POSIX, see <https://www.man7.org/linux/man-pages/man2/getrusage.2.html>.
    ///
    /// Block input operations.
    pub ru_inblock: c_long,
    /// Non-POSIX, see <https://www.man7.org/linux/man-pages/man2/getrusage.2.html>.
    ///
    /// Block output operations.
    pub ru_oublock: c_long,
    /// Non-POSIX, see <https://www.man7.org/linux/man-pages/man2/getrusage.2.html>.
    ///
    /// IPC messages sent.
    pub ru_msgsnd: c_long,
    /// Non-POSIX, see <https://www.man7.org/linux/man-pages/man2/getrusage.2.html>.
    ///
    /// IPC messages received.
    pub ru_msgrcv: c_long,
    /// Non-POSIX, see <https://www.man7.org/linux/man-pages/man2/getrusage.2.html>.
    ///
    /// Signals received.
    pub ru_nsignals: c_long,
    /// Non-POSIX, see <https://www.man7.org/linux/man-pages/man2/getrusage.2.html>.
    ///
    /// Voluntary context switches.
    pub ru_nvcsw: c_long,
    /// Non-POSIX, see <https://www.man7.org/linux/man-pages/man2/getrusage.2.html>.
    ///
    /// Involuntary context switches.
    pub ru_nivcsw: c_long,
}

/// Identifies the `who` argument as a process ID.
pub const PRIO_PROCESS: c_int = 0;
/// Identifies the `who` argument as a process group ID.
pub const PRIO_PGRP: c_int = 1;
/// Identifies the `who` argument as a user ID.
pub const PRIO_USER: c_int = 2;

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/getpriority.html>.
///
/// Obtains the nice value of a process, process group or user.
///
/// A value of `0` for `who` indicates the current process, process group or
/// user.
///
/// A return value of `-1` can either be a valid nice value or indicate that
/// an error occurred. To distinguish between them, first set `errno` to `0`
/// before calling `getpriority()`, then check the value of `errno` after the
/// call to `getpriority()`.
#[unsafe(no_mangle)]
pub extern "C" fn getpriority(which: c_int, who: id_t) -> c_int {
    let r = Sys::getpriority(which, who).or_minus_one_errno();
    if r < 0 {
        return r;
    }
    20 - r
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/setpriority.html>.
///
/// Sets the nice value of a process, process group or user.
///
/// A value of `0` for `who` indicates the current process, process group or
/// user.
///
/// A return value of `0` indicates success, `-1` indicates error.
#[unsafe(no_mangle)]
pub extern "C" fn setpriority(which: c_int, who: id_t, nice: c_int) -> c_int {
    Sys::setpriority(which, who, nice)
        .map(|()| 0)
        .or_minus_one_errno()
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/getrusage.html>.
///
/// Stores the measures of the resources used by the current process or its
/// terminated and waited-for child processes in the `rusage` struct pointed
/// to by `r_usage`.
///
/// A return value of `0` indicates success, `-1` indicates error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getrusage(who: c_int, r_usage: *mut rusage) -> c_int {
    Sys::getrusage(who, unsafe { Out::nonnull(r_usage) })
        .map(|()| 0)
        .or_minus_one_errno()
}
