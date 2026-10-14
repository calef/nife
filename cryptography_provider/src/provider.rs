//! **The provider and what it offers**: the three TLS 1.3 cipher suites and the signature
//! algorithms it verifies, assembled into one `CryptoProvider`.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

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
pub(crate) static SIGNATURE_ALGORITHMS: WebPkiSupportedAlgorithms = WebPkiSupportedAlgorithms {
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
