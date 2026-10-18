#!/bin/sh
# Fetch basalt's packages from one release, re-encode each as NIFEPKG2 and as tar, compress every
# form with zstd at levels 3, 19 and 20, and print the size table.
# Usage: measure-sizes.sh <release tag> <empty work directory>
set -eu
here="$(cd "$(dirname "$0")" && pwd)"
tag="$1"; work="$2"
mkdir -p "$work/pkgs" "$work/out"
gh release download "$tag" -R nifeos/basalt -p '*.nifepkg' -D "$work/pkgs"
python3 -I "$here/scripts/build.py" "$work/out" "$work/pkgs"/*.nifepkg > /dev/null
cd "$work/out"
for f in *.nifepkg2 *.tar; do
    for level in 3 19 20; do zstd -q -f --ultra "-$level" -o "$f.$level.zst" "$f"; done
done
for f in "$work/pkgs"/*.nifepkg; do zstd -q -f -19 -o "$(basename "$f").nifepkg1.19.zst" "$f"; done
python3 -I "$here/scripts/table.py" "$work/out" "$work/pkgs"
