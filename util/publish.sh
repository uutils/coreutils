#!/bin/sh
set -e

ARG=""
if test "$1" != "--do-it"; then
    ARG="--dry-run --allow-dirty"
fi

for dir in src/uucore/ src/uucore_procs/ src/uu/stdbuf/src/libstdbuf/; do
    (
        cd "$dir"
        #shellcheck disable=SC2086
        cargo publish $ARG
    )
    sleep 2s
done

PROGS=$(ls -1d src/uu/*/)
for p in $PROGS; do
    (
        cd "$p"
        #shellcheck disable=SC2086
        cargo publish $ARG
    )
done

#shellcheck disable=SC2086
cargo publish $ARG
