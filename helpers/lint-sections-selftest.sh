#!/bin/sh
#
# helpers/lint-sections-selftest.sh: prove helpers/lint-sections.sh gives the verdict a fail-fast
# run gives, while naming every failure instead of the first.
#
#     helpers/lint-sections-selftest.sh
#
# A fixture lint script has four sections that fail on demand (a test, an `exit 3`, a plain
# command under `set -e`, and one that runs a stubbed cargo) and one that reads a file an earlier
# section wrote. For each of the 16 combinations of planted failures, in both `all` and `no-cargo`
# mode, three things must hold:
#   1. the helper exits non-zero exactly when the fixture run whole, as one `set -e` script, does;
#   2. its summary names exactly the planted sections that run in that mode;
#   3. the last section runs whatever failed before it.
# No tree and no real cargo: it runs in milliseconds, and in `script/lint --no-cargo`.

set -eu
cd "$(dirname "$0")/.."
work="$(mktemp -d "${TMPDIR:-/tmp}/lint-sections-selftest.XXXXXX")"
trap 'rm -rf "$work"' EXIT
fail() { echo "lint-sections-selftest: $1" >&2; exit 1; }

mkdir "$work/bin"
printf '#!/bin/sh\n[ -z "${PLANT_D:-}" ]\n' >"$work/bin/cargo"
chmod +x "$work/bin/cargo"
PATH="$work/bin:$PATH"
OUT="$work/out"
export OUT

cat >"$work/lint" <<'FIXTURE'
set -e
shared=from-the-preamble
echo "==> a"
[ -z "${PLANT_A:-}" ]
echo "==> b reads the preamble"
[ "$shared" = from-the-preamble ]
[ -z "${PLANT_B:-}" ] || exit 3
echo "==> c writes a file"
echo built >"$OUT/built"
if [ -n "${PLANT_C:-}" ]; then false; fi
echo "==> d runs cargo"
cargo clippy
echo "==> e reads what c wrote"
[ -f "$OUT/built" ]
echo "e ran"
FIXTURE

# What `script/lint --no-cargo` ran before the helper: the kept sections as one script.
awk -v mode=emit -f helpers/lint-no-cargo.awk "$work/lint" >"$work/lint-no-cargo"

for combo in 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15; do
    unset PLANT_A PLANT_B PLANT_C PLANT_D
    planted=""
    [ $((combo & 1)) -eq 0 ] || { PLANT_A=1; export PLANT_A; planted="$planted a"; }
    [ $((combo & 2)) -eq 0 ] || { PLANT_B=1; export PLANT_B; planted="$planted b"; }
    [ $((combo & 4)) -eq 0 ] || { PLANT_C=1; export PLANT_C; planted="$planted c"; }
    [ $((combo & 8)) -eq 0 ] || { PLANT_D=1; export PLANT_D; planted="$planted d"; }
    for mode in all no-cargo; do
        if [ "$mode" = all ]; then whole="$work/lint"; want="$planted"; else
            whole="$work/lint-no-cargo"; want="$(printf '%s' "$planted" | sed 's/ d//')"; fi

        rm -rf "$OUT"; mkdir "$OUT"
        set +e
        LINT_NO_CARGO=1 sh "$whole" >/dev/null 2>&1
        fast=$?
        rm -rf "$OUT"; mkdir "$OUT"
        out="$(sh helpers/lint-sections.sh "$work/lint" "$mode" 2>&1)"
        keep=$?
        set -e

        [ $((fast != 0)) -eq $((keep != 0)) ] \
            || fail "planted [$planted] $mode: fail-fast exit $fast, keep-going exit $keep"
        named="$(printf '%s\n' "$out" | sed -n 's/^  \([a-e]\) .*(exit [0-9]*)$/\1/p' | tr '\n' ' ' | sed 's/^/ /; s/ $//')"
        [ "$named" = "$want" ] || fail "planted [$planted] $mode: summary named [$named], wanted [$want]"
        printf '%s\n' "$out" | grep -q '^e ran$' || fail "planted [$planted] $mode: the last section never ran"
    done
done
echo "lint-sections-selftest: ok"
