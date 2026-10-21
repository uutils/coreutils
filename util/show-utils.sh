#!/bin/sh

set -e

CDPATH='' cd "$(dirname "$0")/.."

if [ "$#" -gt 0 ]; then
    :
else
    CARGO_BUILD_TARGET=${CARGO_BUILD_TARGET:-$(rustc --print host-tuple)}
    case "$CARGO_BUILD_TARGET" in
        *windows*)
            set -- --features=windows
            ;;
        *wasip*)
            set -- --features=feat_wasm
            ;;
        *android*)
            set -- --features=feat_os_unix_android,feat_require_selinux
            ;;
        *linux*)
            set -- --features=unix,feat_require_selinux
            ;;
        *)
            set -- --features=unix
            ;;
    esac
fi

cargo tree --depth 1 --format '{lib}' "$@" --prefix none | sed -n 's/^uu_//p'
