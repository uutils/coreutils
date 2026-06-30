#!/usr/bin/env bash

# spell-checker:ignore (utils) grealpath

# A helper script to allow installing coreutils without GNU make
# todo: reuse this script on GNUmakefile. Does it break some system potentially missing coreutils e.g. Gentoo?
# Use GNU version for realpath on *BSD
REALPATH=$(command -v grealpath||command -v realpath)

ME="${0}"
ME_dir="$(dirname -- "${ME}")"
ME_parent_dir="$(dirname -- "${ME_dir}")"
ME_parent_dir_abs="$("${REALPATH}" -mP -- "${ME_parent_dir}" || "${REALPATH}" -- "${ME_parent_dir}")"
cd "${ME_parent_dir_abs}" || exit 1

# https://doc.rust-lang.org/beta/rustc/platform-support.html
: ${CARGO_BUILD_TARGET:=$(rustc --print host-tuple)}
case "$CARGO_BUILD_TARGET" in
    *windows*)
        FEATURES="windows"
        ;;
    *wasip*)
        FEATURES="feat_wasm"
        ;;
    *)
        FEATURES="feat_os_unix"
        ;;
esac

: ${UTILS:=$(cargo tree --depth 1 --features ${FEATURES} --format "{p}" --prefix none | sed -E -n 's/^uu_([^ ]+).*/\1/p')}

# avoid grep dependency for the sake
for util in $UTILS; do
    case " $SKIP_UTILS " in
        *" $util "*) continue ;;
        *) printf "${PROG_PREFIX}%s " "$util" ;;
    esac
done
echo
