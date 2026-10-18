#!/bin/sh
# Build one bare-metal binary per reader and print .text and .rodata in bytes. Each binary runs the
# same operations (parse, the three identity fields, every member's name, bytes and digest, one
# lookup by name) on black-boxed input; `none` is the empty baseline to subtract from both columns
# (its .rodata is the 4,096-byte input buffer).
# Usage: measure-code-size.sh <empty work directory>
set -eu
here="$(cd "$(dirname "$0")" && pwd)"
out="$1"
"$here/materialize.sh" "$out/src"
# The copies sit outside the tree, so name the toolchain the tree pins.
RUSTUP_TOOLCHAIN="$(sed -n 's/^channel = "\(.*\)"/\1/p' "$here/../../../rust-toolchain.toml")"
export RUSTUP_TOOLCHAIN
size="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin/llvm-size"
for opt in 3 s; do
    for f in none manifest_only n1 n2 tar_own tar_own_ustar tar_no_std ruzstd ruzstd_on; do
        CARGO_PROFILE_RELEASE_OPT_LEVEL=$opt cargo build -q --release \
            --manifest-path "$out/src/size/Cargo.toml" --target aarch64-unknown-none-softfloat \
            --no-default-features --features "$f" --target-dir "$out/$opt-$f"
        printf 'opt-level=%s %-14s ' "$opt" "$f"
        "$size" -A "$out/$opt-$f/aarch64-unknown-none-softfloat/release/size" |
            awk '$1==".text"{t=$2} $1==".rodata"{r=$2} END{print ".text", t, ".rodata", r+0}'
    done
done
