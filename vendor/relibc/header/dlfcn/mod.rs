// Seeded from relibc (MIT, vendor/relibc/LICENSE) at 893a3b9133ac, 2026-10-12 (UTC), for milestone 833; nife owns it from here, and its edits say `nife:` where they are (vendor/README.md).
//! `dlfcn.h` implementation.
//!
//! See <https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/dlfcn.h.html>.
//!
//! nife: the constants and `Dl_info` are relibc's; the functions are not. relibc's are its dynamic
//! linker's (`ld_so`), which was not seeded: a nife program is one static image, and the system has
//! no shared objects to load. So `dlopen` and `dlsym` find nothing and say so through `dlerror`, as
//! relibc's own do when they run without its linker. Seeded for fio (milestone 833 (the same
//! storage benchmark on nife and Linux)), which reaches `dlopen` only for an external I/O engine.

use core::{cell::Cell, ptr};

use crate::platform::types::{c_char, c_int, c_void};

pub const RTLD_LAZY: c_int = 1 << 0;
pub const RTLD_NOW: c_int = 1 << 1;
pub const RTLD_NOLOAD: c_int = 1 << 2;
pub const RTLD_GLOBAL: c_int = 1 << 8;
pub const RTLD_LOCAL: c_int = 0x0000;

#[expect(
    clippy::zero_ptr,
    reason = "cbindgen issue: https://github.com/mozilla/cbindgen/issues/948"
)]
pub const RTLD_DEFAULT: *mut c_void = 0 as *mut c_void; // XXX: cbindgen doesn't like ptr::null_mut() for publically exported constants

/// What `dlerror` reports after any of these functions fails.
static NOT_SUPPORTED: &core::ffi::CStr =
    c"dynamic loading is not supported: a nife program is one static image";

std::thread_local! {
    /// Whether this thread has a failure `dlerror` has not yet reported. POSIX keeps it per
    /// thread. nife: `std`'s thread-local in place of relibc's `#[thread_local]`, as for `errno`.
    static PENDING: Cell<bool> = const { Cell::new(false) };
}

fn fail() {
    PENDING.with(|p| p.set(true));
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct Dl_info_t {
    dli_fname: *const c_char,
    dli_fbase: *mut c_void,
    dli_sname: *const c_char,
    dli_saddr: *mut c_void,
}

pub type Dl_info = Dl_info_t;

/// See <https://www.man7.org/linux/man-pages/man3/dladdr.3.html>.
///
/// nife: answers 0, "no shared object holds this address", and leaves `info` alone.
#[unsafe(no_mangle)]
pub extern "C" fn dladdr(addr: *const c_void, info: *mut Dl_info_t) -> c_int {
    let _ = (addr, info);
    0
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/dlopen.html>.
///
/// nife: answers a null pointer, and `dlerror` says why.
#[unsafe(no_mangle)]
pub extern "C" fn dlopen(cfilename: *const c_char, flags: c_int) -> *mut c_void {
    let _ = (cfilename, flags);
    fail();
    ptr::null_mut()
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/dlsym.html>.
///
/// nife: answers a null pointer, and `dlerror` says why.
#[unsafe(no_mangle)]
pub extern "C" fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void {
    let _ = (handle, symbol);
    fail();
    ptr::null_mut()
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/dlclose.html>.
///
/// nife: no `dlopen` ever succeeded, so no handle is valid; answers `-1`.
#[unsafe(no_mangle)]
pub extern "C" fn dlclose(handle: *mut c_void) -> c_int {
    let _ = handle;
    fail();
    -1
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/dlerror.html>.
///
/// The last failure on this thread, once; null when there is none.
#[unsafe(no_mangle)]
pub extern "C" fn dlerror() -> *mut c_char {
    if PENDING.with(|p| p.replace(false)) {
        NOT_SUPPORTED.as_ptr().cast_mut()
    } else {
        ptr::null_mut()
    }
}
