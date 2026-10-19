#!/bin/zsh
# Run each published package's own test suite, unchanged, in a scratch copy this script owns.
# usage: runtests.sh <scratch> <list.tsv>   (list: crate, version, class)
S=$1; LIST=$2
export PATH=$HOME/.cargo/bin:$PATH
export CARGO_TARGET_DIR=$S/testtarget
mkdir -p $S/testws $S/testlogs
rustc --version > $S/testlogs/rustc-version.txt
print -r -- "crate\tversion\tclass\tlockfile\tbuild\tpassed\tfailed\tignored\tseconds" > $S/tests.tsv
while IFS=$'\t' read n v c; do
  key=$n-$v
  [ -e $S/testlogs/$key.done ] && { cat $S/testlogs/$key.row >> $S/tests.tsv; continue; }
  rm -rf $S/testws/$key; cp -R $S/src/$key $S/testws/$key
  grep -q '^\[workspace\]' $S/testws/$key/Cargo.toml || print '\n[workspace]' >> $S/testws/$key/Cargo.toml
  rm -f $S/testws/$key/rust-toolchain $S/testws/$key/rust-toolchain.toml
  cd $S/testws/$key
  lock=generated; [ -f Cargo.lock ] && lock=shipped
  t0=$(date +%s)
  if [ $lock = generated ]; then cargo generate-lockfile > $S/testlogs/$key.log 2>&1; fi
  perl -e 'alarm shift; exec @ARGV' 900 cargo test --no-fail-fast >> $S/testlogs/$key.log 2>&1; rc=$?
  t1=$(date +%s)
  p=$(grep -E '^test result:' $S/testlogs/$key.log | sed -E 's/.* ([0-9]+) passed; ([0-9]+) failed; ([0-9]+) ignored.*/\1 \2 \3/' | awk '{p+=$1;f+=$2;i+=$3} END{print p+0, f+0, i+0}')
  if grep -qE '^test result:' $S/testlogs/$key.log; then b=ok; elif [ $rc = 142 ]; then b=timeout; else b=fail; fi
  row="$n\t$v\t$c\t$lock\t$b\t${p// /\t}\t$((t1-t0))"
  print -r -- "$row" > $S/testlogs/$key.row; print -r -- "$row" >> $S/tests.tsv
  touch $S/testlogs/$key.done
  cd $S
done < $LIST
echo DONE
