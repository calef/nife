#!/bin/sh
# Generates the second test authority beside this script, leaving regenerate.sh's fixtures alone
# (they are pinned by other tests, and regenerating them would change every one). Run once, by hand,
# and commit the output; nothing in a gate runs it.
#
# **A test fixture that protects nothing**, for the same reasons regenerate.sh gives. It stands in
# for Sectigo Public Server Authentication Root E46, the root `github.com` chains to: calef's C2 of
# 2026-10-12 (UTC) on #2048 lets the signed index name a root per listed location from the roots an
# image carries, and the boot test needs a second root to name. The test image carries it as
# `location-root test-second-root <hex>`.
#
#   second-root.der             P-384 root, the stand-in for E46
#   releases-test-chain.pem     leaf for releases.basalt.test (github.com's stand-in) and
#                               offroot.basalt.test (a redirect target off the index's root),
#                               issued directly by second-root, as E46 issues to E36 and on
#   releases-test-key.pem       its P-256 key
#
# Name: provisional, minted 2026-10-13 (UTC) by lane `lane/basalt-on-pages-and-releases`.
set -eu
cd "$(dirname "$0")"
work="$(mktemp -d "${TMPDIR:-/tmp}/second-root-fixtures.XXXXXX")"
trap 'rm -rf "$work"' EXIT

start=20260101000000Z
end=20360101000000Z

sign() { # sign <csr> <issuer-cert or "self"> <issuer-key> <ext-file> <out>
    db="$work/db"; rm -rf "$db"; mkdir -p "$db"; : >"$db/index.txt"; echo 1000 >"$db/serial"
    printf '[ca]\ndefault_ca=d\n[d]\ndatabase=%s/index.txt\nserial=%s/serial\nnew_certs_dir=%s\npolicy=p\nunique_subject=no\n[p]\ncommonName=supplied\n' \
        "$db" "$db" "$db" >"$work/ca.cnf"
    if [ "$2" = self ]; then issuer="-selfsign"; else issuer="-cert $2"; fi
    # shellcheck disable=SC2086
    openssl ca -batch -config "$work/ca.cnf" -in "$1" $issuer -keyfile "$3" -extfile "$4" \
        -startdate "$start" -enddate "$end" -md sha384 -notext -out "$5" 2>"$work/ca.log" ||
        { cat "$work/ca.log" >&2; exit 1; }
}

openssl ecparam -name secp384r1 -genkey -noout -out "$work/root.key"
openssl req -new -key "$work/root.key" -subj "/CN=nife test root, second (not a real certificate authority)" -out "$work/root.csr"
printf 'basicConstraints=critical,CA:TRUE\nkeyUsage=critical,keyCertSign,cRLSign\nsubjectKeyIdentifier=hash\n' >"$work/root.ext"
sign "$work/root.csr" self "$work/root.key" "$work/root.ext" "$work/root.pem"

openssl ecparam -name prime256v1 -genkey -noout -out "$work/leaf.key"
openssl req -new -key "$work/leaf.key" -subj "/CN=releases.basalt.test" -out "$work/leaf.csr"
printf 'basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:releases.basalt.test,DNS:offroot.basalt.test\n' >"$work/leaf.ext"
sign "$work/leaf.csr" "$work/root.pem" "$work/root.key" "$work/leaf.ext" "$work/leaf.pem"

openssl x509 -in "$work/root.pem" -outform der -out second-root.der
cat "$work/leaf.pem" >releases-test-chain.pem
openssl pkcs8 -topk8 -nocrypt -in "$work/leaf.key" -out releases-test-key.pem
echo "regenerate-second-root: fixtures written to $(pwd)"
