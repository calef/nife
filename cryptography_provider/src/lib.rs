//! **A `rustls` crypto provider assembled in this tree, over primitives chosen deliberately**
//! (notes/cryptography-provider.md). §196 (nife carries TLS) made the provider work rather than a
//! purchase, and calef refused the one pure-Rust option, `rustls-rustcrypto`, as low quality: that
//! refusal is about the glue, so this crate is glue and nothing else.
//!
//! §46 (thin primitives or whole subsystems) takes cryptography rather than writing it. **Every
//! cryptographic operation is taken** from audited crates (AES-GCM, ChaCha20-Poly1305, SHA-2,
//! HMAC, X25519, P-256 and P-384 ECDSA, Ed25519). **The glue is written, and glue is not
//! cryptography**: a [`rustls::crypto::CryptoProvider`] is five fields, and this selects, names and
//! plumbs them, computing nothing secret-dependent. The case for owning it was never size; the glue
//! is the least-exposed piece of the chain, so it is the piece worth owning.
//!
//! **TLS 1.3 only**: a package client talks to hosts that have spoken it for years, and 1.2 would
//! add a second record layer, key schedule and RSA key transport.
//!
//! | field | what this provides |
//! |---|---|
//! | `cipher_suites` | `TLS13_AES_128_GCM_SHA256`, `TLS13_AES_256_GCM_SHA384`, `TLS13_CHACHA20_POLY1305_SHA256` |
//! | `kx_groups` | X25519, NIST P-256 |
//! | `signature_verification_algorithms` | ECDSA P-256/SHA-256, ECDSA P-384/SHA-384, Ed25519, and RSA PKCS#1 v1.5 and PSS with SHA-256, SHA-384 and SHA-512 |
//! | `secure_random` | the entropy service, through `entropy_backend` and `getrandom` |
//! | `key_provider` | refuses: this provider loads no private keys |
//!
//! On nife a consuming binary also needs `use entropy_backend as _;` and
//! `cryptography_exerciser/.cargo/config.toml`'s build configuration, or the random source has no
//! backend and, on `x86_64-unknown-nife`, a primitive can execute an instruction the target lacks.
//!
//! # BUGS
//!
//! - An RSA key whose SPKI says `RSASSA-PSS` rather than `rsaEncryption`, and a PKCS#1 v1.5
//!   identifier with absent parameters, are not verified; such a chain fails closed.
//! - No TLS 1.2, no client certificates (`key_provider` refuses every key), and no FIPS claim.

#![no_std]

extern crate alloc;

use alloc::{sync::Arc, vec};

use rustls::crypto::{CryptoProvider, KeyProvider, SecureRandom, WebPkiSupportedAlgorithms};
use rustls::sign::SigningKey;
use rustls::{CipherSuite, Error, SignatureScheme, SupportedCipherSuite};

mod aead;
mod hash;
mod hmac;
mod kx;
mod verify;

mod platform;
mod provider;

use platform::*;
pub use provider::{
    TLS13_AES_128_GCM_SHA256, TLS13_AES_256_GCM_SHA384, TLS13_CHACHA20_POLY1305_SHA256, provider,
};
