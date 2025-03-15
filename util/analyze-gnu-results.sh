#!/usr/bin/env bash
# spell-checker:ignore xpass XPASS testsuite
set -e

# As we do two builds (with and without root), we need to do some trivial maths
# to present the merge results
# this script will export the values in the term

if test $# -ne 4; then
    echo "syntax:"
    echo "$0 testsuite.log root-testsuite.log selinux-testsuite.log selinux-root-testsuite.log"
    exit 1
fi

SUITE_LOG_FILE=$1
ROOT_SUITE_LOG_FILE=$2
SELINUX_SUITE_LOG_FILE=$3
SELINUX_ROOT_SUITE_LOG_FILE=$4

if test ! -f "${SUITE_LOG_FILE}"; then
    echo "${SUITE_LOG_FILE} has not been found"
    exit 1
fi

if test ! -f "${ROOT_SUITE_LOG_FILE}"; then
    echo "${ROOT_SUITE_LOG_FILE} has not been found"
    exit 1
fi

if test ! -f "${SELINUX_SUITE_LOG_FILE}"; then
    echo "${SELINUX_SUITE_LOG_FILE} has not been found"
    exit 1
fi

if test ! -f "${SELINUX_ROOT_SUITE_LOG_FILE}"; then
    echo "${SELINUX_ROOT_SUITE_LOG_FILE} has not been found"
    exit 1
fi

function get_total {
    # Total of tests executed
    # They are the normal number of tests as they are skipped in the normal run
    NON_ROOT=$(sed -n "s/.*# TOTAL: \(.*\)/\1/p" "${SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    echo $NON_ROOT
}

function get_pass {
    # This is the sum of the two test suites.
    # In the normal run, they are SKIP
    NON_ROOT=$(sed -n "s/.*# PASS: \(.*\)/\1/p" "${SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    AS_ROOT=$(sed -n "s/.*# PASS: \(.*\)/\1/p" "${ROOT_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    SELINUX_NON_ROOT=$(sed -n "s/.*# PASS: \(.*\)/\1/p" "${SELINUX_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    SELINUX_AS_ROOT=$(sed -n "s/.*# PASS: \(.*\)/\1/p" "${SELINUX_ROOT_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    echo $((NON_ROOT + AS_ROOT))
}

function get_skip {
    # Calculate skips accounting for all test runs
    # Number of tests skip as user - total test as root + skipped as root - total selinux + skipped selinux - total selinux_root + skipped selinux_root
    TOTAL_AS_ROOT=$(sed -n "s/.*# TOTAL: \(.*\)/\1/p" "${ROOT_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    NON_ROOT=$(sed -n "s/.*# SKIP: \(.*\)/\1/p" "${SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    AS_ROOT=$(sed -n "s/.*# SKIP: \(.*\)/\1/p" "${ROOT_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)

    TOTAL_SELINUX=$(sed -n "s/.*# TOTAL: \(.*\)/\1/p" "${SELINUX_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    SELINUX=$(sed -n "s/.*# SKIP: \(.*\)/\1/p" "${SELINUX_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)

    TOTAL_SELINUX_ROOT=$(sed -n "s/.*# TOTAL: \(.*\)/\1/p" "${SELINUX_ROOT_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    SELINUX_ROOT=$(sed -n "s/.*# SKIP: \(.*\)/\1/p" "${SELINUX_ROOT_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)

    ADJUSTED_SKIP=$((NON_ROOT - TOTAL_AS_ROOT + AS_ROOT - TOTAL_SELINUX + SELINUX - TOTAL_SELINUX_ROOT + SELINUX_ROOT))
    if [[ $ADJUSTED_SKIP -lt 0 ]]; then
        ADJUSTED_SKIP=0
    fi
    echo $ADJUSTED_SKIP
}

function get_fail {
    # They used to be SKIP, now they fail (this is a good news)
    NON_ROOT=$(sed -n "s/.*# FAIL: \(.*\)/\1/p" "${SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    AS_ROOT=$(sed -n "s/.*# FAIL: \(.*\)/\1/p" "${ROOT_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    SELINUX_NON_ROOT=$(sed -n "s/.*# FAIL: \(.*\)/\1/p" "${SELINUX_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    SELINUX_AS_ROOT=$(sed -n "s/.*# FAIL: \(.*\)/\1/p" "${SELINUX_ROOT_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    echo $((NON_ROOT + AS_ROOT + SELINUX_NON_ROOT + SELINUX_AS_ROOT))
}

function get_xpass {
    NON_ROOT=$(sed -n "s/.*# XPASS: \(.*\)/\1/p" "${SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    echo $NON_ROOT
}

function get_error {
    # They used to be SKIP, now they error (this is a good news)
    NON_ROOT=$(sed -n "s/.*# ERROR: \(.*\)/\1/p" "${SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    AS_ROOT=$(sed -n "s/.*# ERROR:: \(.*\)/\1/p" "${ROOT_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    SELINUX_NON_ROOT=$(sed -n "s/.*# ERROR: \(.*\)/\1/p" "${SELINUX_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    SELINUX_AS_ROOT=$(sed -n "s/.*# ERROR:: \(.*\)/\1/p" "${SELINUX_ROOT_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    echo $((NON_ROOT + AS_ROOT))
}

# we don't need the return codes indeed, ignore them
# shellcheck disable=SC2155
{
    export TOTAL=$(get_total)
    export PASS=$(get_pass)
    export SKIP=$(get_skip)
    export FAIL=$(get_fail)
    export XPASS=$(get_xpass)
    export ERROR=$(get_error)
}
