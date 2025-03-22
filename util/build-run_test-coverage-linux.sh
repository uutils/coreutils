#!/usr/bin/env bash

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

set -ux

FEATURES_OPTION="--features feat_os_unix"

ME="${0}"
ME_dir="$(dirname -- "$(readlink -fm -- "${ME}")")"
REPO_main_dir="$(dirname -- "${ME_dir}")"

LLVM_BIN_PATH="$(rustc --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/bin"
LLVM_PROFDATA="${LLVM_BIN_PATH}/llvm-profdata"

PROFRAW_DIR="${REPO_main_dir}/coverage/traces"
PROFDATA_DIR="${REPO_main_dir}/coverage/data"
REPORT_DIR="${REPO_main_dir}/coverage/report"

rm -rf "${PROFRAW_DIR}" && mkdir -p "${PROFRAW_DIR}"
rm -rf "${PROFDATA_DIR}" && mkdir -p "${PROFDATA_DIR}"
rm -rf "${REPORT_DIR}" && mkdir -p "${REPORT_DIR}"

#shellcheck disable=SC2086
UTIL_LIST=$("${ME_dir}"/show-utils.sh ${FEATURES_OPTION})


export CARGO_INCREMENTAL=0
export RUSTFLAGS="-Cinstrument-coverage -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort"
export RUSTDOCFLAGS="-Cpanic=abort"
export RUSTUP_TOOLCHAIN="nightly-gnu"
export LLVM_PROFILE_FILE="${PROFRAW_DIR}/coverage-%m-%p.profraw"

for UTIL in ${UTIL_LIST}; do
    # Build and run tests for the UTIL
    cargo nextest run \
        --profile coverage \
        --no-fail-fast \
        ${FEATURES_OPTION} \
        -p coreutils \
        -E "test(test_${UTIL})" # Filter to only run tests against the selected util.

    echo -e "${UTIL} trace generation: $(du -h -d1 ${PROFRAW_DIR})" >> "${REPORT_DIR}/usage"

    # Aggregate all the profraw files into a profdata file
    "${LLVM_PROFDATA}" merge \
        -sparse \
        -o "${PROFDATA_DIR}/${UTIL}.profdata" \
        ${PROFRAW_DIR}/*.profraw

    # Clear the trace directory to free up space
    rm -rf "${PROFRAW_DIR}" && mkdir -p "${PROFRAW_DIR}"
done;

# Aggregate all the profraw files into a lcov file
grcov \
    "${PROFDATA_DIR}" \
    --binary-path "${REPO_main_dir}/target/debug/coreutils" \
    --output-types lcov \
    --output-path "${REPORT_DIR}/${UTIL}.lcov.info" \
    --llvm \
    --keep-only "${REPO_main_dir}"'/src/*'
