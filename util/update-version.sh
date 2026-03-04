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

FROM="0.6.0"
TO="0.7.0"

PROGS="fuzz/uufuzz/Cargo.toml Cargo.toml"

# update the version of all programs
#shellcheck disable=SC2086
sed -i -e "s|version = \"$FROM\"|version = \"$TO\"|" $PROGS

# todo: sync *.lock automatically...
