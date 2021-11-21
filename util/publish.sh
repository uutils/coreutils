#!/bin/bash
set -e

ARG=""
if test "$1" != "--do-it"; then
   ARG="--dry-run --allow-dirty"
fi

cd src/uucore/
cargo publish $ARG
cd -
sleep 2s

cd src/uucore_procs/
cargo publish $ARG
cd -
sleep 2s

cd src/uu/stdbuf/src/libstdbuf/
cargo publish $ARG
cd -
sleep 2s

PROGS=$(ls -1d src/uu/*/)
for p in $PROGS; do
    cd $p
    cargo publish $ARG
    cd -
done

cargo publish $ARG
