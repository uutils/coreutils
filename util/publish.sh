#!/bin/sh
ARG=""
if test "$1" != "--do-it"; then
    ARG="--dry-run --allow-dirty"
fi

# Figure out any dependencies between the util via Cargo.toml
# We store this as edges in a graph with each line:
# [dependent] [dependency]
# We use ROOT as a the node that should come before all other nodes.
PROGS=$(ls -1d src/uu/*/)
PARTIAL_ORDER=""
for p in $PROGS; do
    DEPENDENCIES=$(grep -oE "^uu_[a-z0-9]+" ${p}Cargo.toml)

    # Turn "src/uu/util/" into "util"
    p=${p#src/uu/}
    p=${p%/}

    PARTIAL_ORDER+="$p ROOT\n"
    while read d; do
        if [ $d ]; then
            # Remove "uu_" prefix
            d=${d#uu_}

            PARTIAL_ORDER+="$p $d\n"
        fi
    done <<<"$DEPENDENCIES"
done

# Apply tsort to get the order in which to publish the crates
TOTAL_ORDER=$(echo -e $PARTIAL_ORDER | tsort | tac)

# Remove the ROOT node from the start
TOTAL_ORDER=${TOTAL_ORDER#ROOT}

set -e
for dir in src/uucore/ src/uucore_procs/ src/uu/stdbuf/src/libstdbuf/; do
    (
        cd "$dir"
        #shellcheck disable=SC2086
        cargo publish $ARG
    )
    sleep 2s
done

for p in $TOTAL_ORDER; do
    (
        cd "src/uu/$p"
        #shellcheck disable=SC2086
        cargo publish $ARG
    )
done

#shellcheck disable=SC2086
cargo publish $ARG
