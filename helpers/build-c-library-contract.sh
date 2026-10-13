#!/usr/bin/env bash
# Build **the C library's contract program**, `fixtures/c/c_library_contract.c`, for the three nife
# targets (milestone 833 (the same storage benchmark on nife and Linux, by porting real fio), for
# the pieces fio 3.40 needed: `getrusage`, `basename`, `select`, `O_SYNC` and the refusals).
#
# It fetches nothing, unlike `helpers/build-ioping.sh`, but it rides the same way: nothing in
# `script/test` runs this, the archive packs `target/c_library_contract/<triple>/c_library_contract`
# when it is on disk, and `system_tests/src/user/c_program_tests.rs` skips when it is not. Building
# the C library in every gate is milestone 869 (the C library's headers cannot drift, and its
# contract is tested in nife)'s to decide.
#
# Name: provisional, lane lane/fio-c-library-pieces, 2026-10-12 (UTC).
#
# Usage: helpers/build-c-library-contract.sh
#        NIFE_CONTRACT_TRIPLES="x86_64-unknown-nife" helpers/build-c-library-contract.sh   (one target)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CC="$("$ROOT/helpers/c-library-cc.sh")"
OUT="$ROOT/target/c_library_contract/obj"

for TRIPLE in ${NIFE_CONTRACT_TRIPLES:-aarch64-unknown-nife riscv64-unknown-nife x86_64-unknown-nife}; do
  mkdir -p "$OUT/$TRIPLE"
  # The flags come on two lines (target, then library), and `read` stops at the first newline.
  read -r -a CFLAGS <<< "$("$ROOT/helpers/c-library-cflags.sh" "$TRIPLE" | tr '\n' ' ')"
  echo "build-c-library-contract: compiling for $TRIPLE"
  "$CC" "${CFLAGS[@]}" -O2 -std=gnu11 -Wall -Werror \
    -c "$ROOT/fixtures/c/c_library_contract.c" -o "$OUT/$TRIPLE/c_library_contract.o"
  "$ROOT/helpers/build-c-program.sh" c_library_contract "$TRIPLE" "$OUT/$TRIPLE/c_library_contract.o"
done
