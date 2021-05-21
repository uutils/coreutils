#!/bin/bash
set -e
BUILDDIR="${PWD}/uutils/target/release"
GNULIB_DIR="${PWD}/gnulib"
pushd gnu

timeout -sKILL 2h make -j "$(nproc)" check SUBDIRS=. RUN_EXPENSIVE_TESTS=no RUN_VERY_EXPENSIVE_TESTS=no VERBOSE=no || : # Kill after 4 hours in case something gets stuck in make
