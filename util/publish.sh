#!/bin/sh
ARG=""
if test "$1" != "--do-it"; then
    ARG="--dry-run --allow-dirty"
fi

# Function to check if the crate is already published
is_already_published() {
    local crate_name=$1
    local crate_version=$2

    # Use the crates.io API to get the latest version of the crate
    local latest_published_version
    latest_published_version=$(curl -s https://crates.io/api/v1/crates/$crate_name | jq -r '.crate.max_version')

    if [ "$latest_published_version" = "$crate_version" ]; then
        return 0
    else
        return 1
    fi
}

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

CRATE_VERSION=$(grep '^version =' Cargo.toml | head -n1 | cut -d '"' -f2)

set -e
CORE_DIRS="src/uucore_procs/ src/uucore/ src/uu/stdbuf/src/libstdbuf/ tests/uutests/ fuzz/uufuzz/"
CORE_COUNT=$(echo $CORE_DIRS | wc -w)
UTIL_COUNT=$(echo $TOTAL_ORDER | wc -w)
TOTAL_COUNT=$((CORE_COUNT + UTIL_COUNT + 1))
CURRENT=0

for dir in $CORE_DIRS; do
    CURRENT=$((CURRENT + 1))
    echo "[$CURRENT/$TOTAL_COUNT] Processing: $dir"
    (
        cd "$dir"
        CRATE_NAME=$(grep '^name =' "Cargo.toml" | head -n1 | cut -d '"' -f2)
        #shellcheck disable=SC2086
        if ! is_already_published "$CRATE_NAME" "$CRATE_VERSION"; then
            cargo publish $ARG
        else
            echo "Skip: $CRATE_NAME $CRATE_VERSION already published"
        fi
    )
    sleep 2s
done

for p in $TOTAL_ORDER; do
    CURRENT=$((CURRENT + 1))
    echo "[$CURRENT/$TOTAL_COUNT] Processing: $p"
    (
        cd "src/uu/$p"
        CRATE_NAME=$(grep '^name =' "Cargo.toml" | head -n1 | cut -d '"' -f2)
        #shellcheck disable=SC2086
        if ! is_already_published "$CRATE_NAME" "$CRATE_VERSION"; then
            cargo publish $ARG
        else
            echo "Skip: $CRATE_NAME $CRATE_VERSION already published"
        fi
    )
done

CURRENT=$((CURRENT + 1))
echo "[$CURRENT/$TOTAL_COUNT] Processing: main coreutils crate"
#shellcheck disable=SC2086
cargo publish $ARG
