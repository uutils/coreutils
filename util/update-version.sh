#!/bin/sh
# This is a stupid helper. I will mass replace all versions (including other crates)
# So, it should be triple-checked

# How to ship a new release:
# 1) update this script
# 2) run it: sh util/update-version.sh
# 3) Do a spot check with "git diff"
# 4) cargo test --release --features unix
# 5) Run util/publish.sh in dry mode (it will fail as packages needs more recent version of uucore)
# 6) Run util/publish.sh --do-it
# 7) In some cases, you might have to fix dependencies and run import

FROM="0.0.12"
TO="0.0.13"

UUCORE_PROCS_FROM="0.0.12"
UUCORE_PROCS_TO="0.0.13"

UUCORE_FROM="0.0.12"
UUCORE_TO="0.0.13"

PROGS=$(ls -1d src/uu/*/Cargo.toml src/uu/stdbuf/src/libstdbuf/Cargo.toml Cargo.toml src/uu/base64/Cargo.toml)

# update the version of all programs
#shellcheck disable=SC2086
sed -i -e "s|version = \"$FROM\"|version = \"$TO\"|" $PROGS

# Update uucore_procs
sed -i -e "s|version = \"$UUCORE_PROCS_FROM\"|version = \"$UUCORE_PROCS_TO\"|" src/uucore_procs/Cargo.toml

# Update the stdbuf stuff
sed -i -e "s|libstdbuf = { version=\"$FROM\"|libstdbuf = { version=\"$TO\"|" src/uu/stdbuf/Cargo.toml
sed -i -e "s|= { optional=true, version=\"$FROM\", package=\"uu_|= { optional=true, version=\"$TO\", package=\"uu_|g" Cargo.toml

# Update uucore itself
sed -i -e "s|version = \"$UUCORE_FROM\"|version = \"$UUCORE_TO\"|" src/uucore/Cargo.toml
# Update crates using uucore
#shellcheck disable=SC2086
sed -i -e "s|uucore = { version=\">=$UUCORE_FROM\",|uucore = { version=\">=$UUCORE_TO\",|" $PROGS
# Update crates using uucore_procs
#shellcheck disable=SC2086
sed -i -e "s|uucore_procs = { version=\">=$UUCORE_PROCS_FROM\",|uucore_procs = { version=\">=$UUCORE_PROCS_TO\",|" $PROGS
