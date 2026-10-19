#!/bin/sh
#
# Selftest for helpers/rustup-retry.sh, run by script/lint. A stub `rustup` on PATH plays a script of
# outcomes, so this needs no network and no toolchain. The retry only ever runs when the CDN
# misbehaves, which no green CI run exercises; without this, a classifier that stopped matching
# rustup's wording would look exactly like a healthy CDN until the next outage.

set -eu
helper="$(cd "$(dirname "$0")" && pwd)/rustup-retry.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

fail() { echo "rustup-retry-selftest: $*" >&2; exit 1; }

# The stub prints the Nth line of $work/plan on its Nth call: "ok", or the error text to fail with.
mkdir "$work/bin"
cat > "$work/bin/rustup" <<'EOF'
#!/bin/sh
n=$(($(cat "$PLAN.n" 2>/dev/null || echo 0) + 1))
echo "$n" > "$PLAN.n"
line=$(sed -n "${n}p" "$PLAN")
[ "$line" = ok ] && { echo "info: installed"; exit 0; }
echo "$line" >&2
exit 1
EOF
chmod +x "$work/bin/rustup"

# The 2026-10-18 failure, verbatim from job 117825390205 (the temporary path shortened).
timeout="error: could not download file from 'https://static.rust-lang.org/dist/2026-10-18/channel-rust-nightly.toml.sha256' to '/home/runner/.rustup/tmp/x_file': error downloading file: error sending request for url (https://static.rust-lang.org/dist/2026-10-18/channel-rust-nightly.toml.sha256): client error (Connect): tcp connect error: Connection timed out (os error 110)"
# The 2026-10-08 failure, from job 113106039294: the CDN mid-publish.
checksum="error: checksum failed for 'https://static.rust-lang.org/dist/channel-rust-nightly.toml', expected: '8f92', calculated: 'e9d9'"

# run PLAN_LINES...: run the helper against a fresh plan; sets $out, $rc and $calls.
run() {
    printf '%s\n' "$@" > "$work/plan"
    rm -f "$work/plan.n"
    set +e
    out=$(PATH="$work/bin:$PATH" PLAN="$work/plan" NIFE_RUSTUP_ATTEMPTS=3 NIFE_RUSTUP_BACKOFF=0 \
        GITHUB_ACTIONS=true "$helper" toolchain install 2>&1)
    rc=$?
    set -e
    calls=$(cat "$work/plan.n")
}

run "$timeout" "$checksum" ok
[ "$rc" -eq 0 ] || fail "recovers: exit $rc, wanted 0 ($out)"
[ "$calls" -eq 3 ] || fail "recovers: $calls calls, wanted 3"

run "$timeout" "$timeout" "$timeout" ok
[ "$rc" -ne 0 ] || fail "gives up loudly: exit 0 after three failures"
[ "$calls" -eq 3 ] || fail "gives up loudly: $calls calls, wanted the bound of 3"
printf '%s' "$out" | grep -q "failed 3 of 3 attempts fetching https://static.rust-lang.org/dist/2026-10-18/channel-rust-nightly.toml.sha256" \
    || fail "gives up loudly: no attempt count and URL ($out)"
printf '%s' "$out" | grep -q "^::error title=rust toolchain download::" || fail "gives up loudly: no annotation ($out)"

run "error: toolchain 'nightly-2026-10-18' does not contain component 'no-such' for target 'x86_64-unknown-linux-gnu'" ok
[ "$rc" -ne 0 ] || fail "real error: exit 0"
[ "$calls" -eq 1 ] || fail "real error: retried a non-download error ($calls calls)"

echo "rustup-retry-selftest: ok"
