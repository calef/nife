#!/bin/sh
#
# helpers/lint-sections.sh: run script/lint's sections one at a time, keep going past a failure,
# and fail at the end if any section failed.
#
#     helpers/lint-sections.sh <lint script> all|no-cargo|clippy
#
# `script/lint` calls this; you do not. Each section is the preamble plus the text from its
# `echo "==> ` header to the next one, as helpers/lint-no-cargo.awk splits them, run in a fresh `sh`
# with $0 naming the lint script. A failing section prints what it always printed, the next section
# runs anyway, and a summary at the end names every section that failed.
#
# Why not stop at the first failure. On 2026-10-18 (UTC) 16 lanes ran lint 178 times and 74 runs
# failed. 25 of those failures were already in the tree at the run before, so a lane learned one
# problem per run. notes/lint-cost-2026-10-18.md has the counts.
#
# **The verdict is the fail-fast verdict.** A run passes exactly when every section passes, which
# is when the old single `set -e` script passed: a section that would have been skipped after an
# earlier failure can only add failures to a run that already fails. Each section runs in order,
# so a section that reads a file an earlier one wrote (the uefi_loader clippy passes read the
# kernels the section before them builds) still finds it. A section cannot see an earlier one's
# shell variables, so anything two sections share belongs in the preamble; a section that breaks
# that rule fails here, loudly, in every run. helpers/lint-sections-selftest.sh holds the verdict
# to fail-fast's across every combination of planted failures.
#
# `no-cargo` runs only the sections `--no-cargo` keeps, with LINT_NO_CARGO=1. `clippy` stops after
# the section that carries the `--clippy` cut, which is where `script/lint --clippy` stopped.
#
# Name: provisional, minted 2026-10-19 (UTC) by lane maintainer/lint-cost.

set -u
lint_self=$1
mode=$2
root="$(cd "$(dirname "$0")/.." && pwd)"
dir="$(mktemp -d "${TMPDIR:-/tmp}/lint-sections.XXXXXX")" || exit 1
trap 'rm -rf "$dir"' EXIT HUP INT TERM
awk -v mode=split -v dir="$dir" -f "$root/helpers/lint-no-cargo.awk" "$lint_self" || exit 1

if [ "$mode" = no-cargo ]; then
    LINT_NO_CARGO=1
    export LINT_NO_CARGO
fi
LINT_SECTION=1
export LINT_SECTION

tab="$(printf '\t')"
ran=0
failed=""
# The index is read on fd 3 so that a section reading stdin cannot eat it.
while IFS="$tab" read -r id keep name <&3; do
    if [ "$mode" = no-cargo ] && [ "$keep" = 0 ]; then
        continue
    fi
    cat "$dir/preamble" "$dir/$id" >"$dir/run"
    ran=$((ran + 1))
    sh -c 'lint_tmp=$1; shift; . "$lint_tmp"' "$lint_self" "$dir/run"
    status=$?
    if [ "$status" -ne 0 ]; then
        failed="$failed  $name (exit $status)
"
    fi
    if [ "$mode" = clippy ] && grep -q '"--clippy"' "$dir/$id"; then
        echo "==> clippy-only mode: the rest of lint still runs before any report or merge"
        break
    fi
done 3<"$dir/index"

if [ -n "$failed" ]; then
    count="$(printf '%s' "$failed" | grep -c .)"
    echo "" >&2
    echo "lint: $count of $ran sections failed. Each one's own output is above its name's ==> line:" >&2
    printf '%s' "$failed" >&2
    exit 1
fi
echo "lint: all $ran sections passed"
