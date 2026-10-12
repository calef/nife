// Seeded from relibc (MIT, vendor/relibc/LICENSE) at 893a3b9133ac, 2026-10-12 (UTC), for milestone 836; nife owns it from here, and its edits say `nife:` where they are (vendor/README.md).
//! `sched.h` implementation.
//!
//! See <https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sched.h.html>.

use crate::{
    error::ResultExt,
    header::time::timespec,
    platform::{
        Pal, Sys,
        types::{c_int, pid_t},
    },
};

// TODO: There are extensions, but adding more member is breaking ABI for pthread_attr_t
/// See <https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sched.h.html>.
///
/// Scheduling parameters required for each supported scheduling policy.
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct sched_param {
    /// Process or thread execution scheduling priority.
    pub sched_priority: c_int,
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sched.h.html>.
///
/// First in first out (FIFO) scheduling policy.
pub const SCHED_FIFO: c_int = 0;
/// See <https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sched.h.html>.
///
/// Round robin scheduling policy.
pub const SCHED_RR: c_int = 1;
/// See <https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sched.h.html>.
///
/// Another scheduling policy.
pub const SCHED_OTHER: c_int = 2;

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/sched_get_priority_max.html>.
// nife: relibc leaves the six functions below as `todo!()` and unexported. nife's scheduler has one
// class and no priorities a process can ask for (`kernel/src/sched.rs`), so every policy's range
// is the single priority 0, every process reports `SCHED_OTHER` at priority 0, and a request to
// change either is refused with `EPERM`, which is what Linux answers an unprivileged caller.

fn valid_policy(policy: c_int) -> bool {
    matches!(policy, SCHED_FIFO | SCHED_RR | SCHED_OTHER)
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/sched_get_priority_max.html>.
#[unsafe(no_mangle)]
pub extern "C" fn sched_get_priority_max(policy: c_int) -> c_int {
    if valid_policy(policy) {
        0
    } else {
        crate::platform::ERRNO.set(crate::header::errno::EINVAL);
        -1
    }
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/sched_get_priority_min.html>.
#[unsafe(no_mangle)]
pub extern "C" fn sched_get_priority_min(policy: c_int) -> c_int {
    sched_get_priority_max(policy)
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/sched_getparam.html>.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sched_getparam(_pid: pid_t, param: *mut sched_param) -> c_int {
    if let Some(param) = unsafe { param.as_mut() } {
        param.sched_priority = 0;
    }
    0
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/sched_getscheduler.html>.
#[unsafe(no_mangle)]
pub extern "C" fn sched_getscheduler(_pid: pid_t) -> c_int {
    SCHED_OTHER
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/sched_rr_get_interval.html>.
#[unsafe(no_mangle)]
pub extern "C" fn sched_rr_get_interval(_pid: pid_t, _time: *mut timespec) -> c_int {
    crate::platform::ERRNO.set(crate::header::errno::ENOSYS);
    -1
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/sched_setparam.html>.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sched_setparam(_pid: pid_t, _param: *const sched_param) -> c_int {
    crate::platform::ERRNO.set(crate::header::errno::EPERM);
    -1
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/sched_setscheduler.html>.
#[unsafe(no_mangle)]
pub extern "C" fn sched_setscheduler(
    _pid: pid_t,
    _policy: c_int,
    _param: *const sched_param,
) -> c_int {
    crate::platform::ERRNO.set(crate::header::errno::EPERM);
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn sched_yield() -> c_int {
    Sys::sched_yield().map(|()| 0).or_minus_one_errno()
}
