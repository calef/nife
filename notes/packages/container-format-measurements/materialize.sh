#!/bin/sh
# Copy the four harnesses (readers, size, hostcheck, fuzz) into <work directory> as buildable
# crates. Their manifests are kept here as Cargo.toml.in so the tree's package gate does not count
# them as nife crates; this names each Cargo.toml and points `size` at this tree's package_archive.
# Usage: materialize.sh <work directory>
set -eu
here="$(cd "$(dirname "$0")" && pwd)"
tree="$(cd "$here/../../.." && pwd)"
work="$1"
mkdir -p "$work"
for crate in readers size hostcheck fuzz; do
    rm -rf "${work:?}/$crate"
    cp -R "$here/$crate" "$work/$crate"
    sed "s#@NIFE@#$tree#" "$here/$crate/Cargo.toml.in" > "$work/$crate/Cargo.toml"
    rm "$work/$crate/Cargo.toml.in"
done
