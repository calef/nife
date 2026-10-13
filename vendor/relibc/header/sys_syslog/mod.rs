// Seeded from relibc (MIT, vendor/relibc/LICENSE) at 893a3b9133ac, 2026-10-12 (UTC), for milestone 833; nife owns it from here, and its edits say `nife:` where they are (vendor/README.md).
//! `syslog.h` implementation.
//!
//! See <https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/syslog.h.html>.
//!
//! Exported as both syslog.h and sys/syslog.h.
//!
//! nife: the constants and `LOG_MASK`/`LOG_UPTO` are relibc's; the logger is not. relibc's sends to
//! `/dev/log` or Redox's `log:` scheme, and nife has no log service a program could reach. So a
//! message is written to standard error when `openlog` asked for `LOG_PERROR` (copy to standard
//! error) or `LOG_CONS` (the console, when the log cannot be reached, which on nife is always), and
//! is otherwise dropped, as it is on a Linux system with no log daemon running. `%m` is not
//! expanded. Seeded for fio (milestone 833 (the same storage benchmark on nife and Linux)), which
//! logs this way only in server mode.

use core::ffi::VaList;
use std::sync::Mutex;

use crate::{
    header::stdio::{fputc, fputs, stderr, vfprintf},
    platform::types::{c_char, c_int},
};

pub const LOG_PID: c_int = 0x01;
pub const LOG_CONS: c_int = 0x02;
pub const LOG_ODELAY: c_int = 0x04;
pub const LOG_NDELAY: c_int = 0x08;
pub const LOG_NOWAIT: c_int = 0x10;
pub const LOG_PERROR: c_int = 0x20;

pub const LOG_KERN: c_int = 0 << 3;
pub const LOG_USER: c_int = 1 << 3;
pub const LOG_MAIL: c_int = 2 << 3;
pub const LOG_DAEMON: c_int = 3 << 3;
pub const LOG_AUTH: c_int = 4 << 3;
pub const LOG_SYSLOG: c_int = 5 << 3;
pub const LOG_LPR: c_int = 6 << 3;
pub const LOG_NEWS: c_int = 7 << 3;
pub const LOG_UUCP: c_int = 8 << 3;
pub const LOG_CRON: c_int = 9 << 3;
pub const LOG_AUTHPRIV: c_int = 10 << 3;
pub const LOG_FTP: c_int = 11 << 3;

pub const LOG_LOCAL0: c_int = 16 << 3;
pub const LOG_LOCAL1: c_int = 17 << 3;
pub const LOG_LOCAL2: c_int = 18 << 3;
pub const LOG_LOCAL3: c_int = 19 << 3;
pub const LOG_LOCAL4: c_int = 20 << 3;
pub const LOG_LOCAL5: c_int = 21 << 3;
pub const LOG_LOCAL6: c_int = 22 << 3;
pub const LOG_LOCAL7: c_int = 23 << 3;
pub const LOG_NFACILITIES: c_int = 24;

// Priorities
pub const LOG_EMERG: c_int = 0;
pub const LOG_ALERT: c_int = 1;
pub const LOG_CRIT: c_int = 2;
pub const LOG_ERR: c_int = 3;
pub const LOG_WARNING: c_int = 4;
pub const LOG_NOTICE: c_int = 5;
pub const LOG_INFO: c_int = 6;
pub const LOG_DEBUG: c_int = 7;

#[unsafe(no_mangle)]
pub const extern "C" fn LOG_UPTO(p: c_int) -> c_int {
    (1 << (p + 1)) - 1
}

#[unsafe(no_mangle)]
pub const extern "C" fn LOG_MASK(p: c_int) -> c_int {
    1 << p
}

/// What `openlog` and `setlogmask` set. `ident` is the caller's pointer, kept as POSIX allows.
struct Logger {
    ident: usize,
    options: c_int,
    mask: c_int,
}

static LOGGER: Mutex<Logger> = Mutex::new(Logger {
    ident: 0,
    options: 0,
    mask: 0xff,
});

fn logger() -> std::sync::MutexGuard<'static, Logger> {
    // A panic aborts a nife process, so a poisoned lock is never observed.
    LOGGER.lock().unwrap_or_else(|e| e.into_inner())
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/setlogmask.html>.
#[unsafe(no_mangle)]
pub extern "C" fn setlogmask(mask: c_int) -> c_int {
    let mut l = logger();
    let old = l.mask;
    if mask != 0 {
        l.mask = mask;
    }
    old
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/openlog.html>.
#[unsafe(no_mangle)]
pub extern "C" fn openlog(ident: *const c_char, opt: c_int, _facility: c_int) {
    let mut l = logger();
    l.ident = ident as usize;
    l.options = opt;
}

/// See <https://www.man7.org/linux/man-pages/man3/vsyslog.3.html>.
///
/// # Safety
///
/// `message` is null or a C string, and `ap` holds what its conversions name.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vsyslog(priority: c_int, message: *const c_char, ap: VaList) {
    let (ident, options, mask) = {
        let l = logger();
        (l.ident as *const c_char, l.options, l.mask)
    };
    if message.is_null()
        || options & (LOG_PERROR | LOG_CONS) == 0
        || mask & LOG_MASK(priority & 7) == 0
    {
        return;
    }
    // SAFETY: `stderr` is the library's own stream, and `ident` and `message` are C strings by
    // `openlog`'s and this function's contracts.
    unsafe {
        let err = stderr;
        if !ident.is_null() {
            fputs(ident, err);
            fputs(c": ".as_ptr(), err);
        }
        vfprintf(err, message, ap);
        fputc(c_int::from(b'\n'), err);
    }
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/syslog.html>.
///
/// # Safety
///
/// As [`vsyslog`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn syslog(priority: c_int, message: *const c_char, __valist: ...) {
    unsafe { vsyslog(priority, message, __valist) };
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/closelog.html>.
#[unsafe(no_mangle)]
pub extern "C" fn closelog() {
    let mut l = logger();
    l.ident = 0;
    l.options = 0;
}
