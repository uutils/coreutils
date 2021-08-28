#!/bin/bash
# spell-checker:ignore (env/vars) BUILDDIR GNULIB SUBDIRS
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
set -e
BUILDDIR="${PWD}/uutils/target/release"
GNULIB_DIR="${PWD}/gnulib"
pushd gnu

export RUST_BACKTRACE=1

if test -n "$1"; then
    # if set, run only the test passed
    export RUN_TEST="TESTS=$1"
fi

timeout -sKILL 2h make -j "$(nproc)" check $RUN_TEST SUBDIRS=. RUN_EXPENSIVE_TESTS=yes RUN_VERY_EXPENSIVE_TESTS=yes VERBOSE=no || : # Kill after 4 hours in case something gets stuck in make
