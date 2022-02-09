#!/bin/bash
# `$0 [TEST]`
# run GNU test (or all tests if TEST is missing/null)
# spell-checker:ignore (env/vars) BUILDDIR GNULIB SUBDIRS

ME_dir="$(dirname -- "$(readlink -fm -- "$0")")"
REPO_main_dir="$(dirname -- "${ME_dir}")"

set -e

### * config (from environment with fallback defaults)

path_UUTILS=${path_UUTILS:-${REPO_main_dir}}
path_GNU=${path_GNU:-${path_UUTILS}/../gnu}
path_GNULIB=${path_GNULIB:-${path_UUTILS}/../gnulib}

###

BUILD_DIR="$(realpath -- "${path_UUTILS}/target/release")"
GNULIB_DIR="$(realpath -- "${path_GNULIB}")"

export BUILD_DIR
export GNULIB_DIR

pushd "$(realpath -- "${path_GNU}")"

export RUST_BACKTRACE=1

if test -n "$1"; then
    # if set, run only the test passed
    export RUN_TEST="TESTS=$1"
fi

#shellcheck disable=SC2086
timeout -sKILL 2h make -j "$(nproc)" check $RUN_TEST SUBDIRS=. RUN_EXPENSIVE_TESTS=yes RUN_VERY_EXPENSIVE_TESTS=yes VERBOSE=no || : # Kill after 4 hours in case something gets stuck in make
