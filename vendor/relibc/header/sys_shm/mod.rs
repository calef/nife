// Seeded from relibc (MIT, vendor/relibc/LICENSE) at 893a3b9133ac, 2026-10-12 (UTC), for milestone 833; nife owns it from here, and its edits say `nife:` where they are (vendor/README.md).
//! `sys/shm.h` implementation.
//!
//! See <https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sys_shm.h.html>.
//!
//! nife: the types and constants are relibc's; the four functions are not. relibc builds a System V
//! segment on `shm_open` and a shared `mmap` of it, and nife has neither: a C program's `mmap` of a
//! file is refused (c_library/README.md), and memory shared between processes is a grant, not a
//! name another process can look up. So each function answers `ENOSYS`, as every call the library
//! does not build does. Seeded for fio (milestone 833 (the same storage benchmark on nife and
//! Linux)), which includes this header unconditionally and, built with `--disable-shm`, never
//! calls it.

use crate::{
    header::{errno::ENOSYS, sys_ipc::ipc_perm},
    platform::{
        ERRNO,
        types::{c_int, c_void, key_t, pid_t, size_t, time_t},
    },
};

#[allow(non_camel_case_types)]
pub type shmatt_t = core::ffi::c_ushort;

pub const SHM_RDONLY: c_int = 0o10000;
pub const SHM_RND: c_int = 0o20000;
pub const SHMLBA: size_t = 4096;

/// What `shmat` returns on failure, `(void *)-1`.
const SHM_FAILED: *mut c_void = -1isize as *mut c_void;

#[repr(C)]
pub struct shmid_ds {
    pub shm_perm: ipc_perm,
    pub shm_segsz: size_t,
    pub shm_lpid: pid_t,
    pub shm_cpid: pid_t,
    pub shm_nattch: shmatt_t,
    pub shm_atime: time_t,
    pub shm_dtime: time_t,
    pub shm_ctime: time_t,
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/shmget.html>.
///
/// nife: answers `-1` with `ENOSYS` (the module comment says why).
#[unsafe(no_mangle)]
pub extern "C" fn shmget(key: key_t, size: size_t, shmflg: c_int) -> c_int {
    let _ = (key, size, shmflg);
    ERRNO.set(ENOSYS);
    -1
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/shmat.html>.
///
/// nife: answers `(void *)-1` with `ENOSYS`.
#[unsafe(no_mangle)]
pub extern "C" fn shmat(shmid: c_int, shmaddr: *const c_void, shmflg: c_int) -> *mut c_void {
    let _ = (shmid, shmaddr, shmflg);
    ERRNO.set(ENOSYS);
    SHM_FAILED
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/shmdt.html>.
///
/// nife: answers `-1` with `ENOSYS`.
#[unsafe(no_mangle)]
pub extern "C" fn shmdt(shmaddr: *const c_void) -> c_int {
    let _ = shmaddr;
    ERRNO.set(ENOSYS);
    -1
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/shmctl.html>.
///
/// nife: answers `-1` with `ENOSYS`.
#[unsafe(no_mangle)]
pub extern "C" fn shmctl(shmid: c_int, cmd: c_int, buf: *mut shmid_ds) -> c_int {
    let _ = (shmid, cmd, buf);
    ERRNO.set(ENOSYS);
    -1
}
