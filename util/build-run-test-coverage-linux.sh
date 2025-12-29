#!/usr/bin/env bash

# spell-checker:ignore (env/flags) Ccodegen Cinstrument Coverflow Cpanic Zpanic
# spell-checker:ignore PROFDATA PROFRAW coreutil librairies nextest profdata profraw rustlib

# This script will build, run and generate coverage reports for the whole
# testsuite.
# The biggest challenge of this process is managing the overwhelming generation
# of trace files that are generated after EACH SINGLE invocation of a coreutil
# in the testsuite. Moreover, because we run the testsuite against the multicall
# binary, each trace file contains coverage information about the WHOLE
# multicall binary, dependencies included, which results in a 5-6 MB file.
# Running the testsuite easily creates +80 GB of trace files, which is
# unmanageable in a CI environment.
#
# A workaround is to run the testsuite util per util, generate a report per
# util, and remove the trace files. Therefore, we end up with several reports
# that will get uploaded to codecov afterwards. The issue with this
# approach is that the `grcov` call, which is responsible for transforming
# `.profraw` trace files into a `lcov` file, takes a lot of time (~20s), mainly
# because it has to browse all the sources. So calling it for each of the 100
# utils (with --all-features) results in an absurdly long execution time
# (almost an hour).

# TODO: Do not instrument 3rd party librairies to save space and performance

# Exit the script if an unexpected error arise
set -e
# Treat unset variables as errors
set -u
# Print expanded commands to stdout before running them
set -x

ME="${0}"
ME_dir="$(dirname -- "$(readlink -fm -- "${ME}")")"
REPO_main_dir="$(dirname -- "${ME_dir}")"

# Features to enable for the `coreutils` package
FEATURES_OPTION=${FEATURES_OPTION:-"--features=feat_os_unix"}
COVERAGE_DIR=${COVERAGE_DIR:-"${REPO_main_dir}/coverage"}

LLVM_PROFDATA="$(find "$(rustc --print sysroot)" -name llvm-profdata)"

PROFRAW_DIR="${COVERAGE_DIR}/traces"
PROFDATA_DIR="${COVERAGE_DIR}/data"
REPORT_DIR="${COVERAGE_DIR}/report"
REPORT_PATH="${REPORT_DIR}/total.lcov.info"

rm -rf "${PROFRAW_DIR}" && mkdir -p "${PROFRAW_DIR}"
rm -rf "${PROFDATA_DIR}" && mkdir -p "${PROFDATA_DIR}"
rm -rf "${REPORT_DIR}" && mkdir -p "${REPORT_DIR}"

#shellcheck disable=SC2086
UTIL_LIST=$("${ME_dir}"/show-utils.sh ${FEATURES_OPTION})

export CARGO_INCREMENTAL=0
export RUSTFLAGS="-Cinstrument-coverage -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort"
export RUSTDOCFLAGS="-Cpanic=abort"
export RUSTUP_TOOLCHAIN="nightly-gnu"
export LLVM_PROFILE_FILE="${PROFRAW_DIR}/coverage-%4m.profraw"

# Disable expanded command printing for the rest of the program
set +x

run_test_and_aggregate() {
    echo "# Running coverage tests for ${1}"

    # Build and run tests for the UTIL
    cargo nextest run \
        --profile coverage \
        --no-fail-fast \
        --color=always \
        2>&1 \
        ${2} \
    | grep -v 'SKIP'
    # Note: Do not print the skipped tests on the output as there will be many.

    echo "## Tests for (${1}) generated $(du -h -d1 ${PROFRAW_DIR} | cut -f 1) of profraw files"

    # Aggregate all the trace files into a profdata file
    PROFDATA_FILE="${PROFDATA_DIR}/${1}.profdata"
    echo "## Aggregating coverage files under ${PROFDATA_FILE}"
    "${LLVM_PROFDATA}" merge \
        -sparse \
        -o ${PROFDATA_FILE} \
        ${PROFRAW_DIR}/*.profraw \
    || true
    # We don't want an error in `llvm-profdata` to abort the whole program
}

for UTIL in ${UTIL_LIST}; do

    if [ "${UTIL}" = "stty" ]; then
        run_test_and_aggregate \
            "${UTIL}" \
            "-p coreutils -p uu_${UTIL} -E test(/^test_${UTIL}::/) ${FEATURES_OPTION}"
    else
        run_test_and_aggregate \
            "${UTIL}" \
            "-p coreutils -E test(/^test_${UTIL}::/) ${FEATURES_OPTION}"
    fi

    echo "## Clear the trace directory to free up space"
    rm -rf "${PROFRAW_DIR}" && mkdir -p "${PROFRAW_DIR}"
done;

echo "Running coverage tests over uucore"
run_test_and_aggregate "uucore" "-p uucore --all-features"

echo "# Aggregating all the profraw files under ${REPORT_PATH}"
grcov \
    "${PROFDATA_DIR}" \
    --binary-path "${REPO_main_dir}/target/debug/" \
    --output-types lcov \
    --output-path ${REPORT_PATH} \
    --llvm \
    --excl-start "^mod test.*\{" \
    --excl-stop "^\}" \
    --keep-only "${REPO_main_dir}"'/src/*'


# Notify the report file to github
echo "report=${REPORT_PATH}" >> $GITHUB_OUTPUT
