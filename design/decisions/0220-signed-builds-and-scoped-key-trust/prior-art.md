# §220's prior art, read 2026-09-26

Moved verbatim from [`0220-signed-builds-and-scoped-key-trust.md`](../0220-signed-builds-and-scoped-key-trust.md) on 2026-10-13 (UTC) to bring that
page under §212 (a prose budget)'s 3,000-word cap by the whole-file count. The page keeps the
heading, and the text below is unchanged.

## Prior art, read 2026-09-26

- Apple, `developer.apple.com/documentation/technotes/tn3126-inside-code-signing-hashes`. The
  signature covers a CodeDirectory of page hashes, entitlements among its special slots. How
  `codeLimit` keeps the signature out of the hashed pages is from memory. Restricted entitlements
  need an Apple-signed provisioning profile (TN3125), a ceiling issued by a third party. Revocation
  is OCSP only.
- Android, `source.android.com/docs/security/features/apksigning`. The signing block sits inside the
  file, excluded from what is protected. "Android currently doesn't perform CA verification". v3
  rotates keys through a lineage, each certificate signing the next. That a stolen key cannot be
  revoked is inference; the pages are silent.
- Fuchsia, `fuchsia.dev/fuchsia-src/concepts/security/verified_execution`. Packages are not signed
  one by one. TUF targets carry a Merkle root, and base packages need no check after boot.
- Sigstore, `docs.sigstore.dev/about/overview/`. A fresh key per signing, a ten-minute certificate
  bound to an identity, and a Rekor log entry. Nothing to revoke; misuse is found by watching the
  log.
- TUF, `theupdateframework.github.io/specification/latest/`. Four roles with key thresholds; root
  N+1 must satisfy N's threshold; delegations are scoped by `paths` globs.
- Authenticode, `learn.microsoft.com/en-us/security-updates/securitybulletins/2013/ms13-098`, and
  the PE format page. The checksum and certificate table are excluded from the hash.
- Linux IMA, `ima-doc.readthedocs.io/en/latest/ima-concepts.html`. The signature is an extended
  attribute over the file digest, so nothing is excluded. `.blacklist` holds revoked keys and
  hashes.

`docs.rs` lists `ed25519-dalek` 3.0.0 as current, while `notes/cryptography-provider.md` called
2.2.0 the "current line" on 2026-09-20. Not reconciled here.
