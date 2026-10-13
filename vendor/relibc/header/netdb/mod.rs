// Seeded from relibc (MIT, vendor/relibc/LICENSE) at 893a3b9133ac, 2026-10-12 (UTC), for milestone 833; nife owns it from here, and its edits say `nife:` where they are (vendor/README.md).
//! `netdb.h` implementation, address lookup only.
//!
//! See <https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/netdb.h.html>.
//!
//! nife: a placeholder for stage 3. Milestone 837 (a C library, stage 3: sockets) seeds relibc's
//! `netdb` whole, `getaddrinfo` for literal addresses first, over nife's socket protocol. Until it
//! lands, this holds `addrinfo` and the `AI_*`, `EAI_*` and `NI_*` constants as relibc has them,
//! and `getaddrinfo` and `getnameinfo` answer `EAI_SYSTEM` with `errno` `ENOSYS`, as every socket
//! call but `socketpair` does today. The host, network, protocol and service databases
//! (`gethostbyname` and the rest) are not here. Seeded for fio (milestone 833 (the same storage
//! benchmark on nife and Linux)), which reaches `getaddrinfo` only in client and server mode.

use crate::{
    header::{
        errno::ENOSYS,
        sys_socket::{sockaddr, socklen_t},
    },
    platform::{
        ERRNO,
        types::{c_char, c_int},
    },
};

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/netdb.h.html>.
#[repr(C)]
#[derive(Debug)]
pub struct addrinfo {
    ai_flags: c_int,           /* AI_PASSIVE, AI_CANONNAME, AI_NUMERICHOST */
    ai_family: c_int,          /* PF_xxx */
    ai_socktype: c_int,        /* SOCK_xxx */
    ai_protocol: c_int,        /* 0 or IPPROTO_xxx for IPv4 and IPv6 */
    ai_addrlen: socklen_t,     /* length of ai_addr */
    ai_canonname: *mut c_char, /* canonical name for hostname */
    ai_addr: *mut sockaddr,    /* binary address */
    ai_next: *mut addrinfo,    /* next structure in linked list */
}

pub const AI_PASSIVE: c_int = 0x0001;
pub const AI_CANONNAME: c_int = 0x0002;
pub const AI_NUMERICHOST: c_int = 0x0004;
pub const AI_V4MAPPED: c_int = 0x0008;
pub const AI_ALL: c_int = 0x0010;
pub const AI_ADDRCONFIG: c_int = 0x0020;
pub const AI_NUMERICSERV: c_int = 0x0400;

pub const EAI_BADFLAGS: c_int = -1;
pub const EAI_NONAME: c_int = -2;
pub const EAI_AGAIN: c_int = -3;
pub const EAI_FAIL: c_int = -4;
pub const EAI_NODATA: c_int = -5;
pub const EAI_FAMILY: c_int = -6;
pub const EAI_SOCKTYPE: c_int = -7;
pub const EAI_SERVICE: c_int = -8;
pub const EAI_ADDRFAMILY: c_int = -9;
pub const EAI_MEMORY: c_int = -10;
pub const EAI_SYSTEM: c_int = -11;
pub const EAI_OVERFLOW: c_int = -12;

pub const NI_MAXHOST: c_int = 1025;
pub const NI_MAXSERV: c_int = 32;

pub const NI_NUMERICHOST: c_int = 0x0001;
pub const NI_NUMERICSERV: c_int = 0x0002;
pub const NI_NOFQDN: c_int = 0x0004;
pub const NI_NAMEREQD: c_int = 0x0008;
pub const NI_DGRAM: c_int = 0x0010;

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/getaddrinfo.html>.
///
/// nife: answers `EAI_SYSTEM` with `errno` `ENOSYS` and a null list (the module comment says why).
///
/// # Safety
///
/// `res_out` names a writable pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getaddrinfo(
    node: *const c_char,
    service: *const c_char,
    hints: *const addrinfo,
    res_out: *mut *mut addrinfo,
) -> c_int {
    let _ = (node, service, hints);
    if !res_out.is_null() {
        // SAFETY: a non-null `res_out` is writable, by this function's contract.
        unsafe { res_out.write(core::ptr::null_mut()) };
    }
    ERRNO.set(ENOSYS);
    EAI_SYSTEM
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/getnameinfo.html>.
///
/// nife: answers `EAI_SYSTEM` with `errno` `ENOSYS`.
#[unsafe(no_mangle)]
pub extern "C" fn getnameinfo(
    addr: *const sockaddr,
    addrlen: socklen_t,
    host: *mut c_char,
    hostlen: socklen_t,
    serv: *mut c_char,
    servlen: socklen_t,
    flags: c_int,
) -> c_int {
    let _ = (addr, addrlen, host, hostlen, serv, servlen, flags);
    ERRNO.set(ENOSYS);
    EAI_SYSTEM
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/freeaddrinfo.html>.
///
/// nife: `getaddrinfo` never hands out a list, so there is none to free.
#[unsafe(no_mangle)]
pub extern "C" fn freeaddrinfo(res: *mut addrinfo) {
    let _ = res;
}

/// See <https://pubs.opengroup.org/onlinepubs/9799919799/functions/gai_strerror.html>.
#[unsafe(no_mangle)]
pub const extern "C" fn gai_strerror(errcode: c_int) -> *const c_char {
    match errcode {
        EAI_BADFLAGS => c"Invalid flags",
        EAI_NONAME => c"Name does not resolve",
        EAI_AGAIN => c"Try again",
        EAI_FAIL => c"Non-recoverable error",
        EAI_NODATA => c"Unknown error",
        EAI_FAMILY => c"Unrecognized address family or invalid length",
        EAI_SOCKTYPE => c"Unrecognized socket type",
        EAI_SERVICE => c"Unrecognized service",
        EAI_ADDRFAMILY => c"Address family for name not supported",
        EAI_MEMORY => c"Out of memory",
        EAI_SYSTEM => c"System error",
        EAI_OVERFLOW => c"Overflow",
        _ => c"Unknown error",
    }
    .as_ptr()
}
