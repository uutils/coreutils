#!/usr/bin/env bash

set -eo pipefail

cd -- "$(dirname -- "$0")/.."

if test "$*";then
  _args=("$@")
else
  CARGO_BUILD_TARGET="${CARGO_BUILD_TARGET:-$(rustc --print host-tuple)}"
  case "$CARGO_BUILD_TARGET" in
    *windows*) _args=("--features=windows");;
    *wasip*) _args=("--features=feat_wasm");;
    *android*) _args=("--features=feat_os_unix_android,feat_require_selinux");;
    *linux*) _args=("--features=unix,feat_require_selinux");;
    # unix is default since we don't support too many non-unix
    *) _args=("--features=unix");;
  esac
fi

cargo tree --depth 1 --format "{lib}" "${_args[@]}" --prefix none | sed -n 's/^uu_//p'
