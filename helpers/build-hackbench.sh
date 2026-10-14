#!/usr/bin/env bash
# Build **hackbench, unmodified**, for the three nife targets (milestone 836 (a C library, stage 2:
# threads), its first consumer, and the program milestone 827 (hackbench on nife and Linux) runs on
# silicon).
#
# An experiment's apparatus on `helpers/build-speedtest1.sh`'s terms: nothing in `script/test` runs it,
# the archive packs `target/hackbench/<triple>/hackbench` when it is on disk, and
# `system_tests/src/user/c_thread_tests.rs` skips when it is not. Fetching it in a gate is §46 (thin
# primitives or whole subsystems)'s decision.
#
# The source is rt-tests 2.11's `src/hackbench/hackbench.c`, one file, from the `v2.11` tag on
# kernel.org, pinned by SHA-256 and compiled untouched. rt-tests is GPL-2.0-or-later; the binary
# this makes is therefore GPL, which §135 (running GPL software is aggregation) already covers for a
# program nife runs.
#
# What differs from rt-tests' own build is on the command line:
#
# - `-D_GNU_SOURCE -DVERSION=2.11 -O2` are rt-tests' Makefile's.
# - `-Wno-incompatible-pointer-types`: hackbench passes a `struct sockaddr_in *` to `bind`,
#   `getsockname`, `connect` and `accept` (its `--inet` mode, which this does not run). glibc
#   declares those parameters as a transparent union under `_GNU_SOURCE`, so the call compiles
#   there; relibc's `sys/socket.h` declares a plain `struct sockaddr *`, which clang now refuses by
#   default. The pointers are the same pointer, so the warning is the only difference.
#
# Usage: helpers/build-hackbench.sh
#        NIFE_HACKBENCH_TRIPLES="x86_64-unknown-nife" helpers/build-hackbench.sh   (one target)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION=2.11
# Read 2026-10-12 (UTC) from the download.
SHA256_HACKBENCH_C=36e5b4c86aee10fa7422e16c2a3cb218e7b040a364fe52f943ad8d68b167ac3e

BUILD="${TMPDIR:-/tmp}/nife-hackbench-$(printf %s "$ROOT" | shasum -a 256 | cut -c1-12)"
mkdir -p "$BUILD"
if [ ! -f "$BUILD/hackbench.c" ]; then
  echo "build-hackbench: fetching rt-tests $VERSION's hackbench.c"
  curl -sSfL --retry 3 --max-time 120 -o "$BUILD/hackbench.c.part" \
    "https://git.kernel.org/pub/scm/utils/rt-tests/rt-tests.git/plain/src/hackbench/hackbench.c?h=v$VERSION"
  mv "$BUILD/hackbench.c.part" "$BUILD/hackbench.c"
fi
got="$(shasum -a 256 "$BUILD/hackbench.c" | cut -d' ' -f1)"
if [ "$got" != "$SHA256_HACKBENCH_C" ]; then
  echo "build-hackbench: hackbench.c has SHA-256 $got, not $SHA256_HACKBENCH_C; refusing it" >&2
  rm -f "$BUILD/hackbench.c"
  exit 1
fi

CC="$("$ROOT/helpers/c-library-cc.sh")"

for TRIPLE in ${NIFE_HACKBENCH_TRIPLES:-aarch64-unknown-nife riscv64-unknown-nife x86_64-unknown-nife}; do
  OBJ="$BUILD/obj/$TRIPLE"
  mkdir -p "$OBJ"
  # The flags come on two lines (target, then library), and `read` stops at the first newline.
  read -r -a CFLAGS <<< "$("$ROOT/helpers/c-library-cflags.sh" "$TRIPLE" | tr '\n' ' ')"
  echo "build-hackbench: compiling hackbench $VERSION for $TRIPLE"
  "$CC" "${CFLAGS[@]}" -O2 -D_GNU_SOURCE -DVERSION=$VERSION -Wno-incompatible-pointer-types \
    -c "$BUILD/hackbench.c" -o "$OBJ/hackbench.o"
  "$ROOT/helpers/build-c-program.sh" hackbench "$TRIPLE" "$OBJ/hackbench.o"
done
