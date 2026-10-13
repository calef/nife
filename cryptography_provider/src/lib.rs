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

/// **The provider.** Hand it to `rustls`' config builder, or install it as the process default.
///
/// # Examples
///
/// ```no_run
/// // The provider, ready to hand to `rustls`.
/// let provider = cryptography_provider::provider();
/// assert_eq!(provider.cipher_suites.len(), 3);
/// assert_eq!(provider.kx_groups.len(), 2);
/// ```
pub fn provider() -> CryptoProvider {
    CryptoProvider {
        cipher_suites: vec![
            TLS13_AES_128_GCM_SHA256,
            TLS13_AES_256_GCM_SHA384,
            TLS13_CHACHA20_POLY1305_SHA256,
        ],
        // X25519 first: it is the default key share a client sends, and it is what almost every
        // modern peer prefers, so putting it first is the difference between one round trip and
        // two on the common path.
        kx_groups: vec![&kx::X25519, &kx::P256],
        signature_verification_algorithms: SIGNATURE_ALGORITHMS,
        secure_random: &Entropy,
        key_provider: &NoPrivateKeys,
    }
}

/// `TLS13_AES_128_GCM_SHA256`.
pub static TLS13_AES_128_GCM_SHA256: SupportedCipherSuite =
    SupportedCipherSuite::Tls13(&rustls::Tls13CipherSuite {
        common: rustls::crypto::CipherSuiteCommon {
            suite: CipherSuite::TLS13_AES_128_GCM_SHA256,
            hash_provider: &hash::SHA256,
            // 2^24, which is `rustls`' own documented figure for AES-GCM: past it an attacker
            // gains an advantage distinguishing the record stream from a random permutation.
            // Copied from that documentation rather than chosen, because choosing it is a
            // cryptographic judgement and this crate makes none.
            confidentiality_limit: 1 << 24,
        },
        hkdf_provider: &rustls::crypto::tls13::HkdfUsingHmac(&hmac::SHA256),
        aead_alg: &aead::AES128_GCM,
        quic: None,
    });

/// `TLS13_AES_256_GCM_SHA384`.
pub static TLS13_AES_256_GCM_SHA384: SupportedCipherSuite =
    SupportedCipherSuite::Tls13(&rustls::Tls13CipherSuite {
        common: rustls::crypto::CipherSuiteCommon {
            suite: CipherSuite::TLS13_AES_256_GCM_SHA384,
            hash_provider: &hash::SHA384,
            confidentiality_limit: 1 << 24,
        },
        hkdf_provider: &rustls::crypto::tls13::HkdfUsingHmac(&hmac::SHA384),
        aead_alg: &aead::AES256_GCM,
        quic: None,
    });

/// `TLS13_CHACHA20_POLY1305_SHA256`.
pub static TLS13_CHACHA20_POLY1305_SHA256: SupportedCipherSuite =
    SupportedCipherSuite::Tls13(&rustls::Tls13CipherSuite {
        common: rustls::crypto::CipherSuiteCommon {
            suite: CipherSuite::TLS13_CHACHA20_POLY1305_SHA256,
            hash_provider: &hash::SHA256,
            // `u64::MAX` for ChaCha20-Poly1305, again `rustls`' own documented figure: the bound
            // that forces rekeying for AES-GCM does not apply to a stream cipher this way.
            confidentiality_limit: u64::MAX,
        },
        hkdf_provider: &rustls::crypto::tls13::HkdfUsingHmac(&hmac::SHA256),
        aead_alg: &aead::CHACHA20_POLY1305,
        quic: None,
    });

/// Every signature algorithm this provider will verify, and the map from TLS's own scheme numbers
/// to them.
///
/// **The two halves are not redundant.** `all` is what `rustls-webpki` walks a certificate chain
/// with; `mapping` is what `rustls` uses for the peer's handshake signature, where the scheme is
/// named on the wire. A scheme absent from `mapping` is one this client will not offer.
static SIGNATURE_ALGORITHMS: WebPkiSupportedAlgorithms = WebPkiSupportedAlgorithms {
    all: &[
        &verify::EcdsaP256Sha256,
        &verify::EcdsaP384Sha384,
        &verify::Ed25519,
        &verify::RSA_PKCS1_SHA256,
        &verify::RSA_PKCS1_SHA384,
        &verify::RSA_PKCS1_SHA512,
        &verify::RSA_PSS_SHA256,
        &verify::RSA_PSS_SHA384,
        &verify::RSA_PSS_SHA512,
    ],
    mapping: &[
        (
            SignatureScheme::ECDSA_NISTP256_SHA256,
            &[&verify::EcdsaP256Sha256],
        ),
        (
            SignatureScheme::ECDSA_NISTP384_SHA384,
            &[&verify::EcdsaP384Sha384],
        ),
        (SignatureScheme::ED25519, &[&verify::Ed25519]),
        // **PSS only in the mapping, and PKCS#1 v1.5 deliberately absent from it.** This table is
        // what verifies the peer's *handshake* signature, and RFC 8446 section 4.4.3 forbids
        // PKCS#1 v1.5 there: a TLS 1.3 server signing with an RSA key must use PSS. The `all`
        // list above is the other question, certificate chains, where v1.5 is what almost every
        // certificate authority actually issues.
        (SignatureScheme::RSA_PSS_SHA256, &[&verify::RSA_PSS_SHA256]),
        (SignatureScheme::RSA_PSS_SHA384, &[&verify::RSA_PSS_SHA384]),
        (SignatureScheme::RSA_PSS_SHA512, &[&verify::RSA_PSS_SHA512]),
    ],
};

/// `rustls`' randomness, which on nife is the entropy service.
///
/// The chain is `rustls` to here, to `rand_core`'s `OsRng`, to `getrandom`, to
/// `entropy_backend`'s hook, to `std::random::SystemRng`, to the entropy service's one endpoint,
/// to the device. Every link but `getrandom`'s is one this project owns.
#[derive(Debug)]
struct Entropy;

impl SecureRandom for Entropy {
    fn fill(&self, buf: &mut [u8]) -> Result<(), rustls::crypto::GetRandomFailed> {
        use rand_core::RngCore;
        // `try_fill_bytes` rather than `fill_bytes`: the infallible one panics on a failing
        // source, and a process that cannot get randomness should fail the handshake rather than
        // die. On nife a draw with no entropy capability granted panics inside `SystemRng`
        // regardless, which is the deliberate choice of milestone 56 (secrets, credentials, and
        // the entropy to make them safe) and not something this layer can soften.
        rand_core::OsRng
            .try_fill_bytes(buf)
            .map_err(|_| rustls::crypto::GetRandomFailed)
    }
}

/// A key provider that refuses every private key, because this provider is for a client that
/// presents no certificate.
///
/// **A refusal rather than an omission.** `rustls` requires the field, so the choice is between
/// answering honestly and pulling in PKCS#8 parsing and a signing path that nothing here would
/// ever exercise. Anything that needs client authentication needs a different provider, and will
/// find out at the call that loads the key rather than at the handshake.
#[derive(Debug)]
struct NoPrivateKeys;

impl KeyProvider for NoPrivateKeys {
    fn load_private_key(
        &self,
        _key_der: pki_types::PrivateKeyDer<'static>,
    ) -> Result<Arc<dyn SigningKey>, Error> {
        Err(Error::General(alloc::string::String::from(
            "this provider loads no private keys: it is for a client that presents no certificate",
        )))
    }
}
