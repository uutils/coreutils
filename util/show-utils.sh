#!/usr/bin/env bash

set -eo pipefail

cd -- "$(dirname -- "$0")/.."

if test "$*";then
  _args=("$@")
else
  export CARGO_BUILD_TARGET="${CARGO_BUILD_TARGET:-$(rustc --print host-tuple)}"
  case "$CARGO_BUILD_TARGET" in
    *windows*) _args=("--features=windows");;
    *wasip*) _args=("--features=feat_wasm");;
    # unix is default since we don't support too many non-unix
    *) _args=("--features=unix");;
  esac
fi
# refs: <https://forge.rust-lang.org/release/platform-support.html> , <https://docs.rs/platforms/0.2.1/platforms/platform/tier1/index.html>
# default utility list
cargo tree --depth 1 --format "{lib}" "${_args[@]}" --prefix none | sed -n 's/^uu_//p'
