#!/bin/bash
# This is a stupid helper. I will mass replace all versions (including other crates)
# So, it should be triple-checked


FROM="0.0.3"
TO="0.0.4"

UUCORE_FROM="0.0.6"
UUCORE_TO="0.0.7"

PROGS=$(ls -1d src/uu/*/Cargo.toml src/uu/stdbuf/src/libstdbuf/Cargo.toml Cargo.toml)

# update the version of all programs
sed -i -e "s|version = \"$FROM\"|version = \"$TO\"|" $PROGS

# Update the stbuff stuff
sed -i -e "s|libstdbuf = { version=\"$FROM\"|libstdbuf = { version=\"$TO\"|" src/uu/stdbuf/Cargo.toml
sed -i -e "s|= { optional=true, version=\"$FROM\", package=\"uu_|= { optional=true, version=\"$TO\", package=\"uu_|g" Cargo.toml

# Update uucore itself
sed -i -e "s|version = \"$UUCORE_FROM\"|version = \"$UUCORE_TO\"|" src/uucore/Cargo.toml
# Update crates using uucore
sed -i -e "s|uucore = { version=\">=$UUCORE_FROM\",|uucore = { version=\">=$UUCORE_TO\",|" $PROGS


