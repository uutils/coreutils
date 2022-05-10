#!/bin/sh
# `run-gnu-test.bash [TEST]`
# run GNU test (or all tests if TEST is missing/null)
#
# UU_MAKE_PROFILE == 'debug' | 'release' ## build profile used for *uutils* build; may be supplied by caller, defaults to 'release'

# spell-checker:ignore (env/vars) GNULIB SRCDIR SUBDIRS ; (utils) shellcheck

# ref: [How the GNU coreutils are tested](https://www.pixelbeat.org/docs/coreutils-testing.html) @@ <https://archive.is/p2ITW>
# * note: to run a single test => `make check TESTS=PATH/TO/TEST/SCRIPT SUBDIRS=. VERBOSE=yes`

ME_dir="$(dirname -- "$(readlink -fm -- "$0")")"
REPO_main_dir="$(dirname -- "${ME_dir}")"

echo "ME_dir='${ME_dir}'"
echo "REPO_main_dir='${REPO_main_dir}'"

set -e

### * config (from environment with fallback defaults); note: GNU and GNULIB are expected to be sibling repo directories

path_UUTILS=${path_UUTILS:-${REPO_main_dir}}
path_GNU="$(readlink -fm -- "${path_GNU:-${path_UUTILS}/../gnu}")"

echo "path_UUTILS='${path_UUTILS}'"
echo "path_GNU='${path_GNU}'"

###

cd "${path_GNU}" && echo "[ pwd:'${PWD}' ]"

export RUST_BACKTRACE=1

if test -n "$1"; then
    # if set, run only the test passed
    export RUN_TEST="TESTS=$1"
fi

# * timeout used to kill occasionally errant/"stuck" processes (note: 'release' testing takes ~1 hour; 'debug' testing takes ~2.5 hours)
# * `gl_public_submodule_commit=` disables testing for use of a "public" gnulib commit (which will fail when using shallow gnulib checkouts)
# * `srcdir=..` specifies the GNU source directory for tests (fixing failing/confused 'tests/factor/tNN.sh' tests and causing no harm to other tests)
#shellcheck disable=SC2086
timeout -sKILL 4h make -j "$(nproc)" check ${RUN_TEST} SUBDIRS=. RUN_EXPENSIVE_TESTS=yes RUN_VERY_EXPENSIVE_TESTS=yes VERBOSE=no gl_public_submodule_commit="" srcdir="${path_GNU}" || : # Kill after 4 hours in case something gets stuck in make

if test -z "$1" && test -n "$CI"; then
    sudo make -j "$(nproc)" check-root SUBDIRS=. RUN_EXPENSIVE_TESTS=yes RUN_VERY_EXPENSIVE_TESTS=yes VERBOSE=no gl_public_submodule_commit="" srcdir="${path_GNU}" TEST_SUITE_LOG="tests/test-suite-root.log" || :
fi
