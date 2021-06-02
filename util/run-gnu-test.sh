#!/bin/bash
# spell-checker:ignore (env/vars) BUILDDIR GNULIB SUBDIRS
set -e
BUILDDIR="${PWD}/uutils/target/release"
GNULIB_DIR="${PWD}/gnulib"
pushd gnu

export RUST_BACKTRACE=1
timeout -sKILL 2h make -j "$(nproc)" check SUBDIRS=. RUN_EXPENSIVE_TESTS=yes RUN_VERY_EXPENSIVE_TESTS=yes VERBOSE=no || : # Kill after 4 hours in case something gets stuck in make
