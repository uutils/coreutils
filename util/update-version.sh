#!/bin/sh
# spell-checker:ignore uuhelp

# This is a stupid helper. I will mass replace all versions (including other crates)
# So, it should be triple-checked

# How to ship a new release:
# 1) update this script
# 2) run it: sh util/update-version.sh
# 3) Do a spot check with "git diff"
# 4) cargo test --release --features unix
# 5) git commit -m "New release" (make sure it includes Cargo.lock)
# 6) Run util/publish.sh in dry mode (it will fail as packages needs more recent version of uucore)
# 7) Run util/publish.sh --do-it
# 8) In some cases, you might have to fix dependencies and run import
# 9) Tag the release - "git tag 0.0.X && git push --tags"
# 10) Create the release on github https://github.com/uutils/coreutils/releases/new
# 11) Make sure we have good release notes

FROM="0.0.28"
TO="0.0.29"

PROGS=$(ls -1d src/uu/*/Cargo.toml src/uu/stdbuf/src/libstdbuf/Cargo.toml src/uucore/Cargo.toml tests/uutests/Cargo.toml Cargo.toml)

# update the version of all programs
#shellcheck disable=SC2086
sed -i -e "s|version = \"$FROM\"|version = \"$TO\"|" $PROGS

# Update uucore_procs
sed -i -e "s|version = \"$FROM\"|version = \"$TO\"|" src/uucore_procs/Cargo.toml

# Update uuhelp_parser
sed -i -e "s|version = \"$FROM\"|version = \"$TO\"|" src/uuhelp_parser/Cargo.toml

# Update the stdbuf stuff
sed -i -e "s|libstdbuf = { version=\"$FROM\"|libstdbuf = { version=\"$TO\"|" src/uu/stdbuf/Cargo.toml
sed -i -e "s|= { optional=true, version=\"$FROM\", package=\"uu_|= { optional=true, version=\"$TO\", package=\"uu_|g" Cargo.toml

# Update the base32 dependency for basenc and base64
sed -i -e "s|uu_base32 = { version=\">=$FROM\"|uu_base32 = { version=\">=$TO\"|" src/uu/base64/Cargo.toml src/uu/basenc/Cargo.toml

# Update the ls dependency for dir and vdir
sed -i -e "s|uu_ls = { version = \">=$FROM\"|uu_ls = { version = \">=$TO\"|" src/uu/dir/Cargo.toml src/uu/vdir/Cargo.toml

# Update uucore itself
sed -i -e "s|version = \"$FROM\"|version = \"$TO\"|" src/uucore/Cargo.toml
# Update crates using uucore
#shellcheck disable=SC2086
sed -i -e "s|uucore = { version=\">=$FROM\",|uucore = { version=\">=$TO\",|" $PROGS
# Update crates using uucore_procs
#shellcheck disable=SC2086
sed -i -e "s|uucore_procs = { version=\">=$FROM\",|uucore_procs = { version=\">=$TO\",|" $PROGS
