#!/bin/zsh
# cargo miri test on a sample of published packages, each in a scratch copy this script owns.
S=$1; shift
export PATH=$HOME/.cargo/bin:$PATH
export CARGO_TARGET_DIR=$S/miritarget
mkdir -p $S/miriws $S/mirilogs
print -r -- "crate\tversion\tresult\tpassed\tfailed\tub_reported\tseconds" > $S/miri.tsv
for key in "$@"; do
  n=${key%-*}; v=${key##*-}
  rm -rf $S/miriws/$key; cp -R $S/src/$key $S/miriws/$key
  grep -q '^\[workspace\]' $S/miriws/$key/Cargo.toml || print '\n[workspace]' >> $S/miriws/$key/Cargo.toml
  cd $S/miriws/$key
  [ -f Cargo.lock ] || cargo +nightly-2026-10-18 generate-lockfile > $S/mirilogs/$key.log 2>&1
  t0=$(date +%s)
  MIRIFLAGS=-Zmiri-disable-isolation perl -e 'alarm shift; exec @ARGV' 1200 cargo +nightly-2026-10-18 miri test --no-fail-fast >> $S/mirilogs/$key.log 2>&1; rc=$?
  t1=$(date +%s)
  p=$(grep -E '^test result:' $S/mirilogs/$key.log | sed -E 's/.* ([0-9]+) passed; ([0-9]+) failed.*/\1 \2/' | awk '{p+=$1;f+=$2} END{print p+0, f+0}')
  ub=$(grep -c 'Undefined Behavior' $S/mirilogs/$key.log)
  if [ $rc = 142 ]; then r=timeout; elif grep -qE '^test result:' $S/mirilogs/$key.log; then r=ran; else r=fail; fi
  print -r -- "$n\t$v\t$r\t${p// /\t}\t$ub\t$((t1-t0))" >> $S/miri.tsv
  cd $S
done
echo DONE
