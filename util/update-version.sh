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

FROM="0.12.0"
TO="0.13.0"

MANIFESTS=$(ls -1d Cargo.toml src/uu/*/Cargo.toml src/uu/stdbuf/src/libstdbuf/Cargo.toml src/uucore/Cargo.toml src/uucore_procs/Cargo.toml tests/uutests/Cargo.toml fuzz/uufuzz/Cargo.toml)

# Only two kinds of lines are rewritten, so that third party crates which
# happen to share our version number (md-5, sha1, sha2, sha3, ...) are left
# alone:
#  1) the crate's own version declaration, anchored at the start of the line
#  2) dependencies carrying a "path =" key, which are the in-tree uutils crates

# 1) the "version = "X"" of each [package] (and of [workspace.package])
#shellcheck disable=SC2086
sed -i -E "s|^version = \"$FROM\"$|version = \"$TO\"|" $MANIFESTS

# 2) the in-tree dependencies, keeping the ">=" prefix when there is one
#shellcheck disable=SC2086
sed -i -E "/path *=/ s|(version *= *\"(>=)?)$FROM\"|\1$TO\"|g" $MANIFESTS

# Update Cargo.lock files
cargo update --workspace
cargo update --workspace --manifest-path fuzz/Cargo.toml

# Sanity check: anything left pointing at the old version is either a third
# party crate (fine) or something this script missed (not fine)
echo "Remaining occurrences of $FROM - please review:"
#shellcheck disable=SC2086
grep -n "$FROM" $MANIFESTS || echo "  (none)"
