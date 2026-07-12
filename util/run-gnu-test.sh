#!/usr/bin/env bash
# `run-gnu-test.bash [TEST]`
# run GNU test (or all tests if TEST is missing/null)

# spell-checker:ignore (env/vars) GNULIB SRCDIR SUBDIRS OSTYPE MAKEFLAGS; (utils) shellcheck greadlink ginstall

# ref: [How the GNU coreutils are tested](https://www.pixelbeat.org/docs/coreutils-testing.html) @@ <https://archive.is/p2ITW>
# * note: to run a single test => `make check TESTS=PATH/TO/TEST/SCRIPT SUBDIRS=. VERBOSE=yes`

# Use GNU make, readlink on *BSD
MAKE=$(command -v gmake||command -v make)
READLINK=$(command -v greadlink||command -v readlink) # Use our readlink to remove a dependency

ME_dir="$(dirname -- "$("${READLINK}" -fm -- "$0")")"
REPO_main_dir="$(dirname -- "${ME_dir}")"

echo "ME_dir='${ME_dir}'"
echo "REPO_main_dir='${REPO_main_dir}'"

set -e

### * config (from environment with fallback defaults); note: GNU and GNULIB are expected to be sibling repo directories

path_UUTILS=${path_UUTILS:-${REPO_main_dir}}
path_GNU="$("${READLINK}" -fm -- "${path_GNU:-${path_UUTILS}/../gnu}")"

echo "path_UUTILS='${path_UUTILS}'"
echo "path_GNU='${path_GNU}'"

# Use GNU nproc for *BSD
NPROC=$(command -v ${path_GNU}/src/nproc||command -v nproc)
MAKEFLAGS="${MAKEFLAGS} -j ${NPROC}"
export MAKEFLAGS
###

cd "${path_GNU}" && echo "[ pwd:'${PWD}' ]"

# --- Self-heal: make the shared GNU harness point at THIS checkout ----------
# ${path_GNU} is shared between sibling uutils checkouts; whichever one last ran
# build-gnu.sh leaves its own target dir patched into the Makefile/local.mk
# PATH, so the harness can silently run a stale/other build. Re-point the PATH
# (and re-create the multicall hardlinks) here so that running this script is
# enough to test the current tree - no full rebuild just because another
# checkout stole the PATH.
if [ -n "${CARGO_TARGET_DIR}" ]; then
    UU_TARGET_BASE="${CARGO_TARGET_DIR}"
else
    UU_TARGET_BASE="${path_UUTILS}/target"
fi
# Locate the multicall binary. Honor PROFILE when set; otherwise probe the usual
# profiles (build-gnu.sh defaults to debug, CI uses release-small) and fall back
# to whatever 'target/*/coreutils' actually exists so the harness adapts to the
# build it is given.
UU_BUILD_DIR=""
for p in ${PROFILE:+"${PROFILE}"} debug release release-small; do
    if [ -x "${UU_TARGET_BASE}/${p}/coreutils" ]; then
        UU_BUILD_DIR="${UU_TARGET_BASE}/${p}"
        break
    fi
done
if [ -z "${UU_BUILD_DIR}" ]; then
    for candidate in "${UU_TARGET_BASE}"/*/coreutils; do
        if [ -x "${candidate}" ]; then UU_BUILD_DIR="$(dirname -- "${candidate}")"; break; fi
    done
fi
if [ -z "${UU_BUILD_DIR}" ]; then
    # No build to re-point at; leave whatever build-gnu.sh already patched in place.
    echo "warning: no coreutils binary under '${UU_TARGET_BASE}'; skipping re-point." >&2
else
    echo "Re-pointing GNU harness at '${UU_BUILD_DIR}'"
    for binary in $("${UU_BUILD_DIR}/coreutils" --list); do
        tgt="${UU_BUILD_DIR}/${binary}"
        [ "${tgt}" -ef "${UU_BUILD_DIR}/coreutils" ] || ln -f "${UU_BUILD_DIR}/coreutils" "${tgt}"
    done
    # The GNU tests invoke `ginstall` rather than `install`.
    [ -e "${UU_BUILD_DIR}/ginstall" ] || [ ! -e "${UU_BUILD_DIR}/install" ] || ln -f "${UU_BUILD_DIR}/install" "${UU_BUILD_DIR}/ginstall"
    uu_path_sed="s/^[[:blank:]]*PATH=.*/  PATH='${UU_BUILD_DIR//\//\\/}\$(PATH_SEPARATOR)'\"\$\$PATH\" \\\/"
    for f in Makefile tests/local.mk; do
        if [ -f "${f}" ]; then sed -i "${uu_path_sed}" "${f}"; fi
    done
    # Keep Makefile.in newer than the local.mk we just edited (and Makefile newer
    # than Makefile.in) so make won't re-run automake/config.status - which would
    # both undo the patch and choke on GNU's factor-test continuation lines that
    # build-gnu.sh strips (automake 1.18: "blank line following trailing backslash").
    for f in Makefile.in Makefile; do
        if [ -f "${f}" ]; then touch "${f}"; fi
    done
