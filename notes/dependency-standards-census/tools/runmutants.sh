#!/bin/zsh
# cargo-mutants on a sample of published packages, each in a scratch copy this script owns.
S=$1; shift
export PATH=$HOME/.cargo/bin:$PATH
mkdir -p $S/mutws $S/mutlogs
for key in "$@"; do
  rm -rf $S/mutws/$key; cp -R $S/src/$key $S/mutws/$key
  grep -q '^\[workspace\]' $S/mutws/$key/Cargo.toml || print '\n[workspace]' >> $S/mutws/$key/Cargo.toml
  cd $S/mutws/$key
  [ -f Cargo.lock ] || cargo generate-lockfile > $S/mutlogs/$key.log 2>&1
  perl -e 'alarm shift; exec @ARGV' 3600 cargo mutants --no-shuffle -j 2 --timeout 120 -o $S/mutlogs/$key >> $S/mutlogs/$key.log 2>&1
  print -r -- "$key rc=$?" >> $S/mutlogs/summary.txt
  tail -3 $S/mutlogs/$key.log >> $S/mutlogs/summary.txt
  cd $S
done
echo DONE
