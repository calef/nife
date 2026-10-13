//! The `getrandom` backend for nife. `getrandom` picks its source by `target_os` and ends in
//! `compile_error!` with no `nife` arm, so `rand`, `uuid`, `ring` and most of crates.io failed to
//! build here although `std::random::SystemRng` already works (notes/crates-io-on-nife.md). This
//! crate is the `custom` backend upstream documents for that case: one `extern "Rust"` function per
//! `getrandom` shape, each a `SystemRng` draw.
//!
//! ```text
//! uuid -> getrandom -> __getrandom_v03_custom (here) -> std::random::SystemRng
//!      -> the entropy service (one endpoint, naming no device) -> virtio-rng
//! ```
//!
//! It is its own workspace at the tree root, like `std_exerciser`: it needs `std`, so it builds
//! only for a `*-unknown-nife` target with build-std, and its `--cfg` must never reach the
//! workspace crates. It is not a patch, since it changes nothing of `getrandom`'s. A vendored fork
//! would avoid the `rustflags` but track a fast-moving crate; the lasting fix is a `nife` arm
//! upstream, shaped like the existing `hermit.rs`.
//!
//! # How to use it
//!
//! Depend on `entropy_backend`, set `rustflags = ["--cfg", "getrandom_backend=\"custom\""]` in the
//! workspace's `.cargo/config.toml` (which source exists is a property of the target, so once per
//! workspace is right), and name the crate in the binary for its side effect:
//!
//! ```text
//! use entropy_backend as _;
//! ```
//!
//! An rlib nothing references is not linked, so without that line the build stops at
//! `undefined symbol: __getrandom_v03_custom`, as with a panic handler in a `no_std` binary.
//!
//! # BUGS
//!
//! - `getrandom` 0.2 needs its own hook: it returns a bare `u32` and needs 0.2's `custom` feature,
//!   which only a manifest can enable, so this crate depends on 0.2 too.
//! - 0.3 and 0.4 both declare `__getrandom_v03_custom`, so one definition serves both; their
//!   `Error` types are layout-identical, which neither crate promises.
//! - A draw panics when the process holds no entropy capability, as `SystemRng` does; there is no
//!   degraded mode, so a program using `rand` must be granted entropy.
//! - Nothing in the tree's gate builds this crate; the probe harness in notes/crates-io-on-nife.md
//!   and its consumers exercise it.

#![feature(random)]

use core::slice;
use std::random::{Rng, SystemRng};

/// `getrandom`'s custom-backend hook, filling `len` bytes at `dest` from [`SystemRng`].
///
/// # Safety
///
/// `getrandom`'s contract: `dest` is valid for writes of `len` bytes, and on `Ok(())` every one of
/// them must be initialized. `fill_bytes` writes the whole slice or panics, so there is no partial
/// path that could return `Ok` over uninitialized memory.
///
/// [`SystemRng`]: std::random::SystemRng
#[unsafe(no_mangle)]
unsafe extern "Rust" fn __getrandom_v03_custom(
    dest: *mut u8,
    len: usize,
) -> Result<(), getrandom::Error> {
    // SAFETY: the caller's contract, restated above. `getrandom` never passes a null pointer with a
    // non-zero length, and a zero length yields an empty slice, which `from_raw_parts_mut` allows
    // for any aligned non-null pointer; `getrandom` passes the address of a real buffer either way.
    let buf = unsafe { slice::from_raw_parts_mut(dest, len) };
    SystemRng.fill_bytes(buf);
    Ok(())
}

/// `getrandom` **0.2**'s custom-backend hook, which is a different symbol with a different
/// signature and the same body.
///
/// This is what `getrandom::register_custom_getrandom!` expands to, written out rather than
/// invoked. The macro exists to type-check the registered function against `getrandom::getrandom`
/// and then emit exactly this; writing it directly costs that check and buys a definition that does
/// not depend on which of 0.2's re-exports are in scope. The signature is upstream's:
/// `unsafe fn __getrandom_custom(*mut u8, usize) -> u32`, Rust ABI, **zero for success** and an
/// error code otherwise. There is no error path here for the same reason there is none above:
/// `fill_bytes` writes the whole slice or panics.
///
/// # Safety
///
/// `getrandom` 0.2's contract, which is the 0.3 one plus a guarantee in our favour: `dest` is valid
/// for writes of `len` bytes, and 0.2 has already zero-filled it (`uninit_slice_fill_zero` in its
/// `custom.rs`) before calling, so nothing here can observe uninitialized memory.
#[unsafe(no_mangle)]
unsafe extern "Rust" fn __getrandom_custom(dest: *mut u8, len: usize) -> u32 {
    // SAFETY: the caller's contract, restated above.
    let buf = unsafe { slice::from_raw_parts_mut(dest, len) };
    SystemRng.fill_bytes(buf);
    0
}