fi
# ---------------------------------------------------------------------------

export RUST_BACKTRACE=1

# Determine if we have SELinux tests
has_selinux_tests=false
if test $# -ge 1; then
    for t in "$@"; do
        if [[ "$t" == *"selinux"* ]]; then
                has_selinux_tests=true
                break
        fi
    done
fi

if [[ "$1" == "run-tty" ]]; then
    # Handle TTY tests - dynamically find tests requiring TTY and run each individually
    shift
    TTY_TESTS=$(grep -r "require_controlling_input_terminal" tests --include="*.sh" --include="*.pl" -l 2>/dev/null)
    echo "Running TTY tests individually:"
    # If a test fails, it can break the implementation of the other tty tests. By running them separately this stops the different tests from being able to break each other
    for test in $TTY_TESTS; do
        echo "  Running: $test"
        script -qec "timeout -sKILL 5m '${MAKE}' check TESTS='$test' SUBDIRS=. RUN_EXPENSIVE_TESTS=yes VERBOSE=no gl_public_submodule_commit='' srcdir='${path_GNU}'" /dev/null || :
    done
    exit 0
elif [[ "$1" == "run-root" && "$has_selinux_tests" == true ]]; then
    # Handle SELinux root tests separately
    shift
    if test -n "$CI"; then
        echo "Running SELinux tests as root"
        # Don't use check-root here as the upstream root tests is hardcoded
        sudo "${MAKE}" check TESTS="$*" SUBDIRS=. RUN_EXPENSIVE_TESTS=yes RUN_VERY_EXPENSIVE_TESTS=yes VERBOSE=no gl_public_submodule_commit="" srcdir="${path_GNU}" TEST_SUITE_LOG="tests/test-suite-root.log" || :
    fi
    exit 0
elif test "$1" != "run-root" && test "$1" != "run-tty"; then
    if test $# -ge 1; then
        # if set, run only the tests passed
        SPECIFIC_TESTS=""
        for t in "$@"; do

            # Construct the full path
            full_path="$path_GNU/$t"

            # Check if the file exists with .sh, .pl extension or without any extension in the $path_GNU directory
            if [ -f "$full_path" ] || [ -f "$full_path.sh" ] || [ -f "$full_path.pl" ]; then
                SPECIFIC_TESTS="$SPECIFIC_TESTS $t"
            else
                echo "Error: Test file $full_path, $full_path.sh, or $full_path.pl does not exist!"
                exit 1
            fi
        done
        # trim it
        SPECIFIC_TESTS=$(echo "$SPECIFIC_TESTS" | xargs)
        echo "Running specific tests: $SPECIFIC_TESTS"
    fi
fi

# * timeout used to kill occasionally errant/"stuck" processes (note: 'release' testing takes ~1 hour; 'debug' testing takes ~2.5 hours)
# * `gl_public_submodule_commit=` disables testing for use of a "public" gnulib commit (which will fail when using shallow gnulib checkouts)
# * `srcdir=..` specifies the GNU source directory for tests (fixing failing/confused 'tests/factor/tNN.sh' tests and causing no harm to other tests)
#shellcheck disable=SC2086

if test "$1" != "run-root" && test "$1" != "run-tty"; then
    # run the regular tests
    if test $# -ge 1; then
        timeout -sKILL 4h "${MAKE}" check TESTS="$SPECIFIC_TESTS" SUBDIRS=. RUN_EXPENSIVE_TESTS=yes RUN_VERY_EXPENSIVE_TESTS=yes VERBOSE=no gl_public_submodule_commit="" srcdir="${path_GNU}" || : # Kill after 4 hours in case something gets stuck in make
    else
        timeout -sKILL 4h "${MAKE}" check SUBDIRS=. RUN_EXPENSIVE_TESTS=yes RUN_VERY_EXPENSIVE_TESTS=yes VERBOSE=no gl_public_submodule_commit="" srcdir="${path_GNU}" || : # Kill after 4 hours in case something gets stuck in make
    fi
else
    # in case we would like to run tests requiring root
    if test -z "$1" -o "$1" == "run-root"; then
        if test -n "$CI"; then
            if test $# -ge 2; then
                echo "Running check-root to run only root tests"
                sudo "${MAKE}" check-root TESTS="$2" SUBDIRS=. RUN_EXPENSIVE_TESTS=yes RUN_VERY_EXPENSIVE_TESTS=yes VERBOSE=no gl_public_submodule_commit="" srcdir="${path_GNU}" TEST_SUITE_LOG="tests/test-suite-root.log" || :
            else
                echo "Running check-root to run only root tests"
                sudo "${MAKE}" check-root SUBDIRS=. RUN_EXPENSIVE_TESTS=yes RUN_VERY_EXPENSIVE_TESTS=yes VERBOSE=no gl_public_submodule_commit="" srcdir="${path_GNU}" TEST_SUITE_LOG="tests/test-suite-root.log" || :
            fi
        fi
    fi
fi
