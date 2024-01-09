#!/usr/bin/env bash
# spell-checker:ignore xpass XPASS testsuite
set -e

# As we do two builds (with and without root), we need to do some trivial maths
# to present the merge results
# this script will export the values in the term

if test $# -ne 2; then
    echo "syntax:"
    echo "$0 testsuite.log root-testsuite.log"
fi

SUITE_LOG_FILE=$1
ROOT_SUITE_LOG_FILE=$2

if test ! -f "${SUITE_LOG_FILE}"; then
    echo "${SUITE_LOG_FILE} has not been found"
    exit 1
fi
if test ! -f "${ROOT_SUITE_LOG_FILE}"; then
    echo "${ROOT_SUITE_LOG_FILE} has not been found"
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
    echo $((NON_ROOT + AS_ROOT))
}

function get_skip {
    # As some of the tests executed as root as still SKIP (ex: selinux), we
    # need to some maths:
    # Number of tests skip as user - total test as root + skipped as root
    TOTAL_AS_ROOT=$(sed -n "s/.*# TOTAL: \(.*\)/\1/p" "${ROOT_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    NON_ROOT=$(sed -n "s/.*# SKIP: \(.*\)/\1/p" "${SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    AS_ROOT=$(sed -n "s/.*# SKIP: \(.*\)/\1/p" "${ROOT_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    echo $((NON_ROOT - TOTAL_AS_ROOT + AS_ROOT))
}

function get_fail {
    # They used to be SKIP, now they fail (this is a good news)
    NON_ROOT=$(sed -n "s/.*# FAIL: \(.*\)/\1/p" "${SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    AS_ROOT=$(sed -n "s/.*# FAIL: \(.*\)/\1/p" "${ROOT_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    echo $((NON_ROOT + AS_ROOT))
}

function get_xpass {
    NON_ROOT=$(sed -n "s/.*# XPASS: \(.*\)/\1/p" "${SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    echo $NON_ROOT
}

function get_error {
    # They used to be SKIP, now they error (this is a good news)
    NON_ROOT=$(sed -n "s/.*# ERROR: \(.*\)/\1/p" "${SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
    AS_ROOT=$(sed -n "s/.*# ERROR:: \(.*\)/\1/p" "${ROOT_SUITE_LOG_FILE}" | tr -d '\r' | head -n1)
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
